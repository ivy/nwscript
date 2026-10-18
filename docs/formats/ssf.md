# SSF V1.1: KotOR's sound sets

A sound set names, for each thing a creature can say in combat and exploration, the talk-table entry to show and play. A creature's `.utc` points into `soundset.2da` (`SoundSetFile`), and the row names the `.ssf`. BioWare's `SSF_Format.pdf` (xoreos-docs, `specs/bioware/`) documents Neverwinter Nights' `SSF V1.0`; KotOR 1 ships `SSF V1.1`, which is a different and simpler layout. Everything below is measured from the 106 sound sets in KotOR 1's `data/templates.bif` (the only place vanilla holds them), the seven K1CP ships, and TSLPatcher run as a black box.

## Layout

All integers little-endian.

| Offset | Size | Value |
| --- | --- | --- |
| 0 | 8 | `SSF V1.1` |
| 8 | 4 | offset of the entry table: 12 in all 113 files |
| 12 | 4 × N | one strref per slot; `0xFFFFFFFF` is none |

There is no entry count and no per-entry sound resref: where NWN's V1.0 has a count, a table of offsets and a data table of `ResRef[16]` + strref per entry, V1.1 has the table offset alone followed by the strrefs. The number of slots is the rest of the file divided by four. The sound to play is the talk-table entry's own voice-over resref (`dialog.tlk`'s `SoundResRef`), not a field of the SSF: Bastila's slot 0 is strref 36850, whose sound is `p_bastila_bat1` and whose text is "You cannot win!".

Sizes: 98 vanilla files and all seven of K1CP's are 172 bytes (40 slots); 8 vanilla files are 208 bytes (49 slots: `darkjedi`, `drdassassin`, `drdmkone`, `drdprobe`, `katarn`, `kinrath`, `player`, `sithtroop`). A writer keeps the slot count it read.

## Slots

Slots 0 to 27 carry KotOR's sound set; the names are those ChangeEdit and TSLPatcher use in a `[SSFList]` section, in slot order, and the meaning is read from the vanilla resrefs (`p_bastila_*`, `p_carth_*`) and the talk table's text.

| Slot | `changes.ini` name | Vanilla resref suffix and meaning |
| --- | --- | --- |
| 0–5 | `Battlecry 1` … `Battlecry 6` | `bat1`…`bat6`, battle cries |
| 6–8 | `Selected 1` … `Selected 3` | `slct1`…`slct3`, answering selection |
| 9–11 | `Attack 1` … `Attack 3` | `atk1`…`atk3`, attack grunts |
| 12–13 | `Pain 1`, `Pain 2` | `hit1`, `hit2`, pain grunts |
| 14 | `Low health` | `low` |
| 15 | `Death` | `dead` |
| 16 | `Critical hit` | `crit` |
| 17 | `Target immune` | `tia` ("I am doing no damage!") |
| 18 | `Place mine` | `lmin` |
| 19 | `Disarm mine` | `dmin` |
| 20 | `Stealth on` | `stlh` |
| 21 | `Search` | `srch` |
| 22 | `Pick lock start` | `block` |
| 23 | `Pick lock fail` | `flock` |
| 24 | `Pick lock done` | `slock` |
| 25 | `Leave party` | `sprty` |
| 26 | `Rejoin party` | `rprty` |
| 27 | `Poisoned` | `pois` |

Slots 28 and up are `0xFFFFFFFF` in every 172-byte file but two: `c_terantank` and `n_ithorian` set slot 33 to a "Selected" entry, which is NWN's index for "Selected". The eight 208-byte files each set one slot, 18, to a death sound (`DarkJedi_die`, "DarkJedi Death Sound"), which is NWN's index for "Death" (KotOR's slot 18 is "Place mine"): these files keep NWN's numbering inside a V1.1 file. Which slot the engine reads for them is not established; a reader and writer carry every slot as it stands.

TSLPatcher's `[SSFList]` writes strrefs by the names above into slots 0 to 27 and leaves the others as they were; it knows no name for slots 28 and up (the strings beside the names in its executable continue `Unknown(29)` … `Unknown(34)`, which no mod writes).

## Provenance

Public documents read: BioWare's `SSF_Format.pdf` and `KeyBIF_Format.pdf` from xoreos-docs (`specs/bioware/`), as text through `pdftotext`, for NWN's V1.0 layout and the KEY/BIF layout used to take the files out of the install.

Measured: the 106 `.ssf` members of a vanilla KotOR 1 install's `data/templates.bif`, listed through `chitin.key`, and the seven `.ssf` files in the K1 Community Patch v1.10.0's `tslpatchdata`, read with a script written from those documents; the talk-table entries each strref names, from the install's `dialog.tlk`. The slot each `changes.ini` name writes was measured by running TSLPatcher 1.2.10b1 (SHA-256 `013655cd…`) under Wine on a hand-made sound set of 40 empty slots with each of the 28 names given a distinct strref, 1000 to 1027: slot `i` came back `1000 + i` and slots 28 to 39 unchanged (`docs/research/2026-10-17-tslpatcher-semantics.md`). The 28 names and the `Unknown(29)`… strings were listed with `strings` over that executable, as its version resource was read before; nothing was disassembled.

Reference projects read: none. Written by an implementer.
