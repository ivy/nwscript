//! The textual assembly form of NCS: one instruction per line, labels in
//! place of jump offsets, `;` comments. [`print`] writes it and [`parse`]
//! reads it back, so `encode(&parse(print(p))?)` equals `encode(p)` for
//! every program. The syntax is documented in `docs/asm.md`.

use crate::{Instruction, Layout, Opcode, Operands, Program, Type};
use nwscript_diag::{Diagnostic, FileId, Span, codes};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::sync::OnceLock;

/// How to print a program.
#[derive(Default)]
pub struct PrintOptions<'a> {
    /// Prefix each line with its file offset and keep jumps as relative
    /// offsets instead of labels.
    pub raw: bool,
    /// The name of an engine routine, written as a comment on `ACTION` lines.
    pub routine_name: Option<&'a dyn Fn(u16) -> Option<String>>,
}

/// A label for every jump target that starts an instruction or ends the
/// program: `sub_OFFSET` for `JSR` targets, `L_OFFSET` for the rest.
fn labels(p: &Program) -> BTreeMap<u32, String> {
    let mut out: BTreeMap<u32, String> = BTreeMap::new();
    let end = p.file_len();
    for (at, i) in p.iter() {
        if let Some(t) = i.jump_target(at)
            && (p.index_at(t).is_some() || t == end)
        {
            let name = if i.opcode == Opcode::Jsr {
                format!("sub_{t:x}")
            } else {
                format!("L_{t:x}")
            };
            let slot = out.entry(t).or_insert_with(|| name.clone());
            if name.starts_with("sub_") {
                *slot = name;
            }
        }
    }
    out
}

fn write_string(out: &mut String, s: &[u8]) {
    out.push('"');
    for &b in s {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\x{b:02x}");
            }
        }
    }
    out.push('"');
}

fn write_float(out: &mut String, f: f32) {
    if f.is_nan() {
        let _ = write!(out, "bits(0x{:08x})", f.to_bits());
    } else {
        let _ = write!(out, "{f:?}");
    }
}

/// The program as assembly text.
pub fn print(p: &Program, opts: &PrintOptions) -> String {
    let labels = if opts.raw { BTreeMap::new() } else { labels(p) };
    let mut out = String::new();
    let _ = writeln!(out, "; {} instructions, {} bytes", p.len(), p.file_len());
    for (at, i) in p.iter() {
        if let Some(l) = labels.get(&at) {
            let _ = writeln!(out, "{l}:");
        }
        let mut line = String::new();
        if opts.raw {
            let _ = write!(line, "{at:>6}: ");
        } else {
            line.push_str("    ");
        }
        let mnemonic = i.opcode.mnemonic(i.ty);
        line.push_str(&mnemonic);
        let mut ops = String::new();
        match &i.operands {
            Operands::None => {}
            Operands::Stack { offset, size } => {
                let _ = write!(ops, "{offset}, {size}");
            }
            Operands::Int(v) | Operands::Object(v) | Operands::Offset(v) => {
                let _ = write!(ops, "{v}");
            }
            Operands::Float(f) => write_float(&mut ops, *f),
            Operands::String(s) => write_string(&mut ops, s),
            Operands::Action { routine, argc } => {
                let _ = write!(ops, "{routine}, {argc}");
            }
            Operands::Size(v) => {
                let _ = write!(ops, "{v}");
            }
            Operands::Jump(rel) => match i.jump_target(at).and_then(|t| labels.get(&t)) {
                Some(l) => ops.push_str(l),
                None => {
                    let _ = write!(ops, "{rel:+}");
                }
            },
            Operands::Destruct {
                size,
                keep_offset,
                keep_size,
            } => {
                let _ = write!(ops, "{size}, {keep_offset}, {keep_size}");
            }
            Operands::StoreState { globals, locals } => {
                let _ = write!(ops, "{globals}, {locals}");
            }
        }
        if !ops.is_empty() {
            let _ = write!(
                line,
                "{:pad$}{ops}",
                "",
                pad = 13usize.saturating_sub(mnemonic.len()).max(1)
            );
        }
        if let (Operands::Action { routine, .. }, Some(name)) = (&i.operands, opts.routine_name)
            && let Some(n) = name(*routine)
        {
            let _ = write!(
                line,
                "{:pad$}; {n}",
                "",
                pad = 36usize.saturating_sub(line.len()).max(1)
            );
        }
        out.push_str(&line);
        out.push('\n');
    }
    if let Some(l) = labels.get(&p.file_len()) {
        let _ = writeln!(out, "{l}:");
    }
    out
}

/// Every documented mnemonic, sorted.
pub fn all_mnemonics() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = mnemonics().keys().map(String::as_str).collect();
    v.sort_unstable();
    v
}

