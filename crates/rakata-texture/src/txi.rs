//! TXI: the texture's instructions to the engine, as text.

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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Txi {
    /// Numeric lines that come before any keyword.
    pub preamble: Vec<String>,
    /// The instructions in file order; a keyword written twice appears
    /// twice.
    pub entries: Vec<TxiEntry>,
}

impl Txi {
    /// Read TXI text. Never fails: bytes are read as Latin-1, lines end at
    /// LF with an optional CR before it, NUL bytes are dropped, and blank
    /// lines are skipped.
    pub fn parse(bytes: &[u8]) -> Txi {
        let text: String = bytes
            .iter()
            .filter(|&&b| b != 0)
            .map(|&b| char::from(b))
            .collect();
        let mut txi = Txi::default();
        for line in text.split('\n') {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '-' | '+' | '.')) {
                match txi.entries.last_mut() {
                    Some(e) => e.rows.push(line.to_string()),
                    None => txi.preamble.push(line.to_string()),
                }
                continue;
            }
            let (key, value) = match line.split_once(|c: char| c.is_whitespace()) {
                Some((k, v)) => (k, v.trim()),
                None => (line, ""),
            };
            txi.entries.push(TxiEntry {
                key: key.to_string(),
                value: value.to_string(),
                rows: Vec::new(),
            });
        }
        txi
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
}
