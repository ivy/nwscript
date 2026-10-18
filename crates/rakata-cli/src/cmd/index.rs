//! `rakata index`: the index of a tree through [`rakata::index`]. `build`
//! makes it, `query` runs SQL over it, `digest` proves two of them equal,
//! `coverage` says what each file contributed.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::json;

use super::Ctx;
use crate::Exit;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::index;

#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  rakata index build --game ~/games/swkotor --tree vanilla
  rakata index build --game CKPT/tree --tree ckpt:KEY --parent ckpt:PARENT --changed CKPT/changed.txt
  rakata index query \"SELECT path, member FROM tree_resources WHERE name = 'k_pman_aioff'\"
  rakata index query --format json \"SELECT * FROM search WHERE search MATCH 'rakghoul*'\"
  rakata index digest --tree vanilla
  rakata index coverage --tree vanilla

The database is --db FILE, or index.sqlite under $XDG_CACHE_HOME/rakata/
(~/.cache/rakata/). `rakata index query --help` lists the tables.")]
pub struct IndexArgs {
    #[command(subcommand)]
    pub action: IndexAction,
}

#[derive(Args, Debug)]
pub struct DbArg {
    /// The database file. Defaults to index.sqlite under $XDG_CACHE_HOME/rakata/ (~/.cache/rakata/).
    #[arg(long, value_name = "FILE")]
    pub db: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum IndexAction {
    /// Index the tree --game names: every file the game loads resources from and every archive member, keyed by content hash.
    #[command(after_help = "\
Cold, the build reads every file of the tree. With --parent, every file the
parent tree holds is carried over unread except those --changed names, and
files the parent does not hold are read; either way a content hash the
database already holds is not parsed again, however many trees, paths or
archives hold it. The database is built in memory and written once.

--changed takes a list of paths relative to the tree, one per line, marked
as a modshop checkpoint's changed.txt marks them (+ added, ~ changed,
- removed) or bare. Script calls are read from each script's bytecode, with
the routine names of the tree's own nwscript.nss.

The output counts the files walked, carried over and read, the hashes
parsed and reused, the coverage by status and the files with no reader by
type, the rows per table, the database's size and the time.

Examples:
  rakata index build --game ~/games/swkotor --tree vanilla
  rakata index build --game CKPT/tree --tree ckpt:KEY --parent ckpt:PARENT --changed CKPT/changed.txt --db idx.sqlite")]
    Build {
        #[command(flatten)]
        db: DbArg,
        /// The tree's label. A tree the database holds under it is replaced. Defaults to the --game path as given.
        #[arg(long, value_name = "LABEL")]
        tree: Option<String>,
        /// A file listing the paths that differ from the parent tree.
        #[arg(long, value_name = "FILE", requires = "parent")]
        changed: Option<PathBuf>,
        /// The tree this one was made from, already in the database.
        #[arg(long, value_name = "LABEL")]
        parent: Option<String>,
        /// Threads to read with; 0 for one per CPU.
        #[arg(long, default_value_t = 0)]
        threads: usize,
    },
    /// Run SQL over the database, read-only, and print the rows: one statement, or a script of several separated by `;`.
    #[command(after_help = "\
SQL is one statement or a script: statements separated by `;`, run in order
on one connection, so a prelude of CREATE TEMP VIEW statements is run once
and every SELECT after it reads its views. --file FILE reads the script from
a file instead (`-` for standard input). The connection is read-only: SELECT,
WITH, EXPLAIN, PRAGMA and CREATE TEMP run, any statement that would change
the database fails with RE1504, and the statements after a failure do not
run.

Each statement that returns rows prints as an aligned table with a header,
NULL as an empty cell, and a blank line between tables. --format json gives
the column names and the rows as arrays, {columns, rows}; a script with more
than one such statement gives an array of them, in order.

Tables, joined through hash (a content's SHA-256); locations says where each
content lies:
  trees(id, label, parent, root)
  resources(hash, name, type, size, first_tree)
  contents(hash, type, status, reason)
  locations(tree, path, member, name, type, hash)
  coverage(tree, path, member, status, reason)
  dialog_nodes(hash, node, kind, idx, speaker, strref, text, voice, script, ...)
  dialog_links(hash, source, position, target, condition, is_child)
  gff_fields(hash, path, label, type, value, strref)
  cells(hash, row, label, column, value)
  strings(hash, strref, text, sound, sound_length, flags)
  calls(hash, offset, routine, routine_id, args)
  refs(from_hash, kind, value, site), kind resref, strref, tag or global
  search(body, kind, hash, site), full text: WHERE search MATCH 'word*'
Views:
  tree_resources(tree, path, member, name, type, hash, status)
  node_text(tree, path, member, hash, node, kind, idx, strref, text)

A node is E3 (EntryList[3]) or R0 (ReplyList[0]). Every column: SELECT name,
sql FROM sqlite_master. The full reference is docs/index.md in the source.

Examples:
  rakata index query \"SELECT t.label, l.path FROM calls c JOIN locations l ON l.hash = c.hash JOIN trees t ON t.id = l.tree WHERE json_extract(c.args, '$[0]') = 'k_pman_aioff'\"
  rakata index query \"SELECT kind, site, body FROM search WHERE search MATCH 'rakghoul*'\"
  rakata index query \"CREATE TEMP VIEW t AS SELECT id FROM trees WHERE label = 'vanilla'; SELECT count(*) FROM locations WHERE tree IN t; SELECT count(*) FROM coverage WHERE tree IN t\"
  rakata index query --file report.sql --db idx.sqlite")]
    Query {
        #[command(flatten)]
        db: DbArg,
        /// The SQL: one statement, or several separated by `;`.
        #[arg(required_unless_present = "file", conflicts_with = "file")]
        sql: Option<String>,
        /// Read the SQL from this file, `-` for standard input.
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Print the content digest of a tree, or of the whole database: equal digests mean equal facts.
    #[command(after_help = "\
The digest is a SHA-256 over what the rows say, sorted, never over row ids
or storage order, so an incremental build and a cold build of the same tree
have the same --tree digest. docs/index.md gives its definition.

Example:
  rakata index digest --tree ckpt:ba55d9e0db548e33 --db idx.sqlite")]
    Digest {
        #[command(flatten)]
        db: DbArg,
        /// The tree; without it, the whole database.
        #[arg(long, value_name = "LABEL")]
        tree: Option<String>,
    },
    /// What each file of a tree contributed: parsed, container, no_reader, failed or outside.
    #[command(after_help = "\
Prints, per tree, the files and archive members with a row, the count of
each status and the types with no reader; --list prints every row too.

Examples:
  rakata index coverage --tree vanilla
  rakata index coverage --tree vanilla --list")]
    Coverage {
        #[command(flatten)]
        db: DbArg,
        /// The tree; without it, every tree.
        #[arg(long, value_name = "LABEL")]
        tree: Option<String>,
        /// Print every row: status, path, member and reason.
        #[arg(long)]
        list: bool,
    },
}

pub(super) fn db_path(db: &DbArg) -> Result<PathBuf, Exit> {
    match db.db.clone().or_else(index::default_db) {
        Some(p) => Ok(p),
        None => {
            eprintln!(
                "error: `rakata index` needs a database: pass --db FILE (neither XDG_CACHE_HOME nor HOME is set)"
            );
            Err(Exit::Usage)
        }
    }
}

pub fn run(ctx: &Ctx, args: &IndexArgs) -> Exit {
    let map = SourceMap::new();
    match &args.action {
        IndexAction::Build {
            db,
            tree,
            changed,
            parent,
            threads,
        } => {
            let game = match ctx.require_game("index build") {
                Ok(g) => g.to_path_buf(),
                Err(e) => return e,
            };
            let db = match db_path(db) {
                Ok(p) => p,
                Err(e) => return e,
            };
            let mut env = ctx.envelope("index build");
            let label = tree.clone().unwrap_or_else(|| game.display().to_string());
            crate::working_on(game.display().to_string());
            let report = match index::build(&index::Build {
                game: &game,
                db: &db,
                label: &label,
                parent: parent.as_deref(),
                changed: changed.as_deref(),
                threads: *threads,
            }) {
                Ok(r) => r,
                Err(d) => return ctx.finish(env, "", vec![d], &map, Exit::Ok),
            };
            let diags: Vec<Diagnostic> = report
                .warnings
                .iter()
                .map(|w| index::diagnostic(w.code, w.message.clone()))
                .collect();
            let text = build_text(&report);
            env.result = serde_json::to_value(&report).expect("the report serializes");
            ctx.finish(env, &text, diags, &map, Exit::Ok)
        }
        IndexAction::Query { db, sql, file } => {
            let db = match db_path(db) {
                Ok(p) => p,
                Err(e) => return e,
            };
            let mut env = ctx.envelope("index query");
            let sql = match (sql, file) {
                (Some(sql), _) => sql.clone(),
                (None, Some(path)) => match read_sql(path) {
                    Ok(s) => s,
                    Err(e) => {
                        let d = index::diagnostic(
                            "RE0001",
                            format!("cannot read {}: {e}", path.display()),
                        );
                        return ctx.finish(env, "", vec![d], &map, Exit::Ok);
                    }
                },
                (None, None) => unreachable!("clap requires SQL or --file"),
            };
            let sets = index::open(&db).and_then(|c| index::query_script(&c, &sql));
            match sets {
                Ok(sets) => {
                    let text = sets.iter().map(table).collect::<Vec<_>>().join("\n");
                    let set = |r: &index::Rows| json!({ "columns": r.columns, "rows": r.rows });
                    env.result = match sets.as_slice() {
                        [one] => set(one),
                        many => many.iter().map(set).collect(),
                    };
                    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
                }
                Err(e) => ctx.finish(env, "", vec![index::error(&e)], &map, Exit::Ok),
            }
        }
        IndexAction::Digest { db, tree } => {
            let db = match db_path(db) {
                Ok(p) => p,
                Err(e) => return e,
            };
            let mut env = ctx.envelope("index digest");
            match index::open(&db).and_then(|c| index::digest(&c, tree.as_deref())) {
                Ok(d) => {
                    env.result = json!({ "tree": tree, "digest": d });
                    ctx.finish(env, &d, Vec::new(), &map, Exit::Ok)
                }
                Err(e) => ctx.finish(env, "", vec![index::error(&e)], &map, Exit::Ok),
            }
        }
        IndexAction::Coverage { db, tree, list } => {
            let db = match db_path(db) {
                Ok(p) => p,
                Err(e) => return e,
            };
            let mut env = ctx.envelope("index coverage");
            match index::open(&db).and_then(|c| index::coverage(&c, tree.as_deref(), *list)) {
                Ok(trees) => {
                    let text = coverage_text(&trees);
                    env.result = json!({ "trees": trees });
                    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
                }
                Err(e) => ctx.finish(env, "", vec![index::error(&e)], &map, Exit::Ok),
            }
        }
    }
}

fn counts(m: &std::collections::BTreeMap<String, usize>) -> String {
    m.iter()
        .map(|(k, v)| format!("{k} {v}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_text(r: &index::BuildReport) -> String {
    let mut s = String::new();
    s.push_str(&format!("indexed `{}`", r.tree));
    if let Some(p) = &r.parent {
        s.push_str(&format!(" from `{p}`"));
    }
    s.push_str(&format!(" into {}", r.db));
    if r.replaced {
        s.push_str(" (replacing the tree of that label)");
    }
    s.push('\n');
    s.push_str(&format!(
        "  files: {} walked, {} carried over from the parent, {} read with {} archive members; {} outside the walk\n",
        r.files, r.files_reused, r.files_read, r.members_read, r.outside
    ));
    s.push_str(&format!(
        "  hashes: {} parsed, {} reused\n",
        r.hashes_parsed, r.hashes_reused
    ));
    s.push_str(&format!("  coverage: {}\n", counts(&r.coverage)));
    let no_reader: usize = r.no_reader.values().sum();
    let mut by_count: Vec<(&String, &usize)> = r.no_reader.iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    s.push_str(&format!(
        "  no reader: {no_reader} ({})\n",
        by_count
            .iter()
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    s.push_str(&format!(
        "  rows: {}\n",
        r.rows
            .iter()
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    let t = &r.seconds;
    s.push_str(&format!(
        "  database: {:.1} MB; {:.2} s (load {:.2}, plan {:.2}, read {:.2}, finish {:.2}, save {:.2})\n",
        r.db_bytes as f64 / 1e6,
        t.total,
        t.load,
        t.plan,
        t.read,
        t.finish,
        t.save
    ));
    s
}

/// The SQL in `path`, or on standard input for `-`.
fn read_sql(path: &std::path::Path) -> std::io::Result<String> {
    if path.as_os_str() == "-" {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
        Ok(s)
    } else {
        std::fs::read_to_string(path)
    }
}

fn cell(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.replace('\n', "\\n").replace('\t', "\\t"),
        other => other.to_string(),
    }
}

/// The rows as an aligned table under a header.
fn table(rows: &index::Rows) -> String {
    let cells: Vec<Vec<String>> = rows
        .rows
        .iter()
        .map(|r| r.iter().map(cell).collect())
        .collect();
    let mut width: Vec<usize> = rows.columns.iter().map(|c| c.chars().count()).collect();
    for r in &cells {
        for (i, c) in r.iter().enumerate() {
            width[i] = width[i].max(c.chars().count());
        }
    }
    let line = |r: &[String]| -> String {
        let mut s = String::new();
        for (i, c) in r.iter().enumerate() {
            if i + 1 == r.len() {
                s.push_str(c);
            } else {
                let pad = width[i] - c.chars().count();
                s.push_str(c);
                s.push_str(&" ".repeat(pad + 2));
            }
        }
        s.trim_end().to_string()
    };
    let mut out = line(&rows.columns);
    out.push('\n');
    for r in &cells {
        out.push_str(&line(r));
        out.push('\n');
    }
    out.push_str(&format!("({} rows)\n", cells.len()));
    out
}

fn coverage_text(trees: &[index::TreeCoverage]) -> String {
    let mut s = String::new();
    for t in trees {
        s.push_str(&format!(
            "{}: {} files, {} archive members; {}\n",
            t.tree,
            t.files,
            t.members,
            counts(&t.by_status)
        ));
        if !t.no_reader.is_empty() {
            let mut by_count: Vec<(&String, &usize)> = t.no_reader.iter().collect();
            by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            s.push_str(&format!(
                "  no reader: {}\n",
                by_count
                    .iter()
                    .map(|(k, v)| format!("{k} {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(rows) = &t.rows {
            for r in rows {
                let at = if r.member.is_empty() {
                    r.path.clone()
                } else {
                    format!("{}!{}", r.path, r.member)
                };
                match &r.reason {
                    Some(reason) => s.push_str(&format!("  {:<9}  {at}  {reason}\n", r.status)),
                    None => s.push_str(&format!("  {:<9}  {at}\n", r.status)),
                }
            }
        }
    }
    s
}
