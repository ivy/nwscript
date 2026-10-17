//! Source files as bytes, and the mapping from byte offsets to lines.
//!
//! NWScript files are Windows-1252 with CRLF line ends, so a file is kept as
//! bytes; text is decoded only for display. Byte offsets are the canonical
//! position; lines and columns are derived, 1-based, columns counted in bytes.

use std::sync::Arc;

/// A file's index in a [`SourceMap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

/// A byte range in one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(file: FileId, start: usize, end: usize) -> Self {
        Span { file, start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// The smallest span covering both, which must be in the same file.
    pub fn to(self, other: Span) -> Span {
        Span {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// A 1-based line and byte column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct SourceFile {
    pub name: String,
    pub bytes: Arc<[u8]>,
    /// True for bytecode and other files that are not text: a diagnostic
    /// shows an offset into them, not a snippet.
    pub binary: bool,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes: Arc<[u8]> = bytes.into();
        let mut line_starts = vec![0];
        line_starts.extend(
            bytes
                .iter()
                .enumerate()
                .filter(|(_, b)| **b == b'\n')
                .map(|(i, _)| i + 1),
        );
        SourceFile {
            name: name.into(),
            bytes,
            line_starts,
            binary: false,
        }
    }

    pub fn line_col(&self, offset: usize) -> LineCol {
        let offset = offset.min(self.bytes.len());
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        LineCol {
            line: line + 1,
            col: offset - self.line_starts[line] + 1,
        }
    }

    /// The byte offset in [`Self::display_text`] of a byte offset in the file.
    pub fn display_offset(&self, offset: usize) -> usize {
        let offset = offset.min(self.bytes.len());
        self.bytes[..offset]
            .iter()
            .map(|&b| decode_1252(b).len_utf8())
            .sum()
    }

    /// The text for display: Windows-1252 decoded, so every byte is one char
    /// and byte offsets stay valid as char offsets.
    pub fn display_text(&self) -> String {
        self.bytes.iter().map(|&b| decode_1252(b)).collect()
    }
}

/// Windows-1252 to Unicode. Undefined bytes map to the C1 control of the same value.
pub fn decode_1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9f => HIGH[(b - 0x80) as usize],
        _ => b as char,
    }
}

/// Every file a run has read, indexed by [`FileId`].
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> FileId {
        self.files.push(SourceFile::new(name, bytes));
        FileId(self.files.len() as u32 - 1)
    }

    /// Add a file that is not text, such as NCS bytecode.
    pub fn add_binary(&mut self, name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> FileId {
        let id = self.add(name, bytes);
        self.files[id.0 as usize].binary = true;
        id
    }

    pub fn get(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    pub fn files(&self) -> impl Iterator<Item = (FileId, &SourceFile)> {
        self.files
            .iter()
            .enumerate()
            .map(|(i, f)| (FileId(i as u32), f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_lines_and_columns() {
        let f = SourceFile::new("a.nss", b"ab\r\ncd\r\n\r\nx".to_vec());
        assert_eq!(f.line_col(0), LineCol { line: 1, col: 1 });
        assert_eq!(f.line_col(4), LineCol { line: 2, col: 1 });
        assert_eq!(f.line_col(5), LineCol { line: 2, col: 2 });
        assert_eq!(f.line_col(10), LineCol { line: 4, col: 1 });
        assert_eq!(f.line_col(99), LineCol { line: 4, col: 2 });
    }

    #[test]
    fn non_utf8_bytes_keep_offsets() {
        let f = SourceFile::new("a.nss", vec![b'a', 0x93, b'b', 0x94]);
        let t = f.display_text();
        assert_eq!(t.chars().count(), 4);
        assert_eq!(t, "a“b”");
    }
}
