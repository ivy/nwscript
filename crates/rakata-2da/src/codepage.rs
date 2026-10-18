//! The game's single-byte code page, Windows-1252, between the file's bytes
//! and the model's strings. Every table measured so far is plain ASCII
//! (`docs/formats/2da.md`, Encoding); the upper half is carried so that a
//! byte above 0x7F, should one occur, reads and writes back unchanged.

/// The characters at 0x80..=0x9F; the rest of the page is Latin-1. Five
/// positions the page leaves undefined map to the control characters of the
/// same value, so every byte decodes and encodes back to itself.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// The bytes as text.
pub fn decode(bytes: &[u8]) -> String {
    if bytes.is_ascii() {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => HIGH[usize::from(b - 0x80)],
            _ => char::from(b),
        })
        .collect()
}

/// The text as bytes, or the first character the page has no byte for.
pub fn encode(text: &str) -> Result<Vec<u8>, char> {
    if text.is_ascii() {
        return Ok(text.as_bytes().to_vec());
    }
    text.chars()
        .map(|c| match u32::from(c) {
            v @ 0x00..=0x7F => Ok(v as u8),
            v @ 0xA0..=0xFF => Ok(v as u8),
            _ => HIGH
                .iter()
                .position(|&h| h == c)
                .map(|i| 0x80 + i as u8)
                .ok_or(c),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_round_trips() {
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(encode(&decode(&all)).unwrap(), all);
    }

    #[test]
    fn a_character_outside_the_page_is_named() {
        assert_eq!(encode("a\u{4E2D}"), Err('\u{4E2D}'));
    }
}
