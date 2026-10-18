//! Reading the index: SQL over the tables, and the coverage summary.

use std::collections::BTreeMap;

use rusqlite::Connection;
use rusqlite::types::ValueRef;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::store;

/// The rows a query returned, each value as JSON: null, a number or a
/// string; a blob as lowercase hex.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Rows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

fn json(v: ValueRef<'_>) -> serde_json::Value {
    match v {
        ValueRef::Null => serde_json::Value::Null,
        ValueRef::Integer(i) => i.into(),
        ValueRef::Real(f) => {
            serde_json::Number::from_f64(f).map_or(serde_json::Value::Null, Into::into)
        }
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned().into(),
        ValueRef::Blob(b) => b
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect::<String>()
            .into(),
    }
}

/// Run `sql` and return every row. The connection is read-only, so a
/// statement that would change the database fails.
pub fn query(conn: &Connection, sql: &str) -> Result<Rows> {
    let fail = |source| Error::Query {
        sql: sql.to_string(),
        source,
    };
    let mut st = conn.prepare(sql).map_err(fail)?;
    let columns: Vec<String> = st.column_names().iter().map(|c| c.to_string()).collect();
    let n = columns.len();
    let mut out = Rows {
        columns,
        rows: Vec::new(),
    };
    let mut rows = st.query([]).map_err(fail)?;
    while let Some(row) = rows.next().map_err(fail)? {
        let mut r = Vec::with_capacity(n);
        for i in 0..n {
            r.push(json(row.get_ref(i).map_err(fail)?));
        }
        out.rows.push(r);
    }
    Ok(out)
}

/// One coverage row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageRow {
    pub path: String,
    pub member: String,
    pub status: String,
    pub reason: Option<String>,
}

/// One tree's coverage.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TreeCoverage {
    pub tree: String,
    /// Files of the tree, and archive members, with a row.
    pub files: usize,
    pub members: usize,
    /// Rows by status.
    pub by_status: BTreeMap<String, usize>,
    /// Files and members with no reader, by type.
    pub no_reader: BTreeMap<String, usize>,
    /// Every row, when asked for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<CoverageRow>>,
}

fn count_by(conn: &Connection, sql: &str, tree: i64) -> Result<BTreeMap<String, usize>> {
    let mut st = conn.prepare(sql).map_err(Error::db("reading coverage"))?;
    let rows = st
        .query_map([tree], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(Error::db("reading coverage"))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (k, n) = row.map_err(Error::db("reading coverage"))?;
        out.insert(if k.is_empty() { "(none)".into() } else { k }, n as usize);
    }
    Ok(out)
}

/// The coverage of the tree labelled `tree`, or of every tree, with every
/// row when `list` is set.
pub fn coverage(conn: &Connection, tree: Option<&str>, list: bool) -> Result<Vec<TreeCoverage>> {
    let labels = match tree {
        Some(t) => {
            store::tree_id(conn, t)?;
            vec![t.to_string()]
        }
        None => store::labels(conn)?,
    };
    let mut out = Vec::new();
    for label in labels {
        let id = store::tree_id(conn, &label)?;
        let (files, members): (i64, i64) = conn
            .query_row(
                "SELECT sum(member = ''), sum(member <> '') FROM coverage WHERE tree = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                        r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    ))
                },
            )
            .map_err(Error::db("reading coverage"))?;
        let by_status = count_by(
            conn,
            "SELECT status, count(*) FROM coverage WHERE tree = ?1 GROUP BY 1",
            id,
        )?;
        let no_reader = count_by(
            conn,
            "SELECT l.type, count(*) FROM coverage v
             JOIN locations l ON l.tree = v.tree AND l.path = v.path AND l.member = v.member
             WHERE v.tree = ?1 AND v.status = 'no_reader' GROUP BY 1",
            id,
        )?;
        let rows = if list {
            let mut st = conn
                .prepare(
                    "SELECT path, member, status, reason FROM coverage WHERE tree = ?1 ORDER BY path, member",
                )
                .map_err(Error::db("reading coverage"))?;
            let rows = st
                .query_map([id], |r| {
                    Ok(CoverageRow {
                        path: r.get(0)?,
                        member: r.get(1)?,
                        status: r.get(2)?,
                        reason: r.get(3)?,
                    })
                })
                .map_err(Error::db("reading coverage"))?;
            Some(
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(Error::db("reading coverage"))?,
            )
        } else {
            None
        };
        out.push(TreeCoverage {
            tree: label,
            files: files as usize,
            members: members as usize,
            by_status,
            no_reader,
            rows,
        });
    }
    Ok(out)
}
