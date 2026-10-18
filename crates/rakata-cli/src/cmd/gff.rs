//! `rakata gff show|set|diff`: a GFF's tree or one field of it, fields
//! assigned by path, and two GFFs compared by path, over `rakata-gff`.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{Value as Json, json};

use super::Ctx;
use super::inspect::{self, DiffArgs, LookupArgs, diff_sides, resource_forms};
use crate::Exit;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::gff::{Field, Gff, LocString, Path, Struct, Value};
use rakata::inspect::{Loaded, diagnostic};
use rakata::tlk::codepage;

#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  rakata gff show dan14_juhani.utc --game DIR --module danm14ac --path Tag
  rakata gff set ./p_juhani.utc Tag=juhani2 FirstName=locstring:0=Juhani -o out.utc
  rakata gff diff dan14_juhani.utc dan14_juhani.utc --game-a VANILLA --game-b BUILD -m danm14ac")]
pub struct GffArgs {
    #[command(subcommand)]
    pub action: GffAction,
}

#[derive(Subcommand, Debug)]
pub enum GffAction {
    /// Print a GFF's tree, or the field or struct one path names.
    #[command(after_help = SHOW)]
    Show {
        /// The GFF: NAME.EXT, PATH or ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        /// A field path, as `Tag`, `ClassList[0].Class` or `EntryList[3].RepliesList[0]`. A field of a single value prints the value alone; a struct or list prints its subtree.
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
        #[command(flatten)]
        lookup: LookupArgs,
    },
    /// Assign fields by path and write the result; every other byte the writer can keep, it keeps.
    #[command(after_help = SET)]
    Set {
        /// The GFF: NAME.EXT, PATH or ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        /// One or more PATH=VALUE or PATH=TYPE:VALUE, applied in order.
        #[arg(value_name = "PATH=VALUE", required = true)]
        assignments: Vec<String>,
        /// Where to write the result; it may be the input.
        #[arg(short, long, value_name = "OUT")]
        output: PathBuf,
        #[command(flatten)]
        lookup: LookupArgs,
    },
    /// Every path on which two GFFs differ, with the value on each side.
    #[command(after_help = DIFF)]
    Diff(DiffArgs),
}

const SHOW: &str = concat!(
    "\
The tree prints one field per line, `Label: TYPE = value`, nested structs and
lists indented; strings are quoted, a CExoLocString shows its strref and each
substring as ID=\"text\" by string id. --format json gives the same tree with
every value typed.

",
    resource_forms!(),
    "

Examples:
  rakata gff show dan14_juhani.utc --game DIR --module danm14ac --path Tag
  rakata gff show ./p_juhani.utc --path ClassList[0]
  rakata --format json gff show 'modules/danm14ac.mod!dan14_juhani.utc' --game DIR"
);

const SET: &str = concat!(
    "\
An assignment is PATH=VALUE, the value read as the type of the field at PATH,
or PATH=TYPE:VALUE, which may change the field's type or create the field in
an existing struct. A field is never created without a type (RE1303). TYPE is
one of byte char word short dword int dword64 int64 float double
string (cexostring) resref (cresref) locstring (cexolocstring) strref vector
orientation void, in any case. Values:
  integers          decimal, within the type's range
  float, double     decimal, as `gff show` prints them
  string            text in the game's code page (Windows-1252)
  resref            at most 16 bytes
  strref            a number, -1 for none
  vector            x,y,z      orientation   a,b,c,d
  void              hexadecimal digits
  locstring         a strref number (-1 for none), keeping the substrings, or
                    ID=TEXT, setting the substring with string id ID (language
                    id times two, plus one for the feminine form)
Structs and lists are set field by field. The output keeps every other byte
the writer can: a file read from the game and written unchanged is identical.

",
    resource_forms!(),
    "

Examples:
  rakata gff set ./p_juhani.utc Tag=juhani2 -o out.utc
  rakata gff set ./p_juhani.utc FirstName=locstring:0=Juhani Plot=byte:1 -o out.utc
  rakata gff set dan14_juhani.utc ClassList[0].ClassLevel=8 --game DIR -m danm14ac -o dan14_juhani.utc"
);

const DIFF: &str = concat!(
    "\
Fields match by label within a struct, in any order; list elements by
position. A field one side lacks shows `(absent)`. The label table and the
layout are not compared. Exit 0 when the trees are the same, 3 when they
differ.

",
    resource_forms!(),
    "

",
    diff_sides!(),
    "

Examples:
  rakata gff diff ./a.utc ./b.utc
  rakata gff diff dan14_juhani.utc dan14_juhani.utc --game-a VANILLA --game-b BUILD -m danm14ac
  rakata --format json gff diff ./in.utc ./out.utc"
);

pub fn run(ctx: &Ctx, args: &GffArgs) -> Exit {
    match &args.action {
        GffAction::Show { name, path, lookup } => show(ctx, name, path.as_deref(), lookup),
        GffAction::Set {
            name,
            assignments,
            output,
            lookup,
        } => set(ctx, name, assignments, output, lookup),
        GffAction::Diff(d) => diff(ctx, d),
    }
}

