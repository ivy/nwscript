//! ERF V1.0 and its signatures MOD, SAV and HAK. Layout: xoreos's
//! `torlack/mod.html` and BioWare's ERF document; the measured facts are in
//! `docs/formats/erf.md`.

use rakata_res::{ResId, ResRef, ResType};

use crate::bytes::{Reader, Writer, u32_field};
use crate::member::{self, Member};
use crate::{Error, Result};

const HEADER_LEN: usize = 160;
const KEY_LEN: usize = 24;
const POSITION_LEN: usize = 8;

/// The signature an ERF-family file carries, which names its use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErfKind {
    /// `ERF V1.0`: `patch.erf`, the texture packs, exported resources.
    Erf,
    /// `MOD V1.0`: a module, or a lip-sync pack under `lips/`.
    Mod,
    /// `SAV V1.0`: a saved game, and each module's state within one.
    Sav,
    /// `HAK V1.0`: a hak pak.
    Hak,
}

impl ErfKind {
    /// The eight signature bytes: the kind and the version.
    pub const fn signature(self) -> &'static str {
        match self {
            ErfKind::Erf => "ERF V1.0",
            ErfKind::Mod => "MOD V1.0",
            ErfKind::Sav => "SAV V1.0",
            ErfKind::Hak => "HAK V1.0",
        }
    }

    fn from_signature(sig: &[u8]) -> Option<Self> {
        [ErfKind::Erf, ErfKind::Mod, ErfKind::Sav, ErfKind::Hak]
            .into_iter()
            .find(|k| k.signature().as_bytes() == sig)
    }
}

/// One localized description string: a language id and its text, kept as
/// the bytes read, since the encoding is the language's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    /// The language identifier, as the file stores it.
    pub language: u32,
    /// The text, as the file stores it.
    pub text: Vec<u8>,
}

/// An ERF-family archive: its header fields and its members, in key-list
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Erf<'a> {
    /// Which signature the file carries.
    pub kind: ErfKind,
    /// The localized description strings; none in any vanilla KotOR file.
    pub descriptions: Vec<Description>,
    /// The build year, counted from 1900: 103 in all but one vanilla file.
    pub build_year: u32,
    /// The build day of the year, counted from 0.
    pub build_day: u32,
    /// The string reference of the description. Vanilla files carry what
    /// looks like uninitialised memory here; it is written back as read.
    pub description_strref: u32,
    members: Vec<Member<'a>>,
}

impl<'a> Erf<'a> {
    /// An empty archive of `kind`, built on day 0 of 1900 (the fields are
    /// public).
    pub fn new(kind: ErfKind) -> Self {
        Self {
            kind,
            descriptions: Vec::new(),
            build_year: 0,
            build_day: 0,
            description_strref: 0,
            members: Vec::new(),
        }
    }

    /// The archive in `bytes`, whichever of the four signatures it carries.
    /// Members borrow from `bytes`.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let r = Reader::new(bytes);
        let sig = r.slice("the signature", 0, 8)?;
        let kind = ErfKind::from_signature(sig).ok_or_else(|| Error::NotThisArchive {
            expected: "ERF V1.0",
            found: String::from_utf8_lossy(sig).into_owned(),
        })?;
        let string_count = r.u32("the header", 0x08)? as usize;
        let count = r.u32("the header", 0x10)? as usize;
        let strings = r.u32("the header", 0x14)? as usize;
        let keys = r.u32("the header", 0x18)? as usize;
        let positions = r.u32("the header", 0x1C)? as usize;
        let build_year = r.u32("the header", 0x20)?;
        let build_day = r.u32("the header", 0x24)?;
        let description_strref = r.u32("the header", 0x28)?;

        // Each description takes at least eight bytes: no more than that
        // many fit in what follows the offset, whatever the count says.
        let room = bytes.len().saturating_sub(strings) / 8;
        let mut descriptions = Vec::with_capacity(string_count.min(room));
        let mut at = strings;
        for _ in 0..string_count {
            let language = r.u32("the description strings", at)?;
            let len = r.u32("the description strings", at + 4)? as usize;
            let text = r.slice("the description strings", at + 8, len)?;
            descriptions.push(Description {
                language,
                text: text.to_vec(),
            });
            at += 8 + len;
        }

