# LYT and VIS

An area's 3D world is a set of room models placed side by side. The room layout, `AREA.lyt`, places each room's model in the area and names the door hooks, the points where the area's doors stand; in the minigame areas it also places the swoop track pieces and obstacles. The room visibility file, `AREA.vis`, lists for each room the other rooms drawn while the camera is in it. Both are ASCII text, named for the area (`m01aa.lyt` for the Endar Spire's first area, module `end_m01aa`), and both live in the BIFs, not in the module capsules.

Every fact below is tagged **measured** (seen in the bytes of a vanilla file, or in a comparison between files) or **inferred**. The measurements are over every LYT and VIS a KotOR 1 install holds, read through `rakata-game` from `$RAKATA_GAME_K1`; the census that makes them is `crates/rakata-lyt/tests/vanilla.rs`, run as `RAKATA_GAME_K1=… cargo test -p rakata-lyt --test vanilla -- --nocapture`, and the round trip is `rakata conformance --game $RAKATA_GAME_K1 --only lyt,vis`.

## Where the files are

124 LYT, all in `data/layouts.bif`, and 112 VIS, spread over `data/lightmaps.bif` to `lightmaps13.bif`; no module capsule (`.rim`, `_s.rim`, `.mod`) holds either type (**measured**: the census finds every one through the KEY and none among the capsule members). Thirteen layouts have no visibility file (`m09zz`, `m12ac` to `m12ah`, `m45mg`, `m47aa`, `mgf_ebonhawk`, `mqatester`, `plcaa`, `stuntroom41ad`) and one visibility file has no layout (`m47ac`) (**measured**). What the engine draws in an area without a VIS is not established.

## Text

Both formats are lines of ASCII ended by CR LF; every byte of every vanilla file is ASCII and every line ending is CR LF (**measured**). Fields are separated by single spaces, with the exceptions listed under each format. Names (models, rooms, hooks) are resrefs, written in mixed case: `m01aa.lyt` names `M01aa_08c` and `m01aa.vis` names the same room `m01aa_08c` (**measured**), so a reader compares names without regard to case.

Numbers are written as C's `printf("%g")` writes them, six significant digits with trailing zeros dropped and an exponent below 1e-4 or from 1e6 on, with three exponent digits as Microsoft's C library writes them, and with `.0` appended where that leaves no point: `39.4685`, `-0.0407561`, `45.16`, `0.0`, `1.0`, `9.76562e-006`, `-4.37114e-007` (**measured**: all 11,286 numbers of the 124 layouts are reproduced by that rule from their float32 values, and no layout writes a negative zero). Six significant digits survive a float32 exactly, so a number read as float32 and written by the rule is the text it came from.

## LYT

```text
#MAXLAYOUT ASCII
filedependancy M01aa.max
beginlayout
   roomcount 15
      M01aa_08c 39.4685 130.475 0.0
      …
   trackcount 0
   obstaclecount 0
   doorhookcount 15
      M01aa_08c Door_02 0 39.5591 135.621 -0.0407561 0.0 0.0 0.0 1.0
      …
donelayout
```

Every vanilla layout has this shape (**measured**, 124 of 124):

