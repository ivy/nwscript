//! `cargo xtask ini-census DIR`: parse every TSLPatcher ini under a directory
//! of unpacked mod archives and print what the reader found, construct by
//! construct, and every line it could not take, with its file. A development
//! tool over the user's own archives; nothing it reads is committed.

use rakata_patch::changes::{FieldType, TableOpKind};
use rakata_patch::{Changes, Ini, Issue, Namespaces, Plan};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str = "\
cargo xtask ini-census: what the mods' changes.ini and namespaces.ini contain

USAGE:
  cargo xtask ini-census DIR [--issues N]

Walks DIR for every changes.ini and namespaces.ini, and every other .ini
under a tslpatchdata folder (the files namespaces.ini names), parses each
with rakata-patch, and prints: the mods (DIR's direct children), the files,
the constructs by kind, and every issue the reader raised, grouped by code,
with up to N examples each (default 20; 0 for all).

EXAMPLE:
  cargo xtask ini-census /tmp/extracted-archives
";

#[derive(Default)]
struct Census {
    mods: std::collections::BTreeSet<String>,
    files: usize,
    namespaces_files: usize,
    changes_files: usize,
    constructs: BTreeMap<&'static str, usize>,
    issues: BTreeMap<&'static str, Vec<(PathBuf, Issue)>>,
}

impl Census {
    fn count(&mut self, what: &'static str, n: usize) {
        *self.constructs.entry(what).or_insert(0) += n;
    }
}

pub fn main(args: &[String]) -> Result<()> {
    let mut dir: Option<PathBuf> = None;
    let mut limit = 20usize;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => {
                print!("{USAGE}");
                return Ok(());
            }
            "--issues" => {
                limit = it
                    .next()
                    .ok_or("--issues needs a number")?
                    .parse()
                    .map_err(|_| "--issues needs a number")?;
            }
            _ if dir.is_none() => dir = Some(PathBuf::from(a)),
            _ => return Err(format!("unexpected argument `{a}`\n{USAGE}").into()),
        }
    }
    let dir = dir.ok_or_else(|| format!("DIR is required\n{USAGE}"))?;
    let mut census = Census::default();
    let mut paths = Vec::new();
    walk(&dir, &mut paths)?;
    paths.sort();
    for path in &paths {
        let rel = path.strip_prefix(&dir).unwrap_or(path);
        if let Some(first) = rel.components().next() {
            census
                .mods
                .insert(first.as_os_str().to_string_lossy().to_string());
        }
        let bytes = std::fs::read(path)?;
        let mut issues = Vec::new();
        let ini = Ini::parse(&bytes, 0, &mut issues);
        census.files += 1;
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        if name == "namespaces.ini" {
            census.namespaces_files += 1;
            let ns = Namespaces::parse(&ini, 0, &mut issues);
            census.count("namespaces: options", ns.options.len());
            census.count(
                "namespaces: options without IniName (default changes.ini)",
                ns.options
                    .iter()
                    .filter(|o| o.ini_name == "changes.ini")
                    .count(),
            );
        } else {
            census.changes_files += 1;
            let changes = Changes::parse(&ini, 0, &mut issues);
            let plan = Plan::build(&changes, 0, &mut issues);
            count_changes(&mut census, &changes);
            census.count("plan: steps", plan.steps.len());
        }
        for i in issues {
            census
                .issues
                .entry(i.code)
                .or_default()
                .push((rel.to_path_buf(), i));
        }
    }
    report(&census, limit);
    Ok(())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_dir() {
            walk(&p, out)?;
            continue;
        }
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        if !name.ends_with(".ini") || name.starts_with("._") || name == "desktop.ini" {
            continue;
        }
        let in_tslpatchdata = p.components().any(|c| {
            c.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case("tslpatchdata")
        });
        if name == "changes.ini" || name == "namespaces.ini" || in_tslpatchdata {
            out.push(p);
        }
    }
    Ok(())
}

