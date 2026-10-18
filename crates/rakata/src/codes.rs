//! The registry of the data toolset's diagnostic codes, declared through
//! `nwscript-diag`'s mechanism under a prefix of their own, so a program
//! reading either binary's output tells them apart: `RE` an error, `RW` a
//! lint, each with a long explanation in `explain/CODE.md` beside this
//! crate, which `rakata explain CODE` prints.
//!
//! Ranges: `RE00xx` inputs and invocation, `RE01xx` resource identity
//! (`rakata-res`), `RE02xx` archives (`rakata-archive`). Each format crate
//! takes a range of its own as it lands.

pub use nwscript_diag::codes::Code;

nwscript_diag::codes! {
    dir "explain";
    RE0001 Error "an input file could not be read";
    RE0101 Error "a resref longer than sixteen bytes";
    RE0102 Error "a resource id without an extension";
    RE0103 Error "an extension that names no resource type";
    RE0201 Error "not an archive of the kind asked for";
    RE0202 Error "an archive truncated before its header or tables end";
    RE0203 Error "an archive member lying beyond the end of the file";
    RE0204 Error "a resource type id wider than sixteen bits";
    RE0205 Error "an archive too large for 32-bit offsets";
    RE0206 Error "a member added under a name the archive already holds";
    RE0207 Error "a member replaced that the archive does not hold";
    RE0208 Error "a BIF with fixed resources";
    RE0209 Error "a BIF member index past the end of its table";
}

/// The rakata code with this id, if registered; case does not matter.
pub fn lookup(id: &str) -> Option<&'static Code> {
    nwscript_diag::codes::lookup_in(ALL, id)
}
