# TGA

TGA (resource type 3) is Truevision's TARGA image format, published in Truevision's TGA File Format Specification (version 2.0, 1991); this document records only which parts of it KotOR 1 uses. The game's TGAs are its area lightmaps and a handful of others: 5,601 in the vanilla install, 5,586 in `data/lightmaps*.bif` (each beside a TXI of the same name, which says `islightmap 1`), 14 in `textures.bif` and one in `legacy.bif`.

## Variants in use

The 18-byte header's image type (offset 2), bits per pixel (16) and image descriptor (17) take four combinations:

| Image type | Bits | Descriptor | Files | Where |
| --- | --- | --- | --- | --- |
| 2, true colour, raw | 32 | 0x00 | 5,474 | lightmaps |
| 2, true colour, raw | 32 | 0x08 | 112 | lightmaps |
| 2, true colour, raw | 24 | 0x00 | 1 | `legacy.bif` |
| 3, greyscale, raw | 8 | 0x08 | 14 | `textures.bif` |

No vanilla TGA is run-length encoded (types 10 and 11) or colour-mapped, none has an image id or a colour map, and every one has origin bits 4 and 5 clear: the first row stored is the bottom of the image, pixels left to right. The x and y origin fields are zero. Every dimension is a power of two and every image square, from 2×2 to 512×512.

127 files end with a TGA 2.0 footer (`TRUEVISION-XFILE.` and a NUL): 107 with an extension area before it, 20 with the 26-byte footer alone right after the pixels. The other 5,474 end with the last pixel.

Mods use more of the specification. The 1,273 TGAs in the `Override/` folder of a modded build measured on 2026-10-18 are 499 of type 2 at 32 bits with descriptor 0x08, 348 of type 2 at 32 bits with 0x00, 70 of type 2 at 24 bits, 268 run-length encoded (type 10) at 32 bits with 0x08, 20 of type 10 at 32 bits with 0x00, 65 of type 10 at 24 bits, and 3 of type 3 at 8 bits with 0x08; none has an image id, a colour map or a non-zero x or y origin. Of the 920 raw ones, 502 end with a TGA 2.0 footer, 394 with the last pixel, and 24 with other bytes after the pixels.

## Alpha

The descriptor's low four bits declare the alpha bits per pixel. In the 5,474 32-bit lightmaps that declare none, the fourth byte of a pixel is 255 throughout in 3,114 files and varies in 2,360; in the 112 that declare 8, it is 255 throughout in 107 and varies in 5. Since the specification gives a fourth byte with no declared alpha bits no meaning, what the engine makes of the varying ones is not established; `rakata-texture` passes the fourth byte through as alpha whatever the descriptor says.

The 14 greyscale files declare 8 alpha bits in an 8-bit pixel, which leaves no bits for the grey value under the specification's reading. `rakata-texture` reads the byte as the grey value and makes the pixel opaque.

## Provenance

Public documents read: Truevision's TGA File Format Specification 2.0 (for the header, packet and footer layouts), and xoreos-docs `specs/torlack/basics.html` for the resource type number.

Reference projects read: none.

Measured: on 2026-10-18, every count above comes from the vanilla KotOR 1 install under `$RAKATA_GAME_K1`, first with short Python scripts reading the header, the tail after the pixels and the fourth byte of each 32-bit pixel of every TGA the KEY's BIFs hold, then with `crates/rakata-texture/tests/vanilla.rs`, which decodes every one, prints the variants, and writes each back byte for byte (`RAKATA_GAME_K1=… cargo test -p rakata-texture --test vanilla -- --nocapture`). The modded build's counts come from a short Python script reading the header and the tail of each loose TGA, and `rakata conformance --game $RAKATA_BUILD --only tga` writes every TGA of that build back byte for byte.

Date: 2026-10-18.

Written by the implementer from measurement; no implementation's source was read.