fn count_changes(c: &mut Census, ch: &Changes) {
    let s = &ch.settings;
    c.count("settings: files", 1);
    c.count("settings: Required", s.required.len());
    c.count(
        "settings: Required without RequiredMsg",
        s.required.iter().filter(|r| r.message.is_none()).count(),
    );
    c.count("tlk: StrRef# appends", ch.tlk.appends.len());
    c.count(
        "tlk: Replace#/ReplaceFile# files",
        ch.tlk.replacements.len(),
    );
    c.count(
        "tlk: replacement entries",
        ch.tlk.replacements.iter().map(|r| r.entries.len()).sum(),
    );
    c.count("install: folders", ch.install.len());
    c.count(
        "install: File#",
        ch.install
            .iter()
            .flat_map(|f| &f.files)
            .filter(|f| !f.replace)
            .count(),
    );
    c.count(
        "install: Replace#",
        ch.install
            .iter()
            .flat_map(|f| &f.files)
            .filter(|f| f.replace)
            .count(),
    );
    c.count(
        "install: !SaveAs",
        ch.install
            .iter()
            .flat_map(|f| &f.files)
            .filter(|f| f.save_as.is_some())
            .count(),
    );
    c.count(
        "install: into a capsule (modules\\*.mod etc.)",
        ch.install
            .iter()
            .filter(|f| f.destination.contains('.'))
            .count(),
    );
    c.count("2da: tables", ch.tables.len());
    for t in &ch.tables {
        for op in &t.ops {
            match &op.kind {
                TableOpKind::ChangeRow(r) => {
                    c.count("2da: ChangeRow", 1);
                    let by = match r.target {
                        Some(rakata_patch::changes::RowTarget::Index(_)) => {
                            "2da: ChangeRow by RowIndex"
                        }
                        Some(rakata_patch::changes::RowTarget::Label(_)) => {
                            "2da: ChangeRow by RowLabel"
                        }
                        Some(rakata_patch::changes::RowTarget::LabelIndex(_)) => {
                            "2da: ChangeRow by LabelIndex"
                        }
                        None => "2da: ChangeRow without a target",
                    };
                    c.count(by, 1);
                    c.count("2da: cells", r.cells.len());
                    c.count("2da: 2DAMEMORY stores", r.stores.len());
                    c.count(
                        "2da: cells = high()",
                        r.cells
                            .iter()
                            .filter(|x| x.value == rakata_patch::Value::High)
                            .count(),
                    );
                    c.count(
                        "2da: cells = token",
                        r.cells.iter().filter(|x| x.value.token().is_some()).count(),
                    );
                }
                TableOpKind::AddRow(r) => {
                    c.count("2da: AddRow", 1);
                    c.count(
                        "2da: AddRow with ExclusiveColumn",
                        usize::from(r.exclusive_column.is_some()),
                    );
                    c.count(
                        "2da: AddRow with RowLabel",
                        usize::from(r.row_label.is_some()),
                    );
                    c.count("2da: cells", r.cells.len());
                    c.count("2da: 2DAMEMORY stores", r.stores.len());
                    c.count(
                        "2da: cells = high()",
                        r.cells
                            .iter()
                            .filter(|x| x.value == rakata_patch::Value::High)
                            .count(),
                    );
                    c.count(
                        "2da: cells = token",
                        r.cells.iter().filter(|x| x.value.token().is_some()).count(),
                    );
                }
                TableOpKind::CopyRow(r) => {
                    c.count("2da: CopyRow", 1);
                    c.count("2da: cells", r.cells.len());
                    c.count("2da: 2DAMEMORY stores", r.stores.len());
                }
                TableOpKind::AddColumn(r) => {
                    c.count("2da: AddColumn", 1);
                    c.count("2da: AddColumn cells", r.cells.len());
                    c.count("2da: 2DAMEMORY stores", r.stores.len());
                }
            }
        }
    }
    c.count("gff: files", ch.gff.len());
    for g in &ch.gff {
        c.count("gff: !Destination", usize::from(g.destination.is_some()));
        c.count("gff: !Filename", usize::from(g.filename.is_some()));
        c.count("gff: !SaveAs", usize::from(g.save_as.is_some()));
        c.count("gff: !ReplaceFile=1", usize::from(g.replace_file));
        c.count("gff: !OverrideType", usize::from(g.override_type.is_some()));
        c.count("gff: edits", g.edits.len());
        for e in &g.edits {
            match e.target {
                rakata_patch::changes::FieldTarget::Value => {}
                rakata_patch::changes::FieldTarget::StrRef => c.count("gff: edits of (strref)", 1),
                rakata_patch::changes::FieldTarget::Substring(_) => {
                    c.count("gff: edits of (lang#)", 1)
                }
            }
            if matches!(e.path, rakata_patch::changes::FieldPath::Memory(_)) {
                c.count("gff: edits at a path in 2DAMEMORY", 1);
            }
            if e.value.token().is_some() {
                c.count("gff: edits = token", 1);
            }
        }
        for a in &g.adds {
            count_add(c, a, 0);
        }
    }
    c.count("hack: files", ch.hacks.len());
    c.count(
        "hack: writes",
        ch.hacks.iter().map(|h| h.writes.len()).sum(),
    );
    c.count("compile: scripts", ch.compile.scripts.len());
    c.count(
        "compile: Replace#",
        ch.compile.scripts.iter().filter(|s| s.replace).count(),
    );
    c.count(
        "compile: with !Destination",
        ch.compile
            .scripts
            .iter()
            .filter(|s| s.destination.is_some())
            .count(),
    );
    c.count("ssf: files", ch.ssf.len());
    c.count("ssf: entries", ch.ssf.iter().map(|s| s.entries.len()).sum());
}

