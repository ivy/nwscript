//! The codes `nws` itself reports, for what goes wrong in the command rather
//! than in a script: declared through `nwscript-diag`'s mechanism under the
//! prefix `NE`, each with a long explanation in `explain/CODE.md` beside
//! this crate. The library's codes (`E`, `W`) stay in `nwscript-diag`;
//! [`lookup`] finds a code in either registry.

pub use nwscript::diag::codes::Code;

nwscript::diag::codes! {
    dir "explain";
    NE0001 Error "the install --game-dir names holds no nwscript.nss";
}

/// The code with this id in the library's registry or this one, if
/// registered; case does not matter.
pub fn lookup(id: &str) -> Option<&'static Code> {
    nwscript::diag::codes::lookup(id).or_else(|| nwscript::diag::codes::lookup_in(ALL, id))
}
