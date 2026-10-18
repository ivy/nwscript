//! `Member`: a named resource in an archive, and the list operations the
//! archive kinds share.

use std::borrow::Cow;

use rakata_res::ResId;

use crate::{Error, Result};

/// One resource in an archive: its name and type, and its bytes, borrowed
/// from the source it was read from until a caller replaces them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member<'a> {
    /// The resource's name and type. Names compare without regard to case.
    pub id: ResId,
    data: Cow<'a, [u8]>,
}

impl<'a> Member<'a> {
    /// A member over borrowed bytes.
    pub fn borrowed(id: ResId, data: &'a [u8]) -> Self {
        Self {
            id,
            data: Cow::Borrowed(data),
        }
    }

    /// A member over bytes of its own.
    pub fn owned(id: ResId, data: Vec<u8>) -> Self {
        Self {
            id,
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

    /// Whether the member is empty; `globaldx.rim` holds such members.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Whether the bytes are still a slice of the source they were read
    /// from, rather than a copy.
    pub fn is_borrowed(&self) -> bool {
        matches!(self.data, Cow::Borrowed(_))
    }

    /// The member with its bytes copied, free of the source's lifetime.
    pub fn into_owned(self) -> Member<'static> {
        Member {
            id: self.id,
            data: Cow::Owned(self.data.into_owned()),
        }
    }
}

/// The position of the first member named `id`, when several share a name
/// and type (no vanilla archive does): the one the lookups answer with.
pub(crate) fn position(members: &[Member<'_>], id: ResId) -> Option<usize> {
    members.iter().position(|m| m.id == id)
}

/// Append a member the list does not yet hold.
pub(crate) fn add<'a>(members: &mut Vec<Member<'a>>, id: ResId, data: Vec<u8>) -> Result<()> {
    if position(members, id).is_some() {
        return Err(Error::AlreadyPresent { id });
    }
    members.push(Member::owned(id, data));
    Ok(())
}

/// Give the member named `id` new bytes, in place. Its position, the
/// spelling of its stored name, and every other member are unchanged.
pub(crate) fn replace<'a>(members: &mut [Member<'a>], id: ResId, data: Vec<u8>) -> Result<()> {
    let i = position(members, id).ok_or(Error::NotPresent { id })?;
    members[i].data = Cow::Owned(data);
    Ok(())
}
