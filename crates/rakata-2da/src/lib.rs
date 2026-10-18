//! The 2DA tables of BioWare's Aurora-family games, KotOR 1 and 2 first: a
//! [`TwoDa`] is named columns and labelled rows of string cells, read from
//! and written to the binary `2DA V2.b` layout of `docs/formats/2da.md`.
//!
//! The reader finds every cell by its offset into the strings section and
//! takes that section to run to the end of the file, whatever the header's
//! cell-data size says: editors ship tables with 0 there, and TSLPatcher
//! leaves a stale value behind when it appends, and the game reads both
//! intact. A reader that trusted the field once read every cell of such a
//! table as empty and wrote it back empty.
//!
//! The writer emits BioWare's layout: names and labels ended by the tab or
//! the NUL the table was read with ([`Terminator`]; a tab for a table built
//! here), each distinct string once, in the order the cells first meet it
//! walking the rows, and the size field equal to the strings' length, so
//! every vanilla table writes back byte for byte.
//! Editing is by row label or index and column name
//! ([`TwoDa::get`], [`TwoDa::set`], [`TwoDa::add_row`],
//! [`TwoDa::add_row_copying`], [`TwoDa::add_column`]), and [`TwoDa::diff`]
//! lists the cells two tables disagree on, by row label and column.
//!
//! A blank cell is the empty string, as BioWare's files carry it. The text
//! format's `****`, which `changes.ini` also writes for "no value", is a
//! convention of the authoring boundary: a caller translating such a file
//! sets the empty string ([`TEXT_BLANK`] names the marker). Should a binary
//! table ever hold the four bytes `****`, they read as that string and write
//! back as it, distinct from the blank.
//!
//! Column names are case-sensitive, as the game looks them up. The layout
//! follows the public format documents and the measured facts in
//! `docs/formats/2da.md`; this crate was written without sight of any other
//! implementation.

mod codepage;
mod error;

use std::collections::HashMap;
use std::fmt;

pub use error::{Error, Result, codes};

/// The nine bytes every table begins with.
const SIGNATURE: &[u8; 9] = b"2DA V2.b\n";

/// The marker the text 2DA format and `changes.ini` write for a blank cell.
/// In the binary layout a blank is the empty string; this is for the caller
/// who reads text and must decide what `****` becomes.
pub const TEXT_BLANK: &str = "****";

/// One row: its label, which the layout stores apart from the cells and the
/// game carries without looking up, and one cell per column.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    pub label: String,
    pub cells: Vec<String>,
}

/// A table: the column names in file order and the rows in file order.
/// The row label is not a column. `terminator` is how the file ends its
/// names and labels, kept so a table writes back as it was read; it is
/// layout, not data, so [`TwoDa::semantic_eq`] and [`TwoDa::diff`] ignore
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TwoDa {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
    pub terminator: Terminator,
}

/// The byte ending each column name and each row label. The two sections
/// share it: no table measured mixes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Terminator {
    /// 0x09, as every member of `data/2da.bif` has it; what a table built
    /// with the API takes.
    #[default]
    Tab,
    /// 0x00, as the tables inside `global.rim` and `miniglobal.rim` have
    /// it. The column list still closes with one more NUL.
    Nul,
}

impl Terminator {
    fn byte(self) -> u8 {
        match self {
            Terminator::Tab => b'\t',
            Terminator::Nul => 0,
        }
    }
}

/// How a caller names a row: by position, or by label (the first row that
/// carries it, labels being free strings the format does not keep unique).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowRef<'a> {
    Index(usize),
    Label(&'a str),
}

impl From<usize> for RowRef<'_> {
    fn from(i: usize) -> Self {
        RowRef::Index(i)
    }
}

impl<'a> From<&'a str> for RowRef<'a> {
    fn from(l: &'a str) -> Self {
        RowRef::Label(l)
    }
}

impl<'a> From<&'a String> for RowRef<'a> {
    fn from(l: &'a String) -> Self {
        RowRef::Label(l)
    }
}

impl fmt::Display for RowRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RowRef::Index(i) => write!(f, "index {i}"),
            RowRef::Label(l) => write!(f, "label `{l}`"),
        }
    }
}