fn parse(loaded: &Loaded) -> Result<Gff, Diagnostic> {
    Gff::parse(&loaded.bytes)
        .map_err(|e| diagnostic(e.code(), &e).note(format!("while reading `{}`", loaded.source)))
}

fn show(ctx: &Ctx, name: &str, path: Option<&str>, lookup: &LookupArgs) -> Exit {
    let mut env = ctx.envelope("gff show");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let loaded = match inspect::load(&mut env, "resource", name, &lookup.lookup(ctx)) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let gff = match parse(&loaded) {
        Ok(g) => g,
        Err(d) => return fail(env, d),
    };
    let mut result = json!({
        "source": loaded.source,
        "file_type": gff.file_type_str(),
    });
    let text = match path {
        None => {
            result["root"] = struct_json(&gff.root);
            let mut out = format!("{} {}\n", gff.file_type_str(), version(&gff));
            tree(&gff.root, 0, &mut out);
            out
        }
        Some(p) => {
            let parsed = match Path::parse(p) {
                Ok(p) => p,
                Err(e) => return fail(env, diagnostic(e.code(), &e)),
            };
            result["path"] = json!(p);
            match gff.get(&parsed) {
                Ok(Value::Struct(s)) => {
                    result["value"] = typed(&Value::Struct(s.clone()));
                    let mut out = format!("struct {}\n", s.id);
                    tree(s, 1, &mut out);
                    out
                }
                Ok(Value::List(l)) => {
                    result["value"] = typed(&Value::List(l.clone()));
                    let mut out = String::new();
                    list(l, 0, &mut out);
                    out
                }
                Ok(v) => {
                    result["value"] = typed(v);
                    bare(v)
                }
                Err(e) => match gff.struct_at(&parsed) {
                    Ok(s) => {
                        result["value"] = typed(&Value::Struct(s.clone()));
                        let mut out = format!("struct {}\n", s.id);
                        tree(s, 1, &mut out);
                        out
                    }
                    Err(_) => return fail(env, diagnostic(e.code(), &e)),
                },
            }
        }
    };
    env.result = result;
    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
}

fn set(
    ctx: &Ctx,
    name: &str,
    assignments: &[String],
    output: &std::path::Path,
    lookup: &LookupArgs,
) -> Exit {
    let mut env = ctx.envelope("gff set");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let loaded = match inspect::load(&mut env, "resource", name, &lookup.lookup(ctx)) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let mut gff = match parse(&loaded) {
        Ok(g) => g,
        Err(d) => return fail(env, d),
    };
    let mut changes = Vec::new();
    let mut text = String::new();
    for a in assignments {
        let done = match rakata::inspect::assign(&mut gff, a) {
            Ok(d) => d,
            Err(d) => return fail(env, d),
        };
        text.push_str(&format!(
            "{}: {} -> {}\n",
            done.path,
            done.before
                .as_ref()
                .map_or_else(|| "(absent)".to_owned(), inline),
            inline(&done.after)
        ));
        changes.push(json!({
            "path": done.path.to_string(),
            "before": done.before.as_ref().map(typed),
            "after": typed(&done.after),
        }));
    }
    let bytes = match gff.to_bytes() {
        Ok(b) => b,
        Err(e) => return fail(env, diagnostic(e.code(), &e)),
    };
    if let Err(d) = inspect::write(&mut env, output, &bytes) {
        return fail(env, d);
    }
    text.push_str(&format!("wrote {}\n", output.display()));
    env.result = json!({
        "source": loaded.source,
        "output": output.display().to_string(),
        "changes": changes,
    });
    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
}

fn diff(ctx: &Ctx, args: &DiffArgs) -> Exit {
    let mut env = ctx.envelope("gff diff");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let (a, b) = match args.load(ctx, &mut env) {
        Ok(ab) => ab,
        Err(d) => return fail(env, d),
    };
    let (ga, gb) = match (parse(&a), parse(&b)) {
        (Ok(x), Ok(y)) => (x, y),
        (Err(d), _) | (_, Err(d)) => return fail(env, d),
    };
    let mut lines = Vec::new();
    let mut changes = Vec::new();
    if ga.file_type != gb.file_type {
        lines.push(format!(
            "(file type): {} -> {}",
            ga.file_type_str(),
            gb.file_type_str()
        ));
    }
    for c in rakata::gff::diff(&ga, &gb) {
        let path = if c.path.is_root() {
            "(root)".to_owned()
        } else {
            c.path.to_string()
        };
        let side = |v: &Option<Value>| v.as_ref().map_or_else(|| "(absent)".to_owned(), inline);
        lines.push(format!("{path}: {} -> {}", side(&c.before), side(&c.after)));
        changes.push(json!({
            "path": c.path.to_string(),
            "before": c.before.as_ref().map(typed),
            "after": c.after.as_ref().map(typed),
        }));
    }
    let same = lines.is_empty();
    env.result = json!({
        "a": a.source,
        "b": b.source,
        "same": same,
        "file_types": [ga.file_type_str(), gb.file_type_str()],
        "changes": changes,
    });
    let text = if same {
        "no differences".to_owned()
    } else {
        lines.join("\n")
    };
    let exit = if same { Exit::Ok } else { Exit::Different };
    ctx.finish(env, &text, Vec::new(), &map, exit)
}

