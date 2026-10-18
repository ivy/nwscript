//! `rakata 2da show|diff`: a table, one row, one column or one cell, and
//! two tables compared by row label and column, over `rakata-2da`.

use clap::{Args, Subcommand};
use serde_json::{Value as Json, json};

use super::Ctx;
use super::inspect::{self, DiffArgs, LookupArgs, diff_sides, resource_forms};
use crate::Exit;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::inspect::{Loaded, diagnostic};
use rakata::twoda::{Change, Error, RowRef, TEXT_BLANK, TwoDa};

const SHOW: &str = concat!(
    "\
The table prints as tab-separated text: a header line of the column names
after an empty first cell, then each row's label and cells. A blank cell
prints as ****. --row prints `column<TAB>value` lines for one row, --column
`label<TAB>value` lines for one column, and both the one cell alone. A row is
named by its label; a number no label matches names the row at that index.
In --format json a blank cell is the empty string.

",
    resource_forms!(),
    "

Examples:
  rakata 2da show appearance.2da --game DIR --row 0
  rakata 2da show globalcat.2da --game DIR --column name
  rakata 2da show ./feat.2da --row 12 --column label
  rakata --format json 2da show 'data/2da.bif!#74.2da' --game DIR"
);

const DIFF: &str = concat!(
    "\
Rows pair by label (where a label repeats, the n-th with the n-th) and
columns by name; order is not compared. An added or removed row prints with
its non-blank cells. Exit 0 when the tables are the same, 3 when they differ.

",
    resource_forms!(),
    "

",
    diff_sides!(),
    "

Examples:
  rakata 2da diff globalcat.2da globalcat.2da --game-a VANILLA --game-b BUILD
  rakata 2da diff ./appearance.2da appearance.2da --game DIR
  rakata --format json 2da diff ./a.2da ./b.2da"
);

#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  rakata 2da show appearance.2da --game DIR --row 0
  rakata 2da diff globalcat.2da globalcat.2da --game-a VANILLA --game-b BUILD")]
pub struct TwoDaArgs {
    #[command(subcommand)]
    pub action: TwoDaAction,
}

#[derive(Subcommand, Debug)]
pub enum TwoDaAction {
    /// Print a table, one row of it, one column, or one cell.
    #[command(after_help = SHOW)]
    Show {
        /// The table: NAME.2da, PATH or ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        /// The row, by label, else by index.
        #[arg(long, value_name = "LABEL")]
        row: Option<String>,
        /// The column, by name; case-sensitive, as the game looks it up.
        #[arg(long, value_name = "C")]
        column: Option<String>,
        #[command(flatten)]
        lookup: LookupArgs,
    },
    /// Every row, column and cell on which two tables differ.
    #[command(after_help = DIFF)]
    Diff(DiffArgs),
}

pub fn run(ctx: &Ctx, args: &TwoDaArgs) -> Exit {
    match &args.action {
        TwoDaAction::Show {
            name,
            row,
            column,
            lookup,
        } => show(ctx, name, row.as_deref(), column.as_deref(), lookup),
        TwoDaAction::Diff(d) => diff(ctx, d),
    }
}

fn parse(loaded: &Loaded) -> Result<TwoDa, Diagnostic> {
    TwoDa::parse(&loaded.bytes)
        .map_err(|e| diagnostic(e.code(), &e).note(format!("while reading `{}`", loaded.source)))
}

/// A cell as text prints it.
fn cell(c: &str) -> &str {
    if c.is_empty() { TEXT_BLANK } else { c }
}

/// The row a `--row` names: by label, else by index.
fn row_index(t: &TwoDa, row: &str) -> Result<usize, Diagnostic> {
    t.row_index(row)
        .or_else(|| row.parse::<usize>().ok().and_then(|i| t.row_index(i)))
        .ok_or_else(|| {
            let e = Error::NoSuchRow {
                row: RowRef::Label(row).to_string(),
                rows: t.rows.len(),
            };
            diagnostic(e.code(), &e)
        })
}

fn column_index(t: &TwoDa, column: &str) -> Result<usize, Diagnostic> {
    t.column_index(column).ok_or_else(|| {
        let e = Error::NoSuchColumn {
            name: column.to_owned(),
        };
        diagnostic(e.code(), &e)
    })
}

fn get(t: &TwoDa, r: usize, c: usize) -> &str {
    t.rows[r].cells.get(c).map_or("", String::as_str)
}

