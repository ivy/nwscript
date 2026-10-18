//! Where a resource's [`Content`] comes from: a walk that reads every file
//! (the round-trip suite's [`walk`](crate::conformance::walk)), or an
//! index of the tree that answers from its `refs`, `cells` and `calls`.
//! Both read the same resources, keyed by the same paths, and give the
//! same contents: a GFF's references from `rakata_gff::facts::refs` (a
//! dialogue's through `rakata_dialog`), a table's cells, a script's calls
//! from `nwscript_ncs::facts::calls`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use nwscript::diag::{Diagnostic, FileId};
use nwscript::ncs::facts::{ArgType, Literal};
use rakata_archive::{Bif, Erf, Key, Rim, Source};
use rakata_index::Routine;
use rakata_res::{RefKind, ResId, ResType};
use serde_json::Value as Json;

use super::rules::{Call, Content, Item, Lit};
use super::sites::TABLE;
use super::universe::Universe;

/// Files this large are memory-mapped rather than read whole.
const MAP_THRESHOLD: u64 = 1 << 20;

/// Whether the lint reads resources of this type.
fn is_checked(ty: ResType) -> bool {
    rakata_index::is_gff(ty) || matches!(ty, ResType::TwoDa | ResType::Ncs)
}

/// What a source gave: the resources it read and those it could not.
#[derive(Default)]
pub(crate) struct Read {
    pub items: Vec<Item>,
    /// Resources of a checked type that did not read: path and reason.
    pub unread: Vec<(String, String)>,
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether the file at `rel` is one the run checks.
pub(crate) fn wanted(listed: Option<&HashSet<String>>, rel: &str) -> bool {
    listed.is_none_or(|l| l.contains(&rel.to_ascii_lowercase()))
}

fn lit(l: &Literal) -> Lit {
    match l {
        Literal::Int(v) => Lit::Int(i64::from(*v)),
        Literal::String(b) => Lit::Str(rakata_tlk::codepage::decode(b)),
        _ => Lit::Other,
    }
}

/// A resource's content from its bytes, or why it does not read.
fn content(ty: ResType, name: &str, bytes: &[u8], routines: &[Routine]) -> Result<Content, String> {
    if ty == ResType::Dlg {
        let d = rakata_dialog::Dialog::parse(format!("{name}.dlg"), bytes)
            .map_err(|e| e.to_string())?;
        return Ok(gff_content(d.gff()));
    }
    if rakata_index::is_gff(ty) {
        let g = rakata_gff::Gff::parse(bytes).map_err(|e| e.to_string())?;
        return Ok(gff_content(&g));
    }
    match ty {
        ResType::TwoDa => {
            let t = rakata_2da::TwoDa::parse(bytes).map_err(|e| e.to_string())?;
            let mut cells = Vec::new();
            for (i, row) in t.rows.iter().enumerate() {
                for (column, value) in t.columns.iter().zip(&row.cells) {
                    if !value.is_empty() {
                        cells.push((i, column.clone(), value.clone()));
                    }
                }
            }
            Ok(Content::TwoDa(cells))
        }
        _ => {
            let program = nwscript::ncs::decode(FileId(0), bytes)
                .map_err(|d| format!("{}: {}", d.code.id, d.message))?;
            let param = |r: u16, k: usize| -> Option<ArgType> {
                routines
                    .get(usize::from(r))
                    .and_then(|f| f.params.get(k))
                    .map(|p| p.ty)
            };
            let ret = |r: u16| routines.get(usize::from(r)).map(|f| f.ret);
            let calls = nwscript::ncs::facts::calls(&program, param, ret)
                .into_iter()
                .map(|c| Call {
                    offset: c.offset,
                    routine: c.routine,
                    args: c.args.iter().map(|a| a.as_ref().map(lit)).collect(),
                })
                .collect();
            Ok(Content::Script(calls))
        }
    }
}

fn gff_content(g: &rakata_gff::Gff) -> Content {
    Content::Gff(
        rakata_gff::facts::refs(g)
            .into_iter()
            .filter(|r| matches!(r.kind, RefKind::ResRef | RefKind::StrRef))
            .map(|r| (r.kind, r.value, r.site))
            .collect(),
    )
}

/// The BIF members' ids from `chitin.key`, by the lowercased path of each
/// BIF under the tree and the member's position.
fn bif_names(key: &Key) -> HashMap<String, HashMap<u32, ResId>> {
    let paths: Vec<String> = key
        .bifs()
        .iter()
        .map(|b| {
            b.components()
                .collect::<Vec<_>>()
                .join("/")
                .to_ascii_lowercase()
        })
        .collect();
    let mut out: HashMap<String, HashMap<u32, ResId>> = HashMap::new();
    for e in key.entries() {
        if let Some(p) = paths.get(e.bif as usize) {
            out.entry(p.clone()).or_default().insert(e.index, e.id);
        }
    }
    out
}

fn ext_of(ty: ResType) -> String {
    ty.extension().unwrap_or_default().to_ascii_lowercase()
}

/// Read every file of the walk that `listed` names (all when `None`) on
/// `threads` threads.
pub(crate) fn walk(
    root: &Path,
    key: &Key,
    universe: &Universe,
    routines: &[Routine],
    listed: Option<&HashSet<String>>,
    threads: usize,
) -> Result<Read, Diagnostic> {
    let files: Vec<(PathBuf, String)> = crate::conformance::walk(root)?
        .into_iter()
        .map(|p| {
            let r = rel(root, &p);
            (p, r)
        })
        .filter(|(_, r)| wanted(listed, r))
        .collect();
    let names = bif_names(key);
    let next = AtomicUsize::new(0);
    let out = Mutex::new(Read::default());
    let threads = match threads {
        0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
        n => n,
    }
    .min(files.len().max(1));
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut mine = Read::default();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((abs, r)) = files.get(i) else { break };
                    read_file(abs, r, &names, universe, routines, &mut mine);
                }
                let mut all = out.lock().expect("no reader panicked");
                all.items.append(&mut mine.items);
                all.unread.append(&mut mine.unread);
            });
        }
    });
    Ok(out.into_inner().expect("no reader panicked"))
}

