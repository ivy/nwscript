//! `TLK V3.0`: the header, the entry table and the string data, as BioWare's
//! Talk Table document lays them out and as `dialog.tlk` confirms
//! (`docs/formats/tlk.md`).

use std::fs;
use std::path::Path;

use rakata_res::{Gender, Language, LocalizedString, ResRef, StrRef};

use crate::codepage;
use crate::error::{Error, Result};

const SIGNATURE: &[u8; 8] = b"TLK V3.0";
const HEADER_LEN: usize = 20;
const ENTRY_LEN: usize = 40;

/// One line of the table: its text, its voice-over and the flags that say
/// which of them the engine should look at.
///
/// The flags are kept as read, not derived from the other fields: BioWare
/// marks a line without text by clearing [`Entry::TEXT_PRESENT`], while
/// another writer may leave it set over an empty text, and the writer
/// reproduces either. [`Entry::new`] sets the flags BioWare's own lines
/// carry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The text, decoded from Windows-1252; empty when the flags say there
    /// is none.
    pub text: String,
    /// The voice-over's resref; `None` when the field is blank.
    pub sound: Option<ResRef>,
    /// The voice-over's length in seconds. Zero in every line BioWare
    /// shipped for KotOR 1.
    pub sound_length: f32,
    /// The presence flags, [`Entry::TEXT_PRESENT`] and its siblings, plus
    /// any bit the documents do not name.
    pub flags: u32,
    /// Unused by the engine; zero in every line BioWare shipped.
    pub volume_variance: u32,
    /// Unused by the engine; zero in every line BioWare shipped.
    pub pitch_variance: u32,
}

impl Entry {
    /// The flag that says the text is to be read.
    pub const TEXT_PRESENT: u32 = 0x1;
    /// The flag that says the sound resref is to be read.
    pub const SOUND_PRESENT: u32 = 0x2;
    /// The flag that says the sound length is to be read.
    pub const SOUND_LENGTH_PRESENT: u32 = 0x4;

    /// The flags of a line BioWare shipped with text: all three presence
    /// bits, whether or not the line has a voice-over.
    pub const BIOWARE_FLAGS: u32 =
        Self::TEXT_PRESENT | Self::SOUND_PRESENT | Self::SOUND_LENGTH_PRESENT;

    /// A line of text with no voice-over, flagged as BioWare flags one.
    pub fn new(text: impl Into<String>) -> Self {
        Entry {
            text: text.into(),
            sound: None,
            sound_length: 0.0,
            flags: Self::BIOWARE_FLAGS,
            volume_variance: 0,
            pitch_variance: 0,
        }
    }

    /// The same line with a voice-over.
    pub fn with_sound(mut self, sound: ResRef, length: f32) -> Self {
        self.sound = Some(sound);
        self.sound_length = length;
        self
    }

    /// Whether the flags say the text is to be read.
    pub fn has_text(&self) -> bool {
        self.flags & Self::TEXT_PRESENT != 0
    }

    /// The text the engine shows: the text when the flags say it is
    /// present, else none.
    pub fn shown_text(&self) -> Option<&str> {
        self.has_text().then_some(self.text.as_str())
    }
}

/// A talk table: the language of its strings and the entries, indexed by
/// strref.
#[derive(Clone, Debug, PartialEq)]
pub struct Tlk {
    pub language: Language,
    pub entries: Vec<Entry>,
}

impl Default for Tlk {
    fn default() -> Self {
        Tlk::new(Language::English)
    }
}

impl Tlk {
    /// An empty table for a language.
    pub fn new(language: Language) -> Self {
        Tlk {
            language,
            entries: Vec::new(),
        }
    }

