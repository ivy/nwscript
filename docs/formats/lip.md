# LIP V1.0

A LIP is KotOR 1 and 2's lip-sync track for one voice line: the line's duration and a list of keyframes, each a time and one of sixteen mouth shapes, which the engine plays against the WAV of the same name while a dialogue node speaks. The game's own LIPs are members of the `lips/*.mod` archives (one ERF per module, named for it); a loose `.lip` in `Override/` is found too, and moves the mouth in the running game (observed with modshop's Juhani mod). There is no string block, no table of offsets, no padding and no trailer: the file is a 16-byte header followed by the keyframes back to back.

Multi-byte values are little-endian. Floats are IEEE 754 single precision (4 bytes). Every fact below is tagged **measured** (seen in bytes of a file on this host), **reference** (what the reference tool accepts when reading and emits when writing), or **inferred**. The vanilla measurements are over the 18,206 members of the 123 `lips/*.mod` archives of a KotOR 1 install, read through `rakata-archive`; the census that makes them is `crates/rakata-lip/tests/vanilla.rs`, run as `RAKATA_GAME_K1=… cargo test -p rakata-lip --test vanilla -- --nocapture`, and the round trip itself is `rakata conformance --game $RAKATA_GAME_K1 --only lip`.

## Header

16 bytes.

| Offset | Size | Type | Content |
| --- | --- | --- | --- |
| 0x00 | 4 | ASCII | Signature, the bytes `LIP ` (with a trailing space). **Measured** in `oaj_juh31.lip`; the **reference** tool refuses a file with any other value. |
| 0x04 | 4 | ASCII | Version, the bytes `V1.0`. **Measured**; the **reference** tool refuses any other value, and no other version is known. |
| 0x08 | 4 | float32 | Duration of the line in seconds. **Measured**: in every vanilla file it is bit for bit the time of the last keyframe (below). |
| 0x0C | 4 | uint32 | Number of keyframes that follow. **Measured**: 16 + 5 × this count is the file's length in every vanilla file. |

## Keyframes

Immediately after the header, the number of keyframes given at 0x0C, each 5 bytes, stored back to back with no alignment or padding between them, so the file length is 16 + 5 × count and keyframe _i_ starts at 16 + 5 × _i_. **Measured**: all 18,206 vanilla members (5,023,596 bytes, 946,460 keyframes) read under this layout and write back byte for byte, so none carries padding, a trailer or a shape above 15; the **reference** tool writes and reads the same layout. The odd stride means the floats from the second keyframe on are not 4-byte aligned in the file.

| Offset | Size | Type | Content |
| --- | --- | --- | --- |
| 0x00 | 4 | float32 | Time of the keyframe in seconds from the start of the line. **Reference**. |
| 0x04 | 1 | uint8 | Mouth shape, 0 to 15 (table below). **Reference**. |

Nothing follows the last keyframe in any vanilla file (**measured**) or in any file the reference tool writes (**reference**). Whether the engine tolerates trailing bytes is not established.

The Read tool's view of `oaj_juh31.lip` (a loose file in the modded build's `Override/`, written by the reference tool from Rhubarb Lip Sync cues) shows the signature and version in clear, then a run of 4-byte groups whose most significant bytes climb through 0x3D, 0x3E, 0x3F and 0x40 (times rising through roughly 0.03 to 0.12, 0.12 to 0.5, 0.5 to 2 and 2 to 8 seconds), each separated by a single byte: the layout above, with the keyframes in increasing time order. **Measured**, but only to that resolution; the tool cannot show exact bytes of a binary file.

## Mouth shapes

The shape byte selects one of sixteen visemes, a Preston Blair-style set. The engine's own names for the poses are not public; the labels below are this document's, chosen from the mouth position, and the sounds column gives the ARPAbet phonemes the reference tool assigns to each number when it makes a LIP from phonemes (**reference**). The numbers are the format. All sixteen values occur in the vanilla files, none above 15 (**measured**: of 946,460 keyframes, 2 is the most frequent at 216,369 and 14 the rarest at 16,609; the census prints the count of each). A byte above 15 names no pose: the reference tool refuses to read such a file, and a writer must never emit one.

| Value | Label | Mouth | Sounds |
| --- | --- | --- | --- |
| 0 | rest | closed, relaxed | silence, pauses |
| 1 | ee | teeth slightly apart, corners wide | IY, IH, EY ("see", "bit", "bait") |
| 2 | eh | relaxed, slightly open | EH, ER ("get", "bird") |
| 3 | ah | open | AA, AE, AH, AW, AY ("father", "cat", "but", "down", "bite") |
| 4 | oh | rounded | AO, OW, OY ("bought", "boat", "boy") |
| 5 | oo | pursed | UW, UH, W ("too", "book", "we") |
| 6 | y | slight smile | Y ("yes") |
| 7 | s | teeth together | S, Z ("stop", "zee") |
| 8 | f | lower lip on upper teeth | F, V ("five", "very") |
| 9 | n | back of tongue up | N, NG ("knee", "ring") |
| 10 | th | tongue between teeth | TH, DH ("thin", "that") |
| 11 | m | lips pressed | M, P, B ("bump", "pop") |
| 12 | t | tongue up behind teeth | T, D ("top", "door") |
| 13 | sh | rounded, relaxed | SH, CH, JH, ZH ("she", "cheese", "jee", "measure") |
| 14 | l | tongue forward | L, R ("lip", "read") |
| 15 | k | back of tongue raised | K, G, HH ("kick", "green", "he") |

