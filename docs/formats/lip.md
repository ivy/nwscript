# LIP V1.0

A LIP is KotOR 1 and 2's lip-sync track for one voice line: the line's duration and a list of keyframes, each a time and one of sixteen mouth shapes, which the engine plays against the WAV of the same name while a dialogue node speaks. The game's own LIPs are members of the `lips/*.mod` archives (one ERF per module, named for it); a loose `.lip` in `Override/` is found too, and moves the mouth in the running game (observed with modshop's Juhani mod). There is no string block, no table of offsets, no padding and no trailer: the file is a 16-byte header followed by the keyframes back to back.

Multi-byte values are little-endian. Floats are IEEE 754 single precision (4 bytes). Every fact below is tagged **measured** (seen in bytes of a file on this host), **reference** (what the reference tool accepts when reading and emits when writing), or **inferred**.

## Header

16 bytes.

| Offset | Size | Type | Content |
| --- | --- | --- | --- |
| 0x00 | 4 | ASCII | Signature, the bytes `LIP ` (with a trailing space). **Measured** in `oaj_juh31.lip`; the **reference** tool refuses a file with any other value. |
| 0x04 | 4 | ASCII | Version, the bytes `V1.0`. **Measured**; the **reference** tool refuses any other value, and no other version is known. |
| 0x08 | 4 | float32 | Duration of the line in seconds. **Reference**; **measured** only as far as the top byte of the field in `oaj_juh31.lip` is 0x40, a positive value in [2, 8). |
| 0x0C | 4 | uint32 | Number of keyframes that follow. **Reference**. |

## Keyframes

Immediately after the header, the number of keyframes given at 0x0C, each 5 bytes, stored back to back with no alignment or padding between them, so the file length is 16 + 5 × count and keyframe _i_ starts at 16 + 5 × _i_. **Reference**: files it writes have exactly this length, and it reads files of this layout. The odd stride means the floats from the second keyframe on are not 4-byte aligned in the file.

| Offset | Size | Type | Content |
| --- | --- | --- | --- |
| 0x00 | 4 | float32 | Time of the keyframe in seconds from the start of the line. **Reference**. |
| 0x04 | 1 | uint8 | Mouth shape, 0 to 15 (table below). **Reference**. |

Nothing follows the last keyframe in any file the reference tool writes (**reference**). Whether the engine tolerates trailing bytes is not established.

The Read tool's view of `oaj_juh31.lip` (a loose file in the modded build's `Override/`, written by the reference tool from Rhubarb Lip Sync cues) shows the signature and version in clear, then a run of 4-byte groups whose most significant bytes climb through 0x3D, 0x3E, 0x3F and 0x40 (times rising through roughly 0.03 to 0.12, 0.12 to 0.5, 0.5 to 2 and 2 to 8 seconds), each separated by a single byte: the layout above, with the keyframes in increasing time order. **Measured**, but only to that resolution; the tool cannot show exact bytes of a binary file.

## Mouth shapes

The shape byte selects one of sixteen visemes, a Preston Blair-style set. The engine's own names for the poses are not public; the labels below are this document's, chosen from the mouth position, and the sounds column gives the ARPAbet phonemes the reference tool assigns to each number when it makes a LIP from phonemes (**reference**). The numbers are the format. A byte above 15 names no pose: the reference tool refuses to read such a file, and a writer must never emit one.

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

The layout itself imposes no order on the keyframes. The engine interpolates between the keyframe before the current playback time and the one after it, so the stream is meant to be in non-decreasing time order; whether the engine sorts, tolerates or misplays an out-of-order file is not established and needs the engine. Every file the reference tool writes has its keyframes in increasing time order with no two keyframes closer than 0.0001 s, and a file read and written back through it comes out that way even if it did not go in so (**reference**); a round trip through that tool is therefore not byte-preserving for an unordered or duplicated input, and the files in the modded build were all written by it, so they are ordered (**measured** for `oaj_juh31.lip`, as above).

The duration at 0x08 is the length of the voice line. In every file the reference tool writes the duration is at least the time of the last keyframe (**reference**), and the files in the modded build end with a shape-0 keyframe placed exactly at the WAV's duration, so in those files duration and last time are equal (**measured** through `voice.py`, the user's tool that produced them). Whether BioWare's own files always satisfy this, and what the engine does when the last keyframe precedes the duration (hold the last shape, or return to rest), is not established; a file may not be assumed to satisfy it.

A file with zero keyframes is well-formed as far as the layout goes: 16 bytes, count 0 (**reference**: such a file is both written and read). What the engine does with it is not established.

## What a writer must reproduce

- `LIP ` and `V1.0`, exactly.
- The duration as a float32 and the count as a uint32, both little-endian.
- Exactly `count` keyframes of 5 bytes each, time then shape, with no padding, no alignment and nothing after the last.
- Floats bit for bit as read when writing a file back; floats are computed only when a LIP is made from cues.
- Shape bytes in 0 to 15.

## What could not be established

The vanilla files themselves were not opened (they are members of `lips/*.mod`, and no tool on this host could extract one for the Read tool), so every count a RIM- or ERF-style document would give (how many LIPs, whether any has zero keyframes, a duplicated time, an out-of-order keyframe, a duration unequal to the last time, or a shape above 15) is unmeasured; `rakata conformance --format lip` over `$RAKATA_GAME_K1` is the measurement. The engine's behaviour on out-of-order keyframes, on a duration that disagrees with the last keyframe, on a shape above 15 and on trailing bytes needs the engine. The engine's internal names for the sixteen shapes are not public.

## Provenance

Public documents read: the file list of xoreos-docs `specs/` (no LIP document exists there; a search of the checkout for "LIP" finds nothing). The KotOR wiki's LIP page was unreachable from this host.

Reference projects read: PyKotor, for the header and keyframe layout (types, sizes, byte order, the absence of padding and trailer), the sixteen shape numbers and the phoneme table, which files it refuses (bad signature, version or shape), and the ordering, spacing and duration properties of the files it writes. modshop's own `games/kotor/mods/juhani-clones/voice.py` (the user's code) for how the loose files in the build were produced.

Measured: on 2026-10-17, `games/kotor/data/game/build/Override/oaj_juh31.lip` in the modded build, viewed with the Read tool, which shows the ASCII signature and version, the duration's top byte, and the rising sequence of 4-byte time groups each followed by one shape byte; the 175 loose `.lip` files in the build are under `Override/`, not `streamwaves/`.

Date: 2026-10-17.

Written by the format-scribe role; no implementer read the source.