/// One way two tables differ, from [`TwoDa::diff`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The second table has a column the first has not.
    ColumnAdded { name: String },
    /// The first table has a column the second has not.
    ColumnRemoved { name: String },
    /// The second table has a row, by label, the first has not.
    RowAdded { label: String },
    /// The first table has a row, by label, the second has not.
    RowRemoved { label: String },
    /// A cell both tables have, with different values.
    Cell {
        label: String,
        column: String,
        before: String,
        after: String,
    },
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Change::ColumnAdded { name } => write!(f, "column `{name}` added"),
            Change::ColumnRemoved { name } => write!(f, "column `{name}` removed"),
            Change::RowAdded { label } => write!(f, "row `{label}` added"),
            Change::RowRemoved { label } => write!(f, "row `{label}` removed"),
            Change::Cell {
                label,
                column,
                before,
                after,
            } => write!(
                f,
                "row `{label}`, column `{column}`: `{before}` -> `{after}`"
            ),
        }
    }
}

/// The reader's cursor over the header sections.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn until(&mut self, what: &'static str, terminator: u8) -> Result<&'a [u8]> {
        let start = self.pos;
        let rel = self.bytes[start..]
            .iter()
            .position(|&b| b == terminator)
            .ok_or(Error::Truncated {
                what,
                offset: start,
                available: self.bytes.len(),
            })?;
        self.pos = start + rel + 1;
        Ok(&self.bytes[start..start + rel])
    }

    fn take(&mut self, what: &'static str, len: usize) -> Result<&'a [u8]> {
        let start = self.pos;
        let end = start.checked_add(len).ok_or(Error::Truncated {
            what,
            offset: start,
            available: self.bytes.len(),
        })?;
        let slice = self.bytes.get(start..end).ok_or(Error::Truncated {
            what,
            offset: start,
            available: self.bytes.len(),
        })?;
        self.pos = end;
        Ok(slice)
    }
}

/// The header taken apart, before the cells are decoded: what the reader
/// and [`inspect`] share.
struct Header<'a> {
    /// The byte ending each column name and row label: tab or NUL.
    terminator: u8,
    columns: Vec<&'a [u8]>,
    labels: Vec<&'a [u8]>,
    /// Row-major, one per cell.
    offsets: Vec<u16>,
    declared_size: u16,
    strings: &'a [u8],
}

impl<'a> Header<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < SIGNATURE.len() || &bytes[..SIGNATURE.len()] != SIGNATURE {
            return Err(Error::NotA2da {
                found: String::from_utf8_lossy(bytes.get(..9).unwrap_or(bytes))
                    .escape_default()
                    .to_string(),
            });
        }
        let mut r = Reader {
            bytes,
            pos: SIGNATURE.len(),
        };
        // Each name ends in a tab (the members of `2da.bif`) or a NUL (the
        // members of `global.rim` and `miniglobal.rim`); the first name's
        // terminator is the file's, and a NUL where a name would begin
        // closes the list. In the tab form a name ended by the list's NUL,
        // its tab missing, is a column all the same.
        let mut columns: Vec<&[u8]> = Vec::new();
        let mut terminator = b'\t';
        loop {
            let at = r.pos;
            let Some(&first) = bytes.get(at) else {
                return Err(Error::Truncated {
                    what: "the column names",
                    offset: at,
                    available: bytes.len(),
                });
            };
            if first == 0 {
                r.pos += 1;
                break;
            }
            let rel = bytes[at..]
                .iter()
                .position(|&b| b == b'\t' || b == 0)
                .ok_or(Error::Truncated {
                    what: "the column names",
                    offset: at,
                    available: bytes.len(),
                })?;
            if columns.is_empty() {
                terminator = bytes[at + rel];
            }
            columns.push(&bytes[at..at + rel]);
            r.pos = at + rel + 1;
            if terminator == b'\t' && bytes[at + rel] == 0 {
                break;
            }
        }
        let rows = u32::from_le_bytes(r.take("the row count", 4)?.try_into().expect("four bytes"))
            as usize;
        let mut labels = Vec::with_capacity(rows);
        for _ in 0..rows {
            labels.push(r.until("the row labels", terminator)?);
        }
        let cells = rows.checked_mul(columns.len()).ok_or(Error::TooLarge {
            what: "the cells",
            size: usize::MAX,
            limit: usize::MAX,
        })?;
        let table = r.take("the cell offsets", cells * 2)?;
        let offsets = table
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        let declared_size = u16::from_le_bytes(
            r.take("the cell data size", 2)?
                .try_into()
                .expect("two bytes"),
        );
        Ok(Header {
            terminator,
            columns,
            labels,
            offsets,
            declared_size,
            strings: &bytes[r.pos..],
        })
    }

    /// The string at `offset`, up to its NUL.
    fn string(&self, offset: u16) -> Option<&'a [u8]> {
        let start = usize::from(offset);
        let rest = self.strings.get(start..)?;
        let end = rest.iter().position(|&b| b == 0)?;
        Some(&rest[..end])
    }
}

