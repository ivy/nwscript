//! Hand-made textures, built here, one for each variant the crate decodes
//! and each error it reports.

use rakata_texture::{Error, ImageType, Layout, Origin, PixelFormat, Tga, Tpc, codes};

/// A TPC header: data size, the float, width, height, encoding, mip count,
/// and 114 zero bytes.
fn tpc_header(data_size: u32, float: f32, w: u16, h: u16, encoding: u8, mips: u8) -> Vec<u8> {
    let mut b = Vec::with_capacity(128);
    b.extend_from_slice(&data_size.to_le_bytes());
    b.extend_from_slice(&float.to_le_bytes());
    b.extend_from_slice(&w.to_le_bytes());
    b.extend_from_slice(&h.to_le_bytes());
    b.push(encoding);
    b.push(mips);
    b.resize(128, 0);
    b
}

/// `len` bytes counting up from `seed`, so each level's bytes are told apart.
fn filler(len: usize, seed: u8) -> Vec<u8> {
    (0..len).map(|i| seed.wrapping_add(i as u8)).collect()
}

#[test]
fn dxt1_with_a_full_chain_and_txi() {
    // 8×8 DXT1: levels 8×8 (4 blocks), 4×4, 2×2 and 1×1 (one block each).
    let mut b = tpc_header(32, 1.0, 8, 8, 2, 4);
    let lens = [32, 8, 8, 8];
    for (i, len) in lens.iter().enumerate() {
        b.extend(filler(*len, i as u8 * 40));
    }
    b.extend_from_slice(b"mipmap 0\r\nblending additive\r\n");
    let t = Tpc::parse(&b).unwrap();
    assert_eq!(t.format, PixelFormat::Dxt1);
    assert!(!t.has_alpha());
    assert_eq!(t.layout, Layout::Single);
    assert_eq!(t.images.len(), 1);
    let levels = &t.images[0].levels;
    assert_eq!(
        levels
            .iter()
            .map(|l| (l.width, l.height, l.offset, l.data.len()))
            .collect::<Vec<_>>(),
        [
            (8, 8, 128, 32),
            (4, 4, 160, 8),
            (2, 2, 168, 8),
            (1, 1, 176, 8)
        ]
    );
    // The levels are the input's bytes, not copies.
    assert!(std::ptr::eq(levels[1].data.as_ptr(), b[160..].as_ptr()));
    assert_eq!(t.pixels.len(), 56);
    let txi = t.txi.unwrap();
    assert_eq!(txi.value("blending"), Some("additive"));
    assert_eq!(txi.int("mipmap"), Some(0));
}

#[test]
fn a_tiny_dxt1_states_sixteen_bytes_and_stores_one_block_per_level() {
    // As the game's 4×4 DXT1 files do: the data-size field says 16 for a
    // level 0 of one 8-byte block, and that is no animation sheet.
    let mut b = tpc_header(16, 1.0, 4, 4, 2, 3);
    b.extend(filler(24, 1));
    let t = Tpc::parse(&b).unwrap();
    assert_eq!(t.layout, Layout::Single);
    assert_eq!(t.level_count(), 3);
    assert!(t.txi.is_none());
    assert!(t.txi_bytes.is_empty());
}

#[test]
fn dxt5_has_alpha_and_the_header_float() {
    let mut b = tpc_header(64, 0.25, 8, 8, 4, 2);
    b.extend(filler(64 + 16, 3));
    let t = Tpc::parse(&b).unwrap();
    assert_eq!(t.format, PixelFormat::Dxt5);
    assert!(t.has_alpha());
    assert_eq!(t.alpha_mean, 0.25);
    assert_eq!(t.images[0].levels[1].data.len(), 16);
}

