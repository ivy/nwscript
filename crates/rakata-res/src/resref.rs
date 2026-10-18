//! `ResRef`: a resource name of at most sixteen bytes.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::Error;

/// The most bytes a resref holds: the width of the name field in KEY, ERF
/// and RIM tables and in GFF `CResRef` fields.
pub const MAX_LEN: usize = 16;

/// A resource name as the engine stores it: at most sixteen bytes, compared
/// without regard to ASCII case.
///
/// The spelling it was read with is kept, so writing it back reproduces the
/// source byte for byte, and [`fmt::Display`] shows it as read. Equality,
/// hashing and ordering fold ASCII case, so `Carth` and `carth` are one name
/// in any map or set.
#[derive(Clone, Copy)]
pub struct ResRef {
    bytes: [u8; MAX_LEN],
    len: u8,
}

impl ResRef {
    /// A resref from text. Fails with [`Error::ResRefTooLong`] when the text
    /// is more than sixteen bytes.
    pub fn new(name: &str) -> Result<Self, Error> {
        Self::from_bytes(name.as_bytes()).ok_or_else(|| Error::ResRefTooLong {
            name: name.to_owned(),
            len: name.len(),
        })
    }

    /// A resref from raw bytes, as an archive table or a GFF field holds
    /// them: the name ends at the first NUL or at the end of the slice.
    /// `None` when the name that remains is more than sixteen bytes.
    pub fn from_bytes(raw: &[u8]) -> Option<Self> {
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let name = &raw[..end];
        if name.len() > MAX_LEN {
            return None;
        }
        let mut bytes = [0u8; MAX_LEN];
        bytes[..name.len()].copy_from_slice(name);
        Some(Self {
            bytes,
            len: name.len() as u8,
        })
    }

    /// The name's bytes, as read, without padding.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    /// The name as text, when it is valid UTF-8. Every name BioWare shipped
    /// is ASCII; a mod's may not be.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.as_bytes()).ok()
    }

    /// The sixteen-byte field as an archive stores it: the name followed by
    /// NUL padding.
    pub fn to_field(&self) -> [u8; MAX_LEN] {
        self.bytes
    }

    /// The length of the name in bytes.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Whether the name is empty, as an unset GFF `CResRef` field is.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn folded(&self) -> impl Iterator<Item = u8> + '_ {
        self.as_bytes().iter().map(u8::to_ascii_lowercase)
    }
}

impl PartialEq for ResRef {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes().eq_ignore_ascii_case(other.as_bytes())
    }
}

impl Eq for ResRef {}

impl Hash for ResRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        for b in self.folded() {
            state.write_u8(b);
        }
    }
}

impl PartialOrd for ResRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.folded().cmp(other.folded())
    }
}

impl fmt::Display for ResRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.as_bytes()))
    }
}

impl fmt::Debug for ResRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResRef({})", String::from_utf8_lossy(self.as_bytes()))
    }
}

impl std::str::FromStr for ResRef {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::codes;

    #[test]
    fn case_folds_for_equality_hash_and_order() {
        let a = ResRef::new("Carth").unwrap();
        let b = ResRef::new("carth").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.cmp(&b), Ordering::Equal);
        let set: HashSet<ResRef> = [a, b].into_iter().collect();
        assert_eq!(set.len(), 1);
        assert!(ResRef::new("Bastila").unwrap() < ResRef::new("carth").unwrap());
    }

    #[test]
    fn keeps_the_spelling_it_was_read_with() {
        let a = ResRef::new("Carth").unwrap();
        assert_eq!(a.to_string(), "Carth");
        assert_eq!(a.as_str(), Some("Carth"));
        assert_eq!(a.as_bytes(), b"Carth");
        assert_eq!(&a.to_field(), b"Carth\0\0\0\0\0\0\0\0\0\0\0");
    }

    #[test]
    fn sixteen_bytes_fit_and_seventeen_do_not() {
        let ok = ResRef::new("k_pebn_pathfind1").unwrap();
        assert_eq!(ok.len(), 16);
        let err = ResRef::new("k_pebn_pathfind01").unwrap_err();
        assert_eq!(err.code(), codes::RESREF_TOO_LONG);
        assert_eq!(
            err,
            Error::ResRefTooLong {
                name: "k_pebn_pathfind01".into(),
                len: 17
            }
        );
        assert!(err.to_string().starts_with("RE0101: "));
    }

    #[test]
    fn from_bytes_stops_at_the_first_nul() {
        let r = ResRef::from_bytes(b"n_darthmalak\0\0\0\0").unwrap();
        assert_eq!(r.as_bytes(), b"n_darthmalak");
        let full = ResRef::from_bytes(b"abcdefghijklmnop").unwrap();
        assert_eq!(full.len(), 16);
        assert!(ResRef::from_bytes(b"abcdefghijklmnopq").is_none());
        assert!(ResRef::from_bytes(b"").unwrap().is_empty());
    }

    #[test]
    fn parses_from_str() {
        let r: ResRef = "Carth".parse().unwrap();
        assert_eq!(r, ResRef::new("CARTH").unwrap());
    }
}
