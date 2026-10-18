//! The registry of the data toolset's diagnostic codes, declared through
//! `nwscript-diag`'s mechanism under a prefix of their own, so a program
//! reading either binary's output tells them apart: `RE` an error, `RW` a
//! lint, each with a long explanation in `explain/CODE.md` beside this
//! crate, which `rakata explain CODE` prints.
//!
//! Ranges: `RE00xx` inputs and invocation, `RE01xx` resource identity
//! (`rakata-res`). Each format crate takes a range of its own as it lands.

pub use nwscript_diag::codes::Code;

nwscript_diag::codes! {
    dir "explain";
    RE0001 Error "an input file could not be read";
    RE0101 Error "a resref longer than sixteen bytes";
    RE0102 Error "a resource id without an extension";
    RE0103 Error "an extension that names no resource type";
}

/// The rakata code with this id, if registered; case does not matter.
pub fn lookup(id: &str) -> Option<&'static Code> {
    nwscript_diag::codes::lookup_in(ALL, id)
}
