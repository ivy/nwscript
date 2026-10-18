//! What a table contributes to an index: one [`Cell`] per cell that is not
//! blank, and the [`Ref`]s its rows make.
//!
//! A 2DA's cells are untyped strings, so a cell is a reference only where
//! the table's role says so: each row of `globalcat.2da` declares a global
//! variable, its `name` cell the global's name, and is recorded as a
//! [`RefKind::Global`](rakata_res::RefKind::Global) reference at the site
//! `row N`. The functions are pure: they read the table and nothing else.

use rakata_res::Ref;

use crate::TwoDa;

/// One cell that is not blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell<'a> {
    /// The row's position.
    pub row: usize,
    /// The row's label.
    pub label: &'a str,
    pub column: &'a str,
    pub value: &'a str,
}

/// The facts of one table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts<'a> {
    /// Every cell that is not blank, row by row, columns in table order.
    pub cells: Vec<Cell<'a>>,
    pub refs: Vec<Ref>,
}

/// The facts of `table`, read as the resource `name` (`globalcat`, without
/// its extension; case does not matter).
pub fn facts<'a>(table: &'a TwoDa, name: &str) -> Facts<'a> {
    let mut out = Facts::default();
    for (i, row) in table.rows.iter().enumerate() {
        for (column, value) in table.columns.iter().zip(&row.cells) {
            if !value.is_empty() {
                out.cells.push(Cell {
                    row: i,
                    label: &row.label,
                    column,
                    value,
                });
            }
        }
    }
    if name.eq_ignore_ascii_case("globalcat")
        && let Some(col) = table.column_index("name")
    {
        for (i, row) in table.rows.iter().enumerate() {
            if let Some(global) = row.cells.get(col).filter(|v| !v.is_empty()) {
                out.refs
                    .push(Ref::global(global.clone(), format!("row {i}")));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rakata_res::RefKind;

    fn globalcat() -> TwoDa {
        let mut t = TwoDa::new(["name", "type"]);
        t.add_row("0", &[("name", "K_JUHANI_CLONES"), ("type", "Number")])
            .unwrap();
        t.add_row("1", &[("type", "Boolean")]).unwrap();
        t
    }

    #[test]
    fn blank_cells_have_no_fact() {
        let t = globalcat();
        let f = facts(&t, "appearance");
        let got: Vec<(usize, &str, &str)> =
            f.cells.iter().map(|c| (c.row, c.column, c.value)).collect();
        assert_eq!(
            got,
            [
                (0, "name", "K_JUHANI_CLONES"),
                (0, "type", "Number"),
                (1, "type", "Boolean")
            ]
        );
        assert_eq!(f.cells[2].label, "1");
        assert!(f.refs.is_empty(), "only globalcat declares globals");
    }

    #[test]
    fn globalcat_rows_declare_globals() {
        let t = globalcat();
        let f = facts(&t, "GlobalCat");
        assert_eq!(f.refs.len(), 1);
        assert_eq!(f.refs[0].kind, RefKind::Global);
        assert_eq!(f.refs[0].value, "K_JUHANI_CLONES");
        assert_eq!(f.refs[0].site, "row 0");
    }
}