fn count_add(c: &mut Census, a: &rakata_patch::changes::AddField, depth: usize) {
    c.count("gff: AddField", 1);
    if depth > 0 {
        c.count("gff: AddField nested in a struct or list", 1);
    }
    let ty: &'static str = match a.field_type {
        FieldType::Byte => "gff: AddField Byte",
        FieldType::Char => "gff: AddField Char",
        FieldType::Word => "gff: AddField Word",
        FieldType::Short => "gff: AddField Short",
        FieldType::Dword => "gff: AddField Dword",
        FieldType::Int => "gff: AddField Int",
        FieldType::Dword64 => "gff: AddField Dword64",
        FieldType::Int64 => "gff: AddField Int64",
        FieldType::Float => "gff: AddField Float",
        FieldType::Double => "gff: AddField Double",
        FieldType::ExoString => "gff: AddField ExoString",
        FieldType::ResRef => "gff: AddField ResRef",
        FieldType::ExoLocString => "gff: AddField ExoLocString",
        FieldType::Void => "gff: AddField Void",
        FieldType::Struct => "gff: AddField Struct",
        FieldType::List => "gff: AddField List",
        FieldType::Orientation => "gff: AddField Orientation",
        FieldType::Position => "gff: AddField Position",
    };
    c.count(ty, 1);
    c.count("gff: AddField substrings (lang#)", a.substrings.len());
    c.count("gff: AddField 2DAMEMORY stores", a.stores.len());
    if a.value.as_ref().is_some_and(|v| v.token().is_some())
        || a.strref.as_ref().is_some_and(|v| v.token().is_some())
    {
        c.count("gff: AddField = token", 1);
    }
    for ch in &a.children {
        count_add(c, ch, depth + 1);
    }
}

fn report(c: &Census, limit: usize) {
    println!("mods: {}", c.mods.len());
    println!(
        "ini files: {} ({} changes files, {} namespaces.ini)",
        c.files, c.changes_files, c.namespaces_files
    );
    println!();
    println!("constructs parsed:");
    for (k, v) in &c.constructs {
        println!("  {v:>7}  {k}");
    }
    println!();
    let total: usize = c.issues.values().map(Vec::len).sum();
    println!("issues: {total}");
    for (code, list) in &c.issues {
        println!("  {code}: {}", list.len());
        let shown = if limit == 0 {
            list.len()
        } else {
            limit.min(list.len())
        };
        for (path, i) in &list[..shown] {
            println!("      {}:{}: {}", path.display(), i.line, i.message);
        }
        if shown < list.len() {
            println!("      ... {} more", list.len() - shown);
        }
    }
}
