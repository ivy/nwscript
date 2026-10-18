//! BIFF V1: the files that hold the base game's resources, indexed by the
//! KEY. Layout: xoreos's `torlack/bif.html` and BioWare's KEY/BIF document;
//! the measured facts are in `docs/formats/keybif.md`.

use std::borrow::Cow;

use rakata_res::ResType;

use crate::bytes::{Reader, Writer, u32_field};
use crate::{Error, Result};

const SIGNATURE: &str = "BIFFV1  ";
const HEADER_LEN: usize = 20;
const ENTRY_LEN: usize = 16;

/// One resource in a BIF. A BIF holds no names; the KEY names its members
/// by position, and the resource id here repeats the KEY's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BifMember<'a> {
    /// The resource id as the KEY stores it: the BIF's position in the
    /// KEY's file table in the upper twelve bits, this member's position in
    /// the lower twenty.
    pub resource_id: u32,
    /// The resource's type.
    pub ty: ResType,
    data: Cow<'a, [u8]>,
}

impl<'a> BifMember<'a> {
    /// A member over bytes of its own.
    pub fn owned(resource_id: u32, ty: ResType, data: Vec<u8>) -> Self {
        Self {
            resource_id,
            ty,
            data: Cow::Owned(data),
        }
    }

    /// The member's bytes.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// The member's length in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the member is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Whether the bytes are still a slice of the source they were read
    /// from, rather than a copy.
    pub fn is_borrowed(&self) -> bool {
        matches!(self.data, Cow::Borrowed(_))
    }
}

/// A BIF: its members, in table order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bif<'a> {
    members: Vec<BifMember<'a>>,
}

impl<'a> Bif<'a> {
    /// An empty BIF.
    pub fn new() -> Self {
        Self::default()
    }

    /// The BIF in `bytes`. Members borrow from `bytes`, so listing a BIF of
    /// hundreds of megabytes reads its header and table and nothing else.
    /// Fails with [`Error::FixedResources`] when the header counts fixed
    /// resources, a table no KotOR file has and this crate does not read.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let r = Reader::new(bytes);
        r.signature(SIGNATURE)?;
        let count = r.u32("the header", 0x08)? as usize;
        let fixed = r.u32("the header", 0x0C)?;
        if fixed != 0 {
            return Err(Error::FixedResources { count: fixed });
        }
        let table = r.u32("the header", 0x10)? as usize;
        let entries = r.slice("the resource table", table, count * ENTRY_LEN)?;
        let mut members = Vec::with_capacity(count);
        for (index, e) in entries.as_chunks::<ENTRY_LEN>().0.iter().enumerate() {
            let er = Reader::new(e);
            let resource_id = er.u32("an entry", 0)?;
            let offset = er.u32("an entry", 4)? as usize;
            let len = er.u32("an entry", 8)? as usize;
            let type_id = er.u32("an entry", 12)?;
            let ty = u16::try_from(type_id).map_err(|_| Error::TypeTooWide { type_id, index })?;
            let data = offset
                .checked_add(len)
                .and_then(|end| bytes.get(offset..end))
                .ok_or(Error::BifMemberBeyondEnd {
                    index,
                    offset,
                    len,
                    available: r.len(),
                })?;
            members.push(BifMember {
                resource_id,
                ty: ResType::from_id(ty),
                data: Cow::Borrowed(data),
            });
        }
        Ok(Self { members })
    }

    /// The members, in table order: the order the KEY's indices count.
    pub fn members(&self) -> &[BifMember<'a>] {
        &self.members
    }

    /// How many members the BIF holds.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether the BIF holds no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// The bytes of the member at `index`, as a [`KeyEntry`](crate::KeyEntry)
    /// gives it.
    pub fn get(&self, index: usize) -> Option<&[u8]> {
        self.members.get(index).map(BifMember::data)
    }

    /// Append a member and return its position.
    pub fn push(&mut self, member: BifMember<'a>) -> usize {
        self.members.push(member);
        self.members.len() - 1
    }

    /// Give the member at `index` new bytes; every other member is
    /// unchanged. Fails with [`Error::NoSuchIndex`] past the end.
    pub fn replace(&mut self, index: usize, data: Vec<u8>) -> Result<()> {
        let len = self.members.len();
        let m = self
            .members
            .get_mut(index)
            .ok_or(Error::NoSuchIndex { index, len })?;
        m.data = Cow::Owned(data);
        Ok(())
    }

    /// The BIF in BioWare's layout: a 20-byte header, the table at 20, and
    /// the members back to back with no padding. Fails with
    /// [`Error::TooLarge`] past 4 GiB.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let count = self.members.len();
        let data = HEADER_LEN + count * ENTRY_LEN;
        let total = data + self.members.iter().map(BifMember::len).sum::<usize>();
        u32_field(total)?;
        let mut w = Writer::with_capacity(total);

        w.bytes(SIGNATURE.as_bytes());
        w.u32(u32_field(count)?);
        w.u32(0);
        w.u32(HEADER_LEN as u32);
        debug_assert_eq!(w.len(), HEADER_LEN);

        let mut cursor = data;
        for m in &self.members {
            w.u32(m.resource_id);
            w.u32(u32_field(cursor)?);
            w.u32(u32_field(m.len())?);
            w.u32(u32::from(m.ty.id()));
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

    #[test]
    fn a_bif_round_trips_contiguously() {
        let mut bif = Bif::new();
        bif.push(BifMember::owned(0x0040_0000, ResType::Ncs, vec![1, 2, 3]));
        bif.push(BifMember::owned(0x0040_0001, ResType::Nss, vec![4]));
        let bytes = bif.to_bytes().unwrap();
        assert_eq!(bytes.len(), 20 + 32 + 4);
        assert_eq!(&bytes[52..56], &[1, 2, 3, 4]);
        let back = Bif::parse(&bytes).unwrap();
        assert_eq!(back, bif);
        assert_eq!(back.get(1), Some(&[4][..]));
        assert!(back.members()[0].is_borrowed());
        assert_eq!(back.members()[1].ty, ResType::Nss);
    }

    #[test]
    fn replace_past_the_end_is_an_error() {
        let mut bif = Bif::new();
        let e = bif.replace(0, vec![]).unwrap_err();
        assert_eq!(e.code(), crate::codes::NO_SUCH_INDEX);
    }

    #[test]
    fn fixed_resources_are_refused() {
        let mut bytes = Bif::new().to_bytes().unwrap();
        bytes[12] = 1;
        let e = Bif::parse(&bytes).unwrap_err();
        assert_eq!(e.code(), crate::codes::FIXED_RESOURCES);
    }
}