fn read_file(
    abs: &Path,
    r: &str,
    names: &HashMap<String, HashMap<u32, ResId>>,
    universe: &Universe,
    routines: &[Routine],
    out: &mut Read,
) {
    let base = r.rsplit('/').next().unwrap_or(r);
    let Some((stem, ext)) = base.rsplit_once('.') else {
        return;
    };
    let Some(ty) = ResType::from_extension(ext) else {
        return;
    };
    let archive = rakata_index::is_archive(ty) && ty != ResType::Key;
    if !archive && !is_checked(ty) {
        return;
    }
    let size = fs::metadata(abs).map_or(0, |m| m.len());
    let src = if size >= MAP_THRESHOLD {
        Source::map(abs)
    } else {
        Source::read(abs)
    };
    let src = match src {
        Ok(s) => s,
        Err(e) => {
            out.unread.push((r.to_string(), e.to_string()));
            return;
        }
    };
    let bytes = src.bytes();
    let scope = universe.scope_of(r);
    let mut one = |path: String, name: String, ty: ResType, data: &[u8]| {
        if !is_checked(ty) {
            return;
        }
        match content(ty, &name, data, routines) {
            Ok(content) => out.items.push(Item {
                path,
                name,
                ext: ext_of(ty),
                scope: scope.clone(),
                content,
            }),
            Err(e) => out.unread.push((path, e)),
        }
    };
    if !archive {
        one(r.to_string(), stem.to_ascii_lowercase(), ty, bytes);
        return;
    }
    let id_name = |id: &ResId| String::from_utf8_lossy(id.name.as_bytes()).to_ascii_lowercase();
    let member = |n: &str, ty: ResType| format!("{r}!{n}.{}", ext_of(ty));
    let failed = match ty {
        ResType::Bif => match Bif::parse(bytes) {
            Ok(bif) => {
                let ids = names.get(&r.to_ascii_lowercase());
                for (i, m) in bif.members().iter().enumerate() {
                    match ids.and_then(|n| n.get(&(i as u32))) {
                        Some(id) => {
                            let n = id_name(id);
                            one(member(&n, m.ty), n, m.ty, m.data());
                        }
                        None => one(
                            format!("{r}!#{i}.{}", ext_of(m.ty)),
                            String::new(),
                            m.ty,
                            m.data(),
                        ),
                    }
                }
                None
            }
            Err(e) => Some(e.to_string()),
        },
        ResType::Rim => match Rim::parse(bytes) {
            Ok(rim) => {
                for m in rim.members() {
                    let n = id_name(&m.id);
                    one(member(&n, m.id.ty), n, m.id.ty, m.data());
                }
                None
            }
            Err(e) => Some(e.to_string()),
        },
        _ => match Erf::parse(bytes) {
            Ok(erf) => {
                for m in erf.members() {
                    let n = id_name(&m.id);
                    one(member(&n, m.id.ty), n, m.id.ty, m.data());
                }
                None
            }
            Err(e) => Some(e.to_string()),
        },
    };
    if let Some(e) = failed {
        out.unread.push((r.to_string(), e));
    }
}

