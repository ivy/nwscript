//! KotOR's sound set file, `SSF V1.1`, as `docs/formats/ssf.md` records
//! it: an eight-byte signature, the offset of the entry table, and from
//! there to the end of the file one 32-bit strref per sound, `0xFFFFFFFF`
//! for none. The sounds' resrefs, which the NWN layout carries beside each
//! strref, are not in the file: the engine takes the voice-over from the
//! talk table entry. Small enough to live with the installer, its one user.

/// The eight bytes every KotOR soundset begins with.
pub const SIGNATURE: &[u8; 8] = b"SSF V1.1";

/// A soundset: one strref per sound slot, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ssf {
    /// The bytes before the entry table, signature and offset included, as
    /// read, so a file read and written again is the file it came from.
    head: Vec<u8>,
    /// The strref of each slot; `0xFFFFFFFF` for none.
    pub strrefs: Vec<u32>,
}

/// Why bytes are not a KotOR soundset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsfError {
    /// The signature is not `SSF V1.1`.
    Signature(Vec<u8>),
    /// The file ends before its header, or its table offset lies past the
    /// end, or the table is not a whole number of entries.
    Truncated { len: usize },
}

impl std::fmt::Display for SsfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SsfError::Signature(s) => write!(
                f,
                "not a KotOR soundset: the signature is `{}`, not `SSF V1.1`",
                String::from_utf8_lossy(s)
            ),
            SsfError::Truncated { len } => write!(
                f,
                "a soundset of {len} bytes whose header or entry table does not fit"
            ),
        }
    }
}

impl std::error::Error for SsfError {}

/// The strref of an empty slot.
pub const NONE: u32 = u32::MAX;

impl Ssf {
    /// The number of slots in every soundset the game ships.
    pub const SLOTS: usize = 40;

    /// An empty soundset of [`Ssf::SLOTS`] slots, laid out as the game's are.
    pub fn new() -> Self {
        let mut head = SIGNATURE.to_vec();
        head.extend_from_slice(&12u32.to_le_bytes());
        Ssf {
            head,
            strrefs: vec![NONE; Self::SLOTS],
        }
    }

    /// Parse a soundset.
    pub fn parse(bytes: &[u8]) -> Result<Self, SsfError> {
        let truncated = SsfError::Truncated { len: bytes.len() };
        if bytes.len() < 12 {
            return Err(if bytes.len() >= 8 && &bytes[..8] != SIGNATURE {
                SsfError::Signature(bytes[..8].to_vec())
            } else {
                truncated
            });
        }
        if &bytes[..8] != SIGNATURE {
            return Err(SsfError::Signature(bytes[..8].to_vec()));
        }
        let offset = u32::from_le_bytes(bytes[8..12].try_into().expect("four bytes")) as usize;
        if offset < 12 || offset > bytes.len() || !(bytes.len() - offset).is_multiple_of(4) {
            return Err(truncated);
        }
        let strrefs = bytes[offset..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect();
        Ok(Ssf {
            head: bytes[..offset].to_vec(),
            strrefs,
        })
    }

    /// The file's bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.head.clone();
        for s in &self.strrefs {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    /// Set a slot's strref, growing the table with empty slots if the file
    /// is shorter than the slot.
    pub fn set(&mut self, slot: usize, strref: u32) {
        if self.strrefs.len() <= slot {
            self.strrefs.resize(slot + 1, NONE);
        }
        self.strrefs[slot] = strref;
    }
}

/// The slot a `[SSFList]` sound name writes, as ChangeEdit spells the
/// names, compared without regard to case (`docs/formats/ssf.md`).
pub fn slot(name: &str) -> Option<usize> {
    SLOT_NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name.trim()))
}

/// ChangeEdit's names for the slots, in slot order.
pub const SLOT_NAMES: [&str; 28] = [
    "Battlecry 1",
    "Battlecry 2",
    "Battlecry 3",
    "Battlecry 4",
    "Battlecry 5",
    "Battlecry 6",
    "Selected 1",
    "Selected 2",
    "Selected 3",
    "Attack 1",
    "Attack 2",
    "Attack 3",
    "Pain 1",
    "Pain 2",
    "Low health",
    "Death",
    "Critical hit",
    "Target immune",
    "Place mine",
    "Disarm mine",
    "Stealth on",
    "Search",
    "Pick lock start",
    "Pick lock fail",
    "Pick lock done",
    "Leave party",
    "Rejoin party",
    "Poisoned",
];

impl Default for Ssf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_soundset_is_172_bytes_and_reads_back() {
        let mut s = Ssf::new();
        s.set(0, 7);
        let bytes = s.to_bytes();
        assert_eq!(bytes.len(), 172);
        assert_eq!(&bytes[..12], b"SSF V1.1\x0c\0\0\0");
        let back = Ssf::parse(&bytes).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.to_bytes(), bytes);
    }

    #[test]
    fn other_signatures_and_short_files_are_refused() {
        assert!(matches!(
            Ssf::parse(b"SSF V1.0\x0c\0\0\0"),
            Err(SsfError::Signature(_))
        ));
        assert!(matches!(
            Ssf::parse(b"SSF V1.1"),
            Err(SsfError::Truncated { .. })
        ));
        assert!(matches!(
            Ssf::parse(b"SSF V1.1\x0c\0\0\0\x01"),
            Err(SsfError::Truncated { .. })
        ));
    }
}
