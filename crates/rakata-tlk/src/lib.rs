//! The talk table of BioWare's Aurora-family games, KotOR 1 and 2 first:
//! `dialog.tlk`, every string the game shows by strref.
//!
//! A [`Tlk`] is read from a file or a byte string, answers a
//! [`rakata_res::StrRef`] with its [`Entry`] (text, voice-over resref and
//! length, flags), resolves a [`rakata_res::LocalizedString`] by BioWare's
//! documented procedure, takes a new entry with [`Tlk::append`] and returns
//! its strref, which is what installing a mod needs, and writes back in
//! BioWare's own layout, so the vanilla `dialog.tlk` read and written again
//! is byte for byte the file it came from; `tests/vanilla.rs` measures that
//! over an install.
//!
//! Text is Windows-1252 in KotOR's files and is decoded and re-encoded so
//! that every byte comes back as it was ([`codepage`]). The layout follows
//! BioWare's Talk Table document and the measured facts in
//! `docs/formats/tlk.md`; this crate was written without sight of any other
//! implementation.

#![forbid(unsafe_code)]

pub mod codepage;
mod error;
mod tlk;

pub use error::{Error, Result, codes};
pub use tlk::{Entry, Tlk};
