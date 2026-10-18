//! The content digest: a SHA-256 over what the rows say, never over where
//! or in what order they are stored, so two databases that hold the same
//! facts about a tree have the same digest however they were built.
//!
//! The digest reads a fixed list of sections. Each section is its name and
//! then its rows, sorted by every column; each row is the byte `r` and each
//! value a tag byte and its bytes: `N` for null, `I` and eight big-endian
//! bytes for an integer, `R` and the eight big-endian bytes of a real's
//! bits, `T` or `B`, an eight-byte big-endian length and the bytes for text
//! or a blob. Trees are named by label, never by their row ids. The input
//! begins with `rakata-index/v<schema>` and the scope, `all` or `tree`.
//!
//! For one tree the sections are its locations, its coverage, the
//! resources and contents its locations hold, and every fact row of those
//! contents. Which tree first held a content, and a tree's parent and
//! root, are left out: they record how the database was built, not what
//! the tree is. For the whole database the sections are every tree with
//! its parent's label, and every row of every table, with each location,
//! coverage row and resource's tree by label.

use rusqlite::types::ValueRef;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::schema;
use crate::store;

fn feed(h: &mut Sha256, v: ValueRef<'_>) {
    match v {
        ValueRef::Null => h.update(b"N"),
        ValueRef::Integer(i) => {
            h.update(b"I");
            h.update(i.to_be_bytes());
        }
        ValueRef::Real(f) => {
            h.update(b"R");
            h.update(f.to_bits().to_be_bytes());
        }
        ValueRef::Text(t) => {
            h.update(b"T");
            h.update((t.len() as u64).to_be_bytes());
            h.update(t);
        }
        ValueRef::Blob(b) => {
            h.update(b"B");
            h.update((b.len() as u64).to_be_bytes());
            h.update(b);
        }
    }
}

/// Feed one section: its name, then every row of `sql` (which orders its
/// own rows), bound to `tree` when it takes a parameter.
fn section(
    conn: &Connection,
    h: &mut Sha256,
    name: &str,
    sql: &str,
    tree: Option<i64>,
) -> Result<()> {
    h.update(b"s");
    h.update((name.len() as u64).to_be_bytes());
    h.update(name.as_bytes());
    let what = || format!("digesting {name}");
    let mut st = conn.prepare(sql).map_err(Error::db(what()))?;
    let n = st.column_count();
    let mut rows = match tree {
        Some(t) => st.query(params![t]),
        None => st.query([]),
    }
    .map_err(Error::db(what()))?;
    while let Some(row) = rows.next().map_err(Error::db(what()))? {
        h.update(b"r");
        for i in 0..n {
            feed(h, row.get_ref(i).map_err(Error::db(what()))?);
        }
    }
    Ok(())
}

/// `ORDER BY 1, 2, …, n` for a table of `n` columns.
fn order_all(conn: &Connection, table: &str) -> Result<String> {
    let n = conn
        .prepare(&format!("SELECT * FROM {table}"))
        .map_err(Error::db(format!("reading {table}")))?
        .column_count();
    Ok((1..=n)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(", "))
}

/// The digest of the database's facts about the tree labelled `tree`, or
/// of the whole database, as sixty-four hex digits.
pub fn digest(conn: &Connection, tree: Option<&str>) -> Result<String> {
    let mut h = Sha256::new();
    h.update(format!("rakata-index/v{}\n", schema::VERSION));
    match tree {
        Some(label) => {
            let id = store::tree_id(conn, label)?;
            h.update(b"tree\n");
            let t = Some(id);
            section(
                conn,
                &mut h,
                "locations",
                "SELECT path, member, name, type, hash FROM locations WHERE tree = ?1 ORDER BY 1, 2",
                t,
            )?;
            section(
                conn,
                &mut h,
                "coverage",
                "SELECT path, member, status, reason FROM coverage WHERE tree = ?1 ORDER BY 1, 2",
                t,
            )?;
            section(
                conn,
                &mut h,
                "resources",
                "SELECT DISTINCT r.hash, r.name, r.type, r.size FROM resources r
                 JOIN locations l ON l.hash = r.hash AND l.name = r.name AND l.type = r.type
                 WHERE l.tree = ?1 ORDER BY 1, 2, 3, 4",
                t,
            )?;
            section(
                conn,
                &mut h,
                "contents",
                "SELECT hash, type, status, reason FROM contents
                 WHERE hash IN (SELECT hash FROM locations WHERE tree = ?1) ORDER BY 1",
                t,
            )?;
            for (table, col) in schema::FACT_TABLES {
                let order = order_all(conn, table)?;
                section(
                    conn,
                    &mut h,
                    table,
                    &format!(
                        "SELECT * FROM {table} WHERE {col} IN (SELECT hash FROM locations WHERE tree = ?1) ORDER BY {order}"
                    ),
                    t,
                )?;
            }
        }
        None => {
            h.update(b"all\n");
            section(
                conn,
                &mut h,
                "trees",
                "SELECT t.label, p.label FROM trees t LEFT JOIN trees p ON p.id = t.parent ORDER BY 1",
                None,
            )?;
            section(
                conn,
                &mut h,
                "locations",
                "SELECT t.label, l.path, l.member, l.name, l.type, l.hash FROM locations l
                 JOIN trees t ON t.id = l.tree ORDER BY 1, 2, 3",
                None,
            )?;
            section(
                conn,
                &mut h,
                "coverage",
                "SELECT t.label, v.path, v.member, v.status, v.reason FROM coverage v
                 JOIN trees t ON t.id = v.tree ORDER BY 1, 2, 3",
                None,
            )?;
            section(
                conn,
                &mut h,
                "resources",
                "SELECT r.hash, r.name, r.type, r.size, t.label FROM resources r
                 JOIN trees t ON t.id = r.first_tree ORDER BY 1, 2, 3",
                None,
            )?;
            section(
                conn,
                &mut h,
                "contents",
                "SELECT hash, type, status, reason FROM contents ORDER BY 1",
                None,
            )?;
            for (table, _) in schema::FACT_TABLES {
                let order = order_all(conn, table)?;
                section(
                    conn,
                    &mut h,
                    table,
                    &format!("SELECT * FROM {table} ORDER BY {order}"),
                    None,
                )?;
            }
        }
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
