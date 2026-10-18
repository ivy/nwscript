# TLK V3.0

The talk table, `dialog.tlk` at the root of a KotOR 1 or 2 install, holds every string the game shows, addressed by strref: a 32-bit index into the table, `0xFFFFFFFF` for none. BioWare's Talk Table document (xoreos-docs, `specs/bioware/TalkTable_Format.pdf`) gives the layout, and this document records what the game's own file confirms, where it is silent and where another writer differs. Each line carries flags saying what is present, a sound resref for its voice-over, a sound length, and the position of its text.

All multi-byte integers are little-endian; offsets in the entry table are counted from the string data offset in the header, not from the start of the file. Text is Windows-1252, one byte per character, with no terminator.

## Header

The header is 20 bytes.

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 4 | Signature, the ASCII bytes `TLK `. |
| 0x04 | 4 | Version, `V3.0`. The one version KotOR 1 ships; the document says earlier versions have no sound-length field. |
| 0x08 | 4 | Language id: 0 English, 1 French, 2 German, 3 Italian, 4 Spanish, 5 Polish, 128 Korean, 129 Chinese Traditional, 130 Chinese Simplified, 131 Japanese. 0 in the vanilla file. |
| 0x0C | 4 | Number of entries. 49,265 in the vanilla file. |
| 0x10 | 4 | Offset of the string data, from the start of the file. Equal to 20 + 40 × count (1,970,620) in the vanilla file, so the string data begins immediately after the entry table. |

A reader locates the string data through the offset at 0x10; a writer emits 20 + 40 × count.

## Entry table

The table begins at offset 20 and holds one 40-byte entry per strref, back to back; strref N is entry N.

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 4 | Flags: `0x1` text present, `0x2` sound resref present, `0x4` sound length present. The vanilla file holds three values and no others: `0x7` in 48,560 entries (a line with text), `0x6` in 398 (a line without text), and `0x8000` in 307 (a line without text; the document does not name this bit). |
| 0x04 | 16 | Sound resref, NUL-padded; the bytes after the first NUL are zero in every vanilla entry. Blank in 16,105 of the 48,958 entries whose flags set `0x2`, so the flag does not mean the field is filled. No entry has a resref with the flag clear. |
| 0x14 | 4 | Volume variance. Zero in every vanilla entry; the document marks it unused. |
| 0x18 | 4 | Pitch variance. Zero in every vanilla entry; the document marks it unused. |
| 0x1C | 4 | Offset of the text from the string data offset. |
| 0x20 | 4 | Length of the text in bytes. |
| 0x24 | 4 | Sound length in seconds, an IEEE 754 single. 0.0 in every vanilla entry, with the flag set or clear; the game takes a line's duration from the voice-over file, not from here. |

## String data

The texts lie back to back in strref order with no separators: the first entry with text is at offset 0, and each following text begins where the previous one ends. No vanilla text is out of order, overlaps, or leaves a gap, and the string data ends at the end of the file (3,423,826 bytes).

An entry whose text flag is clear has offset 0 and length 0, in all 705 vanilla cases. An entry with the text flag set and an empty text does not occur in the vanilla file.

No text in the vanilla file or the build's holds a byte at or above 0x80, so neither file exercises the code page. KotOR's text is taken as Windows-1252, the code page of the Windows builds the game ships for, and that choice is untested by these two files. Five bytes of that code page are undefined (0x81, 0x8D, 0x8F, 0x90, 0x9D); a reader that decodes to text must map them so that they encode back to the same byte.

## Another writer's layout

The talk table of the modded build under `$RAKATA_BUILD` (49,361 entries, written by the installer that assembled the build) differs from BioWare's in two ways and reads back to the same table: every entry's flags are `0x7`, including the 701 whose text is empty, and those empty texts sit at the running offset (the end of the previous text) with length 0, where BioWare clears the flag and writes offset 0. One rule reproduces both files byte for byte: an entry with the text flag set is written at the running offset with its length, empty or not, and an entry with the flag clear at offset 0 with length 0. The build's file is therefore identical after a round trip, and no variant is recorded for it.

## What a writer must reproduce

- The signature `TLK ` and version `V3.0`, the language id, the count, and a string data offset of 20 + 40 × count.
- Each entry's flags, resref field, variances and sound length as read.
- The texts back to back in strref order, each entry's offset and length pointing at its own; offset 0 and length 0 where the text flag is clear.

`rakata-tlk`'s writer applies this and reproduces both files above byte for byte, which is the evidence for it (`RAKATA_GAME_K1=… RAKATA_BUILD=… cargo test -p rakata-tlk --test vanilla -- --nocapture`, and `rakata conformance --only tlk` over each tree).

## What could not be established

What the engine makes of the `0x8000` flag; whether it reads a text whose flag is clear but whose length is not zero; whether it honours a string data offset other than 20 + 40 × count; and whether a sound length other than 0.0 changes anything in KotOR 1. Each needs the engine, not the file.

## Provenance

Public documents read: xoreos-docs `specs/bioware/TalkTable_Format.pdf` (the header, entry and flag layout, the language ids) and `specs/bioware/LocalizedStrings_Format.pdf` (the strref range, the alternate-table bit, the string id as language × 2 + gender, and the procedure for resolving a `CExoLocString`), both read as text through `pdftotext`.

Reference projects read: none.

Measured: on 2026-10-17, every count above comes from parsing the header and every entry of `dialog.tlk` in the vanilla KotOR 1 install under `$RAKATA_GAME_K1` and in the modded build under `$RAKATA_BUILD` with a short Python script, then from `crates/rakata-tlk/tests/vanilla.rs` and `rakata conformance --only tlk`, which write both files back and compare them.

Date: 2026-10-17.

Written by the implementer of `rakata-tlk`, from the public documents and the files alone.