pub(super) fn version(g: &Gff) -> String {
    String::from_utf8_lossy(&g.version).into_owned()
}

/// The tree under `s`, one field per line at `depth`.
pub(super) fn tree(s: &Struct, depth: usize, out: &mut String) {
    for f in &s.fields {
        field(f, depth, out);
    }
}

fn field(f: &Field, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let ty = f.value.field_type();
    match &f.value {
        Value::Struct(s) => {
            out.push_str(&format!("{pad}{}: {ty} {}\n", f.label, s.id));
            tree(s, depth + 1, out);
        }
        Value::List(l) => {
            out.push_str(&format!("{pad}{}: {ty} [{}]\n", f.label, l.len()));
            list(l, depth + 1, out);
        }
        v => out.push_str(&format!("{pad}{}: {ty} = {}\n", f.label, inline(v))),
    }
}

fn list(l: &[Struct], depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    for (i, s) in l.iter().enumerate() {
        out.push_str(&format!("{pad}[{i}] struct {}\n", s.id));
        tree(s, depth + 1, out);
    }
}

fn text(bytes: &[u8]) -> String {
    codepage::decode(bytes)
}

fn quoted(bytes: &[u8]) -> String {
    serde_json::to_string(&text(bytes)).expect("a string serializes")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn floats(f: &[f32]) -> String {
    f.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn locstring(l: &LocString) -> String {
    let mut out = if l.has_strref() {
        format!("strref {}", l.strref)
    } else {
        "strref -1".to_owned()
    };
    for s in &l.substrings {
        out.push_str(&format!(", {}={}", s.id, quoted(&s.text)));
    }
    out
}

/// A value on one line, strings quoted.
fn inline(v: &Value) -> String {
    match v {
        Value::String(b) => quoted(b),
        Value::Struct(s) => format!("struct {} ({} fields)", s.id, s.fields.len()),
        Value::List(l) => format!("list of {}", l.len()),
        v => bare(v),
    }
}

/// A value alone, as `gff show --path` prints a field: strings unquoted,
/// in the form `gff set` reads back.
fn bare(v: &Value) -> String {
    match v {
        Value::Byte(n) => n.to_string(),
        Value::Char(n) => n.to_string(),
        Value::Word(n) => n.to_string(),
        Value::Short(n) => n.to_string(),
        Value::Dword(n) => n.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Dword64(n) => n.to_string(),
        Value::Int64(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::Double(n) => n.to_string(),
        Value::String(b) => text(b),
        Value::ResRef(r) => text(r.as_bytes()),
        Value::LocString(l) => locstring(l),
        Value::Void(b) => hex(b),
        Value::Struct(_) | Value::List(_) => inline(v),
        Value::Orientation(f) => floats(f),
        Value::Vector(f) => floats(f),
        Value::StrRef(n) if *n == LocString::NO_STRREF => "-1".to_owned(),
        Value::StrRef(n) => n.to_string(),
    }
}

/// A value with its type, as the JSON output carries it.
fn typed(v: &Value) -> Json {
    json!({ "type": v.field_type().name(), "value": value_json(v) })
}

pub(super) fn struct_json(s: &Struct) -> Json {
    json!({
        "id": s.id,
        "fields": s.fields.iter().map(|f| json!({
            "label": f.label.to_string(),
            "type": f.value.field_type().name(),
            "value": value_json(&f.value),
        })).collect::<Vec<_>>(),
    })
}

fn value_json(v: &Value) -> Json {
    match v {
        Value::Byte(n) => json!(n),
        Value::Char(n) => json!(n),
        Value::Word(n) => json!(n),
        Value::Short(n) => json!(n),
        Value::Dword(n) => json!(n),
        Value::Int(n) => json!(n),
        Value::Dword64(n) => json!(n),
        Value::Int64(n) => json!(n),
        Value::Float(n) => json!(n),
        Value::Double(n) => json!(n),
        Value::String(b) => json!(text(b)),
        Value::ResRef(r) => json!(text(r.as_bytes())),
        Value::LocString(l) => json!({
            "strref": l.has_strref().then_some(l.strref),
            "substrings": l.substrings.iter().map(|s| json!({
                "id": s.id,
                "language": s.language(),
                "feminine": s.feminine(),
                "text": text(&s.text),
            })).collect::<Vec<_>>(),
        }),
        Value::Void(b) => json!(hex(b)),
        Value::Struct(s) => struct_json(s),
        Value::List(l) => json!(l.iter().map(struct_json).collect::<Vec<_>>()),
        Value::Orientation(f) => json!(f),
        Value::Vector(f) => json!(f),
        Value::StrRef(n) => json!(n),
    }
}
