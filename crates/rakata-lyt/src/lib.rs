//! An area's room layout and room visibility in BioWare's Aurora-family
//! games, KotOR 1 and 2 first: a [`Lyt`] places each room's model in the
//! area and names the door hooks, and a [`Vis`] lists, for each room, the
//! rooms drawn while the camera is in it. Both are text; coordinates are
//! the game's own (metres, Z up), as the file stores them.
//!
//! A file read and written again is byte for byte the file it came from:
//! the writer produces the vanilla files' form, and where a file read
//! departs from it (a tab for spaces, a count one off, a missing final line
//! ending) the departing line is kept as read. The layout is
//! `docs/formats/lyt.md` of this repository, measured over the vanilla
//! files; this crate was written without sight of any other
//! implementation.

#![forbid(unsafe_code)]

mod error;
pub mod facts;
mod lyt;
mod text;
mod vis;

pub use error::{Error, Result, codes};
pub use lyt::{DoorHook, Lyt, Placement, Quaternion};
pub use text::format_float;
pub use vis::{Vis, VisRoom};
