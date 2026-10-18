//! The lip-sync files of BioWare's Aurora-family games, KotOR 1 and 2 first:
//! a [`Lip`] is a voice line's duration and its keyframes, each a time in
//! seconds and one of the sixteen mouth [`Shape`]s the engine knows.
//!
//! A file read and written again is byte for byte the file it came from:
//! the floats travel as the bits they were read with, and the only place a
//! float is computed is [`Lip::from_cues`], which makes a file from the cues
//! of a lip-sync tool. The layout is `docs/formats/lip.md` of this
//! repository; this crate was written without sight of any other
//! implementation.

mod error;

use std::path::Path;

pub use error::{Error, Result, codes};

/// The eight bytes every file begins with.
pub const SIGNATURE: &[u8; 8] = b"LIP V1.0";
/// The header's length: signature, version, duration, count.
pub const HEADER_LEN: usize = 16;
/// A keyframe's length: a float32 time and a shape byte.
pub const KEY_LEN: usize = 5;

/// One of the sixteen mouth poses, numbered as the file stores them. The
/// names are this crate's, from the mouth position; the numbers are the
/// format's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Shape {
    /// Closed, relaxed: silence.
    Rest = 0,
    /// Teeth slightly apart, corners wide: "see", "bit".
    Ee = 1,
    /// Relaxed, slightly open: "get", "bird".
    Eh = 2,
    /// Open: "father", "cat", "but".
    Ah = 3,
    /// Rounded: "bought", "boat".
    Oh = 4,
    /// Pursed: "too", "book", "we".
    Oo = 5,
    /// Slight smile: "yes".
    Y = 6,
    /// Teeth together: "stop", "zee".
    S = 7,
    /// Lower lip on upper teeth: "five", "very".
    F = 8,
    /// Back of tongue up: "knee", "ring".
    N = 9,
    /// Tongue between teeth: "thin", "that".
    Th = 10,
    /// Lips pressed: "bump", "pop".
    M = 11,
    /// Tongue up behind teeth: "top", "door".
    T = 12,
    /// Rounded, relaxed: "she", "cheese".
    Sh = 13,
    /// Tongue forward: "lip", "read".
    L = 14,
    /// Back of tongue raised: "kick", "green", "he".
    K = 15,
}

impl Shape {
    /// Every shape, in file order.
    pub const ALL: [Shape; 16] = [
        Shape::Rest,
        Shape::Ee,
        Shape::Eh,
        Shape::Ah,
        Shape::Oh,
        Shape::Oo,
        Shape::Y,
        Shape::S,
        Shape::F,
        Shape::N,
        Shape::Th,
        Shape::M,
        Shape::T,
        Shape::Sh,
        Shape::L,
        Shape::K,
    ];

    /// The shape a file byte names, or `None` above 15.
    pub fn from_byte(b: u8) -> Option<Shape> {
        Self::ALL.get(usize::from(b)).copied()
    }

    /// The byte the file stores.
    pub fn byte(self) -> u8 {
        self as u8
    }

    /// The short lowercase label: `rest`, `ee`, `eh`, `ah`, `oh`, `oo`, `y`,
    /// `s`, `f`, `n`, `th`, `m`, `t`, `sh`, `l`, `k`.
    pub fn label(self) -> &'static str {
        match self {
            Shape::Rest => "rest",
            Shape::Ee => "ee",
            Shape::Eh => "eh",
            Shape::Ah => "ah",
            Shape::Oh => "oh",
            Shape::Oo => "oo",
            Shape::Y => "y",
            Shape::S => "s",
            Shape::F => "f",
            Shape::N => "n",
            Shape::Th => "th",
            Shape::M => "m",
            Shape::T => "t",
            Shape::Sh => "sh",
            Shape::L => "l",
            Shape::K => "k",
        }
    }

    /// The shape with this label, in any case.
    pub fn from_label(label: &str) -> Option<Shape> {
        Self::ALL
            .into_iter()
            .find(|s| s.label().eq_ignore_ascii_case(label))
    }
}

impl std::fmt::Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// One keyframe: at `time` seconds from the start of the line the mouth is
/// in `shape`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub time: f32,
    pub shape: Shape,
}

impl Key {
    pub fn new(time: f32, shape: Shape) -> Self {
        Key { time, shape }
    }

    /// Whether the two hold the same bits, which `==` does not ask of the
    /// time (`-0.0 == 0.0`, and a NaN equals nothing).
    pub fn bits_eq(&self, other: &Key) -> bool {
        self.time.to_bits() == other.time.to_bits() && self.shape == other.shape
    }
}

