//! RIM V1.0: the read-only module archive. Layout: `docs/formats/rim.md`.

use rakata_res::{ResId, ResRef, ResType};

use crate::bytes::{Reader, Writer, u32_field};
use crate::member::{self, Member};
use crate::{Error, Result};

const SIGNATURE: &str = "RIM V1.0";
const HEADER_LEN: usize = 120;
const ENTRY_LEN: usize = 32;

/// A RIM archive: a flag byte and its members, in table order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rim<'a> {
    /// The byte at offset 0x14: 1 in the six vanilla `rims/*dx.rim` files, 0
    /// in every other. When set, members are aligned to 128 bytes instead of
    /// 4 ([`Rim::alignment`]); what the engine makes of it is not
    /// established. It is written back as read.
    pub flag: u8,
    members: Vec<Member<'a>>,
}

impl Default for Rim<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Rim<'a> {
    /// An empty archive.
    pub fn new() -> Self {
        Self {
            flag: 0,
            members: Vec::new(),
        }
    }

    /// The archive in `bytes`. Members borrow from `bytes`.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let r = Reader::new(bytes);
        r.signature(SIGNATURE)?;
        let count = r.u32("the header", 0x0C)? as usize;
        let table = r.u32("the header", 0x10)? as usize;
        let flag = r.u8("the header", 0x14)?;
        let entries = r.slice("the resource table", table, count * ENTRY_LEN)?;
        let mut members = Vec::with_capacity(count);
        for (index, e) in entries.as_chunks::<ENTRY_LEN>().0.iter().enumerate() {
            let er = Reader::new(e);
            let name = ResRef::from_bytes(&e[..16]).expect("sixteen bytes fit a resref");
            let type_id = er.u32("an entry", 0x10)?;
            let ty = u16::try_from(type_id).map_err(|_| Error::TypeTooWide { type_id, index })?;
            let id = ResId::new(name, ResType::from_id(ty));
            let offset = er.u32("an entry", 0x18)? as usize;
            let len = er.u32("an entry", 0x1C)? as usize;
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
        Ok(Self { flag, members })
    }

    /// The members, in table order.
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

    /// The archive in BioWare's layout: a 120-byte header, the table at 120,
    /// 8 zero bytes, each member followed by 16 zero bytes and alignment to
    /// [`Rim::alignment`], and 10 zero bytes at the end; an empty archive
    /// ends with its header. Fails with [`Error::TooLarge`] past 4 GiB.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let count = self.members.len();
        let table_len = count * ENTRY_LEN;
        let data_len: usize = self.members.iter().map(|m| m.len() + 19).sum();
        let mut w = Writer::with_capacity(HEADER_LEN + table_len + data_len + 18);

        w.bytes(SIGNATURE.as_bytes());
        w.u32(0);
        w.u32(u32_field(count)?);
        w.u32(HEADER_LEN as u32);
        w.u8(self.flag);
        w.zeros(99);
        debug_assert_eq!(w.len(), HEADER_LEN);

        // Members start after the table and 8 zero bytes, aligned; each is
        // followed by 16 zero bytes and then alignment. The alignment is 4,
        // or 128 when the flag is set (`docs/formats/rim.md`, "Member data").
        let align = self.alignment();
        let mut offsets = Vec::with_capacity(count);
        if count > 0 {
            let mut cursor = (HEADER_LEN + table_len + 8).next_multiple_of(align);
            for m in &self.members {
                offsets.push(cursor);
                cursor += m.len() + 16;
                cursor = cursor.next_multiple_of(align);
            }
        }
        for (i, (m, &offset)) in self.members.iter().zip(&offsets).enumerate() {
            w.bytes(&m.id.name.to_field());
            w.u32(u32::from(m.id.ty.id()));
            w.u32(u32_field(i)?);
            w.u32(u32_field(offset)?);
            w.u32(u32_field(m.len())?);
        }
        if count > 0 {
            w.zeros(8);
            w.align(align);
            for (i, m) in self.members.iter().enumerate() {
                debug_assert_eq!(w.len(), offsets[i]);
                w.bytes(m.data());
                if i + 1 < count {
                    w.zeros(16);
                    w.align(align);
                } else {
                    w.zeros(10);
                }
            }
        }
        Ok(w.finish())
    }

    /// The alignment of members in the file: 128 when [`Rim::flag`] is set,
    /// as in the six vanilla `dx` files, and 4 otherwise.
    pub fn alignment(&self) -> usize {
        if self.flag == 0 { 4 } else { 128 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str, ty: ResType) -> ResId {
        ResId::new(ResRef::new(name).unwrap(), ty)
    }

    #[test]
    fn an_empty_rim_is_its_header() {
        let bytes = Rim::new().to_bytes().unwrap();
        assert_eq!(bytes.len(), 120);
        assert_eq!(&bytes[..8], b"RIM V1.0");
        assert_eq!(&bytes[0x10..0x14], &120u32.to_le_bytes());
        let back = Rim::parse(&bytes).unwrap();
        assert!(back.is_empty());
    }

    #[test]
    fn members_round_trip_with_the_documented_gaps() {
        let mut rim = Rim::new();
        rim.add(id("A", ResType::Ncs), vec![1, 2, 3]).unwrap();
        rim.add(id("b", ResType::Utc), vec![9; 7]).unwrap();
        let bytes = rim.to_bytes().unwrap();
        // header 120, table 64, 8 zeros: first member at 192.
        assert_eq!(&bytes[192..195], &[1, 2, 3]);
        // 3 bytes + 16 zeros = 211, aligned to 212.
        assert_eq!(&bytes[212..219], &[9; 7]);
        assert_eq!(bytes.len(), 219 + 10);
        let back = Rim::parse(&bytes).unwrap();
        assert_eq!(back.get(id("a", ResType::Ncs)), Some(&[1, 2, 3][..]));
        assert_eq!(back.members()[0].id.name.to_string(), "A");
        assert!(back.members()[0].is_borrowed());
        assert_eq!(back, rim);
    }

    #[test]
    fn add_and_replace_report_their_codes() {
        let mut rim = Rim::new();
        rim.add(id("a", ResType::Ncs), vec![1]).unwrap();
        let e = rim.add(id("A", ResType::Ncs), vec![2]).unwrap_err();
        assert_eq!(e.code(), crate::codes::ALREADY_PRESENT);
        let e = rim.replace(id("zz", ResType::Ncs), vec![2]).unwrap_err();
        assert_eq!(e.code(), crate::codes::NOT_PRESENT);
        rim.replace(id("A", ResType::Ncs), vec![2]).unwrap();
        assert_eq!(rim.get(id("a", ResType::Ncs)), Some(&[2][..]));
    }

    #[test]
    fn a_member_beyond_the_end_is_an_error() {
        let mut rim = Rim::new();
        rim.add(id("a", ResType::Ncs), vec![1, 2, 3]).unwrap();
        let mut bytes = rim.to_bytes().unwrap();
        // Header 120, one entry, 8 zero bytes: the member begins at 160.
        bytes.truncate(162);
        let e = Rim::parse(&bytes).unwrap_err();
        assert_eq!(e.code(), crate::codes::MEMBER_BEYOND_END);
        let e = Rim::parse(b"ERF V1.0").unwrap_err();
        assert_eq!(e.code(), crate::codes::NOT_THIS_ARCHIVE);
        let e = Rim::parse(b"RIM V1.0\0\0\0\0").unwrap_err();
        assert_eq!(e.code(), crate::codes::TRUNCATED);
    }
}
