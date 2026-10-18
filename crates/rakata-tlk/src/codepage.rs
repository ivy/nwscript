//! Windows-1252, the code page KotOR's talk table and GFF strings are
//! written in, decoded and encoded so that every byte comes back as it was.
//!
//! The five bytes the code page leaves undefined (0x81, 0x8D, 0x8F, 0x90,
//! 0x9D) decode to the C1 control characters of the same value, which no
//! defined byte maps to, so a text read from a file always re-encodes to
//! the bytes it was read from. A character outside the code page cannot be
//! encoded, and [`encode`] names it.

/// The characters bytes 0x80 to 0x9F decode to; the five undefined bytes
/// map to the control character of their own value.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// The character one byte decodes to.
pub fn decode_byte(b: u8) -> char {
    match b {
        0x80..=0x9F => HIGH[(b - 0x80) as usize],
        _ => b as char,
    }
}

/// The byte one character encodes to, if the code page has it.
pub fn encode_char(c: char) -> Option<u8> {
    let cp = c as u32;
    if cp < 0x80 || (0xA0..=0xFF).contains(&cp) {
        return Some(cp as u8);
    }
    HIGH.iter().position(|&h| h == c).map(|i| 0x80 + i as u8)
}

/// The text of a byte string.
pub fn decode(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| decode_byte(b)).collect()
}

/// The bytes of a text, or the first character the code page lacks.
pub fn encode(text: &str) -> Result<Vec<u8>, char> {
    text.chars().map(|c| encode_char(c).ok_or(c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_round_trips() {
        let all: Vec<u8> = (0..=255).collect();
        let text = decode(&all);
        assert_eq!(text.chars().count(), 256);
        assert_eq!(encode(&text).unwrap(), all);
    }

    #[test]
    fn named_characters() {
        assert_eq!(
            decode(b"\x80\x93quoted\x94 caf\xe9 \x85"),
            "€“quoted” café …"
        );
        assert_eq!(
            encode("€“quoted” café …").unwrap(),
            b"\x80\x93quoted\x94 caf\xe9 \x85"
        );
    }

    #[test]
    fn undefined_bytes_are_controls() {
        for b in [0x81u8, 0x8D, 0x8F, 0x90, 0x9D] {
            assert_eq!(decode_byte(b) as u32, b as u32);
            assert_eq!(encode_char(decode_byte(b)), Some(b));
        }
    }

    #[test]
    fn outside_the_code_page() {
        assert_eq!(encode("Ж"), Err('Ж'));
        assert_eq!(encode("a\u{0080}"), Err('\u{0080}'));
    }
}
