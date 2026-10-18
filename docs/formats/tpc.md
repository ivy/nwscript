# TPC and TXI

TPC (resource type 3007) is the texture format of KotOR 1 and 2: a fixed header, the pixel data as the GPU takes it (DXT blocks or plain pixels, every mip level stored), and optionally the texture's TXI text appended after the pixels. TXI (resource type 2022) is that text on its own: lines of keywords and values that tell the engine how to use a texture. KotOR 1 ships 11,536 TPCs (1,570 in `TexturePacks/swpc_tex_gui.erf`, 3,294 in each of `swpc_tex_tpa.erf`, `swpc_tex_tpb.erf` and `swpc_tex_tpc.erf`, the high, medium and low quality packs, and 84 in `patch.erf`) and 5,616 standalone TXIs (5,586 in `data/lightmaps*.bif`, beside the lightmap TGAs, 28 in `textures.bif`, one in `templates.bif`, one in `swpc_tex_gui.erf`). No module or RIM holds a texture.

All multi-byte integers are little-endian. Offsets are counted from the first byte of the file.

## Header

The header is 128 bytes.

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 4 | Data size, u32. Zero for an uncompressed format; non-zero for a DXT format. See below for its value. |
| 0x04 | 4 | A float32. 1.0 in all 5,530 files whose format has no alpha. In the 6,006 files with alpha it lies in 0 to 1; in 1,447 of the 1,495 uncompressed RGBA files it equals the mean of level 0's alpha bytes divided by 255 to within 1e-4. `rakata-texture` calls it `alpha_mean`; what the engine does with it is not established. |
| 0x08 | 2 | Width, u16. |
| 0x0A | 2 | Height, u16. For a cube map, six times the width. |
| 0x0C | 1 | Encoding: 1, 2 or 4 (see Pixel formats). |
| 0x0D | 1 | Mip count: the number of levels each image stores, except for an animation sheet (below), where it is 1. |
| 0x0E | 114 | Zero in every vanilla file. |

## Pixel formats

The encoding byte and whether the data size is zero name the format. These five combinations are every one the vanilla files use:

| Encoding | Data size | Format | Bytes per level of w × h | Files |
| --- | --- | --- | --- | --- |
| 1 | zero | greyscale, 1 byte per pixel | w × h | 12 |
| 2 | zero | RGB, 3 bytes per pixel | 3 × w × h | 207 |
| 4 | zero | RGBA, 4 bytes per pixel | 4 × w × h | 1,495 |
| 2 | non-zero | DXT1 (BC1), 8 bytes per 4×4 block | 8 × ⌈w/4⌉ × ⌈h/4⌉ | 5,311 |
| 4 | non-zero | DXT5 (BC3), 16 bytes per 4×4 block | 16 × ⌈w/4⌉ × ⌈h/4⌉ | 4,511 |

A DXT level smaller than 4×4 (2×2, 1×1) still takes one whole block. The DXT block encodings are the published S3TC/BCn ones. Of the 5,311 DXT1 files, 77 have a block among the first ⌈w/4⌉ × ⌈h/4⌉ of their pixel data in DXT1's three-colour mode that uses the transparent index, so they carry one-bit alpha although the format is the one without an alpha channel.

The first row stored is the top of the image: a renderer that uploads the rows as stored and flips V draws the Endar Spire's wall and ceiling labels the right way up. The channel order of the uncompressed formats is not established: no vanilla texture exists both as an uncompressed TPC and in another format under the same name to compare against. `rakata-texture` passes the bytes as stored.

The dimensions are powers of two in every file except the 189 cube maps, whose height is six times a power of two. 52 files are not square (and not cube maps), such as 32×512.

## Pixel data

The pixel data begins at 0x80. It is one or more images stored one after another, and each image is its mip levels from the largest down, back to back. Level i of a w × h image is max(1, w >> i) × max(1, h >> i). No padding separates levels, images, or the pixel data and the text after it. There are three layouts:

- **Single** (11,226 files): one image of the header's width × height with the header's mip count of levels. The mip count is the full chain to 1×1 in 8,375 of them and 1 in the other 2,851; none has more levels than the full chain. Level counts across all files, counting a cube face or a sheet frame as an image: 1 (2,851), 2 (23), 3 (25), 4 (104), 5 (350), 6 (856), 7 (1,795), 8 (2,427), 9 (2,101), 10 (976), 11 (27), 12 (1).
- **Cube map** (189 files, all DXT5): the height is six times the width. Six square faces of width × width follow one another, each with the header's mip count of levels (the full chain in all 189). Every one of them, and no other file, has `cube 1` in its TXI. The order in which the six faces map to directions is not established from the files.
- **Animation sheet** (121 files: 105 DXT1, 16 DXT5): the TXI says `proceduretype cycle` and gives `numx` and `numy`. The image is divided into `numx` × `numy` frames of (width / numx) × (height / numy), and each frame is stored whole with a full mip chain to 1×1, frame after frame. The header's mip count is 1 in all 121, and the data size is the length of all the frames together: `numx × numy × chain`, exact in all 121. That frames are stored one after another, each with its own chain (rather than each level of every frame together), was measured from the pixels: in 97 of the 104 sheets with frames of at least 16×16, each frame's level 1 matches a 2×2 average of its level 0's block colours better under that order, often by a factor of ten or more; in four (`lbl_static`, `dor_ukn02`, `fx_tex_09`, `LSF_BMap_4`, noise-like images) the other order scores slightly better and neither matches well, and three copies of `LQA_birdsheet` score the same both ways. Which frame is first in the image (row order across, then down, is assumed) is not established.

