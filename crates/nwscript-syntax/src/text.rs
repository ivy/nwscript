//! Bytes and tree text.
//!
//! The tree stores text as `str`, and the source is Windows-1252 bytes. Each
//! byte is stored as the char with the same value, U+0000 to U+00FF (Latin-1),
//! which maps every byte sequence to a string and back without loss. A byte
//! above 0x7F is two UTF-8 bytes in the tree, so a tree's text offsets differ
//! from byte offsets after one; [`OffsetMap`] converts between them.

use rowan::{TextRange, TextSize};
use std::borrow::Cow;

/// The tree text for source bytes: each byte as the char of the same value.
pub fn latin1_str(bytes: &[u8]) -> Cow<'_, str> {
    if bytes.is_ascii() {
        // ASCII bytes are valid UTF-8 with the same meaning.
        Cow::Borrowed(std::str::from_utf8(bytes).unwrap_or_default())
    } else {
        Cow::Owned(bytes.iter().map(|&b| b as char).collect())
    }
}

/// The source bytes for tree text. Every char in a tree is at most U+00FF;
/// a char above that (which no tree holds) becomes `?`.
pub fn latin1_bytes(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect()
}

/// Converts between byte offsets in the source and text offsets in its tree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OffsetMap {
    /// Byte offsets of the bytes above 0x7F, ascending.
    high: Vec<u32>,
}

impl OffsetMap {
    pub fn new(src: &[u8]) -> Self {
        OffsetMap {
            high: src
                .iter()
                .enumerate()
                .filter(|(_, b)| **b >= 0x80)
                .map(|(i, _)| i as u32)
                .collect(),
        }
    }

    /// The text offset of a byte offset.
    pub fn to_text(&self, byte: usize) -> TextSize {
        let before = self.high.partition_point(|&h| (h as usize) < byte);
        TextSize::from((byte + before) as u32)
    }

    /// The byte offset of a text offset. An offset inside a two-byte char
    /// (which no node or token boundary is) maps to that char's byte.
    pub fn to_byte(&self, text: TextSize) -> usize {
        let t = u32::from(text) as usize;
        t - partition(&self.high, t)
    }

    /// The byte range of a text range.
    pub fn to_byte_range(&self, range: TextRange) -> std::ops::Range<usize> {
        self.to_byte(range.start())..self.to_byte(range.end())
    }
}

/// The number of high bytes whose text offset is below `t`. Text offsets of
/// high bytes (`h + i`) are strictly increasing, so this is a binary search.
fn partition(high: &[u32], t: usize) -> usize {
    let (mut lo, mut hi) = (0, high.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        if high[mid] as usize + mid < t {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin1_round_trip() {
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(latin1_bytes(&latin1_str(&all)), all);
    }

    #[test]
    fn offsets_round_trip() {
        let src = b"a\x93b\x94\x95c";
        let m = OffsetMap::new(src);
        let text = latin1_str(src);
        for (byte, (t, _)) in text.char_indices().enumerate() {
            assert_eq!(m.to_text(byte), TextSize::from(t as u32));
            assert_eq!(m.to_byte(TextSize::from(t as u32)), byte);
        }
        assert_eq!(m.to_text(src.len()), TextSize::from(text.len() as u32));
        assert_eq!(m.to_byte(TextSize::from(text.len() as u32)), src.len());
    }
}