- `#MAXLAYOUT ASCII`, then `filedependancy NAME` (the format's spelling) naming the 3ds Max scene the layout was exported from, then `beginlayout`, each at the start of its line.
- Four sections in the order `roomcount`, `trackcount`, `obstaclecount`, `doorhookcount`, each a count line indented three spaces and that many rows indented six. Every section's count line is present even when the count is 0, and every count is the number of rows that follow it.
- `donelayout` at the start of its line, the last line.

| Section | Row | Vanilla rows |
| --- | --- | --- |
| `roomcount` | `MODEL X Y Z`: a room's model and its origin in the area | 1,401 in 124 files |
| `trackcount` | `MODEL X Y Z`: a swoop track piece | 173 in 12 files |
| `obstaclecount` | `MODEL X Y Z`: a swoop race obstacle | 95 in 11 files |
| `doorhookcount` | `ROOM NAME 0 X Y Z W X Y Z`: a door hook | 897 |

The tracks and obstacles are in the minigame areas only: `m03mg`, `m17mg`, `m26mg` and `m45mg` (30 or 31 track pieces, 22 obstacles), `m12ab` to `m12ah` (7 and 1, all at the origin) and `mgf_ebonhawk` (1 track piece, no rooms) (**measured**).

Coordinates are the game's own: metres, Z up, in the area's frame (**measured**: a door hook stands within a millimetre of the position the area's GIT gives the door built on it, below).

### Door hooks

`ROOM NAME UNKNOWN X Y Z QW QX QY QZ`:

- `ROOM`: the room the hook belongs to. In 115 of the 124 layouts every hook names a room of the same layout; in the other 9, all cutscene layouts (`stunt_endbridge` 15 hooks, `stunt_levbridge` 11, `stunt_unkhall` 27, and one each in `stunt_ebobridge`, `stunt_ebocrash`, `stunt_ebodant`, `stunt_ebolev`, `stunt_eboqrts`, `stunt_ebosf`), 59 hooks name rooms of another area's layout (`M01aa_06a`, `M12aa_01d`) (**measured**).
- `NAME`: the hook's name, `Door_01`, `door_08` (**measured**).
- `UNKNOWN`: an integer, 0 in all 897 hooks (**measured**). What it means is not established.
- `X Y Z`: the hook's position in the area.
- `QW QX QY QZ`: the hook's orientation, a unit quaternion with the scalar part first. All 897 have length 1 within 1e-4 (**measured**). The order is established below.

### The quaternion's order

The scalar part comes first: the four numbers are `w x y z`, and every vanilla hook is a turn about the vertical (Z) axis (**measured**), on two independent kinds of evidence.

**Which components are zero.** A turn by angle _a_ about Z is `w = cos(a/2)`, `z = sin(a/2)`, `x = y = 0`. In all 897 hooks the second and third numbers are zero, or within 1e-4 of it, and the first and fourth carry the rotation: 472 have both first and fourth non-zero with the second and third exactly zero (`m01aa.lyt`, `Door_15`: `0.707107 0.0 0.0 -0.707107`, a quarter turn), 7 have the second and third below 1e-4 (`m04aa.lyt`, `door_05`: `0.446198 -1.6254e-007 2.27007e-007 0.894934`), 267 have only the first non-zero (`1.0 0.0 0.0 0.0`, no turn) and 151 only the fourth (`0.0 0.0 0.0 1.0`, a half turn) (**measured**). Read `w x y z`, every hook is a turn about Z. Read `x y z w`, as an MDL stores its orientations (xoreos-docs, `kotor_mdl.html`), the same numbers would be turns about the X axis, every such door tipped on its side.

**Agreement with the doors placed on the hooks.** Each area's GIT places its doors in `Door List`, each with `X`, `Y`, `Z` and `Bearing`, the door's turn about Z in radians. Of the 724 doors in the vanilla GITs, 682 stand within 1 mm of a hook of the area's layout, 31 more within 10 cm, 6 within 1 m and 5 farther (11 to 56 m, in `m02ac`, `m04aa`, `m17aa` and `m28aa`: doors not built on a hook) (**measured**). For the 713 doors within 10 cm, the hook's turn read `w x y z`, 2·atan2(_z_, _w_), equals the door's bearing within 0.01 rad for 703 and differs by a half turn (π) for 10; none differs otherwise. Read `x y z w`, 2·atan2(the third, the fourth), it agrees for 209, is a half turn off for 133 and is otherwise off for 371 (**measured**). An example that agrees: module `danm13`, `M13aa_11a Door_20` at (47.9107, 76.54) is `0.707107 0.0 0.0 -0.707107`, a turn of -π/2, and the GIT's door there has bearing -1.5707942. The ten a half turn off include three in `end_m01aa`: `Door_15` and `Door_16` are `0.707107 0.0 0.0 -0.707107` (-π/2) where the doors' bearing is 1.5707942, and `Door_05` is `0.0 0.0 0.0 1.0` (π) where the door's bearing is 0. A door turned a half turn fills the same doorway; whether the engine places a door by its GIT bearing or by its hook, and which way such a door then opens, is not established.

### Departures from the common form

- A tab instead of six spaces before the one room row of `m12ac`, `m12ad`, `m12ae`, `m12af`, `m12ag` and `m12ah` (`\tM12ac_01a 0.0 0.0 0.0`) (**measured**).
- Bytes after `donelayout` in `m12aa.lyt`: its last line, `donelayout` CR LF, is followed by `M12aa_01m` with no line ending, the name of one of its rooms (**measured**). The other 123 layouts end with `donelayout` CR LF.
- No layout holds a comment other than the first line, a blank line, a keyword in another case, an extra keyword, or a count that disagrees with its rows (**measured**).

## VIS

```text
m01aa_08c 4
  m01aa_08a
  m01aa_08b
  m01aa_07a
  m01aa_07b
m01aa_07a 7
  …
```

A room line, `ROOM COUNT` at the start of the line, and under it the rooms seen from it, one to a line, indented two spaces (**measured**). The 112 vanilla files hold 1,250 room lines and 8,905 rooms seen. The room itself is not normally listed; in 5 room lines it is (`m02ab_02j` in `m02ab.vis`; `m02ac_02l`, `m02ac_02m`, `m02ac_02q` in `m02ac.vis`; `m04aa_04z` in `m04aa.vis`) (**measured**). Whether visibility is symmetric is not required by the format and not checked here.

### Departures from the common form

The count is the number of indented lines that follow, in all but four room lines (**measured**):

- `m38ab.vis`: `m38ab_08 6` and `m38ab_06 8` are followed by 5 and 7 indented lines.
- `m39aa.vis`: `m39aa_15 7` is followed by 8.
- `m17aa.vis`: `m17aa_09w` has no count, at the start of its line, followed directly by the next room line.

A reader that took the count at its word would read the next room line of `m38ab.vis` as a room seen, and drop a room of `m39aa.vis`; one that follows the indentation reads every vanilla file as its indentation shows. Which of the two the engine does is not established; `rakata-lyt` follows the indentation and keeps the count as written only to write the file back.

Other departures (**measured**):

- No line ending after the last line in 13 of the 112 files.
- A trailing space after a room seen: `m17aa_15 ` in `m17aa.vis`, `m22aa_02a ` in `m22aa.vis`.
- Two spaces before the count: `M36aa_03  3`, the first line of `m36aa.vis`.

Files written by other tools depart further. In the modded build (`$RAKATA_BUILD`), a mod's `Override/stunt_starforge.vis` ends with an indented `  StuntRoom45ad 5`: a room seen with a number after it (**measured**). `rakata-lyt` follows the indentation there too, reading the room seen and keeping the line as written; what the engine does with the number is not established. Another mod's `m41ae.vis` (in `modules/unk_m41ae.mod`) ends some lines with LF alone and puts an indented blank line, ended CR LF, after each of them (**measured**); `rakata-lyt` keeps blank lines with the line before them and writes both back.

## The Endar Spire's first area

Module `end_m01aa`, area `m01aa`: `m01aa.lyt` (1,749 bytes, `filedependancy M01aa.max`) places 15 rooms, `M01aa_01a` to `M01aa_10`, all at Z = 0, and 15 door hooks, `Door_01` to `Door_19` with gaps, on 11 of the rooms; no track or obstacle. Its hooks are turns of 0, π/2 (as `-π/2`) or π about Z, standing at Z = -0.0407561 to -0.0424994 or -1.31576. `m01aa.vis` lists the 15 rooms, each seeing 2 to 9 others, and ends without a final line ending (**measured**).

## What a writer must reproduce

- CR LF line endings, the three-space and six-space indents of the layout and the two-space indent of the visibility file.
- `#MAXLAYOUT ASCII`, `filedependancy`, `beginlayout`, the four sections in order with every count present, and `donelayout`.
- Numbers as `%g` with six significant digits, a three-digit exponent and `.0` appended to an integer.
- Each count equal to the rows that follow it, except where a file read is written back as read.

`rakata-lyt` writes that form, and keeps every line of a file read that departs from it (by its position and the line the writer would have produced there), so all 124 layouts and 112 visibility files write back byte for byte (**measured**).

## What could not be established

The meaning of a door hook's integer field; whether the engine reads a VIS by its counts or by its indentation; what it does with the bytes after `m12aa.lyt`'s `donelayout`, a room listing itself, an area with no VIS, or a hook naming a room its layout lacks; and whether a door is turned by its GIT bearing or by its hook where the two are a half turn apart. Each needs the engine.

## Provenance

No public document describes LYT or VIS: the xoreos-docs `specs/` folder (BioWare's Aurora documents, Torlack's notes, `kotor_mdl.html`, `gmax_nwn_mdl_0.3b.ms`) has none, and `kotor_mdl.html` is the source for an MDL's orientation being stored `x y z w`, the order this document shows a layout does not use. Everything here was measured from the vanilla KotOR 1 files and the GITs of the same install; no other tool was read or run.
