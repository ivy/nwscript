//! The façade: compile, decompile, assemble, run and check, with the resolver trait.
//!
//! Every `nws` command is one call into this crate plus argument parsing and
//! output formatting. The crate does no I/O of its own: source files,
//! includes and `nwscript.nss` arrive through a [`Resolver`] the caller
//! supplies, and every file a resolver returns is recorded so the
//! [`Envelope`] can name each input with its hash.

pub mod bytecode;
pub mod decompile;
pub mod engine;
pub mod envelope;
pub mod resolve;
pub mod run;
pub mod snapshot;

pub use envelope::{Envelope, InputRecord, OutputRecord};
pub use nwscript_diag as diag;
pub use nwscript_ncs as ncs;
pub use nwscript_sema as sema;
pub use nwscript_vm as vm;
pub use resolve::{FsResolver, MemResolver, Resolved, Resolver};

use serde::Serialize;

/// A game profile: which `nwscript.nss`, which code generation forms, which
/// engine semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    K1,
    K2,
}

impl Game {
    pub fn name(self) -> &'static str {
        match self {
            Game::K1 => "k1",
            Game::K2 => "k2",
        }
    }
}

/// The hex sha256 of some bytes, the hash every input and output record carries.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
