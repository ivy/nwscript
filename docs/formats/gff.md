# GFF V3.2 in KotOR

The GFF layout is documented by BioWare (`specs/bioware/GFF_Format.pdf` and `LocalizedStrings_Format.pdf` as xoreos publishes them); this page records what those documents leave open, what KotOR adds, and how BioWare's own tools laid out the files the game ships, measured on the vanilla KotOR 1 install and on modshop's modded build. The documents describe a file format; they do not say in what order a writer fills the arrays, and a writer that wants a byte-identical file needs exactly that.

All multi-byte integers are little-endian 32-bit values unless a width is given. Offsets within a section are counted from the section's first byte.

## What KotOR adds to the document

Three field types beyond the document's 0 to 15, all stored in the field data block, each counted in 1,137, 1,570 and 0 of the 1,543,031 fields in the vanilla `modules/*.rim` and `rims/*.rim` members:

| Id | Type | Data |
| --- | --- | --- |
| 16 | Orientation | Four 32-bit floats, 16 bytes. |
| 17 | Vector | Three 32-bit floats, 12 bytes. |
| 18 | StrRef | A size dword, 4, then the strref dword. No vanilla module member carries one; the size value is what the KotOR wiki's GFF page reports and `rakata-gff` writes. |

The sizes of 16 and 17 are confirmed by the data block adding up: with them, every one of the 9,774 module members has a data block exactly as long as its fields' values laid end to end.

Field types 1 (CHAR), 6 (DWORD64), 7 (INT64) and 9 (DOUBLE) appear in no vanilla module member either; the reader and writer handle them as the document says.

## Sections

The six sections follow the header in the document's order, each directly after the last, and the file ends with the list indices, in every vanilla file and every file of the build. A reader still locates each by its header offset.

## How BioWare's tool fills the arrays

The data block, the label table and the list indices follow one rule each; the struct array, the field array and the field indices array show that the files were built incrementally, and a lossless model keeps what it read rather than recomputing it.

**Data block.** Values lie in field-array order, each directly after the last, with no sharing and no gap, in all 9,774 vanilla module members. The size word of a `CExoLocString` equals 8 plus the sum of 8 plus each substring's length.

**Label table.** Labels appear in order of first use in field-array order, each once, in all 9,774 members; no label has bytes other than zero after its first NUL. The field array is the authority on order; the table is derived.

**Field array.** For a file written in one pass the order is depth-first: a struct's fields in order, and when a field is a `Struct` its struct's fields follow it before the parent's next field; when it is a `List`, each element's fields in turn (`m13aa.are`: the `Map` struct field is field 6, its eleven fields are 7 to 17, the root continues at 18). But the order is a history, not a traversal: in every vanilla DLG the `SoundExists` field of every entry and reply is at the very end of the field array, after `StartingList`'s fields, with its label last in the table, because a later tool pass appended it to each struct. A struct's field-index list is in ascending order in every vanilla member, so the field array order is the only order the model has to keep.

**Struct array.** Depth-first as well for a one-pass file: the root, then each struct as its field (or its list element) is reached (`m13aa.are`: `Map` is struct 1, the twenty-one `Rooms` elements are 2 to 22; `dan13_ahlan.dlg`: entry 0 is struct 1, its `AnimList` element 2, its `RepliesList` link 3, entry 1 is 4). 94 of the 9,774 members depart from this, among them every PTH: a path node's `Conections` link structs are allocated before the node's own fields are written (`m13aa.pth`: node struct 1 has fields 4 to 7; its links, structs 2 and 3, have fields 2 and 3), which is construction order in the tool that wrote them.

**Field indices array.** One list per struct with two or more fields (a struct with one field holds the field index in `DataOrDataOffset` itself, as the document says; no vanilla module member has a struct with no fields). The lists are in struct order, each after the last, in 9,745 members. In 29 members the array holds stale lists: `m13aa.git`'s root has 12 fields and its list at 8408, the last 48 bytes of a 9,324-byte array, while bytes 0 to 8 hold `[0, 1]`, 44 to 56 hold `[0, 1, 11]`, 584 to 600 hold `[0, 1, 11, 144]`, and so on: each earlier list is the root's list as it was before one more field (`Creature List`, `Door List`, ...) was added. The tool grew the root one list at a time, reallocating its index list at the end of the array each time and leaving the old one in place. 256 of the 9,324 bytes are stale. The engine reads only what the structs point at.

