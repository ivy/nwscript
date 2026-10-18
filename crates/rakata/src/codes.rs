//! The registry of the data toolset's diagnostic codes, declared through
//! `nwscript-diag`'s mechanism under a prefix of their own, so a program
//! reading either binary's output tells them apart: `RE` an error, `RW` a
//! lint, each with a long explanation in `explain/CODE.md` beside this
//! crate, which `rakata explain CODE` prints.
//!
//! Each crate or command family holds a block of a hundred codes, so
//! parallel work never collides; `AGENTS.md` lists the blocks.

pub use nwscript_diag::codes::Code;

nwscript_diag::codes! {
    dir "explain";
    program "rakata";
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
    RE0305 Error "a write the read-back did not find: the target holds other bytes";
    RE0306 Error "a file or capsule the installer could not write or read back";
    RE0307 Error "a file an install operation needs that nothing holds";
    RE0308 Error "a file an install operation edits that does not parse";
    RE0309 Error "an install operation that cannot apply as written";
    RE0310 Error "a script of [CompileList] that does not compile";
    RE0311 Error "a memory token read during the install that has no value";
    RE0312 Error "a file [Settings] requires that is not installed";
    RW0305 Warning "a `File#` entry skipped because its destination holds the file";
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
    RE1101 Error "a GFF that is not a dialogue";
    RE1102 Error "a node key that matches no node of the dialogue";
    RE1103 Error "a node key that matches more than one node of the dialogue";
    RE1104 Error "a node key that does not parse";
    RE1105 Error "a dialogue several modules hold, with no module named";
    RE1106 Error "a dialogue link or start between nodes it cannot join, or past the end of its list";
    RE1107 Error "a dialogue reordering that is not a permutation of the list";
    RE1108 Error "text for a new dialogue node holding a character Windows-1252 cannot encode";
    RW1101 Warning "a link to a node the dialogue does not have";
    RW1102 Warning "a script or condition no layer of the install holds";
    RW1103 Warning "a script whose bytecode the decompiler could not read back";
    RW1104 Warning "a voiced node whose voice-over no layer or streamwaves/ holds";
    RW1105 Warning "a node whose strref lies past the end of the talk table";
    RW1106 Warning "a condition reading a global globalcat.2da does not declare";
    RE1201 Error "a directory that is not a saved game";
    RE1202 Error "a module the save holds no state for";
    RE1203 Error "a save member lacking a field the game writes";
    RE1204 Error "no creature in the save carries the tag";
    RW1201 Warning "local variables shown by index: the scripts that name them were not found";
    RE1205 Error "a global the install's globalcat.2da does not declare, set without --declare";
    RE1206 Error "a `save set` assignment that does not parse or does not fit the global's type";
    RE1207 Error "a party member the save holds no AVAILNPC for";
    RE1208 Error "a party the game cannot take: a member named twice, or more than two";
    RE1209 Error "a new save's directory that exists and is not empty, or is the save read";
    RE1210 Error "a file of the new save could not be written";
    RW1202 Warning "a global written that the install's globalcat.2da does not declare";
    RE1301 Error "an archive member the archive does not hold";
    RE1302 Error "a `gff set` assignment that does not parse as its field's type";
    RE1303 Error "a `gff set` naming a field that does not exist, without a type to create it";
    RE1304 Error "a strref the talk table does not hold";
    RE1305 Error "an install without a dialog.tlk";
    RE1306 Error "an output file could not be written";
    RE1601 Error "a diff of a directory against a file";
    RW1601 Warning "a resource that does not read as its format, compared by hash";
    RW1602 Warning "a file of a tree the diff could not read";
    RE1401 Error "a module copy of a replaced resource that differs from the hash the mod was written against";
    RE1402 Error "Override already holds a copy of a resource the mod replaces";
    RE1403 Error "the module holds no copy of a resource the mod replaces";
    RE1404 Error "a global globalcat.2da already declares";
    RE1405 Error "a 2DA row label matching several rows, or a new row's label the table already carries";
    RE1406 Error "a blueprint field set to a value of another type";
    RE1407 Error "a dialogue write refused: a node or link it adds or changes refers to what the install lacks";
    RE1408 Error "a mod's script that does not compile";
    RE1409 Error "a file the mod writes could not be written";
    RE1410 Error "a voice line whose span its take's transcript does not hold";
    RE1411 Error "two voice lines with one resref";
    RE1412 Error "a transcript or a set of mouth cues that does not read";
    RE1413 Error "a voice-over for streamwaves/ that is not a RIFF WAVE file";
    RE1414 Error "bytes written as NCS that do not decode";
    RE1501 Error "the index database could not be read or written";
    RE1502 Error "an index database of another schema version";
    RE1503 Error "a tree the index database does not hold";
    RE1504 Error "an index query SQLite rejects";
    RE1505 Error "a changed-paths file that could not be read";
    RW1501 Warning "a changed path the tree does not hold";
    RW1502 Warning "a file of the tree that could not be read";
    RW1503 Warning "a tree without nwscript.nss: calls name routines by number";
    RW1504 Warning "files new to the tree that the changed list does not name";
}

/// The rakata code with this id, if registered; case does not matter.
pub fn lookup(id: &str) -> Option<&'static Code> {
    nwscript_diag::codes::lookup_in(ALL, id)
}
