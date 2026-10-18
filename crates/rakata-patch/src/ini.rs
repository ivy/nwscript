//! The INI dialect TSLPatcher reads, as `docs/formats/changes-ini.md`
//! records it: `[Section]` headers, `Key=Value` lines, `;` comments, blank
//! lines; nothing else. Bytes are Windows-1252. Section and key names are
//! compared without regard to case; everything keeps its order and its line
//! number, because the model built on top reports every finding by line.

use crate::issue::{Issue, codes};
use serde::Serialize;

/// One `Key=Value` line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub key: String,
    pub value: String,
    /// One-based line in the file.
    pub line: u32,
}

/// One `[Section]` and the entries under it, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub line: u32,
    pub entries: Vec<Entry>,
}

impl Section {
    /// The first entry whose key is `key`, case-insensitively.
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.key.eq_ignore_ascii_case(key))
    }

    /// The value of [`Section::get`].
    pub fn value(&self, key: &str) -> Option<&str> {
        self.get(key).map(|e| e.value.as_str())
    }
}

/// A parsed INI file: its sections in file order, duplicates included.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ini {
    pub sections: Vec<Section>,
}

impl Ini {
    /// Parse `bytes` as Windows-1252 text. Every line that is not a header,
    /// an entry, a comment or blank is reported under `RE0301` and skipped;
    /// an entry before the first header is reported under `RE0302`. `file`
    /// is the index the issues carry.
    pub fn parse(bytes: &[u8], file: usize, issues: &mut Vec<Issue>) -> Ini {
        let text = decode_1252(bytes);
        let mut ini = Ini::default();
        for (i, raw) in text.lines().enumerate() {
            let line = i as u32 + 1;
            let s = raw.trim();
            if s.is_empty() || s.starts_with(';') {
                continue;
            }
            if let Some(rest) = s.strip_prefix('[') {
                if let Some(name) = rest.strip_suffix(']') {
                    ini.sections.push(Section {
                        name: name.trim().to_string(),
                        line,
                        entries: Vec::new(),
                    });
                    continue;
                }
                issues.push(Issue::new(
                    codes::MALFORMED_LINE,
                    format!("a section header without its closing `]`: `{s}`"),
                    file,
                    line,
                ));
                continue;
            }
            let Some((k, v)) = s.split_once('=') else {
                issues.push(Issue::new(
                    codes::MALFORMED_LINE,
                    format!("neither a section header nor `Key=Value`: `{s}`"),
                    file,
                    line,
                ));
                continue;
            };
            let entry = Entry {
                key: k.trim().to_string(),
                value: v.trim().to_string(),
                line,
            };
            match ini.sections.last_mut() {
                Some(sec) => sec.entries.push(entry),
                None => issues.push(Issue::new(
                    codes::ENTRY_BEFORE_SECTION,
                    format!("`{}` comes before any `[Section]` header", entry.key),
                    file,
                    line,
                )),
            }
        }
        ini
    }

    /// The first section named `name`, case-insensitively: what the Windows
    /// INI functions TSLPatcher reads through return when a name repeats.
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// Every section named `name`, for reporting repeats.
    pub fn sections_named(&self, name: &str) -> Vec<&Section> {
        self.sections
            .iter()
            .filter(|s| s.name.eq_ignore_ascii_case(name))
            .collect()
    }
}

/// Windows-1252 to text; the 0x80..0x9F block is the code page's, the rest
/// is Latin-1.
pub fn decode_1252(bytes: &[u8]) -> String {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    let body = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    // A file that is valid UTF-8 and has bytes above 0x7F was written by a
    // modern editor; its text is what the author saw.
    if body.iter().any(|&b| b >= 0x80)
        && let Ok(s) = std::str::from_utf8(body)
    {
        return s.to_string();
    }
    body.iter()
        .map(|&b| match b {
            0x80..=0x9f => HIGH[(b - 0x80) as usize],
            _ => b as char,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_entries_comments_and_blanks() {
        let mut issues = Vec::new();
        let ini = Ini::parse(
            b"; a comment\r\n[Settings]\r\nLogLevel = 3\r\n\r\n[ gff.utc ]\r\nA\\B=x=y\r\n",
            0,
            &mut issues,
        );
        assert!(issues.is_empty());
        assert_eq!(ini.sections.len(), 2);
        assert_eq!(
            ini.section("settings").unwrap().value("loglevel"),
            Some("3")
        );
        let e = &ini.section("gff.utc").unwrap().entries[0];
        assert_eq!(
            (e.key.as_str(), e.value.as_str(), e.line),
            ("A\\B", "x=y", 6)
        );
    }

    #[test]
    fn stray_lines_are_reported_by_line_and_skipped() {
        let mut issues = Vec::new();
        let ini = Ini::parse(
            b"Orphan=1\n[S]\n// not a comment\n[Open\nK=V\n",
            3,
            &mut issues,
        );
        let codes: Vec<(&str, u32)> = issues.iter().map(|i| (i.code, i.line)).collect();
        assert_eq!(
            codes,
            [
                (codes::ENTRY_BEFORE_SECTION, 1),
                (codes::MALFORMED_LINE, 3),
                (codes::MALFORMED_LINE, 4),
            ]
        );
        assert!(issues.iter().all(|i| i.file == 3));
        assert_eq!(ini.section("S").unwrap().value("K"), Some("V"));
    }

    #[test]
    fn repeated_sections_answer_with_the_first() {
        let mut issues = Vec::new();
        let ini = Ini::parse(b"[A]\nX=1\n[a]\nX=2\n", 0, &mut issues);
        assert_eq!(ini.section("A").unwrap().value("X"), Some("1"));
        assert_eq!(ini.sections_named("A").len(), 2);
    }

    #[test]
    fn bytes_are_windows_1252_unless_the_file_is_utf8() {
        assert_eq!(decode_1252(b"caf\xe9 \x96 x"), "café – x");
        assert_eq!(decode_1252("café".as_bytes()), "café");
        assert_eq!(decode_1252(b"\xef\xbb\xbf[A]"), "[A]");
    }
}