/// SQL text for a string.
fn sql_str(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn text(v: &Json) -> String {
    match v {
        Json::String(s) => s.clone(),
        Json::Null => String::new(),
        other => other.to_string(),
    }
}

fn int(v: &Json) -> i64 {
    v.as_i64().unwrap_or_default()
}

/// The JSON literal `calls.args` holds, as the walk reads it.
fn json_lit(v: &Json) -> Option<Lit> {
    match v {
        Json::Null => None,
        Json::String(s) => Some(Lit::Str(s.clone())),
        Json::Number(n) => Some(n.as_i64().map_or(Lit::Other, Lit::Int)),
        _ => Some(Lit::Other),
    }
}

/// The tree of `db` whose root is `game`: its id and label.
pub(crate) fn tree_of(
    conn: &rakata_index::Connection,
    game: &Path,
) -> Result<Option<(i64, String)>, Diagnostic> {
    let rows = rakata_index::query(conn, "SELECT id, label, root FROM trees ORDER BY id")
        .map_err(|e| crate::index::error(&e))?;
    let want = game.canonicalize().unwrap_or_else(|_| game.to_path_buf());
    let mut found = None;
    for r in &rows.rows {
        let root = PathBuf::from(text(&r[2]));
        if root.canonicalize().unwrap_or(root) == want {
            found = Some((int(&r[0]), text(&r[1])));
        }
    }
    Ok(found)
}

/// Read the contents of tree `tree` from the index.
pub(crate) fn index(
    conn: &rakata_index::Connection,
    tree: i64,
    universe: &Universe,
    listed: Option<&HashSet<String>>,
) -> Result<Read, Diagnostic> {
    let q = |sql: String| rakata_index::query(conn, &sql).map_err(|e| crate::index::error(&e));
    let only = match listed {
        Some(l) if l.is_empty() => " AND 0".to_string(),
        Some(l) => {
            let list: Vec<String> = l.iter().map(|p| sql_str(p)).collect();
            format!(" AND lower(l.path) IN ({})", list.join(","))
        }
        None => String::new(),
    };
    let key = |path: &str, member: &str| {
        if member.is_empty() {
            path.to_string()
        } else {
            format!("{path}!{member}")
        }
    };

    // Every resource of a checked type, so an empty content is an item too.
    let mut items: Vec<Item> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    let locs = q(format!(
        "SELECT l.path, l.member, l.name, l.type FROM locations l WHERE l.tree = {tree}{only}"
    ))?;
    for r in &locs.rows {
        let (path, member, name, ext) = (text(&r[0]), text(&r[1]), text(&r[2]), text(&r[3]));
        let Some(ty) = ResType::from_extension(&ext) else {
            continue;
        };
        if !is_checked(ty) {
            continue;
        }
        let k = key(&path, &member);
        let content = match ty {
            ResType::TwoDa => Content::TwoDa(Vec::new()),
            ResType::Ncs => Content::Script(Vec::new()),
            _ => Content::Gff(Vec::new()),
        };
        at.insert(k.clone(), items.len());
        items.push(Item {
            path: k,
            name,
            ext,
            scope: universe.scope_of(&path),
            content,
        });
    }

    let refs = q(format!(
        "SELECT l.path, l.member, r.kind, r.value, r.site FROM locations l JOIN refs r ON r.from_hash = l.hash
         WHERE l.tree = {tree} AND r.kind IN ('resref', 'strref') AND l.type NOT IN ('ncs', 'tlk', '2da'){only}
        "
    ))?;
    for r in &refs.rows {
        let Some(&i) = at.get(&key(&text(&r[0]), &text(&r[1]))) else {
            continue;
        };
        let kind = if text(&r[2]) == "resref" {
            RefKind::ResRef
        } else {
            RefKind::StrRef
        };
        if let Content::Gff(v) = &mut items[i].content {
            v.push((kind, text(&r[3]), text(&r[4])));
        }
    }

    let tables: Vec<String> = TABLE.tables().iter().map(|t| sql_str(t)).collect();
    let cells = q(format!(
        "SELECT l.path, l.member, c.row, c.column, c.value FROM locations l JOIN cells c ON c.hash = l.hash
         WHERE l.tree = {tree} AND l.type = '2da' AND l.name IN ({}){only}
        ",
        tables.join(",")
    ))?;
    for r in &cells.rows {
        let Some(&i) = at.get(&key(&text(&r[0]), &text(&r[1]))) else {
            continue;
        };
        if let Content::TwoDa(v) = &mut items[i].content {
            v.push((int(&r[2]) as usize, text(&r[3]), text(&r[4])));
        }
    }

    let calls = q(format!(
        "SELECT l.path, l.member, c.offset, c.routine_id, c.args FROM locations l JOIN calls c ON c.hash = l.hash
         WHERE l.tree = {tree} AND l.type = 'ncs' AND c.args LIKE '%\"%'{only}
        "
    ))?;
    for r in &calls.rows {
        let Some(&i) = at.get(&key(&text(&r[0]), &text(&r[1]))) else {
            continue;
        };
        let args: Vec<Json> = serde_json::from_str(&text(&r[4])).unwrap_or_default();
        if let Content::Script(v) = &mut items[i].content {
            v.push(Call {
                offset: int(&r[2]) as u32,
                routine: int(&r[3]) as u16,
                args: args.iter().map(json_lit).collect(),
            });
        }
    }

    let failed = q(format!(
        "SELECT l.path, l.member, l.reason FROM coverage l WHERE l.tree = {tree} AND l.status = 'failed'{only}"
    ))?;
    let unread = failed
        .rows
        .iter()
        .filter(|r| {
            let base = if text(&r[1]).is_empty() {
                text(&r[0])
            } else {
                text(&r[1])
            };
            base.rsplit_once('.')
                .and_then(|(_, e)| ResType::from_extension(e))
                .is_some_and(|t| is_checked(t) || rakata_index::is_archive(t))
        })
        .map(|r| (key(&text(&r[0]), &text(&r[1])), text(&r[2])))
        .collect::<Vec<_>>();
    // A content that failed is located too; it is unread, not checked.
    let failed: HashSet<&str> = unread.iter().map(|(p, _)| p.as_str()).collect();
    items.retain(|i| !failed.contains(i.path.as_str()));
    Ok(Read { items, unread })
}
