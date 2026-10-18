//! The semantic diff: two trees, two archives or two resources compared by
//! what they hold rather than by their bytes.
//!
//! Trees pair their files by path without regard to case (a case-only
//! rename is a change of its own), archives pair their members by name (a
//! BIF's by position), and each pair whose bytes differ is compared by its
//! format: a GFF by field path, a 2DA by row label and column, a talk table
//! by strref, a dialogue node by node under stable keys ([`dialog`]), a LIP
//! by keyframe, a KEY by resource. A format without a crate is compared by
//! hash, with the header fields known for it, and marked as not compared.
//! A pair whose bytes differ while its data is equal (another writer's
//! layout) is counted in [`Diff::same_data`] and listed nowhere.
//!
//! An `Override/` file one tree has and the other lacks is compared with
//! the copies the other tree's install loads in its place, from `patch.erf`
//! and the BIFs or else from the modules' capsules: a mod that puts a
//! changed dialogue in Override shows the nodes it changed, not a new file.
//!
//! The result is a [`Diff`]: entries grouped by path, each with its sites,
//! which `rakata diff` renders as text ([`Diff::text`]) and serialises as
//! JSON.

mod compare;
pub mod dialog;
mod render;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::codes;
use crate::diag::Diagnostic;
use crate::game::{Install, ModuleName, Profile};
use crate::inspect::{self, Lookup};
use rakata_res::{ContentHash, ResId};

/// How one entry differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    /// Only the second side has it.
    Added,
    /// Only the first side has it.
    Removed,
    /// Both have it, and it differs.
    Changed,
    /// Both have it under names that differ only in case; it may also
    /// differ in content.
    Renamed,
}

/// What is known of one side's bytes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Facts {
    pub size: u64,
    pub sha256: String,
    /// Header fields of a format without a crate, where they are known
    /// (`width`, `height` and `encoding` for a TPC; a WAV's duration).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<Header>,
}

/// One named header field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Header {
    pub field: String,
    pub value: String,
}

/// A GFF value as the diff shows it: its type and its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shown {
    #[serde(rename = "type")]
    pub ty: String,
    pub value: String,
}

/// One field path on which two GFF trees differ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldChange {
    pub path: String,
    pub before: Option<Shown>,
    pub after: Option<Shown>,
}

/// One cell of a row added or removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cell {
    pub column: String,
    pub value: String,
}

/// One place inside a resource where the two sides differ.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "site", rename_all = "snake_case")]
pub enum Site {
    /// A header or file-level field: a GFF's file type, an archive's build
    /// date, a LIP's length, a talk table's language.
    Header {
        field: String,
        before: Option<String>,
        after: Option<String>,
    },
    /// A GFF field, by path.
    Field(FieldChange),
    /// A 2DA column added or removed.
    Column { change: Change, column: String },
    /// A 2DA row added or removed, by label, with its non-blank cells.
    Row {
        change: Change,
        row: String,
        cells: Vec<Cell>,
    },
    /// A 2DA cell both tables have.
    Cell {
        row: String,
        column: String,
        before: String,
        after: String,
    },
    /// A talk-table entry, by strref: one site per field that differs, or
    /// the text of an entry added or removed.
    Strref {
        change: Change,
        strref: u32,
        field: String,
        before: Option<String>,
        after: Option<String>,
    },
    /// A dialogue node, by its stable key ([`dialog`]).
    Node(dialog::NodeChange),
    /// A KEY's resource entry: where the BIFs hold it.
    Resource {
        change: Change,
        resource: String,
        before: Option<String>,
        after: Option<String>,
    },
}

/// One path that differs: a file of a tree, or a member of an archive as
/// `ARCHIVE!MEMBER`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    /// The second side's spelling, or the first's for an entry removed.
    pub path: String,
    pub change: Change,
    /// The first side's spelling, for a rename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renamed_from: Option<String>,
    /// The format the contents were read as: `gff`, `dlg`, `2da`, `tlk`,
    /// `lip`, `archive`; `None` for a format without a crate.
    pub format: Option<String>,
    /// Whether the contents were compared by format. `false` means
    /// contents not compared: the sides are told apart by hash and header.
    pub compared: bool,
    pub a: Option<Facts>,
    pub b: Option<Facts>,
    /// For an `Override/` file one side lacks: the copies the other side's
    /// install loads in its place, against which the file is compared
    /// (one entry per distinct copy). Empty otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub against: Vec<String>,
    pub sites: Vec<Site>,
}

/// A problem met while comparing, with its registered code; the diff goes
/// on past it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub code: &'static str,
    pub message: String,
}

