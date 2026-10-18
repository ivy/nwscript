//! TXI: the texture's instructions to the engine, as text.

use std::collections::BTreeMap;

use crate::{Error, Result};

/// One instruction: a keyword, the rest of its line, and the numeric lines
/// that follow it (a font's `upperleftcoords 96` is followed by 96 rows of
/// coordinates, a `channelscale 4` by four values).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxiEntry {
    /// The keyword as written; the game's files spell some in mixed case
    /// (`Bumpyshinytexture`), and [`Txi::get`] ignores case.
    pub key: String,
    /// The rest of the line after the keyword and its whitespace, trimmed;
    /// empty when the keyword stands alone.
    pub value: String,
    /// The lines after this one that begin with a number, a sign or a
    /// point, trimmed, in order.
    pub rows: Vec<String>,
}

/// A TXI, from a `.txi` resource or the text a TPC carries after its pixel
/// data, as an ordered list of instructions.
///
/// Equality compares the instructions and the preamble; [`Txi::layout`],
/// how the text was spelled, is not compared.
#[derive(Debug, Clone, Default, Eq)]
pub struct Txi {
    /// Numeric lines that come before any keyword.
    pub preamble: Vec<String>,
    /// The instructions in file order; a keyword written twice appears
    /// twice.
    pub entries: Vec<TxiEntry>,
    /// How the text read departs from the form [`Txi::to_bytes`] writes,
    /// so that a TXI read and written again is the bytes it came from.
    /// [`TxiLayout::default`] for a TXI made from nothing.
    pub layout: TxiLayout,
}

impl PartialEq for Txi {
    fn eq(&self, other: &Txi) -> bool {
        self.preamble == other.preamble && self.entries == other.entries
    }
}

/// The spelling of a TXI's text beyond its instructions: the line ending,
/// whether the last line has one, and every line whose bytes differ from
/// the written form (blank lines, extra whitespace, NUL bytes, a stray
/// ending), kept as read.
///
/// The written form of a line is the keyword, a space and the value (the
/// keyword alone when the value is empty), or the row, then the line
/// ending: CR LF unless the first line read ends LF alone. A blank line is
/// not a line of its own: its bytes join the line before it, or the first
/// line when it comes before any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxiLayout {
    lf: bool,
    final_newline: bool,
    /// By line index: the written form the line had when read, and its
    /// bytes as read.
    departures: BTreeMap<usize, (String, Vec<u8>)>,
    /// The bytes of a text with no line that is not blank.
    blank: Vec<u8>,
}

impl Default for TxiLayout {
    /// CR LF line endings, the last line ended too, nothing departing.
    fn default() -> Self {
        TxiLayout {
            lf: false,
            final_newline: true,
            departures: BTreeMap::new(),
            blank: Vec::new(),
        }
    }
}

impl TxiLayout {
    /// How many lines depart from the written form.
    pub fn departures(&self) -> usize {
        self.departures.len()
    }

    /// Whether the last line ends with a line ending.
    pub fn final_newline(&self) -> bool {
        self.final_newline
    }

    fn newline(&self) -> &'static [u8] {
        if self.lf { b"\n" } else { b"\r\n" }
    }

    /// Line `index` as the writer produces it with nothing recorded: its
    /// written form, ended unless it is the last of a text whose last line
    /// has no ending.
    fn expected(&self, canonical: &[u8], last: bool) -> Vec<u8> {
        let mut out = canonical.to_vec();
        if !last || self.final_newline {
            out.extend_from_slice(self.newline());
        }
        out
    }
}

/// Bytes as Latin-1 text.
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// Whether a trimmed line is a numeric row rather than a keyword's.
fn is_row(line: &str) -> bool {
    line.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'))
}

/// One line read: its written form and its bytes, blank lines after it
/// included.
struct RawLine {
    canonical: String,
    raw: Vec<u8>,
    /// Whether the line's own bytes (before any blank lines folded in)
    /// end with LF.
    ended: bool,
    crlf: bool,
}

