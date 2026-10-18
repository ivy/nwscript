//! A face's surface material through `surfacemat.2da`.

use rakata_2da::TwoDa;

use crate::{Error, Result};

/// One row of `surfacemat.2da`: the material's label and whether it is
/// walkable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceMaterial {
    /// The `label` cell, or the empty string where the table has no such
    /// column.
    pub label: String,
    /// Whether the `walk` cell is a number other than 0. A blank cell is 0.
    pub walkable: bool,
}

/// The surface materials, by a face's material number: row `n` of
/// `surfacemat.2da` in file order (in the vanilla table, row `n` is also
/// labelled `n`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SurfaceMaterials {
    /// The rows in file order.
    pub materials: Vec<SurfaceMaterial>,
}

impl SurfaceMaterials {
    /// Reads the `walk` and `label` columns of a `surfacemat.2da`.
    pub fn from_2da(table: &TwoDa) -> Result<SurfaceMaterials> {
        let walk = table
            .column_index("walk")
            .ok_or_else(|| Error::BadSurfaceTable {
                reason: "the table has no `walk` column".into(),
            })?;
        let label = table.column_index("label");
        let materials = table
            .rows
            .iter()
            .map(|row| {
                let cell = row.cells[walk].trim();
                let walkable = match cell {
                    "" => false,
                    s => {
                        s.parse::<i64>().map_err(|_| Error::BadSurfaceTable {
                            reason: format!(
                                "row {}'s `walk` cell {s:?} is not a number",
                                row.label
                            ),
                        })? != 0
                    }
                };
                Ok(SurfaceMaterial {
                    label: label.map(|c| row.cells[c].clone()).unwrap_or_default(),
                    walkable,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(SurfaceMaterials { materials })
    }

    /// Reads a `surfacemat.2da` from its bytes.
    pub fn parse(bytes: &[u8]) -> Result<SurfaceMaterials> {
        let table = TwoDa::parse(bytes).map_err(|e| Error::BadSurfaceTable {
            reason: e.to_string(),
        })?;
        SurfaceMaterials::from_2da(&table)
    }

    /// The material numbered `material`, if the table has that row.
    pub fn get(&self, material: u32) -> Option<&SurfaceMaterial> {
        self.materials.get(material as usize)
    }

    /// Whether `material` is walkable; a material past the table's end is
    /// not.
    pub fn is_walkable(&self, material: u32) -> bool {
        self.get(material).is_some_and(|m| m.walkable)
    }
}
