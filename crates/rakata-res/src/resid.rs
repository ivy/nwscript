//! `ResId`: a name and a type, written `name.ext`.

use std::fmt;
use std::str::FromStr;

use crate::{Error, ResRef, ResType};

/// A resource's identity: its [`ResRef`] and its [`ResType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResId {
    /// The resource's name.
    pub name: ResRef,
    /// The resource's type.
    pub ty: ResType,
}

impl ResId {
    /// A resource id from its parts.
    pub const fn new(name: ResRef, ty: ResType) -> Self {
        Self { name, ty }
    }

    /// A resource id from `name.ext`, as a file name or a `changes.ini`
    /// key writes it. The extension is the part after the last dot, in any
    /// case; the name is everything before it. Fails with
    /// [`Error::NoExtension`] when there is no dot,
    /// [`Error::UnknownExtension`] when the extension names no known type,
    /// and [`Error::ResRefTooLong`] when the name is more than sixteen bytes.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let (name, ext) = text.rsplit_once('.').ok_or_else(|| Error::NoExtension {
            text: text.to_owned(),
        })?;
        let ty = ResType::from_extension(ext).ok_or_else(|| Error::UnknownExtension {
            extension: ext.to_owned(),
        })?;
        Ok(Self {
            name: ResRef::new(name)?,
            ty,
        })
    }
}

impl FromStr for ResId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Self::parse(s)
    }
}

impl fmt::Display for ResId {
    /// `name.ext`, the name as read and the extension lowercase; an
    /// unknown type shows as `name.type<id>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.name, self.ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes;

    #[test]
    fn parses_name_and_extension() {
        let id = ResId::parse("p_Carth.UTC").unwrap();
        assert_eq!(id.name, ResRef::new("p_carth").unwrap());
        assert_eq!(id.ty, ResType::Utc);
        assert_eq!(id.to_string(), "p_Carth.utc");
        assert_eq!("k_ai_master.ncs".parse::<ResId>().unwrap().ty, ResType::Ncs);
    }

    #[test]
    fn a_dot_in_the_name_belongs_to_the_name() {
        let id = ResId::parse("a.b.tga").unwrap();
        assert_eq!(id.name.as_str(), Some("a.b"));
        assert_eq!(id.ty, ResType::Tga);
    }

    #[test]
    fn errors_carry_their_codes() {
        let e = ResId::parse("noext").unwrap_err();
        assert_eq!(e.code(), codes::RESID_NO_EXTENSION);
        let e = ResId::parse("name.zzz").unwrap_err();
        assert_eq!(e.code(), codes::UNKNOWN_EXTENSION);
        let e = ResId::parse("seventeen_letters.utc").unwrap_err();
        assert_eq!(e.code(), codes::RESREF_TOO_LONG);
    }

    #[test]
    fn unknown_types_display_by_id() {
        let id = ResId::new(ResRef::new("x").unwrap(), ResType::Unknown(4242));
        assert_eq!(id.to_string(), "x.type4242");
    }
}