impl TwoDa {
    /// An empty table with these columns.
    pub fn new<S: Into<String>>(columns: impl IntoIterator<Item = S>) -> Self {
        TwoDa {
            columns: columns.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
            terminator: Terminator::Tab,
        }
    }

    /// The table in `bytes`. Cells are found by offset; the header's size
    /// field is not consulted. Fails on a wrong signature, a header running
    /// past the end, or a cell whose string lies past the end or lacks its
    /// NUL.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let h = Header::parse(bytes)?;
        let columns: Vec<String> = h.columns.iter().map(|c| codepage::decode(c)).collect();
        let mut rows = Vec::with_capacity(h.labels.len());
        for (r, label) in h.labels.iter().enumerate() {
            let mut cells = Vec::with_capacity(columns.len());
            for (c, name) in columns.iter().enumerate() {
                let offset = h.offsets[r * columns.len() + c];
                let s = h.string(offset).ok_or_else(|| Error::CellBeyondEnd {
                    row: r,
                    column: name.clone(),
                    offset: usize::from(offset),
                    available: h.strings.len(),
                })?;
                cells.push(codepage::decode(s));
            }
            rows.push(Row {
                label: codepage::decode(label),
                cells,
            });
        }
        let terminator = if h.terminator == 0 {
            Terminator::Nul
        } else {
            Terminator::Tab
        };
        Ok(TwoDa {
            columns,
            rows,
            terminator,
        })
    }

    /// The table in BioWare's layout: names and labels ended by the table's
    /// [`Terminator`], one copy of each distinct string in first-occurrence
    /// row-major order, the size field equal to the strings' length. Fails
    /// when a row's cell count is not the column count, a string holds a
    /// terminator or a character outside the code page, or the strings
    /// exceed the 16-bit offsets.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let end = self.terminator.byte();
        let mut out = Vec::with_capacity(64 + self.rows.len() * self.columns.len() * 4);
        out.extend_from_slice(SIGNATURE);
        for name in &self.columns {
            let bytes = encode(name, || format!("column `{name}`"), &[b'\t', 0])?;
            out.extend_from_slice(&bytes);
            out.push(end);
        }
        out.push(0);
        let rows = u32::try_from(self.rows.len()).map_err(|_| Error::TooLarge {
            what: "the rows",
            size: self.rows.len(),
            limit: u32::MAX as usize,
        })?;
        out.extend_from_slice(&rows.to_le_bytes());
        for (r, row) in self.rows.iter().enumerate() {
            if row.cells.len() != self.columns.len() {
                return Err(Error::RaggedRow {
                    row: r,
                    cells: row.cells.len(),
                    columns: self.columns.len(),
                });
            }
            let bytes = encode(&row.label, || format!("the label of row {r}"), &[b'\t', 0])?;
            out.extend_from_slice(&bytes);
            out.push(end);
        }
        let mut strings: Vec<u8> = Vec::new();
        let mut offsets: HashMap<Vec<u8>, u16> = HashMap::new();
        let mut table = Vec::with_capacity(self.rows.len() * self.columns.len() * 2);
        for (r, row) in self.rows.iter().enumerate() {
            for (cell, name) in row.cells.iter().zip(&self.columns) {
                let bytes = encode(
                    cell,
                    || format!("the cell at row {r}, column `{name}`"),
                    &[0],
                )?;
                let offset = match offsets.get(&bytes) {
                    Some(&o) => o,
                    None => {
                        let o = u16::try_from(strings.len()).map_err(|_| Error::TooLarge {
                            what: "the strings",
                            size: strings.len() + bytes.len() + 1,
                            limit: usize::from(u16::MAX),
                        })?;
                        strings.extend_from_slice(&bytes);
                        strings.push(0);
                        offsets.insert(bytes, o);
                        o
                    }
                };
                table.extend_from_slice(&offset.to_le_bytes());
            }
        }
        let size = u16::try_from(strings.len()).map_err(|_| Error::TooLarge {
            what: "the strings",
            size: strings.len(),
            limit: usize::from(u16::MAX),
        })?;
        out.extend_from_slice(&table);
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&strings);
        Ok(out)
    }

    /// The position of the column with this name, matched as written.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    /// The position of the row: the index itself when it is in range, or
    /// the first row carrying the label.
    pub fn row_index<'a>(&self, row: impl Into<RowRef<'a>>) -> Option<usize> {
        match row.into() {
            RowRef::Index(i) => (i < self.rows.len()).then_some(i),
            RowRef::Label(l) => self.rows.iter().position(|r| r.label == l),
        }
    }

    fn require_row<'a>(&self, row: impl Into<RowRef<'a>>) -> Result<usize> {
        let row = row.into();
        self.row_index(row).ok_or_else(|| Error::NoSuchRow {
            row: row.to_string(),
            rows: self.rows.len(),
        })
    }

    fn require_column(&self, name: &str) -> Result<usize> {
        self.column_index(name).ok_or_else(|| Error::NoSuchColumn {
            name: name.to_string(),
        })
    }

    /// The row, by index or label.
    pub fn row<'a>(&self, row: impl Into<RowRef<'a>>) -> Result<&Row> {
        Ok(&self.rows[self.require_row(row)?])
    }

    /// The cell at this row and column.
    pub fn get<'a>(&self, row: impl Into<RowRef<'a>>, column: &str) -> Result<&str> {
        let r = self.require_row(row)?;
        let c = self.require_column(column)?;
        Ok(self.rows[r].cells.get(c).map_or("", String::as_str))
    }

    /// Set the cell at this row and column.
    pub fn set<'a>(
        &mut self,
        row: impl Into<RowRef<'a>>,
        column: &str,
        value: impl Into<String>,
    ) -> Result<()> {
        let r = self.require_row(row)?;
        let c = self.require_column(column)?;
        let cells = &mut self.rows[r].cells;
        if cells.len() < self.columns.len() {
            cells.resize(self.columns.len(), String::new());
        }
        cells[c] = value.into();
        Ok(())
    }

    /// Append a row of blank cells with these `changes` (column name, value)
    /// applied, and return its index. Fails on a column the table has not,
    /// with the table unchanged.
    pub fn add_row(&mut self, label: impl Into<String>, changes: &[(&str, &str)]) -> Result<usize> {
        let cells = vec![String::new(); self.columns.len()];
        self.push_row(label.into(), cells, changes)
    }

    /// Append a copy of the row `from`, labelled `new_label`, with these
    /// `changes` (column name, value) applied, and return its index. Fails
    /// on a row or column the table has not, with the table unchanged.
    pub fn add_row_copying<'a>(
        &mut self,
        from: impl Into<RowRef<'a>>,
        new_label: impl Into<String>,
        changes: &[(&str, &str)],
    ) -> Result<usize> {
        let source = self.require_row(from)?;
        let mut cells = self.rows[source].cells.clone();
        cells.resize(self.columns.len(), String::new());
        self.push_row(new_label.into(), cells, changes)
    }

    fn push_row(
        &mut self,
        label: String,
        mut cells: Vec<String>,
        changes: &[(&str, &str)],
    ) -> Result<usize> {
        for (column, value) in changes {
            let c = self.require_column(column)?;
            cells[c] = (*value).to_string();
        }
        self.rows.push(Row { label, cells });
        Ok(self.rows.len() - 1)
    }

    /// Append a column, every existing row taking `fill` in it, and return
    /// its index. Fails when a column of that name exists.
    pub fn add_column(&mut self, name: impl Into<String>, fill: &str) -> Result<usize> {
        let name = name.into();
        if self.column_index(&name).is_some() {
            return Err(Error::ColumnPresent { name });
        }
        for row in &mut self.rows {
            row.cells.resize(self.columns.len(), String::new());
            row.cells.push(fill.to_string());
        }
        self.columns.push(name);
        Ok(self.columns.len() - 1)
    }

    /// Whether two tables hold the same columns, labels and cells in the
    /// same order, whatever their terminators: the comparison the
    /// round-trip suite makes.
    pub fn semantic_eq(&self, other: &TwoDa) -> bool {
        self.columns == other.columns && self.rows == other.rows
    }

    /// How `other` differs from this table, by row label and column name.
    /// Rows pair by label; where a label repeats, its first occurrence
    /// pairs with the first, the second with the second, and so on. Row and
    /// column order is not compared.
    pub fn diff(&self, other: &TwoDa) -> Vec<Change> {
        let mut changes = Vec::new();
        for name in &self.columns {
            if other.column_index(name).is_none() {
                changes.push(Change::ColumnRemoved { name: name.clone() });
            }
        }
        for name in &other.columns {
            if self.column_index(name).is_none() {
                changes.push(Change::ColumnAdded { name: name.clone() });
            }
        }
        let shared: Vec<(usize, usize)> = self
            .columns
            .iter()
            .enumerate()
            .filter_map(|(a, name)| other.column_index(name).map(|b| (a, b)))
            .collect();

        let mut theirs: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, row) in other.rows.iter().enumerate() {
            theirs.entry(&row.label).or_default().push(i);
        }
        let mut paired = vec![false; other.rows.len()];
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for row in &self.rows {
            let nth = seen.entry(&row.label).or_insert(0);
            let partner = theirs.get(row.label.as_str()).and_then(|v| v.get(*nth));
            *nth += 1;
            let Some(&partner) = partner else {
                changes.push(Change::RowRemoved {
                    label: row.label.clone(),
                });
                continue;
            };
            paired[partner] = true;
            let theirs = &other.rows[partner];
            for &(a, b) in &shared {
                let before = row.cells.get(a).map_or("", String::as_str);
                let after = theirs.cells.get(b).map_or("", String::as_str);
                if before != after {
                    changes.push(Change::Cell {
                        label: row.label.clone(),
                        column: self.columns[a].clone(),
                        before: before.to_string(),
                        after: after.to_string(),
                    });
                }
            }
        }
        for (i, row) in other.rows.iter().enumerate() {
            if !paired[i] {
                changes.push(Change::RowAdded {
                    label: row.label.clone(),
                });
            }
        }
        changes
    }
}

