//! Building the index of one tree: cold, or from a parent tree the
//! database already holds and the paths that changed since.
//!
//! The database is loaded into memory (or created there), every file the
//! tree's walk names is either carried over from the parent or read, and
//! the result is written to the file once, at the end. Reading is spread
//! over threads, one file at a time; each content hash is parsed by the
//! first thread to meet it and never again, whichever tree, path or
//! archive member holds it, and a hash the database already holds is not
//! parsed at all. One thread writes.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Instant;

use rakata_archive::{Bif, Erf, Key, Rim, Source};
use rakata_res::{ContentHash, ResId, ResType};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::error::{Error, Result, Warning, codes};
use crate::extract::{self, Extracted, Routine, Status};
use crate::schema;
use crate::store;

/// What to build.
#[derive(Debug, Clone)]
pub struct BuildOptions<'a> {
    /// The tree: the folder holding `chitin.key`.
    pub root: &'a Path,
    /// The files of the tree the game loads resources from, as
    /// `rakata::conformance::walk` lists them, under `root`.
    pub files: &'a [PathBuf],
    /// The database file, created when absent.
    pub db: &'a Path,
    /// The tree's label; a tree the database holds under it is replaced.
    pub label: &'a str,
    /// The tree this one was made from, already in the database: files the
    /// changed list does not name are carried over from it unread.
    pub parent: Option<&'a str>,
    /// The paths, relative to `root`, that differ from the parent's
    /// ([`parse_changed`]). Without a parent it is not consulted.
    pub changed: Option<&'a [String]>,
    /// The engine's routines, by number, from the tree's `nwscript.nss`.
    pub routines: &'a [Routine],
    /// Threads to read with; 0 for one per CPU.
    pub threads: usize,
}

/// The paths of a changed-paths file: one per line, optionally after a
/// one-character mark and a space (`+ ` added, `~ ` changed, `- `
/// removed, as a modshop checkpoint's `changed.txt` writes them). Removed
/// paths are left out, since the tree no longer holds them; blank lines and
/// `#` comments are skipped.
pub fn parse_changed(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let b = line.as_bytes();
        let (mark, path) = if b.len() > 2 && b[1] == b' ' && b"+~-".contains(&b[0]) {
            (b[0], &line[2..])
        } else {
            (b'~', line)
        };
        if mark != b'-' {
            out.push(path.replace('\\', "/"));
        }
    }
    out
}

/// What a build did, with its timings.
#[derive(Debug, Clone, Default, Serialize)]
pub struct BuildReport {
    pub tree: String,
    pub parent: Option<String>,
    /// The database file written.
    pub db: String,
    /// Whether a tree of the same label was replaced.
    pub replaced: bool,
    /// Files of the walk, and of those how many were carried over from the
    /// parent and how many read.
    pub files: usize,
    pub files_reused: usize,
    pub files_read: usize,
    /// Archive members listed in the files read.
    pub members_read: usize,
    /// Contents parsed by this build: hashes the database did not hold.
    pub hashes_parsed: usize,
    /// Locations read whose content the database or another location of
    /// this build already held, so it was not parsed again.
    pub hashes_reused: usize,
    /// Files of the tree outside the walk, recorded and not read.
    pub outside: usize,
    /// The tree's coverage rows by status.
    pub coverage: BTreeMap<String, usize>,
    /// Files and members with no reader, by extension.
    pub no_reader: BTreeMap<String, usize>,
    /// Rows per table over the whole database.
    pub rows: BTreeMap<String, u64>,
    /// The database file's size in bytes.
    pub db_bytes: u64,
    pub seconds: Timings,
    pub warnings: Vec<Warning>,
}

/// Where the time went, in seconds.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Timings {
    /// Loading or creating the database in memory.
    pub load: f64,
    /// Listing the tree and carrying over the parent's rows.
    pub plan: f64,
    /// Reading, hashing, parsing and inserting.
    pub read: f64,
    /// Coverage, indexes and counts.
    pub finish: f64,
    /// Writing the file.
    pub save: f64,
    pub total: f64,
}

/// One row a reader sends the writer.
enum Msg {
    Location {
        path: String,
        member: String,
        name: String,
        ty: ResType,
        hash: String,
        size: usize,
        known: bool,
    },
    Content {
        hash: String,
        ty: ResType,
        x: Extracted,
    },
    Unread {
        path: String,
        reason: String,
    },
}

