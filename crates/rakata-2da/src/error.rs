//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;

/// Diagnostic codes for tables: the `RE07xx` range of the `rakata`
/// registry. Each is a stable string a caller may match on and a user may
/// look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// The bytes do not begin with `2DA V2.b` and a line feed.
    pub const NOT_A_2DA: &str = "RE0701";
    /// The column names, the row count, the row labels or the cell offsets
    /// run past the end of the bytes.
    pub const TRUNCATED: &str = "RE0702";
    /// A cell's offset lies past the end of the strings, or its string has
    /// no terminating NUL before the end of the file.
    pub const CELL_BEYOND_END: &str = "RE0703";
    /// A table too large for the layout's 16-bit string offsets or 32-bit
    /// row count.
    pub const TOO_LARGE: &str = "RE0704";
    /// No row has the label, or the index is past the last row.
    pub const NO_SUCH_ROW: &str = "RE0705";
    /// No column has the name.
    pub const NO_SUCH_COLUMN: &str = "RE0706";
    /// A column added under a name the table already has.
    pub const COLUMN_PRESENT: &str = "RE0707";
    /// A string the layout cannot hold: a NUL in a cell, a tab or NUL in a
    /// column name or row label, or a character outside the game's code
    /// page.
    pub const UNREPRESENTABLE: &str = "RE0708";
    /// A row whose cell count differs from the table's column count.
    pub const RAGGED_ROW: &str = "RE0709";
}

/// An error in reading, writing or editing a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes do not begin with the `2DA V2.b` signature and a line
    /// feed.
    NotA2da {
        /// The first nine bytes found, lossily as text.
        found: String,
    },
    /// A header section runs past the end of the bytes.
    Truncated {
        /// What was being read.
        what: &'static str,
        /// The offset at which it begins.
        offset: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// A cell's offset lies past the end of the strings, or its string has
    /// no terminating NUL before the end of the file.
    CellBeyondEnd {
        /// The row, by index.
        row: usize,
        /// The column, by name.
        column: String,
        /// The offset into the strings.
        offset: usize,
        /// The length of the strings section.
        available: usize,
    },
    /// The table is too large for the layout.
    TooLarge {
        /// What overflowed: the strings or the rows.
        what: &'static str,
        /// The size it would have been.
        size: usize,
        /// The most the layout holds.
        limit: usize,
    },
    /// No row has the label, or the index is past the last row.
    NoSuchRow {
        /// The row asked for, as `label \`x\`` or `index n`.
        row: String,
        /// How many rows the table has.
        rows: usize,
    },
    /// No column has the name.
    NoSuchColumn {
        /// The name asked for.
        name: String,
    },
    /// A column added under a name the table already has.
    ColumnPresent {
        /// The name.
        name: String,
    },
    /// A string the layout cannot hold.
    Unrepresentable {
        /// Which string: `the cell at row 3, column \`label\``.
        what: String,
        /// Why: the terminator it contains, or the character outside the
        /// code page.
        reason: String,
    },
    /// A row whose cell count differs from the table's column count.
    RaggedRow {
        /// The row, by index.
        row: usize,
        /// Its cell count.
        cells: usize,
        /// The table's column count.
        columns: usize,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotA2da { .. } => codes::NOT_A_2DA,
            Error::Truncated { .. } => codes::TRUNCATED,
            Error::CellBeyondEnd { .. } => codes::CELL_BEYOND_END,
            Error::TooLarge { .. } => codes::TOO_LARGE,
            Error::NoSuchRow { .. } => codes::NO_SUCH_ROW,
            Error::NoSuchColumn { .. } => codes::NO_SUCH_COLUMN,
            Error::ColumnPresent { .. } => codes::COLUMN_PRESENT,
            Error::Unrepresentable { .. } => codes::UNREPRESENTABLE,
            Error::RaggedRow { .. } => codes::RAGGED_ROW,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::NotA2da { found } => {
                write!(f, "not a `2DA V2.b` table: the file begins `{found}`")
            }
            Error::Truncated {
                what,
                offset,
                available,
            } => write!(
                f,
                "{what} at offset {offset} run past the end of the {available}-byte file"
            ),
            Error::CellBeyondEnd {
                row,
                column,
                offset,
                available,
            } => write!(
                f,
                "the cell at row {row}, column `{column}` points at offset {offset} of the {available}-byte strings and no NUL ends it"
            ),
            Error::TooLarge { what, size, limit } => {
                write!(
                    f,
                    "{what} come to {size}, more than the {limit} the layout holds"
                )
            }
            Error::NoSuchRow { row, rows } => {
                write!(f, "the table has no row with {row}; it has {rows} rows")
            }
            Error::NoSuchColumn { name } => write!(f, "the table has no column `{name}`"),
            Error::ColumnPresent { name } => {
                write!(f, "the table already has a column `{name}`")
            }
            Error::Unrepresentable { what, reason } => {
                write!(f, "{what} cannot be written: {reason}")
            }
            Error::RaggedRow {
                row,
                cells,
                columns,
            } => write!(
                f,
                "row {row} has {cells} cells where the table has {columns} columns"
            ),
        }
    }
}

impl std::error::Error for Error {}
