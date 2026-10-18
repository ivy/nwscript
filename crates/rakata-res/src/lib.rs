//! Resource identity for BioWare's Aurora-family games, KotOR first.
//!
//! Every resource the engine loads is named by a [`ResRef`] (at most sixteen
//! bytes, compared without regard to case) and a [`ResType`] (a numeric id
//! the archives store, with a file extension the loose files use). The pair
//! is a [`ResId`], written `name.ext`. A [`ContentHash`] names a resource's
//! bytes, so an index, a cache and a build key agree on identity.
//!
//! The crate does no I/O and knows nothing of any archive layout; those are
//! the business of the crates above it.

#![forbid(unsafe_code)]

mod error;
mod hash;
mod resid;
mod resref;
mod restype;

pub use error::{Error, codes};
pub use hash::ContentHash;
pub use resid::ResId;
pub use resref::ResRef;
pub use restype::ResType;