/// Files this large are memory-mapped rather than read whole.
const MAP_THRESHOLD: u64 = 1 << 20;

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every regular file under `root`, symlinked directories not followed,
/// as paths relative to `root`.
fn all_files(root: &Path) -> Vec<String> {
    fn go(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries.filter_map(std::result::Result::ok) {
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() {
                go(root, &p, out);
            } else if ft.is_symlink() {
                if p.metadata().is_ok_and(|m| m.is_file()) {
                    out.push(rel(root, &p));
                }
            } else {
                out.push(rel(root, &p));
            }
        }
    }
    let mut out = Vec::new();
    go(root, root, &mut out);
    out.sort();
    out
}

/// The name and type of a file from its name: the resref is the text
/// before the last dot, lowercased.
fn ident(file: &str) -> (String, ResType) {
    let base = file.rsplit('/').next().unwrap_or(file);
    match base.rsplit_once('.') {
        Some((stem, ext)) => (
            stem.to_ascii_lowercase(),
            ResType::from_extension(ext).unwrap_or(ResType::Unknown(u16::MAX)),
        ),
        None => (base.to_ascii_lowercase(), ResType::Unknown(u16::MAX)),
    }
}

/// The text a type is stored as: its extension, or `type<id>`.
pub(crate) fn type_name(ty: ResType, file: &str) -> String {
    match ty {
        ResType::Unknown(u16::MAX) => file
            .rsplit('/')
            .next()
            .and_then(|b| b.rsplit_once('.'))
            .map_or_else(String::new, |(_, e)| e.to_ascii_lowercase()),
        ty => ty.to_string(),
    }
}

/// The parse-once ledger: the hashes the database held when the build
/// began and those claimed since.
struct Claims {
    seen: Mutex<HashSet<String>>,
}

impl Claims {
    /// True for the first caller to present `hash`.
    fn claim(&self, hash: &str) -> bool {
        self.seen
            .lock()
            .expect("no reader panicked")
            .insert(hash.to_string())
    }
}

/// Reading one file: its location, its content when new, and its members.
struct Reader<'a> {
    routines: &'a [Routine],
    claims: &'a Claims,
    /// For each BIF the KEY names, by lowercased path, its members' ids by
    /// position in the BIF's table.
    bif_names: &'a HashMap<String, HashMap<u32, ResId>>,
}

impl Reader<'_> {
    fn read(&self, abs: &Path, path: &str, out: &mut Vec<Msg>) {
        let size = fs::metadata(abs).map_or(0, |m| m.len());
        let source = if size >= MAP_THRESHOLD {
            Source::map(abs)
        } else {
            Source::read(abs)
        };
        let src = match source {
            Ok(s) => s,
            Err(e) => {
                out.push(Msg::Unread {
                    path: path.to_string(),
                    reason: e.to_string(),
                });
                return;
            }
        };
        let bytes = src.bytes();
        let (name, ty) = ident(path);
        let hash = ContentHash::of(bytes).to_hex();
        let fresh = self.claims.claim(&hash);
        out.push(Msg::Location {
            path: path.to_string(),
            member: String::new(),
            name: name.clone(),
            ty,
            hash: hash.clone(),
            size: bytes.len(),
            known: !fresh,
        });
        if extract::is_archive(ty) {
            let status = self.members(path, ty, bytes, out);
            if fresh {
                let (status, reason) = match status {
                    Ok(()) => (Status::Container, None),
                    Err(e) => (Status::Failed, Some(e)),
                };
                out.push(Msg::Content {
                    hash,
                    ty,
                    x: Extracted {
                        status: Some(status),
                        reason,
                        rows: Vec::new(),
                    },
                });
            }
        } else if fresh {
            let x = extract::extract(ty, &name, bytes, self.routines);
            out.push(Msg::Content { hash, ty, x });
        }
    }

    /// List an archive's members, parsing each new content.
    fn members(
        &self,
        path: &str,
        ty: ResType,
        bytes: &[u8],
        out: &mut Vec<Msg>,
    ) -> std::result::Result<(), String> {
        let mut member = |label: String, name: String, mty: ResType, data: &[u8]| {
            let hash = ContentHash::of(data).to_hex();
            let fresh = self.claims.claim(&hash);
            out.push(Msg::Location {
                path: path.to_string(),
                member: label,
                name: name.clone(),
                ty: mty,
                hash: hash.clone(),
                size: data.len(),
                known: !fresh,
            });
            if fresh {
                let x = if extract::is_archive(mty) {
                    Extracted {
                        status: Some(Status::NoReader),
                        reason: Some("an archive inside an archive is not opened".into()),
                        rows: Vec::new(),
                    }
                } else {
                    extract::extract(mty, &name, data, self.routines)
                };
                out.push(Msg::Content { hash, ty: mty, x });
            }
        };
        let id_label = |id: &ResId| {
            let n = String::from_utf8_lossy(id.name.as_bytes()).to_ascii_lowercase();
            (format!("{n}.{}", id.ty), n)
        };
        match ty {
            ResType::Key => Key::parse(bytes).map(|_| ()).map_err(|e| e.to_string()),
            ResType::Bif => {
                let bif = Bif::parse(bytes).map_err(|e| e.to_string())?;
                let names = self.bif_names.get(&path.to_ascii_lowercase());
                for (i, m) in bif.members().iter().enumerate() {
                    let id = names.and_then(|n| n.get(&(i as u32)));
                    let (label, name) = match id {
                        Some(id) => id_label(id),
                        None => (format!("#{i}.{}", m.ty), String::new()),
                    };
                    member(label, name, m.ty, m.data());
                }
                Ok(())
            }
            ResType::Rim => {
                let rim = Rim::parse(bytes).map_err(|e| e.to_string())?;
                for m in rim.members() {
                    let (label, name) = id_label(&m.id);
                    member(label, name, m.id.ty, m.data());
                }
                Ok(())
            }
            _ => {
                let erf = Erf::parse(bytes).map_err(|e| e.to_string())?;
                for m in erf.members() {
                    let (label, name) = id_label(&m.id);
                    member(label, name, m.id.ty, m.data());
                }
                Ok(())
            }
        }
    }
}