### The data size field

In every compressed file that is not an animation sheet (9,701 files), the data size is the length of level 0 of one image, or 16 when level 0 is smaller: the 23 DXT1 files of 4×4 and 2×2 store a level 0 of one 8-byte block and a data size of 16. A reader must not take the field as the pixel data's length; the length follows from the dimensions, the format and the mip count. A compressed file with a mip count of 1 and a data size larger than both its level 0 and 16 is an animation sheet in all 121 cases and in no other.

## TXI after the pixels

Whatever follows the pixel data to the end of the file is the texture's TXI text. 6,207 TPCs carry some, 5,329 end with the pixel data. The text is the same as a standalone TXI's.

## TXI text

A TXI is ASCII text: lines ending CR LF (every line of every vanilla TXI that has more than one), the last often without one. No vanilla TXI contains a NUL or a byte above 127. Each line is a keyword, whitespace, and a value (`mipmap 0`, `blending additive`, `envmaptexture CM_Bright`, `channelscale 4`); a keyword may stand alone. A keyword followed by a count is followed by that many lines that begin with a number: a font's `upperleftcoords 96` and `lowerrightcoords 96` are followed by 96 rows of `x y z`, and `channelscale` and `channeltranslate` by four values. Keywords are lower case except in 136 lines (`Bumpyshinytexture`, `BumpyShinyTexture`), so a reader compares them without case.

The keywords in vanilla KotOR 1, with the number of lines (in standalone TXIs and in TPCs) that begin with each: `mipmap` 8,416, `downsamplemax` 7,319, `compresstexture` 5,806, `islightmap` 5,771, `envmaptexture` 1,014, `blending` 594, `clamp` 468, `downsamplemin` 464, `isbumpmap` 443, `bumpmapscaling` 416, `bumpmaptexture` 360, `bumpyshinytexture` 326, `cube` 189, `proceduretype` 178, `numx`, `numy` and `fps` 137 each, `decal` 126, `isdiffusebumpmap` and `isspecularbumpmap` 105 each, `wateralpha` 66, `distort`, `distortionamplitude` and `speed` 39 each, `defaultheight` and `defaultwidth` 38 each, `channeltranslate` 37, `channelscale` 34, `arturowidth` and `arturoheight` 27 each, `bumpmapscale` 24, the font keywords (`numchars`, `fontheight`, `baselineheight`, `texturewidth`, `spacingr`, `spacingb`, `upperleftcoords`, `lowerrightcoords`, `filter`) 18 each, `caretindent` 12, `specularcolor` 9, `waterwidth` and `waterheight` 6 each, `alphamean` and `type` 3 each, and `compress` and `compresstexure` (sic) once each.

Five TXIs write a keyword twice (`lsf_glowstatic.txi` and `lsf_glowwalk03.txi` repeat `bumpyshinytexture`, `lsf_lit01.txi` and `lbl_menudarr.txi` repeat `mipmap`, `m50aa_01a_a000b4.txi` repeats `downsamplemax`); which one the engine obeys is not established. One resource of type TXI, `grassinfo.txi` in `textures.bif`, is not instructions at all: it begins with a blank line and then `3 grass 5.0 1.5 ...`, a table of numbers.

What each keyword makes the engine do is not established by this document.

## What could not be established

The meaning of the float at 0x04 to the engine; the channel order of uncompressed pixels; the direction of each cube face; the frame order of an animation sheet within the image; which of a repeated TXI keyword wins. Each needs the engine or a texture known by other means, not the files alone.

## Provenance

Public documents read: xoreos-docs `specs/torlack/basics.html`, for the resource type numbers of TPC (3007), TXI (2022) and TGA (3). xoreos-docs has no TPC or TXI document; the DXT block layouts are the published S3TC ones.

Reference projects read: none.

Measured: on 2026-10-18, every count above comes from the vanilla KotOR 1 install under `$RAKATA_GAME_K1`, first with short Python scripts reading the header, the pixel data lengths, the bytes after the pixels, the TXI lines, and the block colours of the animation sheets of every TPC and TXI the KEY's BIFs, `TexturePacks/*.erf`, `patch.erf`, `modules/` and `rims/` hold, then with `crates/rakata-texture/tests/vanilla.rs`, which parses every one of them and prints the counts (`RAKATA_GAME_K1=… cargo test -p rakata-texture --test vanilla -- --nocapture`).

Observed in a renderer: the row order, from the OdysseyUE runtime drawing the Endar Spire (`end_m01aa`) from these structures.

Date: 2026-10-18.

Written by the implementer from measurement; no implementation's source was read.
