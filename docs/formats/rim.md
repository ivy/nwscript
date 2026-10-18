# RIM V1.0

A RIM is the read-only archive KotOR 1 and 2 use for a module's static resources. Each module under `modules/` ships as a pair, `STEM.rim` (the area: IFO, ARE, GIT and similar) and `STEM_s.rim` (blueprints, scripts, dialogs, and other instanced content), and `rims/` holds the global and menu-screen archives. The format is a stripped-down relative of ERF: one flat table maps a resource name and type to an offset and length in the same file. There is no description string block, no separate position table, no date stamp, and no encryption or compression.

All multi-byte integers are little-endian unsigned 32-bit values. Offsets are counted from the first byte of the file.

## Header

The header is 120 bytes (0x78) long.

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 4 | Signature, the ASCII bytes `RIM ` (with a trailing space). |
| 0x04 | 4 | Version, the ASCII bytes `V1.0`. |
| 0x08 | 4 | Reserved. Zero in all 246 vanilla files. |
| 0x0C | 4 | Number of entries in the resource table. |
| 0x10 | 4 | Offset of the resource table. Equal to 120 (0x78) in all 246 vanilla files, so the table begins immediately after the header. |
| 0x14 | 1 | A flag byte: 1 in the six `rims/*dx.rim` files (`chargendx`, `globaldx`, `legaldx`, `mainmenudx`, `miniglobaldx`, `subglobaldx`), 0 in every other vanilla file. In the files that carry it, members are aligned to 128 bytes instead of 4 (see Member data); what the engine makes of it is not established. |
| 0x15 | 99 | Reserved. Zero in all 246 vanilla files. |

A reader should locate the table through the offset at 0x10 rather than assuming 120, and a writer should emit 120 so the output matches BioWare's layout byte for byte.

## Resource table

The table holds the number of entries given at 0x0C, stored back to back with no padding. Each entry is 32 bytes.

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 16 | Resource name (the ResRef), ASCII, padded to 16 bytes with NUL; the bytes after the first NUL are zero in every vanilla entry. A 16-character name has no terminator. Most vanilla names are lower-case but 153 entries carry upper-case letters (`PFHA01` in `rims/chargen.rim` is one); the engine resolves names case-insensitively, so a reader compares them that way and a writer keeps the spelling it read. |
| 0x10 | 4 | Resource type, the same numeric type table ERF and KEY use (see the xoreos `torlack/basics.html` list). The upper 16 bits are zero in all 22,511 vanilla entries. |
| 0x14 | 4 | Entry index. Equal to the entry's zero-based position in the table in all 22,511 vanilla entries. |
| 0x18 | 4 | Offset of the resource's bytes, from the start of the file. |
| 0x1C | 4 | Length of the resource's bytes. |

The same name appears with several types in 198 of the 246 vanilla files (the `dan13_ahlan` entries in `danm13_s.rim` are one example); the pair (name, type) is the key, not the name alone. No vanilla file repeats a (name, type) pair, so the engine's tie-break for a duplicate is unmeasured; a reader that builds a map must still decide on first-wins or last-wins and say which. Entries are in offset order in every vanilla file. 153 entries have length zero, all in `globaldx.rim`, `miniglobaldx.rim` and `mainmenudx.rim`; four files (`legal.rim`, `legaldx.rim`, `subglobal.rim`, `subglobaldx.rim`) have no entries at all and end with the table.

## Member data

Each entry's bytes lie at its offset for its length, stored verbatim: no per-member header, compression or encryption. Members are not contiguous. The layout is one rule with one parameter, the alignment: 4 when the flag byte at 0x14 is 0, 128 when it is 1. After the table come 8 zero bytes and then zero padding to the next multiple of the alignment; after each member come 16 zero bytes and then zero padding to the next multiple of the alignment; and the file ends with 10 zero bytes after the last member. With alignment 4 (the 240 vanilla files whose flag is 0), the first member begins at 128 + 32 × count and the gap between members is 16 to 19 bytes. With alignment 128 (the six `dx` files), the first gap is 8 or 72 bytes depending on where the table ends (`chargendx.rim`: 120 + 30 × 32 + 8 = 1088, padded to 1152 = 9 × 128), the gap between members is 16 to 143 bytes (24 to 136 observed), and zero-length members sit 128 bytes apart. `rakata-archive`'s writer applies this rule and reproduces all 246 vanilla files byte for byte, which is the evidence for it. Nothing in the format requires any of this: a reader follows the offsets and assumes neither adjacency nor the gap size.

## Differences from ERF

Compared with the ERF layout in xoreos's `torlack/mod.html` and BioWare's `ERF_Format.pdf`:

- The header is 120 bytes instead of 160 and carries no string count, string size, string offset, build year, build day, or description string reference.
- There is no localized description block.
- The key list and the resource list are merged. An ERF key is 24 bytes (name, index, 16-bit type, 16-bit reserved) and points at a separate 8-byte (offset, length) record; a RIM entry is 32 bytes and carries name, 32-bit type, index, offset and length together.
- The type field is 32 bits wide rather than 16 bits plus a reserved 16.

## What a writer must reproduce

- The signature `RIM ` and version `V1.0`, exactly.
- Zero in the reserved dword at 0x08 and the 100 reserved bytes at 0x14.
- A table offset of 120, entries of 32 bytes each in offset order, names NUL-padded to 16 bytes with the spelling read, and entry indices equal to the entry's position.
- Offsets that are absolute file positions pointing at the member bytes, with lengths equal to the member sizes.
- The flag byte as read, and the padding it implies: 8 zero bytes after the table then alignment, 16 zero bytes then alignment after each member, 10 zero bytes at the end, with the alignment 4 or 128 by the flag.

Whether the engine tolerates departures from this (a table offset other than 120, non-zero reserved bytes, non-sequential indices, members with no padding) is not established and should not be relied on until measured.

## What could not be established

Why the `dx` files align members to 128 and what the engine does with the flag byte that marks them; how the engine resolves a duplicate (name, type) pair; and what the engine rejects on load. Each of these needs the engine, not the files.

## Provenance

Public documents read: xoreos-docs `specs/torlack/mod.html` (the ERF layout this document contrasts with) and the file list of `specs/bioware/`, which has no RIM document. The KotOR wiki's RIM page was unreachable from this host.

Reference projects read: none. The clean-room hook denied both a Glob and a Grep of the PyKotor checkout, so every RIM-specific fact above comes from the vanilla game files named in the previous section, viewed through the Read tool, together with the ERF document and general knowledge of the format; nothing was derived from any implementation's source.

Measured: on 2026-10-17, every count above (246 files, 22,511 entries, the gap and trailing-byte rules, the flag byte, the upper-case names, the zero-length and empty cases) comes from parsing the header and table of every `modules/*.rim` and `rims/*.rim` in the vanilla KotOR 1 install under `$RAKATA_GAME_K1` with a short Python script. The 128-byte alignment of the flagged files was found by the implementer the same day, from the first differences the writer produced against them (`chargendx.rim` and `mainmenudx.rim` at offset 144, `globaldx.rim` and `miniglobaldx.rim` at 176) and the member offsets in their tables; `crates/rakata-archive/tests/vanilla.rs` writes every vanilla RIM back and asserts it identical (`RAKATA_GAME_K1=… cargo test -p rakata-archive --test vanilla -- --nocapture`).

Date: 2026-10-17.

Written by the format-scribe role; no implementer read the source.