impl Txi {
    /// Read TXI text. Never fails: bytes are read as Latin-1, lines end at
    /// LF with an optional CR before it, NUL bytes are dropped, and blank
    /// lines are skipped. The bytes as read are kept in [`Txi::layout`].
    pub fn parse(bytes: &[u8]) -> Txi {
        let mut txi = Txi::default();
        let mut lines: Vec<RawLine> = Vec::new();
        // Blank lines before the first line that is not.
        let mut leading: Vec<u8> = Vec::new();
        for raw in bytes.split_inclusive(|&b| b == b'\n') {
            let text: String = latin1(&raw.iter().copied().filter(|&b| b != 0).collect::<Vec<_>>());
            let line = text.trim();
            if line.is_empty() {
                match lines.last_mut() {
                    Some(l) => l.raw.extend_from_slice(raw),
                    None => leading.extend_from_slice(raw),
                }
                continue;
            }
            let canonical = if is_row(line) {
                match txi.entries.last_mut() {
                    Some(e) => e.rows.push(line.to_string()),
                    None => txi.preamble.push(line.to_string()),
                }
                line.to_string()
            } else {
                let (key, value) = match line.split_once(|c: char| c.is_whitespace()) {
                    Some((k, v)) => (k, v.trim()),
                    None => (line, ""),
                };
                txi.entries.push(TxiEntry {
                    key: key.to_string(),
                    value: value.to_string(),
                    rows: Vec::new(),
                });
                written_entry(key, value)
            };
            let mut full = std::mem::take(&mut leading);
            full.extend_from_slice(raw);
            lines.push(RawLine {
                canonical,
                raw: full,
                ended: raw.ends_with(b"\n"),
                crlf: raw.ends_with(b"\r\n"),
            });
        }
        let layout = &mut txi.layout;
        if lines.is_empty() {
            layout.blank = leading;
            return txi;
        }
        if let Some(first) = lines.iter().find(|l| l.ended) {
            layout.lf = !first.crlf;
        }
        layout.final_newline = lines.last().is_some_and(|l| l.ended);
        let last = lines.len() - 1;
        for (i, l) in lines.into_iter().enumerate() {
            let canonical = l.canonical.chars().map(|c| c as u8).collect::<Vec<_>>();
            if layout.expected(&canonical, i == last) != l.raw {
                layout.departures.insert(i, (l.canonical, l.raw));
            }
        }
        txi
    }

    /// Write the text: each line in its written form, or as read where
    /// [`Txi::layout`] recorded it departing and the line is still the
    /// same.
    ///
    /// Refuses (`RE2208`) a TXI that would not read back as itself: a
    /// character outside Latin-1, a keyword that is empty, holds
    /// whitespace or reads as a row, a row that does not begin as one, or
    /// a value or row with a line break, a NUL, or whitespace at its ends.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut lines: Vec<String> = Vec::new();
        for row in &self.preamble {
            check_row(row)?;
            lines.push(row.clone());
        }
        for e in &self.entries {
            check_key(&e.key)?;
            check_text(&e.key, &e.value)?;
            lines.push(written_entry(&e.key, &e.value));
            for row in &e.rows {
                check_row(row)?;
                lines.push(row.clone());
            }
        }
        if lines.is_empty() {
            return Ok(self.layout.blank.clone());
        }
        let mut out = Vec::new();
        let last = lines.len() - 1;
        for (i, line) in lines.iter().enumerate() {
            let canonical = encode(line)?;
            match self.layout.departures.get(&i) {
                Some((c, raw)) if c == line => {
                    out.extend_from_slice(raw);
                    // A line read last, without an ending, that is no
                    // longer last.
                    if i != last && !raw.ends_with(b"\n") {
                        out.extend_from_slice(self.layout.newline());
                    }
                }
                _ => out.extend(self.layout.expected(&canonical, i == last)),
            }
        }
        Ok(out)
    }

    /// The first instruction with this keyword, in any case.
    pub fn get(&self, key: &str) -> Option<&TxiEntry> {
        self.entries
            .iter()
            .find(|e| e.key.eq_ignore_ascii_case(key))
    }

    /// The value of the first instruction with this keyword, in any case.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.get(key).map(|e| e.value.as_str())
    }

    /// The first instruction with this keyword whose value's first word
    /// reads as an integer.
    pub fn int(&self, key: &str) -> Option<i64> {
        self.value(key)?.split_whitespace().next()?.parse().ok()
    }

    /// Whether there are no instructions and no preamble.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.preamble.is_empty()
    }
}

fn written_entry(key: &str, value: &str) -> String {
    if value.is_empty() {
        key.to_string()
    } else {
        format!("{key} {value}")
    }
}

fn unwritable(text: &str, reason: &str) -> Error {
    Error::TxiUnwritable {
        line: text.to_string(),
        reason: reason.to_string(),
    }
}

fn encode(line: &str) -> Result<Vec<u8>> {
    line.chars()
        .map(|c| {
            u8::try_from(u32::from(c)).map_err(|_| unwritable(line, "a character outside Latin-1"))
        })
        .collect()
}

/// A value or row: one line, no NUL, nothing the reader would trim.
fn check_text(line: &str, text: &str) -> Result<()> {
    if text.contains(['\n', '\0']) {
        return Err(unwritable(line, "a line break or NUL"));
    }
    if text.trim() != text {
        return Err(unwritable(line, "whitespace at an end"));
    }
    encode(text).map(|_| ())
}

