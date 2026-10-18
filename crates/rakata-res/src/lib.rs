//! Resource identity for BioWare's Aurora-family games, KotOR first.
//!
//! Every resource the engine loads is named by a [`ResRef`] (at most sixteen
//! bytes, compared without regard to case) and a [`ResType`] (a numeric id
//! the archives store, with a file extension the loose files use). The pair
//! is a [`ResId`], written `name.ext`. A [`ContentHash`] names a resource's
//! bytes, so an index, a cache and a build key agree on identity. A
//! [`StrRef`] names a line of the talk table, and a [`LocalizedString`]
//! carries one with the substrings a GFF field embeds per [`Language`] and
//! [`Gender`].
//!
//! The crate does no I/O and knows nothing of any archive layout; those are
//! the business of the crates above it.

#![forbid(unsafe_code)]

mod error;
mod hash;
mod locstring;
mod refs;
mod resid;
mod resref;
mod restype;

pub use error::{Error, codes};
pub use hash::ContentHash;
pub use locstring::{
    Gender, Language, LocalizedString, StrRef, Substring, from_string_id, string_id,
};
pub use refs::{Ref, RefKind};
pub use resid::ResId;
pub use resref::ResRef;
pub use restype::ResType;
