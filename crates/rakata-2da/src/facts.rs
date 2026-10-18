//! What a table contributes to an index: one [`Cell`] per cell that is not
//! blank, and the [`Ref`]s its rows make.
//!
//! A 2DA's cells are untyped strings, so a cell is a reference only where
//! the table's role says so: each row of `globalcat.2da` declares a global
//! variable, its `name` cell the global's name, and is recorded as a
//! [`RefKind::Global`](rakata_res::RefKind::Global) reference at the site
//! `row N`. A column that holds lines of the talk table (`name`,
//! `description`, `spelldesc`, the load-screen hints, a trap's name, and
//! any column whose name contains `strref` or `stringref`) makes a
//! [`RefKind::StrRef`](rakata_res::RefKind::StrRef) reference of each cell
//! that is a decimal number, at the site `row N COLUMN`; `-1` and text are
//! not strrefs. The functions are pure: they read the table and nothing
//! else.

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

/// Whether the column `name` holds strrefs. The names are those the game's
/// tables use for a line of the talk table; case does not matter.
pub fn is_strref_column(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    matches!(
        n.as_str(),
        "name" | "description" | "spelldesc" | "gameplayhint" | "storyhint" | "trapname"
    ) || ["strref", "stringref", "string_ref"]
        .iter()
        .any(|w| n.contains(w))
}

/// A cell as a strref: a decimal number that is not negative.
fn strref(value: &str) -> Option<u32> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
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
    for (k, column) in table.columns.iter().enumerate() {
        if !is_strref_column(column) {
            continue;
        }
        for (i, row) in table.rows.iter().enumerate() {
            if let Some(n) = row.cells.get(k).and_then(|v| strref(v)) {
                out.refs.push(Ref::strref(n, format!("row {i} {column}")));
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
        assert!(
            f.refs.iter().all(|r| r.kind != RefKind::Global),
            "only globalcat declares globals"
        );
    }

    #[test]
    fn strref_columns_name_lines_of_the_talk_table() {
        let mut t = TwoDa::new(["label", "name", "description", "STRREF", "cost"]);
        t.add_row(
            "0",
            &[
                ("label", "Blaster"),
                ("name", "42724"),
                ("description", "-1"),
                ("STRREF", "12"),
                ("cost", "100"),
            ],
        )
        .unwrap();
        t.add_row("1", &[("name", "K_SWG_HELENA"), ("description", "7")])
            .unwrap();
        let f = facts(&t, "baseitems");
        let got: Vec<(&str, &str)> = f
            .refs
            .iter()
            .map(|r| {
                assert_eq!(r.kind, RefKind::StrRef);
                (r.value.as_str(), r.site.as_str())
            })
            .collect();
        assert_eq!(
            got,
            [
                ("42724", "row 0 name"),
                ("7", "row 1 description"),
                ("12", "row 0 STRREF")
            ]
        );
        assert!(is_strref_column("KeyNameStrRef"));
        assert!(!is_strref_column("str"));
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
