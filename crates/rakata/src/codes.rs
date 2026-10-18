//! The registry of the data toolset's diagnostic codes, declared through
//! `nwscript-diag`'s mechanism under a prefix of their own, so a program
//! reading either binary's output tells them apart: `RE` an error, `RW` a
//! lint, each with a long explanation in `explain/CODE.md` beside this
//! crate, which `rakata explain CODE` prints.
//!
//! Ranges: `RE00xx` inputs and invocation, `RE01xx` resource identity
//! (`rakata-res`), `RE02xx` archives (`rakata-archive`), `RE03xx` and
//! `RW03xx` the installer's change files (`rakata-patch`), `RE04xx` the
//! install (`rakata-game`), `RE05xx` the round-trip suite
//! ([`crate::conformance`]), `RE06xx` GFF (`rakata-gff`), `RE07xx` tables
//! (`rakata-2da`), `RE08xx` the talk table (`rakata-tlk`), `RE09xx` lip-sync
//! files (`rakata-lip`), `RE10xx` and `RW10xx` the corpus
//! ([`crate::corpus`]). Each crate takes a range of its own as it lands.

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
    RE0301 Error "a line of a change file that is not a section, a key or a comment";
    RE0302 Error "a key before the first section of a change file";
    RE0303 Error "a value in no form its key takes";
    RE0304 Error "a namespaces.ini without options";
    RW0301 Warning "a list entry naming a section the change file does not have";
    RW0302 Warning "a key the section does not take";
    RW0303 Warning "a section or key written twice";
    RW0304 Warning "a memory token read before anything sets it";
    RE0401 Error "not a game install: no chitin.key";
    RE0402 Error "a BIF the KEY names is missing from the install";
    RE0403 Error "no module capsule carries the name";
    RE0404 Error "no layer of the install holds the resource";
    RW0401 Warning "Override files that differ only in case";
    RE0501 Error "a directory that is not a game install";
    RE0601 Error "not a GFF V3.2 file";
    RE0602 Error "a GFF section or field lying beyond the end of the file";
    RE0603 Error "a GFF struct, field or label index out of range";
    RE0604 Error "a GFF field type id that names no type";
    RE0605 Error "a GFF too large for 32-bit offsets";
    RE0606 Error "a field path that does not parse";
    RE0607 Error "a field path naming nothing in the file";
    RE0608 Error "a GFF label or resref longer than sixteen bytes";
    RE0502 Error "a round trip wrote bytes the format's own reader rejects";
    RE0701 Error "not a 2DA V2.b table";
    RE0702 Error "a 2DA header running past the end of the file";
    RE0703 Error "a 2DA cell whose string lies past the end of the file";
    RE0704 Error "a 2DA too large for its 16-bit string offsets or 32-bit row count";
    RE0705 Error "a 2DA row the table does not have";
    RE0706 Error "a 2DA column the table does not have";
    RE0707 Error "a 2DA column added under a name the table already has";
    RE0708 Error "a 2DA string the layout cannot hold";
    RE0709 Error "a 2DA row with more or fewer cells than the table has columns";
    RE0801 Error "not a `TLK V3.0` talk table";
    RE0802 Error "a talk table truncated before its header, entry table or a string ends";
    RE0803 Error "a talk table string holding a character Windows-1252 cannot encode";
    RE0804 Error "a talk table entry with text but without the text-present flag";
    RE0805 Error "a talk table too large for its 32-bit fields";
    RE0901 Error "not a LIP V1.0 file";
    RE0902 Error "a lip-sync file truncated before its header or keyframes end";
    RE0903 Error "a lip-sync keyframe with a shape above 15";
    RE0904 Error "bytes after the last keyframe of a lip-sync file";
    RE0905 Error "more keyframes than a lip-sync file's 32-bit count holds";
    RE1001 Error "a file of the corpus could not be written";
    RE1002 Error "an install with no nwscript.nss to read the engine's declarations from";
    RE1003 Error "a corpus manifest that does not read";
    RW1001 Warning "a script whose bytecode the decompiler could not read";
    RW1002 Warning "an include library that does not compile, so its functions are not folded";
}

/// The rakata code with this id, if registered; case does not matter.
pub fn lookup(id: &str) -> Option<&'static Code> {
    nwscript_diag::codes::lookup_in(ALL, id)
}
