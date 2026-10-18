//! `[2DAList]` operations on one table: change, add and copy rows, add
//! columns, with the cells' tokens filled and the `2DAMEMORY#` stores made.

use super::{Failure, Tokens};
use crate::changes::*;
use crate::issue::codes;
use crate::value::Value;
use rakata_2da::TwoDa;

fn cannot(msg: String) -> Failure {
    Failure::new(codes::CANNOT_APPLY, msg)
}

/// The column named `name`: as written, else the only column that matches
/// without regard to case.
fn column(t: &TwoDa, name: &str) -> Result<usize, Failure> {
    if let Some(c) = t.column_index(name) {
        return Ok(c);
    }
    let mut hits = t
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.eq_ignore_ascii_case(name));
    match (hits.next(), hits.next()) {
        (Some((i, _)), None) => Ok(i),
        _ => Err(cannot(format!("the table has no column `{name}`"))),
    }
}

/// One more than the highest number in the column, 0 when it has none.
fn high_cell(t: &TwoDa, c: usize) -> String {
    let max = t
        .rows
        .iter()
        .filter_map(|r| r.cells.get(c))
        .filter_map(|v| v.trim().parse::<i64>().ok())
        .max();
    max.map_or(0, |m| m + 1).to_string()
}

/// One more than the highest numeric row label, or the row count.
fn high_label(t: &TwoDa) -> String {
    let max = t
        .rows
        .iter()
        .filter_map(|r| r.label.trim().parse::<i64>().ok())
        .max();
    max.map_or(t.rows.len() as i64, |m| (m + 1).max(t.rows.len() as i64))
        .to_string()
}

/// A cell's text: `****` is the blank cell, tokens their values.
fn cell_text(t: &TwoDa, c: usize, v: &Value, tokens: &Tokens) -> Result<String, Failure> {
    match v {
        Value::High => Ok(high_cell(t, c)),
        Value::Literal(s) if s == rakata_2da::TEXT_BLANK => Ok(String::new()),
        _ => tokens.resolve(v),
    }
}

fn find_row(t: &TwoDa, target: &Option<RowTarget>, tokens: &Tokens) -> Result<usize, Failure> {
    match target {
        None => Err(cannot(
            "no RowIndex, RowLabel or LabelIndex names the row".into(),
        )),
        Some(RowTarget::Index(n)) => {
            let n = *n as usize;
            if n < t.rows.len() {
                Ok(n)
            } else {
                Err(cannot(format!(
                    "row {n} does not exist; the table has {} rows",
                    t.rows.len()
                )))
            }
        }
        Some(RowTarget::Label(l)) => {
            let l = tokens.resolve(&Value::parse(l))?;
            t.rows
                .iter()
                .position(|r| r.label == l)
                .ok_or_else(|| cannot(format!("no row is labelled `{l}`")))
        }
        Some(RowTarget::LabelIndex(l)) => {
            let l = tokens.resolve(&Value::parse(l))?;
            let c = column(t, "label")?;
            t.rows
                .iter()
                .position(|r| r.cells.get(c).is_some_and(|v| *v == l))
                .ok_or_else(|| cannot(format!("no row has `{l}` in its `label` column")))
        }
    }
}

/// Set the cells, in order; a `high()` sees the cells set before it.
fn set_cells(t: &mut TwoDa, row: usize, cells: &[Cell], tokens: &Tokens) -> Result<(), Failure> {
    for cell in cells {
        let c = column(t, &cell.column)?;
        let text = cell_text(t, c, &cell.value, tokens)?;
        let name = t.columns[c].clone();
        t.set(row, &name, text)
            .map_err(|e| cannot(format!("cannot set `{}`: {e}", cell.column)))?;
    }
    Ok(())
}

fn store(t: &TwoDa, row: usize, stores: &[Store], tokens: &mut Tokens) -> Result<(), Failure> {
    for s in stores {
        let v = match &s.source {
            StoreSource::RowIndex => row.to_string(),
            StoreSource::RowLabel => t.rows[row].label.clone(),
            StoreSource::Column(name) => {
                let c = column(t, name)?;
                t.rows[row].cells.get(c).cloned().unwrap_or_default()
            }
        };
        tokens.memory.insert(s.slot, v);
    }
    Ok(())
}

/// The row whose `column` holds what the new row would put there, if any.
fn exclusive_match(
    t: &TwoDa,
    exclusive: &Option<String>,
    cells: &[Cell],
    tokens: &Tokens,
) -> Result<Option<usize>, Failure> {
    let Some(ex) = exclusive else {
        return Ok(None);
    };
    let c = column(t, ex)?;
    let Some(cell) = cells.iter().find(|x| column(t, &x.column).ok() == Some(c)) else {
        return Ok(None);
    };
    let want = cell_text(t, c, &cell.value, tokens)?;
    if want.is_empty() {
        return Ok(None);
    }
    Ok(t.rows
        .iter()
        .position(|r| r.cells.get(c).is_some_and(|v| *v == want)))
}

/// Apply one operation; the returned text says what it did. An operation
/// that fails leaves the table and the tokens as they were.
pub fn apply(t: &mut TwoDa, kind: &TableOpKind, tokens: &mut Tokens) -> Result<String, Failure> {
    let mut table = t.clone();
    let mut toks = tokens.clone();
    let done = apply_to(&mut table, kind, &mut toks)?;
    *t = table;
    *tokens = toks;
    Ok(done)
}