fn check_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(unwritable(key, "an empty keyword"));
    }
    if key.contains(|c: char| c.is_whitespace() || c == '\0') {
        return Err(unwritable(key, "whitespace or NUL in a keyword"));
    }
    if is_row(key) {
        return Err(unwritable(key, "a keyword that reads as a numeric row"));
    }
    encode(key).map(|_| ())
}

fn check_row(row: &str) -> Result<()> {
    if !is_row(row) {
        return Err(unwritable(
            row,
            "a row that does not begin with a digit, a sign or a point",
        ));
    }
    check_text(row, row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_values_rows_and_preamble() {
        let t = Txi::parse(
            b"\r\n3 grass\r\nmipmap 0 \r\nBumpyShinyTexture CM_Bright\r\nupperleftcoords 2\r\n0.0 1.0 0\r\n0.5 1.0 0\r\nisbumpmap\r\n\0",
        );
        assert_eq!(t.preamble, ["3 grass"]);
        assert_eq!(t.entries.len(), 4);
        assert_eq!(t.value("mipmap"), Some("0"));
        assert_eq!(t.int("mipmap"), Some(0));
        assert_eq!(t.value("bumpyshinytexture"), Some("CM_Bright"));
        assert_eq!(t.get("BUMPYSHINYTEXTURE").unwrap().key, "BumpyShinyTexture");
        assert_eq!(
            t.get("upperleftcoords").unwrap().rows,
            ["0.0 1.0 0", "0.5 1.0 0"]
        );
        assert_eq!(t.value("isbumpmap"), Some(""));
        assert_eq!(t.value("absent"), None);
    }

    #[test]
    fn empty_text_is_empty() {
        assert!(Txi::parse(b"").is_empty());
        assert!(Txi::parse(b"\r\n\r\n").is_empty());
    }

    #[test]
    fn latin1_bytes_survive() {
        let t = Txi::parse(b"name caf\xe9");
        assert_eq!(t.value("name"), Some("caf\u{e9}"));
    }

    #[test]
    fn any_bytes_write_back_as_read() {
        for text in [
            &b""[..],
            b"\r\n\r\n",
            b"\0",
            b"mipmap 0",
            b"mipmap 0\r\n",
            b"mipmap 0\n",
            b"\r\n3 grass\r\nmipmap 0 \r\nBumpyShinyTexture CM_Bright\r\nupperleftcoords 2\r\n0.0 1.0 0\r\n0.5 1.0 0\r\nisbumpmap\r\n\0",
            b"a\tb\rc\n\n\nd  e\r\n  \r\n1 2\0 3\n",
            b"name caf\xe9\xff\r\n",
        ] {
            let t = Txi::parse(text);
            assert_eq!(t.to_bytes().unwrap(), text, "{:?}", latin1(text));
        }
    }

    #[test]
    fn written_form_and_edits() {
        let t = Txi::parse(b"mipmap  0\r\nblending additive");
        assert_eq!(t.layout.departures(), 1);
        assert!(!t.layout.final_newline());
        // An entry added after a last line without an ending.
        let mut e = t.clone();
        e.entries.push(TxiEntry {
            key: "decal".into(),
            value: "1".into(),
            rows: vec![],
        });
        assert_eq!(
            e.to_bytes().unwrap(),
            b"mipmap  0\r\nblending additive\r\ndecal 1"
        );
        // A changed line is written in the written form.
        let mut e = t.clone();
        e.entries[0].value = "1".into();
        assert_eq!(e.to_bytes().unwrap(), b"mipmap 1\r\nblending additive");
        // Made from nothing.
        let fresh = Txi {
            entries: vec![TxiEntry {
                key: "upperleftcoords".into(),
                value: "1".into(),
                rows: vec!["0.5 0.5 0".into()],
            }],
            ..Txi::default()
        };
        assert_eq!(
            fresh.to_bytes().unwrap(),
            b"upperleftcoords 1\r\n0.5 0.5 0\r\n"
        );
        assert_eq!(Txi::parse(&fresh.to_bytes().unwrap()), fresh);
    }

    #[test]
    fn what_would_not_read_back_is_refused() {
        let bad = |key: &str, value: &str| Txi {
            entries: vec![TxiEntry {
                key: key.into(),
                value: value.into(),
                rows: vec![],
            }],
            ..Txi::default()
        };
        for t in [
            bad("", "x"),
            bad("two words", ""),
            bad("4key", ""),
            bad("key", " padded"),
            bad("key", "line\nbreak"),
            bad("key", "\u{263a}"),
        ] {
            assert_eq!(t.to_bytes().unwrap_err().code(), "RE2208");
        }
        let row = Txi {
            preamble: vec!["word".into()],
            ..Txi::default()
        };
        assert_eq!(row.to_bytes().unwrap_err().code(), "RE2208");
    }
}