/// Every documented mnemonic and the opcode and type byte it stands for.
fn mnemonics() -> &'static HashMap<String, (Opcode, Type)> {
    static TABLE: OnceLock<HashMap<String, (Opcode, Type)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = HashMap::new();
        for &op in Opcode::ALL {
            for b in 0..=255u8 {
                if op.accepts(Type(b)) {
                    t.insert(op.mnemonic(Type(b)), (op, Type(b)));
                }
            }
        }
        t
    })
}

/// One operand token: its text and byte range in the line's file.
struct Tok<'a> {
    text: &'a str,
    start: usize,
}

enum Target {
    Label(String, Span),
    Rel(i32),
}

struct Pending {
    ins: Instruction,
    target: Option<Target>,
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

/// Split operands on commas outside quotes, trimming each.
fn split_operands(text: &str, base: usize) -> Vec<Tok<'_>> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let (mut start, mut quoted, mut esc) = (0, false, false);
    for (i, &c) in b.iter().enumerate() {
        if quoted {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                quoted = false;
            }
        } else if c == b'"' {
            quoted = true;
        } else if c == b',' {
            out.push((start, i));
            start = i + 1;
        }
    }
    out.push((start, b.len()));
    out.into_iter()
        .filter_map(|(s, e)| {
            let raw = &text[s..e];
            let lead = raw.len() - raw.trim_start().len();
            let t = raw.trim();
            (!t.is_empty()).then_some(Tok {
                text: t,
                start: base + s + lead,
            })
        })
        .collect()
}

/// The end of the code part of a line: the first `;` outside a string.
fn code_end(line: &[u8]) -> usize {
    let (mut quoted, mut esc) = (false, false);
    for (i, &c) in line.iter().enumerate() {
        if quoted {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                quoted = false;
            }
        } else if c == b'"' {
            quoted = true;
        } else if c == b';' {
            return i;
        }
    }
    line.len()
}

fn parse_int<T: TryFrom<i64>>(t: &str) -> Option<T> {
    let (neg, body) = match t.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let v = match body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        Some(h) => i64::from_str_radix(h, 16).ok()?,
        None => body.parse::<i64>().ok()?,
    };
    T::try_from(if neg { -v } else { v }).ok()
}

fn parse_float(t: &str) -> Option<f32> {
    if let Some(inner) = t.strip_prefix("bits(").and_then(|r| r.strip_suffix(')')) {
        return parse_int::<i64>(inner)
            .and_then(|v| u32::try_from(v).ok())
            .map(f32::from_bits);
    }
    t.parse::<f32>().ok()
}

fn parse_string(t: &str) -> Option<Vec<u8>> {
    let inner = t.strip_prefix('"')?.strip_suffix('"')?;
    let b = inner.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            let e = *b.get(i + 1)?;
            i += 2;
            out.push(match e {
                b'"' => b'"',
                b'\\' => b'\\',
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                b'x' => {
                    let h = inner.get(i..i + 2)?;
                    i += 2;
                    u8::from_str_radix(h, 16).ok()?
                }
                _ => return None,
            });
        } else if b[i] == b'"' {
            return None;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Some(out)
}