fn apply_to(t: &mut TwoDa, kind: &TableOpKind, tokens: &mut Tokens) -> Result<String, Failure> {
    match kind {
        TableOpKind::ChangeRow(op) => {
            let row = find_row(t, &op.target, tokens)?;
            set_cells(t, row, &op.cells, tokens)?;
            store(t, row, &op.stores, tokens)?;
            Ok(format!("changed row {row}"))
        }
        TableOpKind::AddRow(op) => {
            if let Some(row) = exclusive_match(t, &op.exclusive_column, &op.cells, tokens)? {
                set_cells(t, row, &op.cells, tokens)?;
                store(t, row, &op.stores, tokens)?;
                return Ok(format!(
                    "row {row} already has the exclusive column's value; changed it"
                ));
            }
            let label = row_label(t, op.row_label.as_ref(), tokens)?;
            let row = t.add_row(label, &[]).map_err(|e| cannot(e.to_string()))?;
            set_cells(t, row, &op.cells, tokens)?;
            store(t, row, &op.stores, tokens)?;
            Ok(format!("added row {row}"))
        }
        TableOpKind::CopyRow(op) => {
            let from = find_row(t, &op.target, tokens)?;
            if let Some(row) = exclusive_match(t, &op.exclusive_column, &op.cells, tokens)? {
                set_cells(t, row, &op.cells, tokens)?;
                store(t, row, &op.stores, tokens)?;
                return Ok(format!(
                    "row {row} already has the exclusive column's value; changed it"
                ));
            }
            let label = row_label(t, op.new_row_label.as_ref(), tokens)?;
            let row = t
                .add_row_copying(from, label, &[])
                .map_err(|e| cannot(e.to_string()))?;
            set_cells(t, row, &op.cells, tokens)?;
            store(t, row, &op.stores, tokens)?;
            Ok(format!("copied row {from} to row {row}"))
        }
        TableOpKind::AddColumn(op) => {
            if op.label.is_empty() {
                return Err(cannot("the column has no ColumnLabel".into()));
            }
            let default = match &op.default {
                None => String::new(),
                Some(Value::Literal(s)) if s == rakata_2da::TEXT_BLANK => String::new(),
                Some(v) => tokens.resolve(v)?,
            };
            let c = t
                .add_column(op.label.clone(), &default)
                .map_err(|e| cannot(e.to_string()))?;
            for cell in &op.cells {
                let row = column_row(t, &cell.row)?;
                let text = cell_text(t, c, &cell.value, tokens)?;
                let name = t.columns[c].clone();
                t.set(row, &name, text).map_err(|e| cannot(e.to_string()))?;
            }
            for s in &op.stores {
                let row = column_row(t, &s.row)?;
                let v = t.rows[row].cells.get(c).cloned().unwrap_or_default();
                tokens.memory.insert(s.slot, v);
            }
            Ok(format!("added column `{}`", op.label))
        }
    }
}

fn row_label(t: &TwoDa, label: Option<&Value>, tokens: &Tokens) -> Result<String, Failure> {
    match label {
        None => Ok(t.rows.len().to_string()),
        Some(Value::High) => Ok(high_label(t)),
        Some(v) => tokens.resolve(v),
    }
}

fn column_row(t: &TwoDa, r: &RowRef) -> Result<usize, Failure> {
    match r {
        RowRef::Index(n) => {
            let n = *n as usize;
            (n < t.rows.len())
                .then_some(n)
                .ok_or_else(|| cannot(format!("row {n} does not exist")))
        }
        RowRef::Label(l) => t
            .rows
            .iter()
            .position(|x| x.label == *l)
            .ok_or_else(|| cannot(format!("no row is labelled `{l}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> TwoDa {
        let mut t = TwoDa::new(["label", "value"]);
        t.add_row("0", &[("label", "a"), ("value", "3")]).unwrap();
        t.add_row("1", &[("label", "b"), ("value", "7")]).unwrap();
        t
    }

    fn cell(c: &str, v: &str) -> Cell {
        Cell {
            column: c.into(),
            value: Value::parse(v),
        }
    }

    #[test]
    fn add_row_takes_its_index_as_label_and_high_takes_the_column_max() {
        let mut t = table();
        let mut tok = Tokens::default();
        let op = TableOpKind::AddRow(RowAdd {
            exclusive_column: None,
            row_label: None,
            cells: vec![cell("label", "c"), cell("Value", "high()")],
            stores: vec![Store {
                slot: 1,
                source: StoreSource::RowIndex,
            }],
        });
        apply(&mut t, &op, &mut tok).unwrap();
        assert_eq!(t.rows[2].label, "2");
        assert_eq!(t.rows[2].cells, ["c", "8"]);
        assert_eq!(tok.memory[&1], "2");
    }

    #[test]
    fn an_exclusive_match_changes_the_row_instead_of_adding() {
        let mut t = table();
        let mut tok = Tokens::default();
        let op = TableOpKind::AddRow(RowAdd {
            exclusive_column: Some("label".into()),
            row_label: None,
            cells: vec![cell("label", "b"), cell("value", "****")],
            stores: vec![Store {
                slot: 0,
                source: StoreSource::RowIndex,
            }],
        });
        apply(&mut t, &op, &mut tok).unwrap();
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[1].cells, ["b", ""]);
        assert_eq!(tok.memory[&0], "1");
    }

    #[test]
    fn a_missing_row_is_an_error_and_leaves_the_table() {
        let mut t = table();
        let before = t.clone();
        let op = TableOpKind::ChangeRow(RowChange {
            target: Some(RowTarget::Index(9)),
            cells: vec![cell("value", "1")],
            stores: vec![],
        });
        let e = apply(&mut t, &op, &mut Tokens::default()).unwrap_err();
        assert_eq!(e.code, codes::CANNOT_APPLY);
        assert_eq!(t, before);
    }
}
