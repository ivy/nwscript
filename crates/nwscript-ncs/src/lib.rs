//! The NCS file: decoding and encoding the bytecode of compiled NWScript.
//!
//! An NCS file is an 8-byte `NCS V1.0` header, a 5-byte program size record
//! (`0x42` and the file's total length, big-endian), and then instructions
//! to the end of the file. Each instruction is an [`Opcode`] byte, a
//! [`Type`] byte, and big-endian [`Operands`] whose [`Layout`] the pair
//! determines. The format is described in Tim Smith's (Torlack's) NCS
//! specification, kept by the xoreos project:
//! <https://github.com/xoreos/xoreos-docs/blob/master/specs/torlack/ncs.html>.
//!
//! [`decode`] reads a file into a [`Program`]; [`encode`] writes it back.
//! Every decoded value is the value as encoded, so `encode(&decode(f, b)?)`
//! equals `b` for every file `decode` accepts. A byte sequence that is not a
//! documented instruction is an error naming its offset, never a guess.
//!
//! # Where errors point
//!
//! An NCS buffer is not source text, but its errors are [`Diagnostic`]s with
//! spans like any other, so the caller registers the bytes in a
//! [`SourceMap`] first and passes the [`FileId`] to [`decode`]; a span's
//! range is the byte range of the offending bytes in the file.
//! [`decode_named`] does both steps for a caller holding a name and bytes.

mod instruction;
mod opcode;

pub use instruction::{Instruction, Operands};
pub use opcode::{Layout, Opcode, Type};

use nwscript_diag::{Diagnostic, FileId, SourceMap, Span, codes};

/// The eight bytes every NCS file starts with.
pub const MAGIC: &[u8; 8] = b"NCS V1.0";
/// The opcode of the program size record at offset 8.
pub const SIZE_RECORD: u8 = 0x42;
/// The offset of the first instruction: the header and the size record.
pub const CODE_START: u32 = 13;

/// A decoded file: its instructions and the file offset of each.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Program {
    instructions: Vec<Instruction>,
    offsets: Vec<u32>,
}

impl Program {
    /// A program of these instructions, laid out from [`CODE_START`].
    pub fn new(instructions: Vec<Instruction>) -> Self {
        let mut offsets = Vec::with_capacity(instructions.len());
        let mut at = CODE_START;
        for i in &instructions {
            offsets.push(at);
            at = at.wrapping_add(i.len() as u32);
        }
        Program {
            instructions,
            offsets,
        }
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// The file offset of each instruction, parallel to [`Self::instructions`].
    pub fn offsets(&self) -> &[u32] {
        &self.offsets
    }

    /// Each instruction with its file offset.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &Instruction)> {
        self.offsets.iter().copied().zip(&self.instructions)
    }

    /// The index of the instruction starting at this file offset.
    pub fn index_at(&self, offset: u32) -> Option<usize> {
        self.offsets.binary_search(&offset).ok()
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// The encoded file's length: what the size record holds.
    pub fn file_len(&self) -> u32 {
        match (self.offsets.last(), self.instructions.last()) {
            (Some(at), Some(i)) => at.wrapping_add(i.len() as u32),
            _ => CODE_START,
        }
    }

    pub fn into_instructions(self) -> Vec<Instruction> {
        self.instructions
    }
}

/// Encode a program as an NCS file.
pub fn encode(program: &Program) -> Vec<u8> {
    let mut out = Vec::with_capacity(program.file_len() as usize);
    out.extend_from_slice(MAGIC);
    out.push(SIZE_RECORD);
    out.extend_from_slice(&[0; 4]);
    for i in program.instructions() {
        i.encode_into(&mut out);
    }
    let len = out.len() as u32;
    out[9..13].copy_from_slice(&len.to_be_bytes());
    out
}

/// Register `bytes` in `sources` under `name` and decode them.
pub fn decode_named(
    name: impl Into<String>,
    bytes: &[u8],
    sources: &mut SourceMap,
) -> Result<Program, Diagnostic> {
    let file = sources.add(name.into(), bytes.to_vec());
    decode(file, bytes)
}

/// Decode an NCS file whose bytes are registered as `file`.
pub fn decode(file: FileId, bytes: &[u8]) -> Result<Program, Diagnostic> {
    let span = |start: usize, end: usize| Span::new(file, start, end.min(bytes.len()));
    let header = CODE_START as usize;

    let magic_len = bytes.len().min(MAGIC.len());
    if bytes[..magic_len] != MAGIC[..magic_len] {
        return Err(Diagnostic::new(&codes::E0101, "not an NCS V1.0 file")
            .primary(span(0, magic_len), "expected `NCS V1.0` here")
            .help("an NCS file starts with the eight bytes `NCS V1.0`"));
    }
    if bytes.len() < header {
        return Err(truncated(
            span(bytes.len(), bytes.len()),
            format!(
                "file ends at offset {} inside the {header}-byte header",
                bytes.len()
            ),
        ));
    }
    if bytes[8] != SIZE_RECORD {
        return Err(Diagnostic::new(
            &codes::E0102,
            format!(
                "expected the program size record (0x42) at offset 8, found 0x{:02X}",
                bytes[8]
            ),
        )
        .primary(span(8, 9), "not 0x42"));
    }
    let declared = u32::from_be_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]);

    let mut instructions = Vec::new();
    let mut offsets = Vec::new();
    let mut at = header;
    while at < bytes.len() {
        let (insn, len) = decode_one(file, bytes, at, declared)?;
        offsets.push(at as u32);
        instructions.push(insn);
        at += len;
    }

    if declared as usize != bytes.len() {
        let len = bytes.len();
        let d = if (declared as usize) > len {
            truncated(
                span(len, len),
                format!(
                    "file is truncated at offset {len}: its size record declares {declared} bytes"
                ),
            )
        } else {
            Diagnostic::new(
                &codes::E0102,
                format!("the size record declares {declared} bytes but the file has {len}"),
            )
            .primary(span(declared as usize, len), "past the declared end")
        };
        return Err(d.secondary(span(9, 13), format!("declares {declared} bytes")));
    }

    Ok(Program {
        instructions,
        offsets,
    })
}