/// `text` as bytes of the code page, refusing any of `forbidden` (the
/// terminators of its section) and any character the page lacks.
fn encode(text: &str, what: impl Fn() -> String, forbidden: &[u8]) -> Result<Vec<u8>> {
    let bytes = codepage::encode(text).map_err(|c| Error::Unrepresentable {
        what: what(),
        reason: format!(
            "`{}` (U+{:04X}) is outside the game's code page",
            c,
            u32::from(c)
        ),
    })?;
    if let Some(b) = bytes.iter().find(|b| forbidden.contains(b)) {
        return Err(Error::Unrepresentable {
            what: what(),
            reason: format!("it contains byte {b:#04x}, the terminator of its section"),
        });
    }
    Ok(bytes)
}

/// The layout facts of a table as written, for telling one writer's files
/// from another's: what the round-trip suite reports for a file it did not
/// reproduce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// Whether column names and row labels end in NUL rather than tab.
    pub nul_terminated: bool,
    /// The header's cell-data size field.
    pub declared_size: u16,
    /// The length of the strings section, from the end of the size field to
    /// the end of the file.
    pub strings_len: usize,
    /// Whether every cell with a given string points at one offset.
    pub shared: bool,
    /// Whether the strings stand in the order the cells first meet them,
    /// row by row: BioWare's order.
    pub first_occurrence_order: bool,
    /// Bytes of the strings section no cell's string covers.
    pub unreferenced: usize,
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "2da: {}-ended names, size {} for {} bytes of strings, {}, {} order, {} unreferenced",
            if self.nul_terminated { "NUL" } else { "tab" },
            self.declared_size,
            self.strings_len,
            if self.shared {
                "one copy per string"
            } else {
                "repeated strings"
            },
            if self.first_occurrence_order {
                "first-occurrence"
            } else {
                "other"
            },
            self.unreferenced
        )
    }
}

