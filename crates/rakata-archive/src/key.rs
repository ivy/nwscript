//! KEY V1: the index over the BIFs. Layout: xoreos's `torlack/key.html` and
//! BioWare's KEY/BIF document; the measured facts are in
//! `docs/formats/keybif.md`.

use std::collections::HashMap;

use rakata_res::{ResId, ResRef, ResType};

use crate::bytes::{Reader, Writer, u32_field};
use crate::{Error, Result};

const SIGNATURE: &str = "KEY V1  ";
const HEADER_LEN: usize = 64;
const FILE_ENTRY_LEN: usize = 12;
const KEY_ENTRY_LEN: usize = 22;

/// One BIF the KEY indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BifRef {
    /// The BIF's size in bytes, as the KEY records it.
    pub size: u32,
    /// The BIF's path relative to the install, as stored: `data\2da.bif`,
    /// with backslashes and without its terminating NUL.
    pub name: Vec<u8>,
    /// The drive flags: 1 in every vanilla entry.
    pub drives: u16,
}

impl BifRef {
    /// The path as text, lossily; the separator is `\`.
    pub fn name_str(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }

    /// The path's components, split on `\` and `/`.
    pub fn components(&self) -> impl Iterator<Item = String> + '_ {
        self.name_str()
            .split(['\\', '/'])
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

/// One resource the KEY names: where in which BIF it lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEntry {
    /// The resource's name and type.
    pub id: ResId,
    /// The position of its BIF in [`Key::bifs`]: the upper twelve bits of
    /// the resource id.
    pub bif: u32,
    /// Its position in that BIF's table: the lower twenty bits.
    pub index: u32,
}

impl KeyEntry {
    /// The 32-bit resource id as the KEY and the BIF store it.
    pub const fn resource_id(&self) -> u32 {
        (self.bif << 20) | (self.index & 0xF_FFFF)
    }
}

/// A KEY file: the BIFs it indexes and the resources they hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// The build year, counted from 1900: 103 in the vanilla file.
    pub build_year: u32,
    /// The build day of the year, counted from 0.
    pub build_day: u32,
    bifs: Vec<BifRef>,
    entries: Vec<KeyEntry>,
    by_id: HashMap<ResId, usize>,
}

impl Default for Key {
    fn default() -> Self {
        Self::new()
    }
}

impl Key {
    /// An empty KEY.
    pub fn new() -> Self {
        Self {
            build_year: 0,
            build_day: 0,
            bifs: Vec::new(),
            entries: Vec::new(),
            by_id: HashMap::new(),
        }
    }

    /// The KEY in `bytes`. Nothing borrows from `bytes`: a KEY is names and
    /// numbers, half a megabyte for the whole game.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let r = Reader::new(bytes);
        r.signature(SIGNATURE)?;
        let bif_count = r.u32("the header", 0x08)? as usize;
        let key_count = r.u32("the header", 0x0C)? as usize;
        let file_table = r.u32("the header", 0x10)? as usize;
        let key_table = r.u32("the header", 0x14)? as usize;
        let build_year = r.u32("the header", 0x18)?;
        let build_day = r.u32("the header", 0x1C)?;

        let files = r.slice("the file table", file_table, bif_count * FILE_ENTRY_LEN)?;
        let mut bifs = Vec::with_capacity(bif_count);
        for e in files.as_chunks::<FILE_ENTRY_LEN>().0.iter() {
            let er = Reader::new(e);
            let size = er.u32("a file entry", 0)?;
            let name_offset = er.u32("a file entry", 4)? as usize;
            let name_len = er.u16("a file entry", 8)? as usize;
            let drives = er.u16("a file entry", 10)?;
            let raw = r.slice("a BIF name", name_offset, name_len)?;
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            bifs.push(BifRef {
                size,
                name: raw[..end].to_vec(),
                drives,
            });
        }

