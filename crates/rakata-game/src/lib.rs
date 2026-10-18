//! The install of a BioWare Aurora-family game, KotOR 1 and 2: one model of
//! which copy of a resource the engine loads, from where, and why.
//!
//! [`Install::open`] reads `chitin.key` and lists the folders once;
//! [`Install::resolve`] walks the engine's search order (`Override/`, the
//! module's capsules in their precedence, KotOR 1's `patch.erf`, the BIFs)
//! and returns every copy as a [`Candidate`] with its [`Location`], content
//! hash and [`Verdict`]. Names compare without regard to case at every step,
//! since the game runs under Wine on case-sensitive file systems and mods
//! write names in every case; two `Override/` files that differ only in case
//! make the answer [ambiguous](Resolution::is_ambiguous), which is reported
//! rather than guessed. The install is also `nwscript`'s [`Resolver`], so a
//! script compiles against the includes the game would load.
//!
//! The dependency runs from this crate to `nwscript`, never back.

#![forbid(unsafe_code)]

mod archive;
mod error;
mod install;
mod listing;
mod profile;
mod resolver;

pub use error::{Error, Result, codes};
pub use install::{Candidate, Install, Layer, Location, ModuleName, Resolution, Verdict};
pub use nwscript::Resolver;
pub use profile::Profile;
