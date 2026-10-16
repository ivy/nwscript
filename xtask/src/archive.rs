//! Readers for the archive formats the corpus comes from: KEY/BIF (the
//! game's base data) and RIM/ERF/MOD (modules). Layouts follow the KotOR
//! modding wiki's format pages. Only what extraction needs is read.

use std::fmt;

/// A resource type id as stored in the archives.
pub const NSS: u16 = 2009;
pub const NCS: u16 = 2010;

#[derive(Debug)]
pub struct FormatError(pub String);

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FormatError {}

type Result<T> = std::result::Result<T, FormatError>;

fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(FormatError(msg.into()))
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| FormatError(format!("truncated at offset {o}")))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| FormatError(format!("truncated at offset {o}")))
}

fn slice(b: &[u8], o: usize, n: usize) -> Result<&[u8]> {
    b.get(o..o + n)
        .ok_or_else(|| FormatError(format!("range {o}+{n} beyond end {}", b.len())))
}

/// A NUL-padded resref, lowercased: the engine treats names case-insensitively.
fn resref(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).to_ascii_lowercase()
}

/// One entry of a KEY file: a resource and where it lives.
pub struct KeyEntry {
    pub name: String,
    pub restype: u16,
    pub bif: usize,
    pub index: usize,
}

pub struct Key {
    /// BIF paths as stored, with `\` separators.
    pub bifs: Vec<String>,
    pub entries: Vec<KeyEntry>,
}

pub fn read_key(b: &[u8]) -> Result<Key> {
    if slice(b, 0, 8)? != b"KEY V1  " {
        return err("not a KEY V1 file");
    }
    let bif_count = u32_at(b, 8)? as usize;
    let key_count = u32_at(b, 12)? as usize;
    let file_table = u32_at(b, 16)? as usize;
    let key_table = u32_at(b, 20)? as usize;
    let mut bifs = Vec::with_capacity(bif_count);
    for i in 0..bif_count {
        let e = file_table + i * 12;
        let name_off = u32_at(b, e + 4)? as usize;
        let name_len = u16_at(b, e + 8)? as usize;
        let raw = slice(b, name_off, name_len)?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        bifs.push(String::from_utf8_lossy(&raw[..end]).into_owned());
    }
    let mut entries = Vec::with_capacity(key_count);
    for i in 0..key_count {
        let e = key_table + i * 22;
        let id = u32_at(b, e + 18)?;
        entries.push(KeyEntry {
            name: resref(slice(b, e, 16)?),
            restype: u16_at(b, e + 16)?,
            bif: (id >> 20) as usize,
            index: (id & 0xF_FFFF) as usize,
        });
    }
    Ok(Key { bifs, entries })
}

/// The variable-resource table of a BIF: (offset, size) by index.
pub fn read_bif(b: &[u8]) -> Result<Vec<(usize, usize)>> {
    if slice(b, 0, 8)? != b"BIFFV1  " {
        return err("not a BIFF V1 file");
    }
    let count = u32_at(b, 8)? as usize;
    let table = u32_at(b, 16)? as usize;
    (0..count)
        .map(|i| {
            let e = table + i * 16;
            Ok((u32_at(b, e + 4)? as usize, u32_at(b, e + 8)? as usize))
        })
        .collect()
}

/// A resource found in a capsule (RIM, ERF, MOD).
pub struct Entry<'a> {
    pub name: String,
    pub restype: u16,
    pub data: &'a [u8],
}

pub fn read_rim(b: &[u8]) -> Result<Vec<Entry<'_>>> {
    if slice(b, 0, 8)? != b"RIM V1.0" {
        return err("not a RIM V1.0 file");
    }
    let count = u32_at(b, 12)? as usize;
    let table = u32_at(b, 16)? as usize;
    (0..count)
        .map(|i| {
            let e = table + i * 32;
            let off = u32_at(b, e + 24)? as usize;
            let size = u32_at(b, e + 28)? as usize;
            Ok(Entry {
                name: resref(slice(b, e, 16)?),
                restype: u32_at(b, e + 16)? as u16,
                data: slice(b, off, size)?,
            })
        })
        .collect()
}

pub fn read_erf(b: &[u8]) -> Result<Vec<Entry<'_>>> {
    let sig = slice(b, 0, 8)?;
    if !matches!(sig, b"ERF V1.0" | b"MOD V1.0" | b"SAV V1.0") {
        return err("not an ERF/MOD V1.0 file");
    }
    let count = u32_at(b, 16)? as usize;
    let keys = u32_at(b, 24)? as usize;
    let resources = u32_at(b, 28)? as usize;
    (0..count)
        .map(|i| {
            let k = keys + i * 24;
            let r = resources + i * 8;
            let off = u32_at(b, r)? as usize;
            let size = u32_at(b, r + 4)? as usize;
            Ok(Entry {
                name: resref(slice(b, k, 16)?),
                restype: u16_at(b, k + 20)?,
                data: slice(b, off, size)?,
            })
        })
        .collect()
}