/// The layout of the table in `bytes`.
pub fn inspect(bytes: &[u8]) -> Result<Layout> {
    let h = Header::parse(bytes)?;
    let mut first: HashMap<&[u8], u16> = HashMap::new();
    let mut shared = true;
    let mut first_occurrence_order = true;
    let mut next = 0usize;
    let mut covered = vec![false; h.strings.len()];
    for (i, &offset) in h.offsets.iter().enumerate() {
        let s = h.string(offset).ok_or_else(|| Error::CellBeyondEnd {
            row: i / h.columns.len().max(1),
            column: codepage::decode(
                h.columns
                    .get(i % h.columns.len().max(1))
                    .unwrap_or(&&b""[..]),
            ),
            offset: usize::from(offset),
            available: h.strings.len(),
        })?;
        let start = usize::from(offset);
        for c in &mut covered[start..=start + s.len()] {
            *c = true;
        }
        match first.get(s) {
            Some(&o) if o != offset => shared = false,
            Some(_) => {}
            None => {
                first.insert(s, offset);
                if start != next {
                    first_occurrence_order = false;
                }
                next += s.len() + 1;
            }
        }
    }
    Ok(Layout {
        nul_terminated: h.terminator == 0,
        declared_size: h.declared_size,
        strings_len: h.strings.len(),
        shared,
        first_occurrence_order,
        unreferenced: covered.iter().filter(|&&c| !c).count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worked example of `docs/formats/2da.md`: four columns, three
    /// rows, every cell distinct.
    fn worked_example() -> TwoDa {
        let mut t = TwoDa::new(["label", "name_strref", "description_strref", "aistate"]);
        t.add_row(
            "0",
            &[
                ("label", "DEFAULT"),
                ("name_strref", "1116"),
                ("description_strref", "1109"),
                ("aistate", "0"),
            ],
        )
        .unwrap();
        t.add_row(
            "1",
            &[
                ("label", "GRENADE"),
                ("name_strref", "1120"),
                ("description_strref", "1112"),
                ("aistate", "4"),
            ],
        )
        .unwrap();
        t.add_row(
            "2",
            &[
                ("label", "JEDI"),
                ("name_strref", "1121"),
                ("description_strref", "1114"),
                ("aistate", "5"),
            ],
        )
        .unwrap();
        t
    }

    #[test]
    fn the_worked_example_writes_the_documented_offsets_and_size() {
        let bytes = worked_example().to_bytes().unwrap();
        assert!(
            bytes.starts_with(b"2DA V2.b\nlabel\tname_strref\tdescription_strref\taistate\t\0")
        );
        let h = Header::parse(&bytes).unwrap();
        assert_eq!(h.labels, [b"0", b"1", b"2"]);
        assert_eq!(h.offsets, [0, 8, 13, 18, 20, 28, 33, 38, 40, 45, 50, 55]);
        assert_eq!(h.declared_size, 57);
        assert_eq!(h.strings.len(), 57);
        assert!(bytes.ends_with(b"JEDI\x001121\x001114\x005\x00"));
        assert_eq!(TwoDa::parse(&bytes).unwrap(), worked_example());
    }

    #[test]
    fn identical_strings_are_stored_once_in_first_occurrence_order() {
        let mut t = TwoDa::new(["a", "b"]);
        t.add_row("0", &[("a", ""), ("b", "1")]).unwrap();
        t.add_row("1", &[("a", "1"), ("b", "")]).unwrap();
        t.add_row("2", &[("a", "x"), ("b", "1")]).unwrap();
        let bytes = t.to_bytes().unwrap();
        let h = Header::parse(&bytes).unwrap();
        assert_eq!(h.offsets, [0, 1, 1, 0, 3, 1]);
        assert_eq!(h.strings, b"\x001\0x\0");
        assert_eq!(h.declared_size, 5);
        let layout = inspect(&bytes).unwrap();
        assert!(layout.shared && layout.first_occurrence_order);
        assert_eq!(layout.unreferenced, 0);
        assert_eq!(
            layout.to_string(),
            "2da: tab-ended names, size 5 for 5 bytes of strings, one copy per string, first-occurrence order, 0 unreferenced"
        );
    }

    #[test]
    fn a_literal_four_asterisk_cell_is_kept_apart_from_the_blank() {
        let mut t = TwoDa::new(["a"]);
        t.add_row("0", &[("a", "****")]).unwrap();
        t.add_row("1", &[]).unwrap();
        let again = TwoDa::parse(&t.to_bytes().unwrap()).unwrap();
        assert_eq!(again.get(0, "a").unwrap(), TEXT_BLANK);
        assert_eq!(again.get(1, "a").unwrap(), "");
        assert_eq!(again, t);
    }

    #[test]
    fn rows_are_found_by_label_or_index_and_labels_need_not_be_the_index() {
        let mut t = TwoDa::new(["v"]);
        t.add_row("217", &[("v", "a")]).unwrap();
        t.add_row("63", &[("v", "b")]).unwrap();
        t.add_row("63", &[("v", "c")]).unwrap();
        assert_eq!(t.get("217", "v").unwrap(), "a");
        assert_eq!(t.get("63", "v").unwrap(), "b");
        assert_eq!(t.get(2, "v").unwrap(), "c");
        assert_eq!(t.row_index("0"), None);
        let e = t.get("nope", "v").unwrap_err();
        assert_eq!(e.code(), codes::NO_SUCH_ROW);
        assert_eq!(t.get(3, "v").unwrap_err().code(), codes::NO_SUCH_ROW);
        assert_eq!(t.get(0, "V").unwrap_err().code(), codes::NO_SUCH_COLUMN);
        let again = TwoDa::parse(&t.to_bytes().unwrap()).unwrap();
        assert_eq!(again, t);
    }

    #[test]
    fn set_add_row_copying_and_add_column() {
        let mut t = worked_example();
        t.set("1", "aistate", "7").unwrap();
        assert_eq!(t.get(1, "aistate").unwrap(), "7");
        let i = t
            .add_row_copying("2", "3", &[("label", "SITH"), ("aistate", "6")])
            .unwrap();
        assert_eq!(i, 3);
        assert_eq!(t.rows[3].label, "3");
        assert_eq!(t.get(3, "name_strref").unwrap(), "1121");
        assert_eq!(t.get(3, "label").unwrap(), "SITH");
        let before = t.clone();
        assert_eq!(
            t.add_row_copying("9", "4", &[]).unwrap_err().code(),
            codes::NO_SUCH_ROW
        );
        assert_eq!(
            t.add_row_copying("2", "4", &[("zzz", "1")])
                .unwrap_err()
                .code(),
            codes::NO_SUCH_COLUMN
        );
        assert_eq!(t, before, "a failed add leaves the table as it was");
        let c = t.add_column("extra", "****").unwrap();
        assert_eq!(c, 4);
        assert!(t.rows.iter().all(|r| r.cells[4] == "****"));
        assert_eq!(
            t.add_column("extra", "").unwrap_err().code(),
            codes::COLUMN_PRESENT
        );
        assert_eq!(TwoDa::parse(&t.to_bytes().unwrap()).unwrap(), t);
    }

    #[test]
    fn diff_lists_the_changed_cell_by_label_and_column_only() {
        let a = worked_example();
        let mut b = a.clone();
        b.set("1", "aistate", "9").unwrap();
        assert_eq!(
            a.diff(&b),
            [Change::Cell {
                label: "1".into(),
                column: "aistate".into(),
                before: "4".into(),
                after: "9".into(),
            }]
        );
        assert!(a.diff(&a).is_empty());
    }

    #[test]
    fn diff_names_added_and_removed_rows_and_columns_and_pairs_repeated_labels() {
        let mut a = TwoDa::new(["v", "w"]);
        a.add_row("x", &[("v", "1")]).unwrap();
        a.add_row("x", &[("v", "2")]).unwrap();
        a.add_row("gone", &[]).unwrap();
        let mut b = TwoDa::new(["v", "n"]);
        b.add_row("x", &[("v", "1")]).unwrap();
        b.add_row("x", &[("v", "3")]).unwrap();
        b.add_row("new", &[]).unwrap();
        assert_eq!(
            a.diff(&b),
            [
                Change::ColumnRemoved { name: "w".into() },
                Change::ColumnAdded { name: "n".into() },
                Change::Cell {
                    label: "x".into(),
                    column: "v".into(),
                    before: "2".into(),
                    after: "3".into(),
                },
                Change::RowRemoved {
                    label: "gone".into()
                },
                Change::RowAdded {
                    label: "new".into()
                },
            ]
        );
    }

    #[test]
    fn malformed_tables_are_refused_with_their_codes() {
        assert_eq!(
            TwoDa::parse(b"2DA V2.0\n").unwrap_err().code(),
            codes::NOT_A_2DA
        );
        assert_eq!(TwoDa::parse(b"2DA").unwrap_err().code(), codes::NOT_A_2DA);
        let good = worked_example().to_bytes().unwrap();
        for cut in [10, 40, 60, 70, 80] {
            let e = TwoDa::parse(&good[..cut]).unwrap_err();
            assert_eq!(e.code(), codes::TRUNCATED, "cut at {cut}: {e}");
        }
        // The last string's NUL gone: that cell has no end.
        let e = TwoDa::parse(&good[..good.len() - 1]).unwrap_err();
        assert_eq!(e.code(), codes::CELL_BEYOND_END);
        assert!(e.to_string().contains("row 2, column `aistate`"), "{e}");
        // An offset past the strings.
        let mut bad = good.clone();
        let table = bad.len() - 57 - 2 - 24;
        bad[table..table + 2].copy_from_slice(&200u16.to_le_bytes());
        assert_eq!(
            TwoDa::parse(&bad).unwrap_err().code(),
            codes::CELL_BEYOND_END
        );
    }

    #[test]
    fn unwritable_strings_and_ragged_rows_are_refused() {
        let mut t = TwoDa::new(["a"]);
        t.add_row("0", &[("a", "x\0y")]).unwrap();
        assert_eq!(t.to_bytes().unwrap_err().code(), codes::UNREPRESENTABLE);
        t.rows[0].cells[0] = "\u{4E2D}".into();
        assert_eq!(t.to_bytes().unwrap_err().code(), codes::UNREPRESENTABLE);
        t.rows[0].cells[0] = "caf\u{E9}".into();
        assert_eq!(TwoDa::parse(&t.to_bytes().unwrap()).unwrap(), t);
        t.rows[0].label = "a\tb".into();
        assert_eq!(t.to_bytes().unwrap_err().code(), codes::UNREPRESENTABLE);
        t.rows[0].label = "0".into();
        t.rows[0].cells.push("extra".into());
        assert_eq!(t.to_bytes().unwrap_err().code(), codes::RAGGED_ROW);
        let mut t = TwoDa::new(["a\tb"]);
        t.add_row("0", &[]).unwrap();
        assert_eq!(t.to_bytes().unwrap_err().code(), codes::UNREPRESENTABLE);
    }

    #[test]
    fn a_strings_section_past_sixteen_bits_is_refused() {
        let mut t = TwoDa::new(["a"]);
        for i in 0..7000 {
            t.add_row(i.to_string(), &[("a", &format!("string-{i:05}"))])
                .unwrap();
        }
        assert_eq!(t.to_bytes().unwrap_err().code(), codes::TOO_LARGE);
    }

    #[test]
    fn nul_ended_names_and_labels_are_kept_and_written_back_as_read() {
        let t = worked_example();
        assert_eq!(t.terminator, Terminator::Tab);
        let tabbed = t.to_bytes().unwrap();
        let mut bytes = b"2DA V2.b\nlabel\0name_strref\0description_strref\0aistate\0\0".to_vec();
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(b"0\x001\x002\0");
        let tail = tabbed.len() - 57 - 2 - 24;
        bytes.extend_from_slice(&tabbed[tail..]);
        let read = TwoDa::parse(&bytes).unwrap();
        assert_eq!(read.terminator, Terminator::Nul);
        assert!(read.semantic_eq(&t));
        assert!(read.diff(&t).is_empty());
        assert_ne!(read, t, "the terminator is part of the model");
        assert_eq!(read.to_bytes().unwrap(), bytes);
        let mut retabbed = read.clone();
        retabbed.terminator = Terminator::Tab;
        assert_eq!(retabbed.to_bytes().unwrap(), tabbed);
        let layout = inspect(&bytes).unwrap();
        assert!(layout.nul_terminated);
        assert!(
            layout
                .to_string()
                .starts_with("2da: NUL-ended names, size 57 for 57 bytes"),
            "{layout}"
        );
        assert_eq!(t.to_bytes().unwrap(), tabbed);
    }

    #[test]
    fn a_name_whose_tab_is_missing_before_the_nul_is_still_a_column() {
        let mut bytes = b"2DA V2.b\na\tb\0".to_vec();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(b"0\t");
        bytes.extend_from_slice(&[0, 0, 2, 0]);
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(b"x\0y\0");
        let t = TwoDa::parse(&bytes).unwrap();
        assert_eq!(t.columns, ["a", "b"]);
        assert_eq!(t.get(0, "b").unwrap(), "y");
    }
}