/// The BIF members' names from `chitin.key`, by the lowercased path of
/// each BIF under the tree and the member's position.
fn bif_names(root: &Path, files: &[PathBuf]) -> HashMap<String, HashMap<u32, ResId>> {
    let mut out: HashMap<String, HashMap<u32, ResId>> = HashMap::new();
    let Some(key_path) = files.iter().find(|p| {
        p.file_name()
            .is_some_and(|f| f.eq_ignore_ascii_case("chitin.key"))
    }) else {
        return out;
    };
    let Ok(bytes) = fs::read(key_path) else {
        return out;
    };
    let Ok(key) = Key::parse(&bytes) else {
        return out;
    };
    let base = key_path
        .parent()
        .map_or_else(String::new, |p| rel(root, p).to_ascii_lowercase());
    let paths: Vec<String> = key
        .bifs()
        .iter()
        .map(|b| {
            let p = b
                .components()
                .collect::<Vec<_>>()
                .join("/")
                .to_ascii_lowercase();
            if base.is_empty() {
                p
            } else {
                format!("{base}/{p}")
            }
        })
        .collect();
    for e in key.entries() {
        if let Some(p) = paths.get(e.bif as usize) {
            out.entry(p.clone()).or_default().insert(e.index, e.id);
        }
    }
    out
}

