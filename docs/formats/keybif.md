# KEY V1 and BIFF V1 in KotOR

The KEY and BIF layouts are documented by xoreos (`specs/torlack/key.html`, `specs/torlack/bif.html`) and BioWare (`specs/bioware/KeyBIF_Format.pdf`); this page records what the xoreos pages leave as unknown, one place where they disagree with the files, and the layout facts a writer needs, all measured on the vanilla KotOR 1 install: one `chitin.key` naming 26 BIFs under `data\`, 25,836 resources in all.

All multi-byte integers are little-endian. Offsets are counted from the first byte of the file.

## KEY header

The header is 64 bytes. Beyond the fields the xoreos page names (signature `KEY V1  `, BIF count, resource count, file table offset, key table offset), the forty bytes from 0x18 are: at 0x18 the build year from 1900 (103), at 0x1C the build day from 0 (309), and 32 zero bytes. The file table is at 64 in the vanilla file.

## KEY file table

Each 12-byte entry is the BIF's size, the offset of its name, the name's length (16 bits) and the drive flags (16 bits). The drive flags are 1 in all 26 entries. The name length includes the terminating NUL: `data\2da.bif` is stored as 13 bytes, the twelve characters and a NUL, and the length field reads 13. The xoreos page says the length excludes the terminator; the file says otherwise. Names use `\` as the separator and name a path relative to the install, which the engine resolves without regard to case (the folder is `data`; a Windows install may spell it otherwise). The sizes recorded match the BIF files' sizes on disk for all 26.

The names are stored back to back immediately after the file table, in table order, and the key table begins immediately after the last name's NUL. The key table ends the file.

## KEY key table

Each 22-byte entry is the name (16 bytes, NUL-padded, the bytes after the first NUL zero), the type (16 bits) and the resource id (32 bits, at offset 18, unaligned as the xoreos page warns). The resource id's upper 12 bits index the file table and its lower 20 bits index the BIF's table. All 25,836 names are lower-case. Entries are grouped by BIF in file-table order, and within a BIF the indices run 0, 1, 2, … in table order, so the key table is the concatenation of the BIFs' tables with names attached. No (name, type) pair repeats.

## BIF

The header is 20 bytes: signature `BIFFV1  `, variable resource count, fixed resource count, and the offset of the variable resource table. The fixed resource count is 0 in all 26 files, so the fixed resource table's layout is unmeasured and `rakata-archive` refuses a BIF that counts any (`RE0208`). The table is at 20 in all 26 files.

Each 16-byte table entry is the resource id, the member's offset, its length and its type (32 bits, the upper 16 zero in all 25,836 entries). The resource id equals the KEY's for that member in every entry: upper 12 bits the BIF's position in the KEY's file table, lower 20 bits the entry's position in this table. The types agree with the KEY's in every entry.

Members lie in table order, contiguous: the first begins immediately after the table (20 + 16 × count) and each after the previous, with no padding and no trailing bytes, in all 26 files. The largest, `data/models.bif`, is 954,052,730 bytes holding 6,444 members.

## Reading

Listing an archive means reading its header and tables and nothing else. `rakata-archive`'s `Source` offers the file read whole or memory-mapped; the mapped form is the one place in the workspace that uses `unsafe` (a mapping changes under a slice if another process rewrites the file), and this measurement is what earned it. Over all 401 archives of the vanilla install (2,015 MB: the KEY, 26 BIFs, 128 ERFs, 246 RIMs), opening each and listing it, in a release build on the build host on 2026-10-17:

| Page cache | Read whole | Mapped |
| --- | --- | --- |
| Cold (every archive evicted with `dd iflag=nocache` first) | 866–955 ms | 58–74 ms |
| Warm | 406–418 ms | 3.9–4.9 ms |

Listing `data/models.bif` mapped takes 40 µs and copies no member (`tests/vanilla.rs`, `listing_the_largest_bif_copies_no_member`, which checks that every member's bytes lie inside the mapping). The proposal's target of a cold start under a hundred milliseconds is met only by the mapped source.

## Provenance

Public documents read: xoreos-docs `specs/torlack/key.html` and `specs/torlack/bif.html` in full. BioWare's `KeyBIF_Format.pdf` could not be rendered on the build host (no PDF tooling), so the build-date fields, the fixed resource count and the drive flags are named here from the bytes and the xoreos pages' field positions, not from BioWare's names for them.

Reference projects read: none.

Measured: on 2026-10-17, by a Python script over `chitin.key` and the 26 BIFs of the vanilla KotOR 1 install under `$RAKATA_GAME_K1`, and by `crates/rakata-archive/tests/vanilla.rs`, which writes each back and asserts it identical and prints the timings above with `--nocapture`.

Written by the implementer of `rakata-archive`, from the files alone.