#[test]
fn uncompressed_grey_rgb_and_rgba() {
    for (enc, format, bpp) in [
        (1u8, PixelFormat::Grey, 1usize),
        (2, PixelFormat::Rgb, 3),
        (4, PixelFormat::Rgba, 4),
    ] {
        // 4×2 with two levels: 4×2 then 2×1.
        let mut b = tpc_header(0, 1.0, 4, 2, enc, 2);
        b.extend(filler(8 * bpp + 2 * bpp, enc));
        let t = Tpc::parse(&b).unwrap();
        assert_eq!(t.format, format);
        assert!(!t.format.is_compressed());
        let l = &t.images[0].levels;
        assert_eq!((l[0].width, l[0].height, l[0].data.len()), (4, 2, 8 * bpp));
        assert_eq!((l[1].width, l[1].height, l[1].data.len()), (2, 1, 2 * bpp));
        assert_eq!(l[1].data, &b[128 + 8 * bpp..128 + 10 * bpp]);
    }
}

#[test]
fn a_cube_map_has_six_faces_each_with_its_chain() {
    // Six 8×8 DXT5 faces stacked: the header says 8×48.
    let mut b = tpc_header(64, 1.0, 8, 48, 4, 4);
    let per_face = 64 + 16 + 16 + 16;
    for face in 0..6u8 {
        b.extend(filler(per_face, face * 10));
    }
    b.extend_from_slice(b"cube 1\r\n");
    let t = Tpc::parse(&b).unwrap();
    assert_eq!(t.layout, Layout::Cube);
    assert_eq!(t.images.len(), 6);
    assert_eq!(t.image_size(), (8, 8));
    for (i, face) in t.images.iter().enumerate() {
        assert_eq!(face.levels.len(), 4);
        assert_eq!(face.levels[0].offset, 128 + i * per_face);
        assert_eq!(face.levels[0].data[0], i as u8 * 10);
    }
    assert_eq!(t.txi.unwrap().int("cube"), Some(1));
}

#[test]
fn an_animation_sheet_is_frames_each_with_a_full_chain() {
    // 16×16 DXT1 as 2×2 frames of 8×8; each frame 8×8, 4×4, 2×2, 1×1 =
    // 32 + 8 + 8 + 8 bytes. The header says one mip and the whole size.
    let per_frame = 56;
    let mut b = tpc_header(4 * per_frame as u32, 1.0, 16, 16, 2, 1);
    for frame in 0..4u8 {
        b.extend(filler(per_frame, frame * 50));
    }
    b.extend_from_slice(b"proceduretype cycle\r\nnumx 2\r\nnumy 2\r\nfps 8\r\n");
    let t = Tpc::parse(&b).unwrap();
    assert_eq!(t.layout, Layout::Flipbook { numx: 2, numy: 2 });
    assert_eq!(t.mip_count, 1);
    assert_eq!(t.images.len(), 4);
    assert_eq!(t.level_count(), 4);
    assert_eq!(t.image_size(), (8, 8));
    assert_eq!(t.images[2].levels[0].offset, 128 + 2 * per_frame);
    assert_eq!(t.images[3].levels[3].data.len(), 8);
    assert_eq!(t.txi.unwrap().int("fps"), Some(8));
}

#[test]
fn tpc_errors_carry_their_codes() {
    let short = Tpc::parse(&[0; 100]).unwrap_err();
    assert_eq!(short.code(), codes::TPC_TRUNCATED);

    let unknown = Tpc::parse(&tpc_header(16, 1.0, 4, 4, 1, 1)).unwrap_err();
    assert_eq!(
        unknown,
        Error::TpcUnknownFormat {
            encoding: 1,
            compressed: true
        }
    );
    assert_eq!(unknown.code(), "RE2202");

    let mut cut = tpc_header(32, 1.0, 8, 8, 2, 2);
    cut.extend(filler(35, 0));
    assert_eq!(Tpc::parse(&cut).unwrap_err().code(), "RE2201");

    let zero = Tpc::parse(&tpc_header(0, 1.0, 0, 4, 4, 1)).unwrap_err();
    assert_eq!(zero.code(), codes::TPC_BAD_LAYOUT);

    // A sheet-sized data field with no numx/numy in the TXI.
    let mut sheet = tpc_header(64, 1.0, 8, 8, 2, 1);
    sheet.extend(filler(64, 0));
    sheet.extend_from_slice(b"mipmap 0\r\n");
    assert_eq!(Tpc::parse(&sheet).unwrap_err().code(), "RE2203");
    assert!(
        Tpc::parse(&sheet)
            .unwrap_err()
            .to_string()
            .starts_with("RE2203: ")
    );
}

