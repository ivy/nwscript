//! The JSON document every command writes in `--format json` mode, with
//! `game` the profile: the shared [`nwscript_diag::Envelope`] over [`Game`].

use crate::Game;

pub use nwscript_diag::{InputRecord, OutputRecord};

/// The envelope with the game profile in its `game` field.
pub type Envelope = nwscript_diag::Envelope<Game>;