/// A lip-sync track: the line's duration in seconds and its keyframes, in
/// file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lip {
    /// The duration of the line, in seconds.
    pub length: f32,
    /// The keyframes, in the order the file holds them.
    pub keys: Vec<Key>,
}

impl Lip {
    /// Parse a file's bytes. Refuses a file that does not begin `LIP V1.0`,
    /// one shorter than its header and count require, a shape byte above
    /// 15, and bytes after the last keyframe, each with its code.
    pub fn parse(bytes: &[u8]) -> Result<Lip> {
        let head = &bytes[..bytes.len().min(SIGNATURE.len())];
        if head != SIGNATURE {
            return Err(Error::NotALip {
                found: String::from_utf8_lossy(head).into_owned(),
            });
        }
        if bytes.len() < HEADER_LEN {
            return Err(Error::Truncated {
                what: "header",
                needed: HEADER_LEN,
                available: bytes.len(),
            });
        }
        let length = f32::from_le_bytes(bytes[8..12].try_into().expect("four bytes"));
        let count = u32::from_le_bytes(bytes[12..16].try_into().expect("four bytes"));
        let count = usize::try_from(count).expect("a u32 fits a usize");
        let needed = HEADER_LEN + count * KEY_LEN;
        if bytes.len() < needed {
            return Err(Error::Truncated {
                what: "keyframes",
                needed,
                available: bytes.len(),
            });
        }
        if bytes.len() > needed {
            return Err(Error::TrailingBytes {
                len: bytes.len() - needed,
            });
        }
        let keys = bytes[HEADER_LEN..]
            .as_chunks::<KEY_LEN>()
            .0
            .iter()
            .enumerate()
            .map(|(index, k)| {
                let time = f32::from_le_bytes(k[..4].try_into().expect("four bytes"));
                let shape = Shape::from_byte(k[4]).ok_or(Error::BadShape { index, value: k[4] })?;
                Ok(Key { time, shape })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Lip { length, keys })
    }

    /// Read a file.
    pub fn read(path: impl AsRef<Path>) -> Result<Lip> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&bytes)
    }

    /// The file's bytes: the header, then each keyframe as read, floats bit
    /// for bit. Fails only when the keyframes outnumber a 32-bit count.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let count = u32::try_from(self.keys.len()).map_err(|_| Error::TooManyKeys {
            len: self.keys.len(),
        })?;
        let mut out = Vec::with_capacity(HEADER_LEN + self.keys.len() * KEY_LEN);
        out.extend_from_slice(SIGNATURE);
        out.extend_from_slice(&self.length.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        for k in &self.keys {
            out.extend_from_slice(&k.time.to_le_bytes());
            out.push(k.shape.byte());
        }
        Ok(out)
    }

    /// Write a file.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        std::fs::write(path, self.to_bytes()?).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    /// A track from a lip-sync tool's cues, each the time a shape begins,
    /// over a line `length` seconds long. The keyframes are the cues in
    /// non-decreasing time order (a stable sort, so cues at one time keep
    /// their order), followed by a rest at `length` when the last cue is
    /// before it, so the mouth closes as the line ends. This is the one
    /// place the crate computes a float rather than copying one.
    pub fn from_cues(cues: impl IntoIterator<Item = (f32, Shape)>, length: f32) -> Lip {
        let mut keys: Vec<Key> = cues
            .into_iter()
            .map(|(time, shape)| Key { time, shape })
            .collect();
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        if keys.last().is_none_or(|k| k.time < length) {
            keys.push(Key {
                time: length,
                shape: Shape::Rest,
            });
        }
        Lip { length, keys }
    }

    /// Whether the two hold the same bits: the same duration and the same
    /// keyframes bit for bit, which `==` does not ask of the floats.
    pub fn bits_eq(&self, other: &Lip) -> bool {
        self.length.to_bits() == other.length.to_bits()
            && self.keys.len() == other.keys.len()
            && self.keys.iter().zip(&other.keys).all(|(a, b)| a.bits_eq(b))
    }

    /// Whether the keyframes are in non-decreasing time order.
    pub fn is_sorted(&self) -> bool {
        self.keys.windows(2).all(|w| w[0].time <= w[1].time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three() -> Lip {
        Lip {
            length: 1.5,
            keys: vec![
                Key::new(0.0, Shape::Rest),
                Key::new(0.25, Shape::Ah),
                Key::new(1.5, Shape::Rest),
            ],
        }
    }

    #[test]
    fn three_keys_round_trip() {
        let lip = three();
        let bytes = lip.to_bytes().unwrap();
        assert_eq!(bytes.len(), HEADER_LEN + 3 * KEY_LEN);
        assert_eq!(&bytes[..8], SIGNATURE);
        assert_eq!(&bytes[12..16], &3u32.to_le_bytes());
        let again = Lip::parse(&bytes).unwrap();
        assert!(again.bits_eq(&lip));
        assert_eq!(again.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn a_shape_above_fifteen_is_a_diagnostic() {
        let mut bytes = three().to_bytes().unwrap();
        bytes[HEADER_LEN + KEY_LEN + 4] = 16;
        let err = Lip::parse(&bytes).unwrap_err();
        assert_eq!(err.code(), codes::BAD_SHAPE);
        assert!(matches!(
            err,
            Error::BadShape {
                index: 1,
                value: 16
            }
        ));
        bytes[HEADER_LEN + KEY_LEN + 4] = 255;
        assert_eq!(Lip::parse(&bytes).unwrap_err().code(), codes::BAD_SHAPE);
    }

    #[test]
    fn a_wrong_signature_or_version_is_refused() {
        let mut bytes = three().to_bytes().unwrap();
        bytes[0..8].copy_from_slice(b"LIP V1.1");
        assert_eq!(Lip::parse(&bytes).unwrap_err().code(), codes::NOT_A_LIP);
        assert_eq!(Lip::parse(b"RIM ").unwrap_err().code(), codes::NOT_A_LIP);
        assert_eq!(Lip::parse(b"").unwrap_err().code(), codes::NOT_A_LIP);
    }

    #[test]
    fn a_short_file_is_truncated_not_a_panic() {
        let bytes = three().to_bytes().unwrap();
        assert_eq!(
            Lip::parse(&bytes[..12]).unwrap_err().code(),
            codes::TRUNCATED
        );
        assert_eq!(
            Lip::parse(&bytes[..HEADER_LEN + 7]).unwrap_err().code(),
            codes::TRUNCATED
        );
        let mut huge = bytes[..HEADER_LEN].to_vec();
        huge[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Lip::parse(&huge).unwrap_err().code(), codes::TRUNCATED);
    }

    #[test]
    fn trailing_bytes_are_refused() {
        let mut bytes = three().to_bytes().unwrap();
        bytes.push(0);
        let err = Lip::parse(&bytes).unwrap_err();
        assert_eq!(err.code(), codes::TRAILING_BYTES);
        assert!(matches!(err, Error::TrailingBytes { len: 1 }));
    }

    #[test]
    fn zero_keys_is_sixteen_bytes() {
        let lip = Lip {
            length: 0.0,
            keys: Vec::new(),
        };
        let bytes = lip.to_bytes().unwrap();
        assert_eq!(bytes.len(), HEADER_LEN);
        assert!(Lip::parse(&bytes).unwrap().bits_eq(&lip));
    }

    #[test]
    fn floats_keep_their_bits() {
        let lip = Lip {
            length: f32::from_bits(0x8000_0000),
            keys: vec![Key::new(f32::from_bits(0x7fc0_0001), Shape::K)],
        };
        let bytes = lip.to_bytes().unwrap();
        let again = Lip::parse(&bytes).unwrap();
        assert!(again.bits_eq(&lip));
        assert_eq!(again.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn from_cues_sorts_and_closes_the_mouth() {
        let lip = Lip::from_cues([(0.5, Shape::M), (0.0, Shape::Rest), (0.5, Shape::Ah)], 2.0);
        assert_eq!(lip.length, 2.0);
        assert!(lip.is_sorted());
        assert_eq!(
            lip.keys,
            vec![
                Key::new(0.0, Shape::Rest),
                Key::new(0.5, Shape::M),
                Key::new(0.5, Shape::Ah),
                Key::new(2.0, Shape::Rest),
            ]
        );
        let ends_at_length = Lip::from_cues([(1.0, Shape::Rest)], 1.0);
        assert_eq!(ends_at_length.keys.len(), 1);
        let empty = Lip::from_cues(std::iter::empty(), 0.5);
        assert_eq!(empty.keys, vec![Key::new(0.5, Shape::Rest)]);
    }

    #[test]
    fn shapes_number_sixteen_and_name_themselves() {
        for (i, s) in Shape::ALL.iter().enumerate() {
            assert_eq!(usize::from(s.byte()), i);
            assert_eq!(Shape::from_byte(s.byte()), Some(*s));
            assert_eq!(Shape::from_label(s.label()), Some(*s));
        }
        assert_eq!(Shape::from_byte(16), None);
        assert_eq!(Shape::from_label("TH"), Some(Shape::Th));
        assert_eq!(Shape::from_label("x"), None);
    }
}