fn show(
    ctx: &Ctx,
    name: &str,
    row: Option<&str>,
    column: Option<&str>,
    lookup: &LookupArgs,
) -> Exit {
    let mut env = ctx.envelope("2da show");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let loaded = match inspect::load(&mut env, "resource", name, &lookup.lookup(ctx)) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let t = match parse(&loaded) {
        Ok(t) => t,
        Err(d) => return fail(env, d),
    };
    let r = match row.map(|r| row_index(&t, r)).transpose() {
        Ok(r) => r,
        Err(d) => return fail(env, d),
    };
    let c = match column.map(|c| column_index(&t, c)).transpose() {
        Ok(c) => c,
        Err(d) => return fail(env, d),
    };
    let mut result = json!({ "source": loaded.source });
    let text = match (r, c) {
        (Some(r), Some(c)) => {
            result["row"] = json!(t.rows[r].label);
            result["column"] = json!(t.columns[c]);
            result["value"] = json!(get(&t, r, c));
            cell(get(&t, r, c)).to_owned()
        }
        (Some(r), None) => {
            result["row"] = json!(t.rows[r].label);
            result["index"] = json!(r);
            result["cells"] = cells(&t, r);
            t.columns
                .iter()
                .enumerate()
                .map(|(i, name)| format!("{name}\t{}", cell(get(&t, r, i))))
                .collect::<Vec<_>>()
                .join("\n")
        }
        (None, Some(c)) => {
            result["column"] = json!(t.columns[c]);
            result["values"] = Json::Array(
                (0..t.rows.len())
                    .map(|r| json!({ "row": t.rows[r].label, "value": get(&t, r, c) }))
                    .collect(),
            );
            (0..t.rows.len())
                .map(|r| format!("{}\t{}", t.rows[r].label, cell(get(&t, r, c))))
                .collect::<Vec<_>>()
                .join("\n")
        }
        (None, None) => {
            result["columns"] = json!(t.columns);
            result["rows"] = Json::Array(
                t.rows
                    .iter()
                    .map(|row| json!({ "label": row.label, "cells": row.cells }))
                    .collect(),
            );
            let mut out = String::new();
            for name in &t.columns {
                out.push('\t');
                out.push_str(name);
            }
            for (r, row) in t.rows.iter().enumerate() {
                out.push('\n');
                out.push_str(&row.label);
                for i in 0..t.columns.len() {
                    out.push('\t');
                    out.push_str(cell(get(&t, r, i)));
                }
            }
            out
        }
    };
    env.result = result;
    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
}

/// One row's cells by column name, in a JSON object.
fn cells(t: &TwoDa, r: usize) -> Json {
    let mut m = serde_json::Map::new();
    for (i, name) in t.columns.iter().enumerate() {
        m.insert(name.clone(), json!(get(t, r, i)));
    }
    Json::Object(m)
}

/// The rows of `t` labelled `label`, the n-th of them for the n-th time a
/// label is asked, as the diff pairs them.
struct Rows<'a> {
    t: &'a TwoDa,
    seen: std::collections::HashMap<&'a str, usize>,
}

impl<'a> Rows<'a> {
    fn new(t: &'a TwoDa) -> Self {
        Rows {
            t,
            seen: Default::default(),
        }
    }

    fn next(&mut self, label: &'a str) -> Option<usize> {
        let nth = self.seen.entry(label).or_insert(0);
        let found = self
            .t
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.label == label)
            .nth(*nth)
            .map(|(i, _)| i);
        *nth += 1;
        found
    }
}

/// A row's non-blank cells, as `column=value`.
fn summary(t: &TwoDa, r: usize) -> String {
    t.columns
        .iter()
        .enumerate()
        .filter(|(i, _)| !get(t, r, *i).is_empty())
        .map(|(i, name)| format!("{name}={}", get(t, r, i)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn diff(ctx: &Ctx, args: &DiffArgs) -> Exit {
    let mut env = ctx.envelope("2da diff");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let (a, b) = match args.load(ctx, &mut env) {
        Ok(ab) => ab,
        Err(d) => return fail(env, d),
    };
    let (ta, tb) = match (parse(&a), parse(&b)) {
        (Ok(x), Ok(y)) => (x, y),
        (Err(d), _) | (_, Err(d)) => return fail(env, d),
    };
    let changes = ta.diff(&tb);
    let mut added = Rows::new(&tb);
    let mut removed = Rows::new(&ta);
    let mut lines = Vec::new();
    let mut out = Vec::new();
    for c in &changes {
        match c {
            Change::ColumnAdded { name } => {
                lines.push(format!("column `{name}` added"));
                out.push(json!({ "kind": "column_added", "column": name }));
            }
            Change::ColumnRemoved { name } => {
                lines.push(format!("column `{name}` removed"));
                out.push(json!({ "kind": "column_removed", "column": name }));
            }
            Change::RowAdded { label } => {
                let r = added.next(label).expect("the added row is in B");
                lines.push(format!("row `{label}` added: {}", summary(&tb, r)));
                out.push(json!({ "kind": "row_added", "row": label, "cells": cells(&tb, r) }));
            }
            Change::RowRemoved { label } => {
                let r = removed.next(label).expect("the removed row is in A");
                lines.push(format!("row `{label}` removed: {}", summary(&ta, r)));
                out.push(json!({ "kind": "row_removed", "row": label, "cells": cells(&ta, r) }));
            }
            Change::Cell {
                label,
                column,
                before,
                after,
            } => {
                lines.push(format!(
                    "row `{label}`, column `{column}`: {} -> {}",
                    cell(before),
                    cell(after)
                ));
                out.push(json!({
                    "kind": "cell",
                    "row": label,
                    "column": column,
                    "before": before,
                    "after": after,
                }));
            }
        }
    }
    let same = changes.is_empty();
    env.result = json!({ "a": a.source, "b": b.source, "same": same, "changes": out });
    let text = if same {
        "no differences".to_owned()
    } else {
        lines.join("\n")
    };
    let exit = if same { Exit::Ok } else { Exit::Different };
    ctx.finish(env, &text, Vec::new(), &map, exit)
}
