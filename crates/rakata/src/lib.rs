//! The façade of the data toolset: one API over the `rakata-*` crates, the
//! diagnostic codes they report under, the round-trip suite every format
//! registers with ([`conformance`]), and the build identity the `rakata`
//! binary and the Python module print.
//!
//! The agent surface, the JSON [`Envelope`] and the [`Exit`] codes, is the
//! one `nws` uses, shared through `nwscript-diag`; only the `game` field
//! differs, an install directory here where `nws` records a profile.

pub mod codes;
pub mod conformance;
pub mod corpus;
pub mod formats;
pub mod inspect;
pub mod patch;
pub mod save;

pub use nwscript_diag as diag;
pub use nwscript_diag::{Exit, InputRecord, OutputRecord};
pub use rakata_2da as twoda;
pub use rakata_game as game;
pub use rakata_gff as gff;
pub use rakata_lip as lip;
pub use rakata_res as res;
pub use rakata_tlk as tlk;

use std::path::PathBuf;

/// The build identity: the workspace version, then the git describe of the
/// checkout it was built from and, when that checkout had uncommitted
/// changes, a short hash of them (`0.0.0+49a9b40.dirty.3fa1c2d8`). It is what
/// `rakata --version` prints and what a build system keys a local build on.
pub const VERSION: &str = env!("RAKATA_VERSION");

/// The envelope with the install `--game` named in its `game` field.
pub type Envelope = nwscript_diag::Envelope<PathBuf>;

/// The envelope of a `rakata` command.
pub fn envelope(command: &str, game: Option<PathBuf>) -> Envelope {
    Envelope::new("rakata", VERSION, command, game)
}
