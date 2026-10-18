# ERF V1.0 in KotOR

The ERF layout is documented by xoreos (`specs/torlack/mod.html`) and BioWare (`specs/bioware/ERF_Format.pdf`); this page records only what those documents leave open or describe for Neverwinter Nights and KotOR does differently, measured on the vanilla KotOR 1 install. KotOR 1 ships 128 ERF-family files: `patch.erf` and the four `texturepacks/*.erf` with the signature `ERF V1.0`, and 123 `lips/*.mod` with `MOD V1.0`; `modules/` holds RIMs only. A saved game's outer file and each module's state within it carry `SAV V1.0`; `HAK V1.0` is Neverwinter Nights' and appears in no KotOR file, but the reader accepts it.

All multi-byte integers are little-endian unsigned 32-bit values unless a width is given. Offsets are counted from the first byte of the file.

## Header fields

| Offset | Content | Measured |
| --- | --- | --- |
| 0x08 | Number of description strings | 0 in all 128 files. |
| 0x0C | Size of the string block in bytes | 0 in all 128 files, so whether the figure counts the 8-byte header of each string is unmeasured; `rakata-archive` writes the block's whole size, including the headers, and reads the strings by their count, not by this figure. |
| 0x14 | Offset of the string block | 160 in all 128 files: the block begins at the end of the header. |
| 0x18 | Offset of the key list | 160 + the string block size in all 128 files. |
| 0x1C | Offset of the resource (position) list | key list offset + 24 × count in all 128 files. The 8-bytes-per-resource blank block the xoreos document reports between the two lists in Neverwinter Nights `MOD` files is absent from every KotOR file, `MOD V1.0` included. |
| 0x20 | Build year, from 1900 | 103 in 127 files, 104 in one (`lips/danm14ae_loc.mod` is 103 day 309; `patch.erf` is 104 day 47). |
| 0x24 | Build day, from 0 | 309 in 114 files, 145 in 13, 47 in 1. |
| 0x28 | Description string reference | Not a strref: 62 distinct values across the 128 files, among them 0 (30 files), 4294967295 (1 file), 110 (11), and values such as 1769352801 that are ASCII when read as bytes. The field is uninitialised memory in BioWare's writer, and a writer that wants a byte-identical file writes back what it read. |
| 0x2C | 116 reserved bytes | Zero in all 128 files. |

## Key list

Each 24-byte key holds the name (16 bytes, NUL-padded, the bytes after the first NUL zero), the entry's index (equal to its position in all 29,756 vanilla entries), the type (16 bits) and a reserved 16 bits (zero in all entries). 9,247 of the 29,756 names carry upper-case letters, nearly all of them in the texture packs; the engine compares names without regard to case, and a writer keeps the spelling read. No vanilla file repeats a (name, type) pair.

## Member data

Members lie at their offsets, in table order, contiguous: in all 114 non-empty files the first member begins immediately after the position list and each member immediately after the previous one, with no padding and no trailing bytes. The 14 empty files (`lips/*.mod` such as `legal.mod`) are a 160-byte header and nothing else. `rakata-archive` writes this layout and reproduces all 128 files byte for byte.

## Provenance

Public documents read: xoreos-docs `specs/torlack/mod.html` in full. BioWare's `ERF_Format.pdf` could not be rendered on the build host (no PDF tooling), so every figure above is measured, not read.

Reference projects read: none.

Measured: on 2026-10-17, by a Python script over the header, key list and position list of every `patch.erf`, `lips/*.mod` and `TexturePacks/*.erf` in the vanilla KotOR 1 install under `$RAKATA_GAME_K1`, and by `crates/rakata-archive/tests/vanilla.rs`, which writes every one back and asserts it identical.

Written by the implementer of `rakata-archive`, from the files alone.