/// A TGA header for an image of `ty` at `bits` with descriptor `desc`.
fn tga_header(ty: u8, w: u16, h: u16, bits: u8, desc: u8) -> Vec<u8> {
    let mut b = vec![0, 0, ty, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    b.extend_from_slice(&w.to_le_bytes());
    b.extend_from_slice(&h.to_le_bytes());
    b.push(bits);
    b.push(desc);
    b
}

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn pixels(t: &Tga) -> Vec<[u8; 4]> {
    t.rgba.as_chunks::<4>().0.to_vec()
}

#[test]
fn tga_32_bit_bottom_left_comes_out_top_first_with_its_alpha() {
    // 2×2, the file's first row is the bottom one: red, green; then the top:
    // blue, and white with alpha 7. Stored B, G, R, A. Descriptor 0, as the
    // game's lightmaps have, with the fourth byte still the alpha.
    let mut b = tga_header(2, 2, 2, 32, 0);
    for px in [
        [0, 0, 255, 255],
        [0, 255, 0, 255],
        [255, 0, 0, 255],
        [255, 255, 255, 7],
    ] {
        b.extend_from_slice(&px);
    }
    b.extend_from_slice(b"\0\0\0\0\0\0\0\0TRUEVISION-XFILE.\0");
    let t = Tga::parse(&b).unwrap();
    assert_eq!((t.width, t.height, t.bits, t.alpha_bits), (2, 2, 32, 0));
    assert_eq!(t.image_type, ImageType::TrueColor);
    assert_eq!(t.origin, Origin::BottomLeft);
    assert_eq!(pixels(&t), [BLUE, [255, 255, 255, 7], RED, GREEN]);
}

#[test]
fn tga_24_bit_top_left_and_right_origins() {
    let mut b = tga_header(2, 2, 1, 24, 0x20);
    b.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
    let t = Tga::parse(&b).unwrap();
    assert_eq!(t.origin, Origin::TopLeft);
    assert_eq!(pixels(&t), [RED, GREEN]);

    let mut b = tga_header(2, 2, 1, 24, 0x30);
    b.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
    let t = Tga::parse(&b).unwrap();
    assert_eq!(t.origin, Origin::TopRight);
    assert_eq!(pixels(&t), [GREEN, RED]);

    let mut b = tga_header(2, 1, 2, 24, 0x10);
    b.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
    let t = Tga::parse(&b).unwrap();
    assert_eq!(t.origin, Origin::BottomRight);
    assert_eq!(pixels(&t), [GREEN, RED]);
}

#[test]
fn tga_with_an_image_id_and_a_colour_map_skips_both() {
    let mut b = tga_header(2, 1, 1, 24, 0x20);
    b[0] = 3; // image id length
    b[1] = 1; // a colour map, two 24-bit entries
    b[5] = 2;
    b[7] = 24;
    b.extend_from_slice(b"abc");
    b.extend_from_slice(&[9; 6]);
    b.extend_from_slice(&[255, 0, 0]);
    assert_eq!(pixels(&Tga::parse(&b).unwrap()), [BLUE]);
}

#[test]
fn tga_16_bit() {
    // 0x7c00 is red in 5-5-5; with an alpha bit declared and clear, the
    // pixel is transparent; without one it is opaque.
    let mut b = tga_header(2, 1, 1, 16, 0x21);
    b.extend_from_slice(&0x7c00u16.to_le_bytes());
    assert_eq!(pixels(&Tga::parse(&b).unwrap()), [[255, 0, 0, 0]]);
    let mut b = tga_header(2, 1, 1, 16, 0x20);
    b.extend_from_slice(&0x03e0u16.to_le_bytes());
    assert_eq!(pixels(&Tga::parse(&b).unwrap()), [GREEN]);
}

#[test]
fn tga_grey_raw_and_with_alpha() {
    let mut b = tga_header(3, 2, 1, 8, 0x28);
    b.extend_from_slice(&[0, 200]);
    let t = Tga::parse(&b).unwrap();
    assert_eq!(t.image_type, ImageType::Grey);
    assert_eq!(pixels(&t), [[0, 0, 0, 255], [200, 200, 200, 255]]);

    let mut b = tga_header(3, 1, 1, 16, 0x28);
    b.extend_from_slice(&[90, 30]);
    assert_eq!(pixels(&Tga::parse(&b).unwrap()), [[90, 90, 90, 30]]);
}

#[test]
fn tga_rle_runs_and_raw_packets() {
    // 4×1 true colour, 24-bit: a run of three red, then one raw white.
    let mut b = tga_header(10, 4, 1, 24, 0x20);
    b.extend_from_slice(&[0x82, 0, 0, 255, 0x00, 255, 255, 255]);
    let t = Tga::parse(&b).unwrap();
    assert_eq!(t.image_type, ImageType::TrueColorRle);
    assert_eq!(pixels(&t), [RED, RED, RED, WHITE]);

    // 32-bit, bottom-left, 2×2: one raw packet of two, then a run of two.
    let mut b = tga_header(10, 2, 2, 32, 8);
    b.extend_from_slice(&[0x01, 255, 0, 0, 255, 0, 255, 0, 255, 0x81, 0, 0, 255, 128]);
    let t = Tga::parse(&b).unwrap();
    assert_eq!(
        pixels(&t),
        [[255, 0, 0, 128], [255, 0, 0, 128], BLUE, GREEN]
    );

    // Grey RLE: a run of two.
    let mut b = tga_header(11, 2, 1, 8, 0x20);
    b.extend_from_slice(&[0x81, 77]);
    let t = Tga::parse(&b).unwrap();
    assert_eq!(t.image_type, ImageType::GreyRle);
    assert_eq!(pixels(&t), [[77, 77, 77, 255], [77, 77, 77, 255]]);
}

#[test]
fn tga_errors_carry_their_codes() {
    assert_eq!(
        Tga::parse(&[0; 10]).unwrap_err().code(),
        codes::TGA_TRUNCATED
    );

    let mut b = tga_header(2, 2, 2, 24, 0);
    b.extend_from_slice(&[0; 11]);
    assert_eq!(Tga::parse(&b).unwrap_err().code(), "RE2204");

    let mapped = Tga::parse(&tga_header(1, 1, 1, 8, 0)).unwrap_err();
    assert_eq!(
        mapped,
        Error::TgaUnsupported {
            image_type: 1,
            bits: 8
        }
    );
    assert_eq!(mapped.code(), "RE2205");
    assert_eq!(
        Tga::parse(&tga_header(2, 1, 1, 8, 0)).unwrap_err().code(),
        "RE2205"
    );

    let mut over = tga_header(10, 2, 1, 24, 0);
    over.extend_from_slice(&[0x82, 0, 0, 255]);
    assert_eq!(
        Tga::parse(&over).unwrap_err().code(),
        codes::TGA_RLE_OVERRUN
    );

    let mut cut = tga_header(10, 2, 1, 24, 0);
    cut.extend_from_slice(&[0x01, 0, 0]);
    assert_eq!(Tga::parse(&cut).unwrap_err().code(), "RE2204");
}