    /// The table in a file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&bytes)
    }

    /// The table in a byte string.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            if bytes.len() >= 8 && &bytes[..8] != SIGNATURE {
                return Err(not_a_talk_table(bytes));
            }
            return Err(Error::Truncated {
                what: "header".to_string(),
                offset: 0,
                len: HEADER_LEN,
                available: bytes.len(),
            });
        }
        if &bytes[..8] != SIGNATURE {
            return Err(not_a_talk_table(bytes));
        }
        let language = Language::from_id(u32_at(bytes, 8));
        let count = u32_at(bytes, 12) as usize;
        let strings_at = u32_at(bytes, 16) as usize;
        let table_len = count * ENTRY_LEN;
        if HEADER_LEN + table_len > bytes.len() {
            return Err(Error::Truncated {
                what: "entry table".to_string(),
                offset: HEADER_LEN,
                len: table_len,
                available: bytes.len(),
            });
        }
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let at = HEADER_LEN + i * ENTRY_LEN;
            let e = &bytes[at..at + ENTRY_LEN];
            let flags = u32_at(e, 0);
            let sound = ResRef::from_bytes(&e[4..20]).filter(|r| !r.is_empty());
            let volume_variance = u32_at(e, 20);
            let pitch_variance = u32_at(e, 24);
            let offset = u32_at(e, 28) as usize;
            let size = u32_at(e, 32) as usize;
            let sound_length = f32::from_le_bytes(e[36..40].try_into().expect("four bytes"));
            let start = strings_at.saturating_add(offset);
            let text = match start.checked_add(size) {
                Some(end) if end <= bytes.len() => codepage::decode(&bytes[start..end]),
                _ => {
                    return Err(Error::Truncated {
                        what: format!("the text of strref {i}"),
                        offset: start,
                        len: size,
                        available: bytes.len(),
                    });
                }
            };
            entries.push(Entry {
                text,
                sound,
                sound_length,
                flags,
                volume_variance,
                pitch_variance,
            });
        }
        Ok(Tlk { language, entries })
    }

    /// The number of entries: the strref the next [`Tlk::append`] returns.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry a strref names, if the strref is set and in range. The
    /// alternate-table bit is ignored: a caller holding both tables decides
    /// which to ask.
    pub fn get(&self, strref: StrRef) -> Option<&Entry> {
        self.entries.get(strref.index()?)
    }

    /// The text the engine shows for a strref: that of the entry it names,
    /// when the entry's flags say it has text.
    pub fn text(&self, strref: StrRef) -> Option<&str> {
        self.get(strref)?.shown_text()
    }

    /// Add an entry at the end and return its strref, the entry count
    /// before the append.
    pub fn append(&mut self, entry: Entry) -> StrRef {
        let strref = StrRef::new(self.entries.len() as u32);
        self.entries.push(entry);
        strref
    }

    /// The text a localised string shows, by BioWare's procedure
    /// ([`LocalizedString::resolve_with`]) with this table answering for
    /// the strref. The caller passes the table for `gender`: `dialog.tlk`
    /// for the masculine or neutral form, `dialogf.tlk` for the feminine
    /// where the language has one.
    pub fn resolve<'a>(
        &'a self,
        string: &'a LocalizedString,
        language: Language,
        gender: Gender,
    ) -> Option<&'a str> {
        string.resolve_with(language, gender, |s| self.text(s))
    }

    /// The table in BioWare's layout: the header, the entries back to back,
    /// then every text in strref order with no separators, an entry whose
    /// flags clear [`Entry::TEXT_PRESENT`] pointing at offset 0 with size 0.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let count = u32::try_from(self.entries.len()).map_err(|_| Error::TooLarge {
            what: "entry count",
            size: self.entries.len(),
        })?;
        let strings_at = HEADER_LEN + self.entries.len() * ENTRY_LEN;
        let mut out = Vec::with_capacity(strings_at);
        out.extend_from_slice(SIGNATURE);
        out.extend_from_slice(&self.language.id().to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(
            &u32::try_from(strings_at)
                .map_err(|_| Error::TooLarge {
                    what: "entry table",
                    size: strings_at,
                })?
                .to_le_bytes(),
        );
        let mut strings = Vec::new();
        for (i, e) in self.entries.iter().enumerate() {
            let (offset, size) = if e.has_text() {
                let bytes = codepage::encode(&e.text).map_err(|ch| Error::NotEncodable {
                    strref: i as u32,
                    ch,
                })?;
                let offset = strings.len();
                strings.extend_from_slice(&bytes);
                (offset, bytes.len())
            } else if e.text.is_empty() {
                (0, 0)
            } else {
                return Err(Error::TextWithoutFlag { strref: i as u32 });
            };
            out.extend_from_slice(&e.flags.to_le_bytes());
            out.extend_from_slice(&e.sound.map_or([0; 16], |r| r.to_field()));
            out.extend_from_slice(&e.volume_variance.to_le_bytes());
            out.extend_from_slice(&e.pitch_variance.to_le_bytes());
            out.extend_from_slice(&field(offset, "string data")?.to_le_bytes());
            out.extend_from_slice(&field(size, "string data")?.to_le_bytes());
            out.extend_from_slice(&e.sound_length.to_le_bytes());
        }
        out.extend_from_slice(&strings);
        Ok(out)
    }

    /// Write the table to a file in BioWare's layout.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let bytes = self.to_bytes()?;
        fs::write(path, bytes).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