fn truncated(at: Span, message: String) -> Diagnostic {
    Diagnostic::new(&codes::E0105, message)
        .primary(at, "file ends here")
        .help("the file was cut short; extract or copy it again")
}

/// Decode the instruction at `at`, returning it and its length.
fn decode_one(
    file: FileId,
    bytes: &[u8],
    at: usize,
    declared: u32,
) -> Result<(Instruction, usize), Diagnostic> {
    let span = |start: usize, end: usize| Span::new(file, start, end.min(bytes.len()));
    let end = bytes.len();
    let short = |need: usize| {
        truncated(
            span(at, end),
            format!(
                "instruction at offset {at} needs {need} bytes, but the file ends at offset {end}"
            ),
        )
        .note(format!("the file's size record declares {declared} bytes"))
    };

    let byte = bytes[at];
    let Some(opcode) = Opcode::from_byte(byte) else {
        return Err(Diagnostic::new(
            &codes::E0103,
            format!("unknown opcode 0x{byte:02X} at offset {at}"),
        )
        .primary(span(at, at + 1), "not an NCS opcode"));
    };
    if at + 2 > end {
        return Err(short(2));
    }
    let ty = Type(bytes[at + 1]);
    let Some(layout) = opcode.layout(ty) else {
        return Err(Diagnostic::new(
            &codes::E0104,
            format!(
                "{opcode} does not take type 0x{:02X} (at offset {at})",
                ty.0
            ),
        )
        .primary(span(at + 1, at + 2), "unexpected type byte")
        .secondary(span(at, at + 1), format!("{opcode}")));
    };

    let ops = at + 2;
    let need = match layout {
        Layout::String => {
            if ops + 2 > end {
                return Err(short(4));
            }
            4 + usize::from(u16::from_be_bytes([bytes[ops], bytes[ops + 1]]))
        }
        l => 2 + l.fixed_len().unwrap_or(0),
    };
    if at + need > end {
        return Err(short(need));
    }
    let b = &bytes[ops..at + need];
    let i32_at = |i: usize| i32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let u32_at = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let u16_at = |i: usize| u16::from_be_bytes([b[i], b[i + 1]]);
    let operands = match layout {
        Layout::None => Operands::None,
        Layout::Stack => Operands::Stack {
            offset: i32_at(0),
            size: u16_at(4),
        },
        Layout::Int => Operands::Int(i32_at(0)),
        Layout::Float => Operands::Float(f32::from_bits(u32_at(0))),
        Layout::String => Operands::String(b[2..].to_vec()),
        Layout::Object => Operands::Object(i32_at(0)),
        Layout::Action => Operands::Action {
            routine: u16_at(0),
            argc: b[2],
        },
        Layout::Size => Operands::Size(u16_at(0)),
        Layout::Offset => Operands::Offset(i32_at(0)),
        Layout::Jump => Operands::Jump(i32_at(0)),
        Layout::Destruct => Operands::Destruct {
            size: u16_at(0),
            keep_offset: u16_at(2),
            keep_size: u16_at(4),
        },
        Layout::StoreState => Operands::StoreState {
            globals: u32_at(0),
            locals: u32_at(4),
        },
    };
    Ok((Instruction::new(opcode, ty, operands), need))
}
