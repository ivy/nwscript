//! `cargo xtask grammar-census`: count every lexical and syntactic form the
//! vanilla scripts use, so each claim in `docs/grammar.md` is evidenced. The
//! output holds keys, numbers and file names only, never source text.

mod lex;
mod parse;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str = "\
cargo xtask grammar-census: count the grammar's productions in the corpus

USAGE:
  cargo xtask grammar-census [--corpus DIR] [--check FILE] [--doc FILE]
  cargo xtask grammar-census [--corpus DIR] --where KEY

Reads DIR/nss/*.nss (nwscript.nss among them). --corpus defaults to
$NWSCRIPT_CORPUS, then ~/.cache/nwscript/corpus/k1.

Without --check, prints the census. --check FILE compares it to FILE (the
committed docs/grammar-census.txt) and fails on any difference. --doc FILE
checks that every census key is cited in FILE as `key` (N in M) or `key` (0)
with the census's numbers, and that every corpus file named in a citation
uses the construct. --where KEY prints FILE:LINE for each place KEY was
counted instead.

EXAMPLE:
  cargo xtask grammar-census > docs/grammar-census.txt
  cargo xtask grammar-census --check docs/grammar-census.txt --doc docs/grammar.md
";

/// Keys reported even when no file uses them, so absence is evidenced too.
const DECLARED: &[&str] = &[
    "action_arg.not_a_call",
    "lex.float.digits_dot_digits_exp",
    "lex.float.digits_exp",
    "sem.call_before_declaration",
    "assoc.left.additive.order_matters",
    "assoc.left.multiplicative",
    "assoc.left.multiplicative.order_matters",
    "top.empty_declaration",
    "top.global.multi_declarator",
    "expr.assign.to_member",
    "pp.include.cycle",
    "pp.include.reached_twice",
    "top.function.prototype_repeated",
    "top.function.definition_differs_from_prototype",
    "param.default.in_definition_only",
    "expr.binary.add.string_literal_operand",
    "expr.call.user.undeclared",
    "expr.call.user.arity_mismatch",
    "expr.call.engine.default_used.vector_literal",
    "expr.member.xyz",
    "function.return.struct",
    "lex.comment.line.at_eof",
    "lex.comment.line.contains_block_close",
    "lex.comment.block.contains_line_comment",
    "lex.int.hex.upper_x",
    "lex.int.hex.upper_digits",
    "lex.string.contains_comment_marker",
    "param.type.struct",
    "pp.define.no_value",
    "pp.include.after_declaration",
    "pp.include.trailing_comment",
    "pp.space_after_hash",
    "prec.logor_over_logand",
    "prec.logand_over_bitor",
    "prec.bitor_over_bitxor",
    "prec.bitxor_over_bitand",
    "prec.bitor_over_equality",
    "prec.bitor_over_relational",
    "prec.bitand_over_equality",
    "prec.bitand_over_relational",
    "prec.equality_over_relational",
    "prec.relational_over_shift",
    "prec.shift_over_additive",
    "prec.additive_over_multiplicative",
    "stmt.decl.type.struct",
    "stmt.do_while.body_unbraced",
    "stmt.for.body_unbraced",
    "stmt.if.dangling_else",
    "stmt.switch.default_not_last",
    "stmt.switch.jump_inside_braces",
    "stmt.switch.trailing_label_without_statement",
    "stmt.while.body_unbraced",
    "assoc.left.equality.order_matters",
    "assoc.left.relational.order_matters",
    "assoc.left.shift",
    "decl.const",
    "entry.both",
    "entry.has_params",
    "expr.assign.chained",
    "expr.assign.operand_unparenthesized",
    "expr.assign.target_not_lvalue",
    "expr.assign.mul_assign",
    "expr.assign.mod_assign",
    "expr.assign.bitand_assign",
    "expr.assign.bitor_assign",
    "expr.assign.bitxor_assign",
    "expr.assign.shl_assign",
    "expr.assign.shr_assign",
    "expr.assign.ushr_assign",
    "expr.binary.bitxor",
    "expr.binary.shr",
    "expr.binary.ushr",
    "expr.call.engine.arity_mismatch",
    "expr.conditional",
    "expr.member.chained",
    "expr.member.of_call",
    "expr.postfix.target_not_lvalue",
    "expr.string.adjacent",
    "expr.unary.plus",
    "function.body.empty",
    "lex.comment.block.contains_block_open",
    "lex.comment.block.unterminated",
    "lex.comment.line.backslash_end",
    "lex.ctrl_z",
    "lex.float.digits_dot",
    "lex.float.dot_digits",
    "lex.float.digits_f",
    "lex.float.digits_dot_digits_upper_f",
    "lex.hash_mid_line",
    "lex.int.hex",
    "lex.int.leading_zero",
    "lex.line_end.cr",
    "lex.line_end.lf",
    "lex.non_ascii.code",
    "lex.non_ascii.string",
    "lex.number.followed_by_ident_char",
    "lex.string.escape.n",
    "lex.string.escape.quote",
    "lex.string.escape.backslash",
    "lex.string.unterminated",
    "lex.ws.vt_ff",
    "param.list.void",
    "param.required_after_default",
    "param.unnamed",
    "pp.include.not_quoted",
    "pp.include.target_missing",
    "pp.include.trailing_text",
    "pp.include.with_extension",
    "pp.inside_function",
    "pp.other",
    "stmt.break.outside",
    "stmt.continue",
    "stmt.continue.outside_loop",
    "stmt.for.comma_list",
    "stmt.for.init_declaration",
    "stmt.for.parts.none",
    "stmt.struct_definition",
    "stmt.switch.label_nested",
    "stmt.switch.nested",
    "stmt.switch.statement_before_first_label",
    "stmt.switch.empty",
    "struct.member.multi_declarator",
    "top.function.prototype_after_definition",
    "top.function.redefinition",
    "top.global.init.call",
    "top.global.init.expression",
];

#[derive(Default)]
pub struct Entry {
    pub count: u64,
    pub files: BTreeSet<String>,
    pub in_nwscript: u64,
}

#[derive(Default)]
pub struct Census {
    pub entries: BTreeMap<String, Entry>,
    pub values: BTreeMap<String, (i64, String)>,
    pub errors: Vec<(String, u32, String)>,
    pub includes: Vec<(String, String)>,
    /// With `--where KEY`, the places that key was counted.
    pub trace: Option<String>,
    pub places: Vec<(String, u32)>,
}

impl Census {
    pub fn hit_at(&mut self, key: &str, file: &str, line: u32) {
        if self.trace.as_deref() == Some(key) {
            self.places.push((file.into(), line));
        }
        self.hit(key, file);
    }

    pub fn hit(&mut self, key: &str, file: &str) {
        let e = self.entries.entry(key.to_string()).or_default();
        e.count += 1;
        e.files.insert(file.to_string());
        if file == "nwscript.nss" {
            e.in_nwscript += 1;
        }
    }

    pub fn max(&mut self, key: &str, v: i64, file: &str) {
        let cur = self
            .values
            .entry(key.to_string())
            .or_insert((v, file.into()));
        if v > cur.0 {
            *cur = (v, file.into());
        }
    }

    pub fn error(&mut self, file: &str, line: u32, msg: &str) {
        self.errors.push((file.into(), line, msg.into()));
    }

    pub fn include(&mut self, file: &str, target: &str) {
        self.includes.push((file.into(), target.into()));
    }
}

struct Opts {
    corpus: PathBuf,
    check: Option<PathBuf>,
    doc: Option<PathBuf>,
    trace: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Option<Opts>> {
    let mut o = Opts {
        corpus: crate::corpus::default_out(),
        check: None,
        doc: None,
        trace: None,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{a} needs a value"))
        };
        match a.as_str() {
            "--corpus" => o.corpus = val()?.into(),
            "--check" => o.check = Some(val()?.into()),
            "--doc" => o.doc = Some(val()?.into()),
            "--where" => o.trace = Some(val()?),
            "--help" | "-h" => return Ok(None),
            other => return Err(format!("unknown argument {other}\n\n{USAGE}").into()),
        }
    }
    Ok(Some(o))
}

pub fn main(args: &[String]) -> Result<()> {
    let Some(o) = parse_args(args)? else {
        print!("{USAGE}");
        return Ok(());
    };
    let census = run(&o.corpus, o.trace.clone())?;
    if o.trace.is_some() {
        for (f, l) in &census.places {
            println!("{f}:{l}");
        }
        return Ok(());
    }
    let text = render(&census);
    let mut failed = Vec::new();
    if let Some(p) = &o.check {
        let old = fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        if old.replace("\r\n", "\n") != text {
            failed.push(format!(
                "{} differs from a fresh census; regenerate it with `cargo xtask grammar-census > {}`",
                p.display(),
                p.display()
            ));
        }
    }
    if let Some(p) = &o.doc {
        let doc = fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        failed.extend(check_doc(&census, &doc));
    }
    if o.check.is_none() && o.doc.is_none() {
        print!("{text}");
        return Ok(());
    }
    if failed.is_empty() {
        eprintln!("census matches");
        Ok(())
    } else {
        for f in &failed {
            eprintln!("{f}");
        }
        Err(format!("{} problem(s)", failed.len()).into())
    }
}

fn run(corpus: &std::path::Path, trace: Option<String>) -> Result<Census> {
    let dir = corpus.join("nss");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .map_err(|e| {
            format!(
                "{}: {e}; set NWSCRIPT_CORPUS or run `cargo xtask corpus extract`",
                dir.display()
            )
        })?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".nss"))
        .collect();
    names.sort();

    let nwscript = fs::read(dir.join("nwscript.nss"))?;
    let mut scratch = Census::default();
    let toks = lex::lex(&nwscript, "nwscript.nss", &mut scratch);
    let none = parse::Engine::new();
    let mut p = parse::Parser::new(toks, "nwscript.nss", &mut scratch, &none, &none);
    p.unit();
    let engine = p.protos;

    // Every script's functions, so calls into include files know their defaults.
    let mut user = parse::Engine::new();
    for name in names.iter().filter(|n| *n != "nwscript.nss") {
        let src = fs::read(dir.join(name))?;
        let toks = lex::lex(&src, name, &mut scratch);
        let mut p = parse::Parser::new(toks, name, &mut scratch, &engine, &none);
        p.unit();
        for (k, v) in p.functions {
            user.entry(k).or_insert(v);
        }
    }

    let mut c = Census {
        trace,
        ..Census::default()
    };
    for k in DECLARED {
        c.entries.entry((*k).to_string()).or_default();
    }
    for k in lex::KEYWORDS {
        c.entries.entry(format!("lex.keyword.{k}")).or_default();
    }
    for name in &names {
        let src = fs::read(dir.join(name))?;
        c.hit("file", name);
        let toks = lex::lex(&src, name, &mut c);
        let mut p = parse::Parser::new(toks, name, &mut c, &engine, &user);
        p.unit();
    }
    let lower: BTreeSet<String> = names.iter().map(|n| n.to_ascii_lowercase()).collect();
    let includes = std::mem::take(&mut c.includes);
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (file, target) in &includes {
        let t = target.to_ascii_lowercase();
        let t = if t.ends_with(".nss") { t } else { t + ".nss" };
        graph.entry(file.to_ascii_lowercase()).or_default().push(t);
    }
    for name in &names {
        // Every include reached from this file, counting repeats along all chains.
        let mut seen = BTreeSet::new();
        let mut stack = graph.get(name).cloned().unwrap_or_default();
        let mut twice = false;
        let mut steps = 0;
        while let Some(t) = stack.pop() {
            steps += 1;
            if steps > 10_000 {
                c.hit("pp.include.cycle", name);
                break;
            }
            if !seen.insert(t.clone()) {
                twice = true;
                continue;
            }
            stack.extend(graph.get(&t).cloned().unwrap_or_default());
        }
        if twice {
            c.hit("pp.include.reached_twice", name);
        }
        if name.starts_with("k_inc_") && graph.contains_key(name) {
            c.hit("pp.include.in_include_file", name);
        }
    }
    for (file, target) in includes {
        let t = target.to_ascii_lowercase();
        let t = if t.ends_with(".nss") { t } else { t + ".nss" };
        if !lower.contains(&t) {
            c.hit("pp.include.target_missing", &file);
        }
        c.hit("pp.include.target", &t);
    }
    Ok(c)
}

fn render(c: &Census) -> String {
    let mut s = String::new();
    let files = c.entries.get("file").map_or(0, |e| e.count);
    let _ = writeln!(
        s,
        "# cargo xtask grammar-census over {files} files in nss/ (nwscript.nss among them)"
    );
    let _ = writeln!(
        s,
        "# key, uses, files using it, uses in nwscript.nss, up to three files using it"
    );
    for (k, e) in &c.entries {
        let ex: Vec<&str> = e.files.iter().take(3).map(String::as_str).collect();
        let _ = writeln!(
            s,
            "{k:<52} {:>6} {:>5} {:>5}  {}",
            e.count,
            e.files.len(),
            e.in_nwscript,
            ex.join(" ")
        )
        .map(|_| ());
    }
    let _ = writeln!(s, "# values: key, largest, file");
    for (k, (v, f)) in &c.values {
        let _ = writeln!(s, "{k:<52} {v:>6} {f}");
    }
    let _ = writeln!(s, "# parse errors: file:line, message");
    for (f, l, m) in &c.errors {
        let _ = writeln!(s, "{f}:{l}: {m}");
    }
    s.lines().map(|l| l.trim_end().to_string() + "\n").collect()
}

/// Check `doc`'s citations against the census; returns the problems found.
fn check_doc(c: &Census, doc: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let mut cited = BTreeSet::new();
    // Fenced blocks hold grammar and examples, not citations.
    let mut prose = String::new();
    let mut fenced = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if !fenced {
            prose.push_str(line);
            prose.push('\n');
        }
    }
    let mut rest = prose.as_str();
    while let Some(i) = rest.find('`') {
        let after = &rest[i + 1..];
        let Some(j) = after.find('`') else { break };
        let key = &after[..j];
        let tail = &after[j + 1..];
        rest = tail;
        let Some(e) = c.entries.get(key) else {
            continue;
        };
        let Some(paren) = tail.strip_prefix(" (") else {
            continue;
        };
        let Some(close) = paren.find(')') else {
            continue;
        };
        let inner = &paren[..close];
        cited.insert(key.to_string());
        let (nums, files) = inner.split_once(':').unwrap_or((inner, ""));
        let parse = |s: &str| s.trim().replace(',', "").parse::<u64>().ok();
        let (n, m) = match nums.split_once(" in ") {
            Some((a, b)) => (parse(a), parse(b)),
            None => (parse(nums), Some(0)),
        };
        if n != Some(e.count) || m != Some(e.files.len() as u64) {
            problems.push(format!(
                "`{key}` cited as ({nums}), census has {} in {}",
                e.count,
                e.files.len()
            ));
        }
        for f in files
            .split(|ch: char| ch == ',' || ch.is_whitespace() || ch == '`')
            .filter(|f| f.ends_with(".nss"))
        {
            if !e.files.contains(f) {
                problems.push(format!("`{key}` cites {f}, which does not use it"));
            }
        }
    }
    for k in c.entries.keys() {
        if k != "file" && !cited.contains(k) {
            problems.push(format!("census key `{k}` is not cited"));
        }
    }
    problems
}