fn field(n: usize, what: &'static str) -> Result<u32> {
    u32::try_from(n).map_err(|_| Error::TooLarge { what, size: n })
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

fn not_a_talk_table(bytes: &[u8]) -> Error {
    Error::NotATalkTable {
        found: String::from_utf8_lossy(&bytes[..bytes.len().min(8)]).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `bytes`, kept as a seed for the fuzz targets in the directory
    /// `$RAKATA_FUZZ_SEEDS` names, when it is set.
    fn seed(bytes: &[u8]) -> &[u8] {
        if let Some(dir) = std::env::var_os("RAKATA_FUZZ_SEEDS") {
            use std::hash::{DefaultHasher, Hash, Hasher};
            let mut h = DefaultHasher::new();
            bytes.hash(&mut h);
            let dir = std::path::Path::new(&dir);
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join(format!("{:016x}", h.finish())), bytes).unwrap();
        }
        bytes
    }

    fn sample() -> Tlk {
        let mut t = Tlk::new(Language::English);
        t.append(Entry::new("Bastila"));
        t.append(Entry::new("Café “Taris”").with_sound(ResRef::new("n_bast_01").unwrap(), 1.5));
        t.append(Entry {
            text: String::new(),
            sound: None,
            sound_length: 0.0,
            flags: 0x8000,
            volume_variance: 0,
            pitch_variance: 0,
        });
        t.append(Entry::new(""));
        t
    }

    #[test]
    fn writes_the_documented_layout() {
        let t = sample();
        let b = t.to_bytes().unwrap();
        assert_eq!(&b[..8], b"TLK V3.0");
        assert_eq!(u32_at(&b, 8), 0);
        assert_eq!(u32_at(&b, 12), 4);
        assert_eq!(u32_at(&b, 16), 20 + 4 * 40);
        // Entry 0: flags 7, blank resref, offset 0, size 7.
        assert_eq!(u32_at(&b, 20), 7);
        assert_eq!(&b[24..40], &[0; 16]);
        assert_eq!(u32_at(&b, 48), 0);
        assert_eq!(u32_at(&b, 52), 7);
        // Entry 1: resref, the text in Windows-1252, the length.
        assert_eq!(&b[64..73], b"n_bast_01");
        assert_eq!(u32_at(&b, 88), 7);
        assert_eq!(u32_at(&b, 92), 12);
        assert_eq!(f32::from_le_bytes(b[96..100].try_into().unwrap()), 1.5);
        // Entry 2: no text, so offset 0 and size 0, flags as given.
        assert_eq!(u32_at(&b, 100), 0x8000);
        assert_eq!(u32_at(&b, 128), 0);
        assert_eq!(u32_at(&b, 132), 0);
        // Entry 3: text present but empty sits at the running offset.
        assert_eq!(u32_at(&b, 168), 19);
        assert_eq!(u32_at(&b, 172), 0);
        assert_eq!(&b[180..], b"BastilaCaf\xe9 \x93Taris\x94");
    }

    #[test]
    fn round_trips() {
        let t = sample();
        let b = t.to_bytes().unwrap();
        let again = Tlk::parse(seed(&b)).unwrap();
        assert_eq!(again, t);
        assert_eq!(again.to_bytes().unwrap(), b);
    }

    #[test]
    fn append_returns_the_previous_count_and_keeps_earlier_entries() {
        let mut t = sample();
        let before = t.to_bytes().unwrap();
        let strref = t.append(Entry::new("Juhani"));
        assert_eq!(strref, StrRef::new(4));
        let b = t.to_bytes().unwrap();
        let again = Tlk::parse(seed(&b)).unwrap();
        assert_eq!(again.len(), 5);
        assert_eq!(again.text(strref), Some("Juhani"));
        assert_eq!(again.entries[..4], Tlk::parse(&before).unwrap().entries[..]);
    }

    #[test]
    fn get_and_text() {
        let t = sample();
        assert_eq!(t.text(StrRef::new(0)), Some("Bastila"));
        assert_eq!(
            t.get(StrRef::new(1)).unwrap().sound.unwrap().as_str(),
            Some("n_bast_01")
        );
        assert_eq!(t.text(StrRef::new(2)), None);
        assert_eq!(t.text(StrRef::new(3)), Some(""));
        assert_eq!(t.get(StrRef::new(4)), None);
        assert_eq!(t.get(StrRef::NONE), None);
        assert_eq!(
            t.text(StrRef::new(StrRef::ALTERNATE | 1)),
            Some("Café “Taris”")
        );
    }

    #[test]
    fn resolves_localized_strings() {
        let t = sample();
        let by_ref = LocalizedString::from_strref(StrRef::new(0));
        assert_eq!(
            t.resolve(&by_ref, Language::English, Gender::Male),
            Some("Bastila")
        );
        let embedded = LocalizedString::embedded(Language::English, Gender::Male, "Hello");
        assert_eq!(
            t.resolve(&embedded, Language::English, Gender::Male),
            Some("Hello")
        );
        let no_text = LocalizedString::from_strref(StrRef::new(2));
        assert_eq!(t.resolve(&no_text, Language::English, Gender::Male), None);
        assert_eq!(
            t.resolve(&LocalizedString::EMPTY, Language::English, Gender::Male),
            None
        );
    }

    #[test]
    fn refuses_what_it_cannot_write() {
        let mut t = Tlk::default();
        t.append(Entry::new("Ж"));
        let e = t.to_bytes().unwrap_err();
        assert_eq!(e.code(), "RE0803");
        assert!(e.to_string().contains("U+0416"), "{e}");
        let mut t = Tlk::default();
        let mut e = Entry::new("text");
        e.flags = Entry::SOUND_PRESENT;
        t.append(e);
        assert_eq!(t.to_bytes().unwrap_err().code(), "RE0804");
    }

    #[test]
    fn refuses_what_it_cannot_read() {
        assert_eq!(
            Tlk::parse(b"RIM V1.0\0\0\0\0\0\0\0\0\0\0\0\0")
                .unwrap_err()
                .code(),
            "RE0801"
        );
        assert_eq!(Tlk::parse(b"TLK V3.0\0\0").unwrap_err().code(), "RE0802");
        let mut b = sample().to_bytes().unwrap();
        b[12] = 9;
        assert_eq!(Tlk::parse(&b).unwrap_err().code(), "RE0802");
        let mut b = sample().to_bytes().unwrap();
        b[52] = 200;
        let e = Tlk::parse(&b).unwrap_err();
        assert_eq!(e.code(), "RE0802");
        assert!(e.to_string().contains("strref 0"), "{e}");
    }
}