## Time order, duplicates and the duration

The layout itself imposes no order on the keyframes. The engine interpolates between the keyframe before the current playback time and the one after it, so the stream is meant to be in non-decreasing time order; whether the engine sorts, tolerates or misplays an out-of-order file is not established and needs the engine. Every vanilla file is in non-decreasing time order, and every one begins with a shape-0 keyframe at time 0.0 (**measured**).

BioWare's files do hold duplicated times: 227 of the 18,206 vanilla members have two consecutive keyframes at bit-for-bit the same time, never more than two and never out of order (**measured**). In 180 of them both are shape 0, a rest doubled mid-line: `STUNT_07_loc.mod`, member `nglobebast06865_.lip`, keyframes 2 and 3 at offsets 0x1A and 0x1F, both time 0x3EC41894 (0.383 s), both shape 0. In the other 47 they are keyframes 0 and 1, both at time 0x00000000, the first shape 0 and the second another shape, so the line opens on a consonant: `STUNT_19_loc.mod`, member `nm45adc01001002_.lip`, offsets 0x10 and 0x15, shapes 0 then 11. `localization.mod` holds 30 of the 227. A writer must therefore keep duplicated times as read.

Every file the reference tool writes has its keyframes in increasing time order with no two keyframes closer than 0.0001 s, and a file read and written back through it comes out that way even if it did not go in so (**reference**); a round trip through that tool is therefore not byte-preserving for the 227 vanilla files above, whose doubled keyframes it would move or merge. The files in the modded build's `Override/` were all written by community tools, and all 175 are ordered with no duplicated time (**measured**).

The duration at 0x08 is the length of the voice line. In every vanilla file the last keyframe's time is bit for bit the duration, and that last keyframe is shape 0 (**measured**, 18,206 of 18,206). In every file the reference tool writes the duration is at least the time of the last keyframe (**reference**). The 175 loose files in the modded build all end at the duration too; the 80 `oaj_*.lip` of modshop's Juhani mod end on shape 0, placed at the WAV's duration by `voice.py`, and 47 `ldr_*.lip` from another mod end on a speaking shape (**measured**), which only shows that a community writer emits such a file, not what the engine does with it. What the engine does when the last keyframe precedes the duration (hold the last shape, or return to rest), or is not a rest, is not established; a reader may not assume either.

A file with zero keyframes is well-formed as far as the layout goes: 16 bytes, count 0 (**reference**: such a file is both written and read). No vanilla file and no file in the modded build has zero keyframes (**measured**), and what the engine does with one is not established.

## What a writer must reproduce

- `LIP ` and `V1.0`, exactly.
- The duration as a float32 and the count as a uint32, both little-endian.
- Exactly `count` keyframes of 5 bytes each, time then shape, with no padding, no alignment and nothing after the last.
- Floats bit for bit as read when writing a file back; floats are computed only when a LIP is made from cues.
- Shape bytes in 0 to 15.

## What could not be established

The engine's behaviour on out-of-order keyframes, on a duration that disagrees with the last keyframe, on a last keyframe that is not a rest, on a zero-keyframe file, on a shape above 15 and on trailing bytes needs the engine; no vanilla file shows any of these, so the readers refuse the last two and keep the rest as read. The engine's internal names for the sixteen shapes are not public.

## Provenance

Public documents read: the file list of xoreos-docs `specs/` (no LIP document exists there; a search of the checkout for "LIP" finds nothing). The KotOR wiki's LIP page was unreachable from this host.

Reference projects read: PyKotor, for the header and keyframe layout (types, sizes, byte order, the absence of padding and trailer), the sixteen shape numbers and the phoneme table, which files it refuses (bad signature, version or shape), and the ordering, spacing and duration properties of the files it writes. modshop's own `games/kotor/mods/juhani-clones/voice.py` (the user's code) for how the loose files in the build were produced.

Measured: on 2026-10-17, `games/kotor/data/game/build/Override/oaj_juh31.lip` in the modded build, viewed with the Read tool, which shows the ASCII signature and version, the duration's top byte, and the rising sequence of 4-byte time groups each followed by one shape byte; the 175 loose `.lip` files in the build are under `Override/`, not `streamwaves/`.

Measured: on 2026-10-17, by the implementer of `rakata-lip`, the 18,206 members of the 123 `lips/*.mod` archives of the vanilla KotOR 1 install (`$RAKATA_GAME_K1`) and the 175 loose `.lip` files of the modded build's `Override/` (`$RAKATA_BUILD`), through `crates/rakata-lip/tests/vanilla.rs` and `rakata conformance --only lip`: every file round-trips byte for byte (the build's 18,432 `.lip` in all, loose and in its `lips/*.mod`, likewise), and the counts of time order, duplicated times, first and last keyframes and shapes are as stated above.

Date: 2026-10-17.

Written by the format-scribe role; no implementer read the source.