impl Warning {
    /// The warning as a diagnostic.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::new(code(self.code), self.message.clone())
    }
}

/// The comparison of two sides.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diff {
    /// The two sides as named.
    pub a: String,
    pub b: String,
    /// `tree` for two directories, `resource` for anything else.
    pub kind: &'static str,
    /// Files and members paired and compared.
    pub compared: usize,
    /// Pairs whose bytes differ while their data is equal: a layout
    /// another writer chose. They are not entries.
    pub same_data: usize,
    /// Every path that differs, sorted by path.
    pub entries: Vec<Entry>,
    #[serde(skip)]
    pub warnings: Vec<Warning>,
}

impl Diff {
    /// Whether the two sides hold the same data.
    pub fn is_same(&self) -> bool {
        self.entries.is_empty()
    }

    /// The diff as text: one block per path, its sites indented beneath.
    pub fn text(&self) -> String {
        render::text(self)
    }
}

/// What a diff is asked to do beyond its two sides.
#[derive(Debug, Clone)]
pub struct Options {
    /// Keep only entries whose path matches one of these: a glob (`*`,
    /// `?`), a path, or a directory or archive the entry lies in; case does
    /// not matter. Empty keeps everything.
    pub only: Vec<String>,
    /// The game the trees' installs belong to, for the Override lookups.
    pub profile: Profile,
    /// The install a `NAME.EXT` side is resolved in (`--game`), and the
    /// module whose capsules are searched.
    pub game: Option<PathBuf>,
    pub module: Option<String>,
}

impl Default for Options {
    /// Everything kept, KotOR 1, no install.
    fn default() -> Self {
        Options {
            only: Vec::new(),
            profile: Profile::K1,
            game: None,
            module: None,
        }
    }
}

fn code(id: &str) -> &'static codes::Code {
    codes::lookup(id).unwrap_or_else(|| panic!("diagnostic code {id} is not registered"))
}

/// Compare two sides named as `rakata diff` takes them: two directories
/// are trees; anything else is a resource [`inspect::load`] reads (a path,
/// an `ARCHIVE!MEMBER`, or a `NAME.EXT` resolved in [`Options::game`]).
pub fn diff(a: &str, b: &str, options: &Options) -> Result<Diff, Diagnostic> {
    let (da, db) = (Path::new(a).is_dir(), Path::new(b).is_dir());
    match (da, db) {
        (true, true) => diff_trees(Path::new(a), Path::new(b), options),
        (false, false) => {
            let lookup = Lookup {
                game: options.game.as_deref(),
                module: options.module.as_deref(),
                profile: options.profile,
            };
            let la = inspect::load(a, &lookup)?;
            let lb = inspect::load(b, &lookup)?;
            let mut d = diff_resources((&la.name, &la.bytes), (&lb.name, &lb.bytes), options);
            d.a = la.source;
            d.b = lb.source;
            Ok(d)
        }
        _ => {
            let (dir, file) = if da { (a, b) } else { (b, a) };
            Err(Diagnostic::new(
                &codes::RE1601,
                format!("`{dir}` is a directory and `{file}` is not: a diff compares two trees or two resources"),
            )
            .help("name two directories to compare trees, or two files, archive members or resources"))
        }
    }
}

/// Compare two resources, each a name (whose extension says its format)
/// and its bytes. An archive is recursed into its members.
pub fn diff_resources(a: (&str, &[u8]), b: (&str, &[u8]), options: &Options) -> Diff {
    let mut cx = compare::Cx::new(None, None, options.profile);
    let path = if a.0.eq_ignore_ascii_case(b.0) {
        b.0.to_owned()
    } else {
        format!("{} -> {}", a.0, b.0)
    };
    cx.compared += 1;
    if a.1 != b.1 {
        let ext = extension(b.0).or_else(|| extension(a.0));
        cx.pair(&path, ext.as_deref(), a.1, b.1, Change::Changed, None);
    }
    cx.finish(a.0.to_owned(), b.0.to_owned(), "resource", options)
}

