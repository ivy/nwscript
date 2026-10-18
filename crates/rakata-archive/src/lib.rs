//! The archives of BioWare's Aurora-family games, KotOR 1 and 2 first: the
//! [`Key`] over the [`Bif`]s that hold the base game, the [`Erf`] family
//! (ERF, MOD, SAV, HAK) and the [`Rim`] a module ships as.
//!
//! Every reader parses a byte slice, borrowed from a [`Source`] (the whole
//! file read into memory, or memory-mapped) or from anywhere else, and the
//! members it lists are slices of that same buffer: nothing is copied until a
//! caller asks for owned bytes. Every writer emits BioWare's own layout, so an
//! archive read from the game and written again is byte for byte the file it
//! came from; the suite in `tests/vanilla.rs` measures that over an install.
//! [`Rim::replace`], [`Erf::replace`] and their `add` counterparts change one
//! member and leave every other member's bytes untouched, which the fixtures
//! in `tests/fixtures.rs` hold the writer to.
//!
//! Names compare without regard to case, as the engine compares them, and are
//! written back in the spelling they were read with ([`rakata_res::ResRef`]).
//! The layouts follow the public format documents and the measured facts in
//! `docs/formats/` of this repository; this crate was written without sight of
//! any other implementation.

mod bif;
mod bytes;
mod erf;
mod error;
mod key;
mod member;
mod mmap;
mod rim;
mod source;

pub use bif::{Bif, BifMember};
pub use erf::{Description, Erf, ErfKind};
pub use error::{Error, Result, codes};
pub use key::{BifRef, Key, KeyEntry};
pub use member::Member;
pub use rim::Rim;
pub use source::Source;
