//! Globals: the engine keeps a global only when `globalcat.2da` declares
//! its name and type, so a mod's story flags are rows added to that table.

use std::collections::HashSet;

use rakata_2da::TwoDa;

use super::{Error, Result};

/// A global's type, as `globalcat.2da`'s `type` column spells it. KotOR 1's
/// table holds these two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlobalType {
    /// Read with `GetGlobalBoolean`, set with `SetGlobalBoolean`.
    Boolean,
    /// Read with `GetGlobalNumber`, set with `SetGlobalNumber`.
    Number,
}

impl GlobalType {
    /// The `type` cell.
    pub const fn name(self) -> &'static str {
        match self {
            GlobalType::Boolean => "Boolean",
            GlobalType::Number => "Number",
        }
    }
}

/// The names the table declares, uppercase, as the engine compares them.
pub(crate) fn declared(table: &TwoDa) -> HashSet<String> {
    let Some(c) = table.column_index("name") else {
        return HashSet::new();
    };
    table
        .rows
        .iter()
        .filter_map(|r| r.cells.get(c))
        .map(|n| n.to_ascii_uppercase())
        .collect()
}

/// Declare the global `name` of type `ty` in `globalcat.2da`'s `table`: a
/// row appended, labelled with its index, as the game's rows are. Returns
/// the row's index. Fails, with the table unchanged, when a row already
/// declares the name in any case (`RE1404`), or when the table lacks the
/// `name` or `type` column (`RE0706`).
pub fn declare(table: &mut TwoDa, name: &str, ty: GlobalType) -> Result<usize> {
    let c = table
        .column_index("name")
        .ok_or_else(|| rakata_2da::Error::NoSuchColumn {
            name: "name".into(),
        })?;
    if table.column_index("type").is_none() {
        return Err(rakata_2da::Error::NoSuchColumn {
            name: "type".into(),
        }
        .into());
    }
    if let Some(row) = table
        .rows
        .iter()
        .position(|r| r.cells.get(c).is_some_and(|n| n.eq_ignore_ascii_case(name)))
    {
        return Err(Error::GlobalDeclared {
            name: name.to_string(),
            row,
        });
    }
    let label = table.rows.len().to_string();
    Ok(table.add_row(label, &[("name", name), ("type", ty.name())])?)
}