/// Compare two trees: every file at any depth, paired by path in any case.
/// A directory reached through a symbolic link is not descended (a build
/// that links `override` to `Override` holds each file once); the link's
/// target is compared instead.
pub fn diff_trees(a: &Path, b: &Path, options: &Options) -> Result<Diff, Diagnostic> {
    let wa = walk(a)?;
    let wb = walk(b)?;
    let mut cx = compare::Cx::new(
        Some(a.to_path_buf()),
        Some(b.to_path_buf()),
        options.profile,
    );
    for (ea, eb) in pair(&wa, &wb) {
        let wanted = |e: &Walked| {
            options.only.is_empty()
                || options.only.iter().any(|p| matches(p, &e.rel))
                || is_archive(&e.rel)
        };
        match (ea, eb) {
            (Some(x), Some(y)) => {
                if !wanted(x) && !wanted(y) {
                    continue;
                }
                cx.compared += 1;
                cx.files(x, y);
            }
            (Some(x), None) => {
                if wanted(x) {
                    cx.one_side(x, Change::Removed);
                }
            }
            (None, Some(y)) => {
                if wanted(y) {
                    cx.one_side(y, Change::Added);
                }
            }
            (None, None) => {}
        }
    }
    Ok(cx.finish(
        a.display().to_string(),
        b.display().to_string(),
        "tree",
        options,
    ))
}

/// Whether `path` is kept by the `--only` pattern `pat`.
fn matches(pat: &str, path: &str) -> bool {
    let p = pat.trim_end_matches('/');
    if crate::conformance::glob_match(p, path) {
        return true;
    }
    let (pl, sl) = (p.to_ascii_lowercase(), path.to_ascii_lowercase());
    if let Some(rest) = sl.strip_prefix(&pl) {
        return rest.is_empty() || rest.starts_with('/') || rest.starts_with('!');
    }
    // A pattern without a separator also matches a file or member name.
    if !p.contains('/') && !p.contains('!') {
        let name = path.rsplit(['/', '!']).next().unwrap_or(path);
        return crate::conformance::glob_match(p, name);
    }
    false
}

fn extension(name: &str) -> Option<String> {
    let file = name.rsplit(['/', '!']).next().unwrap_or(name);
    file.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .filter(|e| !e.is_empty())
}

fn is_archive(path: &str) -> bool {
    extension(path).is_some_and(|e| {
        matches!(
            e.as_str(),
            "key" | "bif" | "erf" | "mod" | "sav" | "hak" | "rim"
        )
    })
}

/// One file or link of a tree.
#[derive(Debug, Clone)]
pub(crate) struct Walked {
    /// The path from the tree's root, `/`-separated.
    pub rel: String,
    pub what: Kind,
}

#[derive(Debug, Clone)]
pub(crate) enum Kind {
    File {
        path: PathBuf,
        /// Device and inode: two names of one file hold the same bytes.
        id: (u64, u64),
    },
    /// A symbolic link to a directory, and where it points.
    Link { target: PathBuf },
}

/// Every file under `root`, at any depth, sorted by path.
fn walk(root: &Path) -> Result<Vec<Walked>, Diagnostic> {
    fn go(dir: &Path, rel: &str, out: &mut Vec<Walked>) -> std::io::Result<()> {
        for e in fs::read_dir(dir)? {
            let e = e?;
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            let here = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            let lmeta = fs::symlink_metadata(&path)?;
            if lmeta.file_type().is_symlink() {
                match fs::metadata(&path) {
                    Ok(m) if m.is_dir() => {
                        out.push(Walked {
                            rel: here,
                            what: Kind::Link {
                                target: fs::read_link(&path)?,
                            },
                        });
                        continue;
                    }
                    Ok(m) => {
                        out.push(file(here, path, &m));
                        continue;
                    }
                    // A dangling link: compared by where it points.
                    Err(_) => {
                        out.push(Walked {
                            rel: here,
                            what: Kind::Link {
                                target: fs::read_link(&path)?,
                            },
                        });
                        continue;
                    }
                }
            }
            if lmeta.is_dir() {
                go(&path, &here, out)?;
            } else {
                out.push(file(here, path, &lmeta));
            }
        }
        Ok(())
    }
    fn file(rel: String, path: PathBuf, m: &fs::Metadata) -> Walked {
        #[cfg(unix)]
        let id = {
            use std::os::unix::fs::MetadataExt;
            (m.dev(), m.ino())
        };
        #[cfg(not(unix))]
        let id = (0, 0);
        Walked {
            rel,
            what: Kind::File { path, id },
        }
    }
    let mut out = Vec::new();
    go(root, "", &mut out).map_err(|e| {
        Diagnostic::new(
            &codes::RE0001,
            format!("cannot read the tree `{}`: {e}", root.display()),
        )
    })?;
    out.sort_by(|x, y| x.rel.cmp(&y.rel));
    Ok(out)
}