        let key_list = r.slice("the key list", keys, count * KEY_LEN)?;
        let position_list = r.slice("the resource list", positions, count * POSITION_LEN)?;
        let mut members = Vec::with_capacity(count);
        for (k, p) in key_list
            .as_chunks::<KEY_LEN>()
            .0
            .iter()
            .zip(position_list.as_chunks::<POSITION_LEN>().0.iter())
        {
            let name = ResRef::from_bytes(&k[..16]).expect("sixteen bytes fit a resref");
            let ty = Reader::new(k).u16("a key", 0x14)?;
            let id = ResId::new(name, ResType::from_id(ty));
            let pr = Reader::new(p);
            let offset = pr.u32("a position", 0)? as usize;
            let len = pr.u32("a position", 4)? as usize;
            let data = offset
                .checked_add(len)
                .and_then(|end| bytes.get(offset..end))
                .ok_or(Error::MemberBeyondEnd {
                    id,
                    offset,
                    len,
                    available: r.len(),
                })?;
            members.push(Member::borrowed(id, data));
        }
        Ok(Self {
            kind,
            descriptions,
            build_year,
            build_day,
            description_strref,
            members,
        })
    }

    /// The members, in key-list order.
    pub fn members(&self) -> &[Member<'a>] {
        &self.members
    }

    /// How many members the archive holds.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether the archive holds no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// The bytes of the member named `id`, compared without regard to case.
    pub fn get(&self, id: ResId) -> Option<&[u8]> {
        member::position(&self.members, id).map(|i| self.members[i].data())
    }

    /// Whether the archive holds a member named `id`.
    pub fn contains(&self, id: ResId) -> bool {
        member::position(&self.members, id).is_some()
    }

    /// Append a member. Fails with [`Error::AlreadyPresent`] when the
    /// archive holds one of that name and type.
    pub fn add(&mut self, id: ResId, data: Vec<u8>) -> Result<()> {
        member::add(&mut self.members, id, data)
    }

    /// Give the member named `id` new bytes; every other member is
    /// unchanged. Fails with [`Error::NotPresent`] when there is none.
    pub fn replace(&mut self, id: ResId, data: Vec<u8>) -> Result<()> {
        member::replace(&mut self.members, id, data)
    }

    /// Remove the member named `id` and return it, if present.
    pub fn remove(&mut self, id: ResId) -> Option<Member<'a>> {
        member::position(&self.members, id).map(|i| self.members.remove(i))
    }

    /// The archive in BioWare's layout: a 160-byte header, the description
    /// strings at 160, the key list, the resource list, and the members
    /// back to back with no padding. Fails with [`Error::TooLarge`] past
    /// 4 GiB.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let count = self.members.len();
        let strings_len: usize = self.descriptions.iter().map(|d| 8 + d.text.len()).sum();
        let keys = HEADER_LEN + strings_len;
        let positions = keys + count * KEY_LEN;
        let data = positions + count * POSITION_LEN;
        let total = data + self.members.iter().map(Member::len).sum::<usize>();
        u32_field(total)?;
        let mut w = Writer::with_capacity(total);

        w.bytes(self.kind.signature().as_bytes());
        w.u32(u32_field(self.descriptions.len())?);
        w.u32(u32_field(strings_len)?);
        w.u32(u32_field(count)?);
        w.u32(HEADER_LEN as u32);
        w.u32(u32_field(keys)?);
        w.u32(u32_field(positions)?);
        w.u32(self.build_year);
        w.u32(self.build_day);
        w.u32(self.description_strref);
        w.zeros(116);
        debug_assert_eq!(w.len(), HEADER_LEN);

        for d in &self.descriptions {
            w.u32(d.language);
            w.u32(u32_field(d.text.len())?);
            w.bytes(&d.text);
        }
        for (i, m) in self.members.iter().enumerate() {
            w.bytes(&m.id.name.to_field());
            w.u32(u32_field(i)?);
            w.u16(m.id.ty.id());
            w.u16(0);
        }
        let mut cursor = data;
        for m in &self.members {
            w.u32(u32_field(cursor)?);
            w.u32(u32_field(m.len())?);
            cursor += m.len();
        }
        for m in &self.members {
            w.bytes(m.data());
        }
        Ok(w.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str, ty: ResType) -> ResId {
        ResId::new(ResRef::new(name).unwrap(), ty)
    }

    #[test]
    fn an_empty_mod_is_its_header() {
        let mut erf = Erf::new(ErfKind::Mod);
        erf.build_year = 103;
        erf.build_day = 145;
        let bytes = erf.to_bytes().unwrap();
        assert_eq!(bytes.len(), 160);
        assert_eq!(&bytes[..8], b"MOD V1.0");
        let back = Erf::parse(&bytes).unwrap();
        assert_eq!(back, erf);
    }

    #[test]
    fn strings_and_members_round_trip() {
        let mut erf = Erf::new(ErfKind::Hak);
        erf.descriptions.push(Description {
            language: 0,
            text: b"A hak".to_vec(),
        });
        erf.description_strref = 0xFFFF_FFFF;
        erf.add(id("Alpha", ResType::Utc), vec![1, 2]).unwrap();
        erf.add(id("beta", ResType::Ncs), vec![3]).unwrap();
        let bytes = erf.to_bytes().unwrap();
        // header 160, one 13-byte string, two 24-byte keys, two 8-byte
        // positions: data at 237.
        assert_eq!(&bytes[237..240], &[1, 2, 3]);
        assert_eq!(bytes.len(), 240);
        let back = Erf::parse(&bytes).unwrap();
        assert_eq!(back, erf);
        assert_eq!(back.get(id("ALPHA", ResType::Utc)), Some(&[1, 2][..]));
        assert_eq!(back.members()[0].id.name.to_string(), "Alpha");
    }

    #[test]
    fn a_rim_is_not_an_erf() {
        let e = Erf::parse(b"RIM V1.0\0\0\0\0\0\0\0\0").unwrap_err();
        assert_eq!(e.code(), crate::codes::NOT_THIS_ARCHIVE);
        assert!(e.to_string().contains("ERF V1.0"));
    }
}