/// Build the index of one tree into `o.db`.
pub fn build(o: &BuildOptions) -> Result<BuildReport> {
    let start = Instant::now();
    let mut report = BuildReport {
        tree: o.label.to_string(),
        parent: o.parent.map(str::to_string),
        db: o.db.display().to_string(),
        ..BuildReport::default()
    };

    // Load.
    let mut conn = store::load(o.db)?;
    let fresh = store::is_empty(&conn)?;
    if fresh {
        conn.execute_batch(schema::TABLES)
            .map_err(Error::db("creating the tables"))?;
        conn.execute(
            "INSERT INTO meta VALUES ('schema', ?1)",
            params![schema::VERSION.to_string()],
        )
        .map_err(Error::db("recording the schema version"))?;
    } else {
        store::check_schema(&conn, o.db)?;
    }
    report.seconds.load = start.elapsed().as_secs_f64();

    // Plan.
    let plan_start = Instant::now();
    let parent_id = match o.parent {
        Some(p) => Some(store::tree_id(&conn, p)?),
        None => None,
    };
    let root_text = o.root.display().to_string();
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM trees WHERE label = ?1",
            params![o.label],
            |r| r.get(0),
        )
        .optional()
        .map_err(Error::db("looking up the tree"))?;
    let tree = match existing {
        Some(id) => {
            report.replaced = true;
            conn.execute("DELETE FROM locations WHERE tree = ?1", params![id])
                .map_err(Error::db("dropping the tree's locations"))?;
            conn.execute("DELETE FROM coverage WHERE tree = ?1", params![id])
                .map_err(Error::db("dropping the tree's coverage"))?;
            conn.execute(
                "UPDATE trees SET parent = ?2, root = ?3 WHERE id = ?1",
                params![id, parent_id, root_text],
            )
            .map_err(Error::db("updating the tree"))?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO trees(label, parent, root) VALUES (?1, ?2, ?3)",
                params![o.label, parent_id, root_text],
            )
            .map_err(Error::db("adding the tree"))?;
            conn.last_insert_rowid()
        }
    };

    let walked: Vec<(PathBuf, String)> = o
        .files
        .iter()
        .map(|p| (p.clone(), rel(o.root, p)))
        .collect();
    report.files = walked.len();
    let walked_lower: HashSet<String> =
        walked.iter().map(|(_, r)| r.to_ascii_lowercase()).collect();
    let everything = all_files(o.root);
    let outside: Vec<&String> = everything
        .iter()
        .filter(|r| !walked_lower.contains(&r.to_ascii_lowercase()))
        .collect();
    report.outside = outside.len();

    let changed: HashSet<String> = o
        .changed
        .unwrap_or(&[])
        .iter()
        .map(|p| p.to_ascii_lowercase())
        .collect();
    if o.changed.is_some() {
        let all_lower: HashSet<String> =
            everything.iter().map(|r| r.to_ascii_lowercase()).collect();
        let mut missing: Vec<&String> = o
            .changed
            .unwrap_or(&[])
            .iter()
            .filter(|p| !all_lower.contains(&p.to_ascii_lowercase()))
            .collect();
        missing.sort();
        if !missing.is_empty() {
            report.warnings.push(Warning {
                code: codes::CHANGED_MISSING,
                message: format!(
                    "{} changed path(s) the tree does not hold, such as `{}`",
                    missing.len(),
                    missing[0]
                ),
            });
        }
    }

    // Carry the parent's rows over for every file the changed list does not
    // name.
    let mut to_read: Vec<(PathBuf, String)> = Vec::new();
    if let Some(pid) = parent_id {
        let mut parent_paths: HashMap<String, String> = HashMap::new();
        {
            let mut st = conn
                .prepare("SELECT DISTINCT path FROM locations WHERE tree = ?1")
                .map_err(Error::db("listing the parent's paths"))?;
            let rows = st
                .query_map(params![pid], |r| r.get::<_, String>(0))
                .map_err(Error::db("listing the parent's paths"))?;
            for p in rows {
                let p = p.map_err(Error::db("listing the parent's paths"))?;
                parent_paths.insert(p.to_ascii_lowercase(), p);
            }
        }
        conn.execute_batch(
            "CREATE TEMP TABLE carry(old TEXT PRIMARY KEY, new TEXT NOT NULL) WITHOUT ROWID",
        )
        .map_err(Error::db("planning the carry-over"))?;
        let mut unlisted: Vec<String> = Vec::new();
        {
            let tx = conn
                .transaction()
                .map_err(Error::db("planning the carry-over"))?;
            {
                let mut ins = tx
                    .prepare("INSERT OR IGNORE INTO carry VALUES (?1, ?2)")
                    .map_err(Error::db("planning the carry-over"))?;
                for (abs, r) in &walked {
                    let lower = r.to_ascii_lowercase();
                    match parent_paths.get(&lower) {
                        Some(old) if !changed.contains(&lower) => {
                            ins.execute(params![old, r])
                                .map_err(Error::db("planning the carry-over"))?;
                            report.files_reused += 1;
                        }
                        Some(_) => to_read.push((abs.clone(), r.clone())),
                        None => {
                            if !changed.contains(&lower) {
                                unlisted.push(r.clone());
                            }
                            to_read.push((abs.clone(), r.clone()));
                        }
                    }
                }
            }
            tx.execute(
                "INSERT INTO locations
                 SELECT ?1, c.new, l.member, l.name, l.type, l.hash
                 FROM locations l JOIN carry c ON c.old = l.path
                 WHERE l.tree = ?2",
                params![tree, pid],
            )
            .map_err(Error::db("carrying the parent's locations over"))?;
            tx.execute_batch("DROP TABLE carry")
                .map_err(Error::db("planning the carry-over"))?;
            tx.commit()
                .map_err(Error::db("carrying the parent's locations over"))?;
        }
        if !unlisted.is_empty() {
            unlisted.sort();
            report.warnings.push(Warning {
                code: codes::UNLISTED,
                message: format!(
                    "{} file(s) the parent tree does not hold are not in the changed list, such as `{}`; they were read",
                    unlisted.len(),
                    unlisted[0]
                ),
            });
        }
    } else {
        to_read = walked.clone();
    }
    report.files_read = to_read.len();
    report.seconds.plan = plan_start.elapsed().as_secs_f64();

    // Read.
    let read_start = Instant::now();
    let mut known: HashSet<String> = HashSet::new();
    {
        let mut st = conn
            .prepare("SELECT hash FROM contents")
            .map_err(Error::db("listing the known contents"))?;
        let rows = st
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(Error::db("listing the known contents"))?;
        for h in rows {
            known.insert(h.map_err(Error::db("listing the known contents"))?);
        }
    }
    let claims = Claims {
        seen: Mutex::new(known),
    };
    let names = bif_names(o.root, o.files);
    let reader = Reader {
        routines: o.routines,
        claims: &claims,
        bif_names: &names,
    };
    // Largest first, so the BIFs do not finish last.
    to_read.sort_by_key(|(abs, _)| std::cmp::Reverse(fs::metadata(abs).map_or(0, |m| m.len())));
    let threads = match o.threads {
        0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
        n => n,
    }
    .min(to_read.len().max(1));
    let next = AtomicUsize::new(0);
    let (tx_msgs, rx_msgs) = mpsc::sync_channel::<Vec<Msg>>(threads * 4);
    let mut unread: Vec<(String, String)> = Vec::new();
    let write_result: Result<()> = std::thread::scope(|s| {
        for _ in 0..threads {
            let tx_msgs = tx_msgs.clone();
            let reader = &reader;
            let next = &next;
            let to_read = &to_read;
            s.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((abs, r)) = to_read.get(i) else {
                        break;
                    };
                    let mut out = Vec::new();
                    reader.read(abs, r, &mut out);
                    if tx_msgs.send(out).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx_msgs);
        let txn = conn
            .transaction()
            .map_err(Error::db("starting the write"))?;
        let mut failure = None;
        for batch in rx_msgs {
            if failure.is_some() {
                continue;
            }
            if let Err(e) = write(&txn, tree, batch, &mut report, &mut unread) {
                failure = Some(e);
            }
        }
        if let Some(e) = failure {
            return Err(e);
        }
        txn.commit().map_err(Error::db("finishing the write"))
    });
    write_result?;
    report.seconds.read = read_start.elapsed().as_secs_f64();

    // Finish: coverage, the indexes of a new database, counts.
    let finish_start = Instant::now();
    {
        let txn = conn
            .transaction()
            .map_err(Error::db("recording coverage"))?;
        txn.execute(
            "INSERT OR IGNORE INTO coverage
             SELECT l.tree, l.path, l.member, c.status, c.reason
             FROM locations l JOIN contents c ON c.hash = l.hash
             WHERE l.tree = ?1",
            params![tree],
        )
        .map_err(Error::db("recording coverage"))?;
        {
            let mut ins = txn
                .prepare("INSERT OR IGNORE INTO coverage VALUES (?1, ?2, '', ?3, ?4)")
                .map_err(Error::db("recording coverage"))?;
            unread.sort();
            for (path, reason) in &unread {
                ins.execute(params![tree, path, Status::Failed.name(), reason])
                    .map_err(Error::db("recording coverage"))?;
            }
            for path in &outside {
                ins.execute(params![
                    tree,
                    path,
                    Status::Outside.name(),
                    Option::<String>::None
                ])
                .map_err(Error::db("recording coverage"))?;
            }
        }
        txn.commit().map_err(Error::db("recording coverage"))?;
    }
    if !unread.is_empty() {
        report.warnings.push(Warning {
            code: codes::UNREADABLE,
            message: format!(
                "{} file(s) of the tree could not be read, such as `{}`: {}",
                unread.len(),
                unread[0].0,
                unread[0].1
            ),
        });
    }
    if fresh {
        conn.execute_batch(schema::INDEXES)
            .map_err(Error::db("creating the indexes"))?;
    }
    if report.replaced {
        store::drop_orphans(&conn)?;
    }
    {
        let mut st = conn
            .prepare("SELECT status, count(*) FROM coverage WHERE tree = ?1 GROUP BY status")
            .map_err(Error::db("counting coverage"))?;
        let rows = st
            .query_map(params![tree], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(Error::db("counting coverage"))?;
        for row in rows {
            let (k, n) = row.map_err(Error::db("counting coverage"))?;
            report.coverage.insert(k, n as usize);
        }
        let mut st = conn
            .prepare(
                "SELECT l.type, count(*) FROM coverage v
                 JOIN locations l ON l.tree = v.tree AND l.path = v.path AND l.member = v.member
                 WHERE v.tree = ?1 AND v.status = 'no_reader' GROUP BY l.type",
            )
            .map_err(Error::db("counting coverage"))?;
        let rows = st
            .query_map(params![tree], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(Error::db("counting coverage"))?;
        for row in rows {
            let (k, n) = row.map_err(Error::db("counting coverage"))?;
            report
                .no_reader
                .insert(if k.is_empty() { "(none)".into() } else { k }, n as usize);
        }
    }
    report.rows = store::row_counts(&conn)?;
    report.seconds.finish = finish_start.elapsed().as_secs_f64();

    // Save.
    let save_start = Instant::now();
    report.db_bytes = store::save(&conn, o.db)?;
    report.seconds.save = save_start.elapsed().as_secs_f64();
    report.seconds.total = start.elapsed().as_secs_f64();
    Ok(report)
}

/// Insert one reader's batch.
fn write(
    txn: &rusqlite::Transaction<'_>,
    tree: i64,
    batch: Vec<Msg>,
    report: &mut BuildReport,
    unread: &mut Vec<(String, String)>,
) -> Result<()> {
    for m in batch {
        match m {
            Msg::Location {
                path,
                member,
                name,
                ty,
                hash,
                size,
                known,
            } => {
                let ty = type_name(ty, if member.is_empty() { &path } else { &member });
                if !member.is_empty() {
                    report.members_read += 1;
                }
                if known {
                    report.hashes_reused += 1;
                }
                txn.prepare_cached("INSERT OR IGNORE INTO locations VALUES (?1,?2,?3,?4,?5,?6)")
                    .and_then(|mut s| s.execute(params![tree, path, member, name, ty, hash]))
                    .map_err(Error::db("recording a location"))?;
                txn.prepare_cached("INSERT OR IGNORE INTO resources VALUES (?1,?2,?3,?4,?5)")
                    .and_then(|mut s| s.execute(params![hash, name, ty, size as i64, tree]))
                    .map_err(Error::db("recording a resource"))?;
            }
            Msg::Content { hash, ty, x } => {
                report.hashes_parsed += 1;
                let status = x.status.unwrap_or(Status::NoReader);
                txn.prepare_cached("INSERT OR IGNORE INTO contents VALUES (?1,?2,?3,?4)")
                    .and_then(|mut s| {
                        s.execute(params![hash, type_name(ty, ""), status.name(), x.reason])
                    })
                    .map_err(Error::db("recording a content"))?;
                for (table, row) in x.rows {
                    let mut st = txn
                        .prepare_cached(table.insert())
                        .map_err(Error::db("preparing a fact"))?;
                    let mut values: Vec<rusqlite::types::Value> = Vec::with_capacity(row.len() + 1);
                    values.push(rusqlite::types::Value::Text(hash.clone()));
                    values.extend(row);
                    st.execute(rusqlite::params_from_iter(values))
                        .map_err(Error::db(format!("recording a fact in {table:?}")))?;
                }
            }
            Msg::Unread { path, reason } => unread.push((path, reason)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_lists_take_marks_and_drop_removals() {
        let text = "~ Override/appearance.2da\n+ Override/dan14_juhani.dlg\n- Override/gone.ncs\n\n# note\nmodules\\danm14ac.mod\n";
        assert_eq!(
            parse_changed(text),
            [
                "Override/appearance.2da",
                "Override/dan14_juhani.dlg",
                "modules/danm14ac.mod"
            ]
        );
    }

    #[test]
    fn names_and_types_come_from_the_file_name() {
        assert_eq!(
            ident("Override/K_PMAN_AIOFF.ncs"),
            ("k_pman_aioff".into(), ResType::Ncs)
        );
        assert_eq!(type_name(ident("swkotor.exe").1, "swkotor.exe"), "exe");
        assert_eq!(type_name(ResType::Dlg, ""), "dlg");
    }
}