/// Read assembly text into a program. Every error is reported, each with
/// the span of the text at fault; the program is returned only when there
/// are none.
pub fn parse(file: FileId, text: &[u8]) -> Result<Program, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();
    // Label name to the index of the instruction it precedes, and where it was defined.
    let mut defined: HashMap<String, (usize, Span)> = HashMap::new();
    let mut line_start = 0;
    for raw_line in text.split_inclusive(|&b| b == b'\n') {
        let base = line_start;
        line_start += raw_line.len();
        let code = &raw_line[..code_end(raw_line)];
        let Ok(code) = std::str::from_utf8(code) else {
            errors.push(
                Diagnostic::new(
                    &codes::E0111,
                    "assembly text must be ASCII; write other bytes in strings as \\xNN",
                )
                .primary(Span::new(file, base, base + code.len()), ""),
            );
            continue;
        };
        let mut rest = code;
        let mut at = base;
        // Skip leading whitespace.
        let skip = |rest: &mut &str, at: &mut usize| {
            let n = rest.len() - rest.trim_start().len();
            *rest = &rest[n..];
            *at += n;
        };
        skip(&mut rest, &mut at);
        // A raw offset column: digits and a colon.
        if let Some(colon) = rest.find(':')
            && !rest[..colon].is_empty()
            && rest[..colon].bytes().all(|c| c.is_ascii_digit())
        {
            rest = &rest[colon + 1..];
            at += colon + 1;
            skip(&mut rest, &mut at);
        }
        // Labels: `name:`.
        while let Some(colon) = rest.find(':') {
            let name = &rest[..colon];
            if name.is_empty()
                || !is_ident_start(name.as_bytes()[0])
                || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
            {
                break;
            }
            let span = Span::new(file, at, at + colon);
            if let Some((_, first)) = defined.get(name) {
                errors.push(
                    Diagnostic::new(&codes::E0113, format!("label `{name}` is defined twice"))
                        .primary(span, "defined again here")
                        .secondary(*first, "first defined here"),
                );
            } else {
                defined.insert(name.to_string(), (pending.len(), span));
            }
            rest = &rest[colon + 1..];
            at += colon + 1;
            skip(&mut rest, &mut at);
        }
        if rest.trim().is_empty() {
            continue;
        }
        let mlen = rest
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(rest.len());
        let mnemonic = &rest[..mlen];
        let mspan = Span::new(file, at, at + mlen);
        let Some(&(opcode, ty)) = mnemonics().get(mnemonic) else {
            errors.push(Diagnostic::new(&codes::E0110, format!("unknown instruction `{mnemonic}`")).primary(mspan, "not an NCS mnemonic").help("mnemonics carry their type suffix, as `CONSTI`, `EQUALFF` or `RSADDE2`; `docs/asm.md` lists them"));
            continue;
        };
        let ops = split_operands(&rest[mlen..], at + mlen);
        let layout = opcode
            .layout(ty)
            .expect("the table holds documented pairs only");
        let want = match layout {
            Layout::None => 0,
            Layout::Int
            | Layout::Float
            | Layout::String
            | Layout::Object
            | Layout::Size
            | Layout::Offset
            | Layout::Jump => 1,
            Layout::Stack | Layout::Action | Layout::StoreState => 2,
            Layout::Destruct => 3,
        };
        if ops.len() != want {
            let span = ops.last().map_or(mspan, |t| {
                Span::new(file, ops[0].start, t.start + t.text.len())
            });
            errors.push(
                Diagnostic::new(
                    &codes::E0111,
                    format!(
                        "`{mnemonic}` takes {want} operand{}, found {}",
                        if want == 1 { "" } else { "s" },
                        ops.len()
                    ),
                )
                .primary(span, ""),
            );
            continue;
        }
        let bad = |t: &Tok, what: &str| {
            Diagnostic::new(&codes::E0111, format!("`{}` is not {what}", t.text)).primary(
                Span::new(file, t.start, t.start + t.text.len()),
                format!("expected {what}"),
            )
        };
        macro_rules! num {
            ($t:expr, $ty:ty, $what:expr) => {
                match parse_int::<$ty>($t.text) {
                    Some(v) => v,
                    None => {
                        errors.push(bad(&$t, $what));
                        continue;
                    }
                }
            };
        }
        let mut target = None;
        let operands = match layout {
            Layout::None => Operands::None,
            Layout::Stack => Operands::Stack {
                offset: num!(ops[0], i32, "a 32-bit offset"),
                size: num!(ops[1], u16, "a 16-bit size"),
            },
            Layout::Int => Operands::Int(num!(ops[0], i32, "a 32-bit integer")),
            Layout::Object => Operands::Object(num!(ops[0], i32, "a 32-bit object id")),
            Layout::Offset => Operands::Offset(num!(ops[0], i32, "a 32-bit offset")),
            Layout::Size => Operands::Size(num!(ops[0], u16, "a 16-bit size")),
            Layout::Float => match parse_float(ops[0].text) {
                Some(f) => Operands::Float(f),
                None => {
                    errors.push(bad(
                        &ops[0],
                        "a float (`1.5`, `-0.0`, `inf` or `bits(0x7fc00000)`)",
                    ));
                    continue;
                }
            },
            Layout::String => match parse_string(ops[0].text) {
                Some(s) if s.len() <= usize::from(u16::MAX) => Operands::String(s),
                _ => {
                    errors.push(bad(&ops[0], "a quoted string of at most 65535 bytes (escapes: \\\" \\\\ \\n \\r \\t \\xNN)"));
                    continue;
                }
            },
            Layout::Action => Operands::Action {
                routine: num!(ops[0], u16, "a routine number"),
                argc: num!(ops[1], u8, "an argument count"),
            },
            Layout::Destruct => Operands::Destruct {
                size: num!(ops[0], u16, "a 16-bit size"),
                keep_offset: num!(ops[1], u16, "a 16-bit offset"),
                keep_size: num!(ops[2], u16, "a 16-bit size"),
            },
            Layout::StoreState => Operands::StoreState {
                globals: num!(ops[0], u32, "a byte count"),
                locals: num!(ops[1], u32, "a byte count"),
            },
            Layout::Jump => {
                let t = &ops[0];
                let span = Span::new(file, t.start, t.start + t.text.len());
                if t.text.starts_with(['+', '-']) {
                    target = Some(Target::Rel(num!(ops[0], i32, "a relative offset")));
                } else if is_ident_start(t.text.as_bytes()[0]) {
                    target = Some(Target::Label(t.text.to_string(), span));
                } else {
                    errors.push(bad(t, "a label or a signed relative offset such as `+8`"));
                    continue;
                }
                Operands::Jump(0)
            }
        };
        pending.push(Pending {
            ins: Instruction::new(opcode, ty, operands),
            target,
        });
    }
    let program = Program::new(pending.iter().map(|p| p.ins.clone()).collect());
    let end = program.file_len();
    let offset_of = |idx: usize| program.offsets().get(idx).copied().unwrap_or(end);
    let mut resolved = Vec::with_capacity(pending.len());
    for (idx, p) in pending.into_iter().enumerate() {
        let mut ins = p.ins;
        match p.target {
            Some(Target::Rel(r)) => ins.operands = Operands::Jump(r),
            Some(Target::Label(name, span)) => match defined.get(&name) {
                Some(&(target_idx, _)) => {
                    ins.operands =
                        Operands::Jump(offset_of(target_idx) as i32 - offset_of(idx) as i32)
                }
                None => errors.push(
                    Diagnostic::new(&codes::E0112, format!("no label `{name}`"))
                        .primary(span, "jumps to an undefined label"),
                ),
            },
            None => {}
        }
        resolved.push(ins);
    }
    if errors.is_empty() {
        Ok(Program::new(resolved))
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode;

    fn prog() -> Program {
        Program::new(vec![
            Instruction::new(Opcode::Jsr, Type::VOID, Operands::Jump(8)),
            Instruction::new(Opcode::Retn, Type::VOID, Operands::None),
            Instruction::new(
                Opcode::Const,
                Type::STRING,
                Operands::String(b"a\"b\\c\x93;d".to_vec()),
            ),
            Instruction::new(
                Opcode::Const,
                Type::FLOAT,
                Operands::Float(f32::from_bits(0x7fc0_0001)),
            ),
            Instruction::new(Opcode::Const, Type::FLOAT, Operands::Float(-0.0)),
            Instruction::new(
                Opcode::Action,
                Type::VOID,
                Operands::Action {
                    routine: 1,
                    argc: 1,
                },
            ),
            Instruction::new(Opcode::Jz, Type::VOID, Operands::Jump(6)),
            Instruction::new(Opcode::Retn, Type::VOID, Operands::None),
        ])
    }

    #[test]
    fn docs_list_every_mnemonic() {
        let doc = include_str!("../../../docs/asm.md");
        let missing: Vec<&str> = all_mnemonics()
            .into_iter()
            .filter(|m| !doc.contains(&format!("`{m}`")))
            .collect();
        assert!(missing.is_empty(), "docs/asm.md lacks {missing:?}");
    }

    #[test]
    fn mnemonics_are_unique() {
        let mut n = 0;
        for &op in Opcode::ALL {
            n += (0..=255u8).filter(|&b| op.accepts(Type(b))).count();
        }
        assert_eq!(n, mnemonics().len());
    }

    #[test]
    fn round_trips_labels_strings_and_odd_floats() {
        let p = prog();
        for raw in [false, true] {
            let text = print(
                &p,
                &PrintOptions {
                    raw,
                    routine_name: None,
                },
            );
            let back =
                parse(FileId(0), text.as_bytes()).unwrap_or_else(|e| panic!("{text}\n{e:?}"));
            assert_eq!(encode(&back), encode(&p), "{text}");
        }
        let text = print(&p, &PrintOptions::default());
        assert!(text.contains("sub_15:"), "{text}");
        assert!(text.contains("JSR          sub_15"), "{text}");
    }

    #[test]
    fn routine_names_are_comments() {
        let name = |r: u16| (r == 1).then(|| "PrintString".to_string());
        let text = print(
            &prog(),
            &PrintOptions {
                raw: false,
                routine_name: Some(&name),
            },
        );
        assert!(text.contains("; PrintString"), "{text}");
        assert!(parse(FileId(0), text.as_bytes()).is_ok());
    }

    #[test]
    fn errors_point_at_the_text() {
        let src = b"    CONSTI 1\n    FROB 2\n    JMP nowhere\n    CONSTI x\nL:\nL:\n";
        let errs = parse(FileId(0), src).unwrap_err();
        let codes: Vec<&str> = errs.iter().map(|d| d.code.id).collect();
        assert_eq!(codes, ["E0110", "E0111", "E0113", "E0112"]);
        let frob = errs[0].primary_span().unwrap();
        assert_eq!(&src[frob.start..frob.end], b"FROB");
    }
}
