//! The database file: loaded into memory whole, written back whole, and
//! opened read-only for queries.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::error::{Error, Result};
use crate::schema;

/// The database at `db` in memory, or an empty one when there is no file.
pub(crate) fn load(db: &Path) -> Result<Connection> {
    let mut conn = Connection::open_in_memory().map_err(Error::db("opening memory"))?;
    if db.exists() {
        conn.restore(
            rusqlite::MAIN_DB,
            db,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(Error::db(format!("loading {}", db.display())))?;
    }
    Ok(conn)
}

/// Whether the database has no tables yet.
pub(crate) fn is_empty(conn: &Connection) -> Result<bool> {
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
            [],
            |r| r.get(0),
        )
        .map_err(Error::db("reading the schema"))?;
    Ok(n == 0)
}

/// Fails unless the database is of this build's schema.
pub(crate) fn check_schema(conn: &Connection, db: &Path) -> Result<()> {
    let found: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |r| {
            r.get(0)
        })
        .optional()
        .unwrap_or(None);
    match found {
        Some(v) if v == schema::VERSION.to_string() => Ok(()),
        other => Err(Error::Schema {
            path: db.to_path_buf(),
            found: other.unwrap_or_else(|| "(none)".into()),
            expected: schema::VERSION,
        }),
    }
}

/// The database at `db`, read-only, its schema checked.
pub fn open(db: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(Error::db(format!("opening {}", db.display())))?;
    check_schema(&conn, db)?;
    Ok(conn)
}

/// Every tree label, in the order the trees were added.
pub(crate) fn labels(conn: &Connection) -> Result<Vec<String>> {
    let mut st = conn
        .prepare("SELECT label FROM trees ORDER BY id")
        .map_err(Error::db("listing the trees"))?;
    let rows = st
        .query_map([], |r| r.get(0))
        .map_err(Error::db("listing the trees"))?;
    rows.collect::<rusqlite::Result<Vec<String>>>()
        .map_err(Error::db("listing the trees"))
}

/// The id of the tree labelled `label`.
pub(crate) fn tree_id(conn: &Connection, label: &str) -> Result<i64> {
    let id: Option<i64> = conn
        .query_row(
            "SELECT id FROM trees WHERE label = ?1",
            params![label],
            |r| r.get(0),
        )
        .optional()
        .map_err(Error::db("looking up the tree"))?;
    match id {
        Some(id) => Ok(id),
        None => Err(Error::NoSuchTree {
            label: label.to_string(),
            known: labels(conn)?,
        }),
    }
}

/// Drop the contents no location holds any more, with their facts.
pub(crate) fn drop_orphans(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DELETE FROM contents WHERE hash NOT IN (SELECT hash FROM locations);
         DELETE FROM resources WHERE hash NOT IN (SELECT hash FROM contents);",
    )
    .map_err(Error::db("dropping unreferenced contents"))?;
    for (table, col) in schema::FACT_TABLES {
        conn.execute_batch(&format!(
            "DELETE FROM {table} WHERE {col} NOT IN (SELECT hash FROM contents)"
        ))
        .map_err(Error::db(format!("dropping unreferenced rows of {table}")))?;
    }
    Ok(())
}

/// The tables whose rows `rakata index build` counts.
pub const COUNTED: &[&str] = &[
    "trees",
    "resources",
    "contents",
    "locations",
    "coverage",
    "dialog_nodes",
    "dialog_links",
    "gff_fields",
    "cells",
    "strings",
    "calls",
    "script_strings",
    "refs",
    "search",
];

/// Rows per table.
pub(crate) fn row_counts(conn: &Connection) -> Result<BTreeMap<String, u64>> {
    let mut out = BTreeMap::new();
    for t in COUNTED {
        let n: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get(0))
            .map_err(Error::db(format!("counting {t}")))?;
        out.insert((*t).to_string(), n as u64);
    }
    Ok(out)
}

/// Write the database to `db`, compacted, replacing any file there: to a
/// file beside it first, then renamed over it. Returns the size written.
pub(crate) fn save(conn: &Connection, db: &Path) -> Result<u64> {
    if let Some(dir) = db.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(|source| Error::Io {
            what: format!("creating {}", dir.display()),
            source,
        })?;
    }
    let mut tmp = db.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = std::path::PathBuf::from(tmp);
    let _ = fs::remove_file(&tmp);
    conn.execute("VACUUM INTO ?1", params![tmp.to_string_lossy()])
        .map_err(Error::db(format!("writing {}", tmp.display())))?;
    fs::rename(&tmp, db).map_err(|source| Error::Io {
        what: format!("renaming {} to {}", tmp.display(), db.display()),
        source,
    })?;
    Ok(fs::metadata(db).map_or(0, |m| m.len()))
}