**List indices array.** One block per `List` field, a count dword then the struct indices, in field order, each after the last, in 9,763 members; 11 members hold stale blocks the same way (`m13aa.git`'s `TriggerList`, grown ten times, left nine old blocks totalling 180 bytes).

A writer that reproduces BioWare's files therefore keeps, per struct and per field, its array position as read, and per struct and per list its index offset and count as read, and writes the two index arrays back from the bytes it read, overwriting the live lists in place. A struct or field added since the read goes after every one that was read, and an index list that grew is appended at the end of its array with the old list left in place, which is the behaviour the files show BioWare's own tool had. A file built from scratch gets the depth-first one-pass layout. `rakata-gff` does this and reproduces every GFF of the vanilla install byte for byte: 11,143 files and members across the BIFs, RIMs, ERFs and Override, every one identical (`rakata conformance --game $RAKATA_GAME_K1 --only gff`).

## Values

A `CExoLocString`'s strref is `0xFFFFFFFF` for none in 34,323 of the vanilla module members' 77,447 locstrings; a writer keeps the word as read. Every substring in a vanilla module member has string id 0 (English, masculine or neutral): 170 substrings, all of them in IFO `Mod_Name` fields and a few DLG `Text`s. Floats are written from their bits; no float is reformatted. A `CResRef` is a size byte (16 at most) then the characters; a `CExoString` a size dword then the bytes, in the game's code page.

## What other writers do

Measured over modshop's build (21,146 GFF members, 192 mods' worth of files), three layouts differ from BioWare's and read back to the same data. Each is named in `conformance/rakata-known.toml` by the clause of the layout line that recognises it:

- 53 UTC and UTT files have a data block that ends with four or eight zero bytes no value covers, counted in the header's `FieldDataCount`. The writer is a community blueprint editor with no signature in the file.
- 1 UTI, from KotOR Tool (it carries `KTInfoVersion`, `KTInfoDate` and `KTGameVerIndex` fields), has a `CExoLocString` whose size word disagrees with the substrings that follow it. The engine reads the substrings by count; `rakata-gff` writes the size the substrings add up to.
- 1 DLG has two `WORD` fields (`CameraAnimation`, `CameraID` of one entry) whose data dword is `0xFFFFFFFF`: a -1 written at dword width into a sixteen-bit type. `rakata-gff` writes the low sixteen bits.

No file in the build is different after re-read, and none fails to read.

## What could not be established

What the engine does with the stale index bytes, a `StrRef` field whose size word is not 4, a struct with no fields (its `DataOrDataOffset` is unmeasured; `rakata-gff` writes 0), or a `CExoLocString` whose size word is wrong: each needs the engine, not the files. Whether the engine's loader tolerates sections in another order or apart is likewise unmeasured.

## Provenance

Public documents read: BioWare's `GFF_Format.pdf`, `LocalizedStrings_Format.pdf` and `CommonGFFStructs.pdf` from xoreos-docs `specs/bioware/`, rendered with `pdftotext`. The KotOR wiki's GFF page is cited from memory for the `StrRef` layout only; no file measured carries one.

Reference projects read: none.

Measured: on 2026-10-17, by a Python script over the header and the six sections of every GFF member of the vanilla `modules/*.rim` and `rims/*.rim` (9,774 members; the counts above), by `crates/rakata-gff/tests/vanilla.rs` (every GFF in every RIM, MOD and BIF of the install written back and asserted identical, counted by file type) and by `rakata conformance` over the vanilla install and the build (`$RAKATA_GAME_K1`, `$RAKATA_BUILD`), whose layout line for each non-identical file is what the variants above are recognised by.

Written by the implementer of `rakata-gff`, from the documents and the files alone.
