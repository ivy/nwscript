//! `ContentHash`: SHA-256 over a resource's bytes.

use std::fmt;
use std::str::FromStr;

use sha2::{Digest, Sha256};

/// The SHA-256 of a resource's bytes: the identity an index, a cache and a
/// build key share. Shown as sixty-four lowercase hex digits, the form
/// `sha256sum` prints.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// The hash of `bytes`.
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// A hash from its thirty-two raw bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The thirty-two raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The hash as lowercase hex.
    pub fn to_hex(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({self})")
    }
}

/// The text was not sixty-four hex digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseHashError {
    text: String,
}

impl fmt::Display for ParseHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not a SHA-256: sixty-four hex digits", self.text)
    }
}

impl std::error::Error for ParseHashError {}

impl FromStr for ContentHash {
    type Err = ParseHashError;

    /// Sixty-four hex digits, in either case.
    fn from_str(s: &str) -> Result<Self, ParseHashError> {
        let err = || ParseHashError { text: s.to_owned() };
        if s.len() != 64 || !s.is_ascii() {
            return Err(err());
        }
        let mut out = [0u8; 32];
        for (i, pair) in s.as_bytes().chunks(2).enumerate() {
            let digits = std::str::from_utf8(pair).map_err(|_| err())?;
            out[i] = u8::from_str_radix(digits, 16).map_err(|_| err())?;
        }
        Ok(Self(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `sha256sum tests/fixtures/greeting.txt`
    const GREETING_SHA256: &str =
        "e81277aba0198d009c538e6c2b5a7bcf460152d54f5f7bde11e35837c47c5e4b";

    #[test]
    fn matches_sha256sum_on_a_fixture() {
        let bytes = include_bytes!("../tests/fixtures/greeting.txt");
        assert_eq!(ContentHash::of(bytes).to_string(), GREETING_SHA256);
    }

    #[test]
    fn the_empty_input_has_the_well_known_digest() {
        assert_eq!(
            ContentHash::of(b"").to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hex_round_trips_in_either_case() {
        let h = ContentHash::of(b"carth");
        assert_eq!(h.to_string().parse::<ContentHash>().unwrap(), h);
        assert_eq!(
            h.to_string().to_uppercase().parse::<ContentHash>().unwrap(),
            h
        );
        assert_eq!(ContentHash::from_bytes(*h.as_bytes()), h);
        assert!("abc".parse::<ContentHash>().is_err());
        assert!("zz".repeat(32).parse::<ContentHash>().is_err());
    }
}
