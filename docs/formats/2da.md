# 2DA V2.b

A 2DA is a table of strings: named columns, labelled rows, one string per cell. KotOR 1 and 2 ship every table in the binary `V2.b` layout described here, as members of `data/2da.bif`, and mods drop replacements as loose `*.2da` files in `Override/`. The text `V2.0` layout that BioWare's `2DA_Format.pdf` documents (for Neverwinter Nights) is a different encoding of the same table: `V2.b` keeps the text format's model (a row label column outside the named columns, string cells the application parses as integers or floats, a blank cell) and replaces its whitespace-separated lines with a string table and an offset per cell. No `V2.0` signature occurs anywhere in KotOR 1's `data/2da.bif` (measured: a search of the whole file), so whether the engine also loads a text table is not established.

Every fact below is tagged: **measured** means seen in bytes (vanilla members of `data/2da.bif` under `$RAKATA_GAME_K1`, or loose tables in the modded build's `Override/`, viewed with a tool that renders bytes as text, so numbers were decoded from printable offset bytes); **reference** means established from what the reference tool accepts when it loads a table and the bytes it emits when it saves one, run as a black box on that question; **inferred** means a consequence of the layout or of the public text-format document, not seen directly.

All multi-byte integers are little-endian and unsigned. There is no alignment and no padding anywhere: every section starts where the previous one ends.

## Layout

| Section | Size | Content |
| --- | --- | --- |
| Signature and version | 9 | The ASCII bytes `2DA V2.b` (with a space after `2DA`) followed by one byte 0x0A. Measured in every vanilla member and every loose table examined. The reference tool rejects any other signature or version and any other ninth byte, so a 0x0D 0x0A pair is not a variant it accepts; whether the engine accepts one is unmeasured. |
| Column names | variable | Each name as its bytes followed by one 0x09 (tab), the last included, then one 0x00 ending the list. There is no count: the number of columns is the number of tabs before the NUL. Measured (`label\tname\t2daname\twalkrate\trunrate\t\0` in `creaturespeed.2da`). Thirteen of the 153 tables inside `rims/global.rim`, and the same thirteen inside `rims/miniglobal.rim`, end each name in a NUL instead, the list still closing with one more NUL: `label\0armortype\0weapon\0…\0\0` in `appearancesndset.2da`, so the column list there ends in a double NUL. The thirteen are `appearance`, `appearancesndset`, `baseitems`, `bodybag`, `doortypes`, `genericdoors`, `heads`, `inventorysnds`, `placeableobjsnds`, `placeables`, `portraits`, `soundset` and `traps`; the other 140 in each RIM are tab-ended. Measured on every 2DA member of both RIMs, identical bytes in the two archives and in the build's copies; every other section of those tables is as in `2da.bif`. A reader takes the first name's terminator as the file's. Names are case-sensitive and the engine looks them up as written (reference: the tool keeps case and matches change files against it); a name cannot contain a tab or NUL (inferred from the terminators). |
| Row count | 4 | uint32, the number of rows. Measured (`feat.2da` carries 130, byte 0x82 then three zeros; `globalcat.2da` 1200). |
| Row labels | variable | One label per row, each as its bytes followed by one 0x09, the last included. No terminator after the list: the offset table begins right after the last tab. Measured (`0\t1\t…\t20\t` then offset bytes). In the RIM-held tables described under Column names the labels end in NUL instead (`0\x001\x00…15\0` in `appearancesndset.2da`; measured). Most vanilla labels are the decimal row index from `0`, but 20 of the 209 tables in `data/2da.bif` carry something else: names (`key0`, `key1`, … in `bindablekeys.2da`; `<abutton>`, `<bbutton>`, … in `dialogtokens.2da`; `c_drdassassin`, … in `droiddischarge.2da`; `action200`, … in `keymap.2da`; `default`, `classsel`, … in `loadscreens.2da`; `danm13`, … in `modulesave.2da`; `0a`, `0b`, … in `movies.2da`; `man28aa_security01`, … in `plot.2da`; `swingshort`, … in `swingsounds.2da`), indices starting at `1` (`categories`, `iprp_spellcost`, `stringtokens`, `visemes`), at `1001` (`visualeffects`), or a non-contiguous set (`87`, `88`, `94`, … in `combatanimations`; `217`, `218`, `219`, `362`, … in `weapondischarge`), and two tables repeat a label (`effecticon.2da`, 62 rows, rows 59, 60 and 61 all labelled `63`; `iprp_resistcost.2da`, 7 rows, rows 0 and 1 both labelled `0`). The build's loose `effecticon.2da`, `visualeffects.2da` and `weapondischarge.2da` keep those labels. Measured, on every member of `data/2da.bif`, by comparing each label with its row's index. The format imposes nothing: a label is any tab-free byte string, need not be numeric, unique, or in order; tools that add rows write whatever label the change file names (reference, and the row-label semantics of `changes.ini`). The engine addresses rows by index; the label is carried, not used for lookup (inferred from the text format, whose row numbers are "for the convenience of the person reading"). |
| Cell offsets | 2 × rows × columns | One uint16 per cell, row-major: all of row 0's cells in column order, then row 1's, and so on. Each is a byte offset into the cell strings, relative to the first byte of the strings section (not to the file start). Measured on the three-row table below. Several cells, in any rows and columns, may hold the same offset: identical strings are stored once (measured, see Sharing). |
| Cell data size | 2 | uint16, the length in bytes of the strings section that follows, NUL terminators included. Equal to that length in every vanilla table checked (57 for the three-row table below, 44 for `forceadjust.2da`). Not trustworthy in the wild: see The size field. |
| Cell strings | to end of file | NUL-terminated byte strings, back to back, in the order described under Sharing. A cell's value is the bytes from its offset up to the next NUL. The last string's NUL is the last byte of the file; nothing follows it (measured: in `2da.bif` the next member's `2DA V2.b` begins immediately after it). |

### A worked example

A vanilla four-column, three-row table (columns `label`, `name_strref`, `description_strref`, `aistate`; rows `0`, `1`, `2`) has the strings `DEFAULT`, `1116`, `1109`, `0`, `GRENADE`, `1120`, `1112`, `4`, `JEDI`, `1121`, `1114`, `5`, each NUL-terminated, in that order; its twelve offsets are 0, 8, 13, 18, 20, 28, 33, 38, 40, 45, 50, 55 and its size field is 57, the block's length. Every cell is distinct so nothing is shared. Measured; the printable offset bytes `!`, `&`, `(`, `-`, `2`, `7` and the size byte `9` are 33, 38, 40, 45, 50, 55 and 57.

## Sharing and order of the strings

In BioWare's files the strings stand in the order the cells are first met walking the table row by row and, within a row, column by column, and each distinct string stands once: a cell whose string already occurred, anywhere earlier in that walk, carries the earlier string's offset. The empty string is shared like any other. Measured on vanilla members of `2da.bif`:

- `feat.2da`: the block begins with the empty string, then `0`, `1`, `0x00`, then `ADVANCED_JEDI_DEFENSE`, `148`, `1126`, `I_jedidef02`, `4`, `55`, … The first four are the cells of row 0, a placeholder row (`FEAT_ADVANCED_JEDI_DEFENSE` is 1 in `nwscript.nss`), whose repeated values stand once; every later row's `0`, `1` and blank cells point back at them.
- The actions table (`label`, `string_ref`, `iconresref`, 39 rows): `MoveToPoint` at 0, the empty string at 12, `IR_MOVETO` at 13, `PickUpItem` at 23, `ir_pickup` at 34, …; every row's `string_ref` cell holds offset 12.
- A six-column, 21-row table of small integers has four strings in all (`2`, `0`, `4`, `6`) for 126 cells.
- `forceadjust.2da` in the build's `Override/` (11 rows × 2 columns, written by a modding tool): eleven strings for 22 cells, row 5's two cells both at offset 42 (`1`), size 44.

The reference tool emits this same layout when it saves a table (first occurrence in row-major order, one copy per distinct string, size field equal to the block length), and the repository's acceptance test for `rakata-2da` is that every vanilla table writes back identical; so this is the layout a writer produces, while the format itself forbids no other (see Tolerance).

## The size field

The size field is redundant with the offsets and the file length, and files in the wild carry wrong values that the game accepts:

- Some editors write 0 there. Five of the build's mods ship such tables (`docs/proposals/rakata.md`, the `rakata-2da` epic); the game loads them with their cells intact. Two survive in the build's `Override/` (`camerastyle.2da`, 16 columns × 9 rows, 178 bytes of strings; `pazaakdecks.2da`, 11 × 4, 115 bytes), and in both every distinct string is stored once, as BioWare stores it, but in an order that is neither row-major first occurrence nor column-major (`camerastyle.2da`'s block begins `2`, `3`, `4`, `DEFAULT`, `QAVis`, `76`, … where row 0 begins `DEFAULT`, `3.2`, `83`, …), with no unreferenced bytes. Measured. A tool that sized the strings section by this field saw every cell of those tables as empty, which is how a build once blanked every feat name without any failure at build time.
- TSLPatcher 1.2.9b leaves the field as it was in the input while appending new strings past it: in the measured case (`docs/research/tslpatcher-oracle.md`), a `baseitems.2da` whose field still said 3,894 with a 3,984-byte block, the last rows' strings lying past the declared end. Every Windows player of that mod runs the game on that file.

So the engine does not bound the strings by this field (inferred from the files it loads). The strings section is the bytes from the end of the size field to the end of the file, and a cell's value is defined by its offset into that span and the next NUL, whatever the field says. The field's correct value is the span's length, and BioWare's files carry it.

## The blank cell

In `V2.b` a blank cell is an empty string: an offset pointing at a lone NUL (measured: the actions table's `string_ref` column, `creaturespeed.2da`'s row 0 `name`, `feat.2da`'s placeholder row, and the double NUL visible in `skills.2da` and `upcrystals.2da`). The four-asterisk marker `****` that the text format and `changes.ini` use for "no value" (`2DA_Format.pdf`; `docs/formats/changes-ini.md`) does not occur as a cell in any vanilla KotOR 1 table: the only run of four asterisks in the whole of `data/2da.bif` is the key name `*****` in `keymap.2da`, and no loose table in the build's `Override/` has one (measured, by searching the bytes). The blank that BioWare's own files carry for a `****` in the text sense is therefore the empty string. The text-format document says the engine returns an empty string, or 0 for a numeric read, from a `****` cell; whether KotOR's engine treats a literal four-byte `****` cell in a binary table the same way is not measured, so such a cell is four bytes of content as far as this document can say, distinct from the empty string in the file.

## Encoding

Cells, column names and row labels are byte strings; the only bytes with meaning are the terminators (NUL for cells and the column list, tab for names and labels), so a cell may contain a tab or a newline, and nothing may contain its own terminator. Every table examined is plain ASCII, and the loose `feat.2da` has no byte above 0x7F (measured); localized text lives in `dialog.tlk` and tables carry its string references as decimal numbers (`2DA_Format.pdf`, StrRefs). The reference tool treats the bytes as ASCII. Any byte above 0x7F would be in the game's single-byte code page, Windows-1252, which agrees with ASCII below 0x80 (inferred).

The offsets and the size are 16 bits, so a strings section cannot exceed 65,535 bytes; a table whose distinct strings need more cannot be expressed in this layout (inferred). The row count is 32 bits and the number of cells is rows × columns with no stated bound.

## What a writer must reproduce

- `2DA V2.b` and 0x0A, exactly.
- Column names in table order, each with a trailing tab, then one NUL; no count. For a table read in the NUL-ended form, each name with a trailing NUL instead, then the closing NUL.
- The row count, then every label with a trailing tab (or NUL, in that form), the spelling as read. Names and labels share the terminator: no table measured mixes them.
- One uint16 offset per cell in row-major order, relative to the start of the strings.
- The strings in first-occurrence row-major order, one copy per distinct string, the empty string included, each NUL-terminated, with the size field equal to their total length, and nothing after the last NUL.

With these, every vanilla table writes back byte for byte: the 209 in `data/2da.bif` and the 153 in each of `global.rim` and `miniglobal.rim`, the 13 NUL-ended ones in each included (`rakata-2da`'s acceptance criterion; the reference tool's output does the same for `data/2da.bif`).

## What a reader must tolerate

- Column names and row labels ending in NUL rather than tab, as the tables inside `global.rim` and `miniglobal.rim` have them.
- A size field of 0, or smaller than the strings (stale after an append); the strings section runs to the end of the file regardless.
- Offsets in no particular order, strings stored several times or never shared, strings no cell points at (the stale-field file has none of these, but the format forbids none).
- Row labels that are not the row index, not numeric, or not unique.
- An offset past the end of the file, or a string without a terminating NUL before the end of file: a malformed file, to be reported rather than read as empty.
- A cell equal to `****`: four bytes of content, kept as they are.

## What could not be established

Whether the engine accepts a text `V2.0` table, a 0x0D 0x0A line end after the version, a table with zero columns or zero rows, or an offset past the end of the strings; what it does with a literal `****` cell; and whether it has any limit on rows or columns. Each needs the engine, not the files.

## Provenance

Public documents read: xoreos-docs `specs/bioware/2DA_Format.pdf` (BioWare's text `V2.0` format: the table model, the `****` blank, StrRefs, the data types). The KotOR wiki's 2DA page was unreachable from this host. Repository documents read: `docs/research/tslpatcher-oracle.md` (the stale size field, measured against TSLPatcher 1.2.9b), `docs/proposals/rakata.md` and the `rakata-2da` epic (the zero size field shipped by five mods), `docs/formats/changes-ini.md` (`****` in change files).

Reference project consulted: PyKotor, for the facts tagged "reference", stated here as what the tool accepts and emits: the exact nine-byte header it requires, the tab and NUL terminators and the implied column count, the four-byte row count, labels as free strings, uint16 offsets relative to the strings section, the uint16 size, the layout of the strings it emits, and its ASCII treatment of the bytes. Each was then checked against bytes where the tools here permitted, and the tags say which were.

Measured: on 2026-10-17, by viewing bytes through the Read tool and searching them with ripgrep: members of KotOR 1's `data/2da.bif` under `$RAKATA_GAME_K1` (the three-row table of the worked example, the actions table, a six-column integer table, `keymap.2da`, `feat.2da`, `skills.2da`, `creaturespeed.2da`, `baseitems.2da`), and loose tables written by modding tools in the build's `Override/` under `$RAKATA_BUILD` (`creaturespeed`, `forceadjust`, `featgain`, `skills`, `feat`, `globalcat`, `upcrystals`). The offset and size numbers quoted were decoded from printable offset bytes; counts of `****` and of `2DA V2.0` are whole-file searches. `nwscript.nss` from the corpus supplied `FEAT_ADVANCED_JEDI_DEFENSE = 1`.

Measured: on 2026-10-17, by the implementer of `rakata-2da`, with scripts that decode the header integers and compare each label with its row index and each string's position with its first-occurrence position: every 2DA member of KotOR 1's `data/2da.bif` (209 tables: size field equal to the strings in all, one copy per string in first-occurrence order in all, 20 with labels other than the row index, 2 with a repeated label), every 2DA member of `rims/global.rim` and `rims/miniglobal.rim` (153 each, 13 of them with NUL-ended names and labels), the 22 loose tables of the build's `Override/` (2 with a size field of 0 and strings in another order), and the loaded `feat.2da` of vanilla and the build (rows 0, 23 and 59 blank in `label` in both). In every one of these tables with rows, names and labels end in the same byte: none mixes tab and NUL.

Date: 2026-10-17.

Written by the format-scribe role, extended with the implementer's measurements; no implementer read the source.