/// The two trees' entries paired: the exact spelling first, then a
/// spelling that differs only in case; what is left over stands alone.
/// Sorted by the second side's path where there is one.
fn pair<'w>(a: &'w [Walked], b: &'w [Walked]) -> Vec<(Option<&'w Walked>, Option<&'w Walked>)> {
    let mut exact: HashMap<&str, usize> = HashMap::new();
    let mut folded: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, e) in b.iter().enumerate() {
        exact.insert(&e.rel, i);
        folded
            .entry(e.rel.to_ascii_lowercase())
            .or_default()
            .push(i);
    }
    let mut used = vec![false; b.len()];
    let mut partner: Vec<Option<usize>> = vec![None; a.len()];
    for (i, e) in a.iter().enumerate() {
        if let Some(&j) = exact.get(e.rel.as_str()) {
            used[j] = true;
            partner[i] = Some(j);
        }
    }
    for (i, e) in a.iter().enumerate() {
        if partner[i].is_some() {
            continue;
        }
        if let Some(js) = folded.get(&e.rel.to_ascii_lowercase())
            && let Some(&j) = js.iter().find(|&&j| !used[j])
        {
            used[j] = true;
            partner[i] = Some(j);
        }
    }
    let mut out: Vec<(Option<&Walked>, Option<&Walked>)> = a
        .iter()
        .zip(&partner)
        .map(|(e, p)| (Some(e), p.map(|j| &b[j])))
        .collect();
    out.extend(
        b.iter()
            .zip(&used)
            .filter(|(_, u)| !**u)
            .map(|(e, _)| (None, Some(e))),
    );
    out
}

/// The facts of a side's bytes, with the header fields known for its
/// extension.
fn facts(name: &str, bytes: &[u8]) -> Facts {
    Facts {
        size: bytes.len() as u64,
        sha256: ContentHash::of(bytes).to_hex(),
        header: compare::header_fields(extension(name).as_deref(), bytes, bytes.len() as u64),
    }
}

/// How much of a file's head [`file_facts`] keeps for its header fields.
const HEAD: usize = 64 * 1024;

/// The facts of a file read in pieces, its bytes not held.
fn file_facts(name: &str, path: &Path) -> std::io::Result<Facts> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut f = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut head = Vec::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if head.len() < HEAD {
            let take = n.min(HEAD - head.len());
            head.extend_from_slice(&buf[..take]);
        }
        hash.update(&buf[..n]);
        size += n as u64;
    }
    Ok(Facts {
        size,
        sha256: hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
        header: compare::header_fields(extension(name).as_deref(), &head, size),
    })
}

/// Whether two files hold the same bytes, read in pieces.
fn same_bytes(a: &Path, b: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    if fs::metadata(a)?.len() != fs::metadata(b)?.len() {
        return Ok(false);
    }
    let (mut fa, mut fb) = (fs::File::open(a)?, fs::File::open(b)?);
    let (mut ba, mut bb) = (vec![0u8; 1 << 20], vec![0u8; 1 << 20]);
    loop {
        let n = fa.read(&mut ba)?;
        if n == 0 {
            return Ok(true);
        }
        fb.read_exact(&mut bb[..n])?;
        if ba[..n] != bb[..n] {
            return Ok(false);
        }
    }
}

/// The copies of an Override file's resource that an install loads in its
/// place: the one it loads outside any module (`patch.erf`, the BIFs) and
/// the one each module holding the resource loads, distinct by bytes, each
/// as a location name [`inspect::load`] takes back.
fn copies_elsewhere(
    install: &Install,
    modules: &mut Option<HashMap<ResId, Vec<ModuleName>>>,
    id: ResId,
) -> Vec<(String, Vec<u8>)> {
    let index = modules.get_or_insert_with(|| {
        let mut m: HashMap<ResId, Vec<ModuleName>> = HashMap::new();
        for module in install.modules() {
            for rid in install.module_ids(module).unwrap_or_default() {
                m.entry(rid).or_default().push(module.clone());
            }
        }
        m
    });
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut add = |found: Option<(crate::game::Location, Vec<u8>)>| {
        if let Some((location, bytes)) = found
            && !out.iter().any(|(_, b)| *b == bytes)
        {
            out.push((location_name(install.root(), &location, id), bytes));
        }
    };
    add(install.read(id, None).ok().flatten());
    for module in index.get(&id).into_iter().flatten() {
        add(install.read(id, Some(module)).ok().flatten());
    }
    out
}

/// An install's location as a name [`inspect::load`] takes back.
fn location_name(root: &Path, l: &crate::game::Location, id: ResId) -> String {
    let shown = |p: &Path| p.strip_prefix(root).unwrap_or(p).display().to_string();
    match l {
        crate::game::Location::File { path } => shown(path),
        crate::game::Location::Capsule { path, member } => format!("{}!{member}", shown(path)),
        crate::game::Location::Bif { path, index, .. } => {
            format!("{}!#{index}.{}", shown(path), id.ty)
        }
    }
}

#[cfg(test)]
mod tests;