        let keys = r.slice("the key table", key_table, key_count * KEY_ENTRY_LEN)?;
        let mut entries = Vec::with_capacity(key_count);
        let mut by_id = HashMap::with_capacity(key_count);
        for e in keys.as_chunks::<KEY_ENTRY_LEN>().0.iter() {
            let er = Reader::new(e);
            let name = ResRef::from_bytes(&e[..16]).expect("sixteen bytes fit a resref");
            let ty = er.u16("a key entry", 0x10)?;
            let resource_id = er.u32("a key entry", 0x12)?;
            let entry = KeyEntry {
                id: ResId::new(name, ResType::from_id(ty)),
                bif: resource_id >> 20,
                index: resource_id & 0xF_FFFF,
            };
            by_id.entry(entry.id).or_insert(entries.len());
            entries.push(entry);
        }
        Ok(Self {
            build_year,
            build_day,
            bifs,
            entries,
            by_id,
        })
    }

    /// The BIFs, in file-table order; a [`KeyEntry::bif`] indexes this.
    pub fn bifs(&self) -> &[BifRef] {
        &self.bifs
    }

    /// The resources, in key-table order.
    pub fn entries(&self) -> &[KeyEntry] {
        &self.entries
    }

    /// Where the resource named `id` lives, compared without regard to
    /// case; the first entry when several share a name and type.
    pub fn get(&self, id: ResId) -> Option<&KeyEntry> {
        self.by_id.get(&id).map(|&i| &self.entries[i])
    }

    /// Append a BIF and return its position.
    pub fn add_bif(&mut self, bif: BifRef) -> usize {
        self.bifs.push(bif);
        self.bifs.len() - 1
    }

    /// Append a resource entry. Fails with [`Error::AlreadyPresent`] when
    /// the KEY names one of that name and type.
    pub fn add(&mut self, entry: KeyEntry) -> Result<()> {
        if self.by_id.contains_key(&entry.id) {
            return Err(Error::AlreadyPresent { id: entry.id });
        }
        self.by_id.insert(entry.id, self.entries.len());
        self.entries.push(entry);
        Ok(())
    }

    /// The KEY in BioWare's layout: a 64-byte header, the file table at 64,
    /// the BIF names back to back and NUL-terminated, then the key table,
    /// which ends the file.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let names_len: usize = self.bifs.iter().map(|b| b.name.len() + 1).sum();
        let names = HEADER_LEN + self.bifs.len() * FILE_ENTRY_LEN;
        let key_table = names + names_len;
        let total = key_table + self.entries.len() * KEY_ENTRY_LEN;
        u32_field(total)?;
        let mut w = Writer::with_capacity(total);

        w.bytes(SIGNATURE.as_bytes());
        w.u32(u32_field(self.bifs.len())?);
        w.u32(u32_field(self.entries.len())?);
        w.u32(HEADER_LEN as u32);
        w.u32(u32_field(key_table)?);
        w.u32(self.build_year);
        w.u32(self.build_day);
        w.zeros(32);
        debug_assert_eq!(w.len(), HEADER_LEN);

        let mut name_at = names;
        for b in &self.bifs {
            w.u32(b.size);
            w.u32(u32_field(name_at)?);
            w.u16(
                u16::try_from(b.name.len() + 1)
                    .map_err(|_| Error::TooLarge { size: total as u64 })?,
            );
            w.u16(b.drives);
            name_at += b.name.len() + 1;
        }
        for b in &self.bifs {
            w.bytes(&b.name);
            w.u8(0);
        }
        for e in &self.entries {
            w.bytes(&e.id.name.to_field());
            w.u16(e.id.ty.id());
            w.u32(e.resource_id());
        }
        Ok(w.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_round_trips() {
        let mut key = Key::new();
        key.build_year = 103;
        key.build_day = 309;
        key.add_bif(BifRef {
            size: 42,
            name: b"data\\2da.bif".to_vec(),
            drives: 1,
        });
        let id = ResId::new(ResRef::new("Appearance").unwrap(), ResType::TwoDa);
        key.add(KeyEntry {
            id,
            bif: 0,
            index: 7,
        })
        .unwrap();
        let bytes = key.to_bytes().unwrap();
        assert_eq!(bytes.len(), 64 + 12 + 13 + 22);
        assert_eq!(&bytes[76..89], b"data\\2da.bif\0");
        let back = Key::parse(&bytes).unwrap();
        assert_eq!(back, key);
        let e = back
            .get(ResId::new(
                ResRef::new("APPEARANCE").unwrap(),
                ResType::TwoDa,
            ))
            .unwrap();
        assert_eq!((e.bif, e.index, e.resource_id()), (0, 7, 7));
        assert_eq!(
            back.bifs()[0].components().collect::<Vec<_>>(),
            ["data", "2da.bif"]
        );
        let e = key
            .add(KeyEntry {
                id,
                bif: 0,
                index: 8,
            })
            .unwrap_err();
        assert_eq!(e.code(), crate::codes::ALREADY_PRESENT);
    }

    #[test]
    fn resource_ids_split_as_documented() {
        let e = KeyEntry {
            id: ResId::new(ResRef::new("x").unwrap(), ResType::Ncs),
            bif: 4,
            index: 0x29,
        };
        assert_eq!(e.resource_id(), 0x0040_0029);
    }
}
