//! Rows added to a game table by copying another, so a new appearance, a
//! new item type or a new portrait starts from one the engine already
//! draws.
//!
//! A row's label, in these calls, is what the game's tables mean by one:
//! the `label` column where the table has one (as `appearance.2da` does,
//! whose row labels are their own indices), and the row label otherwise.

use rakata_2da::TwoDa;

use super::{Error, Result};

/// How a row is named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKey {
    /// By position.
    Index(usize),
    /// By label, as the module's introduction defines one; it must name
    /// one row.
    Label(String),
}

/// The column holding the labels: `label` when the table has one.
fn label_column(table: &TwoDa) -> Option<usize> {
    table.column_index("label")
}

/// Every row carrying `label`, matched as written.
fn rows_labelled(table: &TwoDa, label: &str) -> Vec<usize> {
    let col = label_column(table);
    table
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            let have = match col {
                Some(c) => r.cells.get(c).map_or("", String::as_str),
                None => r.label.as_str(),
            };
            have == label
        })
        .map(|(i, _)| i)
        .collect()
}

/// The one row `key` names. Fails when no row matches (`RE0705`) and when a
/// label matches several (`RE1405`).
pub fn find(table: &TwoDa, key: &RowKey) -> Result<usize> {
    match key {
        RowKey::Index(i) => table
            .row_index(*i)
            .ok_or_else(|| no_such(table, format!("index {i}"))),
        RowKey::Label(l) => match rows_labelled(table, l).as_slice() {
            [one] => Ok(*one),
            [] => Err(no_such(table, format!("label `{l}`"))),
            many => Err(Error::LabelTaken {
                label: l.clone(),
                rows: many.to_vec(),
                new: false,
            }),
        },
    }
}

fn no_such(table: &TwoDa, row: String) -> Error {
    Error::TwoDa(rakata_2da::Error::NoSuchRow {
        row,
        rows: table.rows.len(),
    })
}

/// Append a copy of the row `from`, labelled `new_label`, with `cells`
/// (column, value) changed, and return its index. Where the table has a
/// `label` column the new label goes there and the row label is the new
/// index, as the game's own rows are numbered; otherwise the new label is
/// the row label. Fails, with the table unchanged, when `from` names no row
/// or several, when a row already carries `new_label` (`RE1405`), and when
/// a column is missing (`RE0706`).
pub fn add_row_copying(
    table: &mut TwoDa,
    from: &RowKey,
    new_label: &str,
    cells: &[(String, String)],
) -> Result<usize> {
    let source = find(table, from)?;
    let taken = rows_labelled(table, new_label);
    if !taken.is_empty() {
        return Err(Error::LabelTaken {
            label: new_label.to_string(),
            rows: taken,
            new: true,
        });
    }
    let mut changes: Vec<(&str, &str)> = cells
        .iter()
        .map(|(c, v)| (c.as_str(), v.as_str()))
        .collect();
    let row_label = match label_column(table) {
        Some(_) => {
            changes.insert(0, ("label", new_label));
            table.rows.len().to_string()
        }
        None => new_label.to_string(),
    };
    Ok(table.add_row_copying(source, row_label, &changes)?)
}
