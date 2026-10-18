//! Hand-made textures, built here, written, and read back: each writer
//! gives back the bytes it read, an image made from nothing reads back as
//! itself, and what a writer cannot write is refused with its code.

use rakata_texture::{
    ImageType, Layout, Origin, PixelFormat, Tga, TgaLayout, Tpc, Txi, TxiEntry, codes,
};

/// `bytes`, kept as a seed for the fuzz targets in the directory
/// `$RAKATA_FUZZ_SEEDS` names, when it is set.
fn seed(bytes: &[u8]) -> &[u8] {
    if let Some(dir) = std::env::var_os("RAKATA_FUZZ_SEEDS") {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{:016x}", h.finish())), bytes).unwrap();
    }
    bytes
}

/// A TPC header: data size, the float, width, height, encoding, mip count,
/// and 114 bytes of `reserved`.
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

fn filler(len: usize, seed: u8) -> Vec<u8> {
    (0..len).map(|i| seed.wrapping_add(i as u8)).collect()
}

fn tga_header(ty: u8, w: u16, h: u16, bits: u8, desc: u8) -> Vec<u8> {
    let mut b = vec![0, 0, ty, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    b.extend_from_slice(&w.to_le_bytes());
    b.extend_from_slice(&h.to_le_bytes());
    b.push(bits);
    b.push(desc);
    b
}

#[track_caller]
fn tpc_round_trip(b: &[u8]) -> Tpc<'_> {
    let t = Tpc::parse(seed(b)).unwrap();
    let written = t.to_bytes().unwrap();
    assert_eq!(written, b);
    let back = Tpc::parse(&written).unwrap();
    // A NaN float is compared by its bits, through the bytes above.
    if !t.alpha_mean.is_nan() {
        assert_eq!(back, t);
    }
    t
}

#[track_caller]
fn tga_round_trip(b: &[u8]) -> Tga {
    let t = Tga::parse(seed(b)).unwrap();
    let written = t.to_bytes().unwrap();
    assert_eq!(written, b);
    let back = Tga::parse(&written).unwrap();
    assert_eq!(back, t);
    assert_eq!(back.layout, t.layout);
    t
}

#[test]
fn tpc_with_its_txi_reserved_bytes_and_an_odd_float() {
    let mut b = tpc_header(32, 0.123_456_79, 8, 8, 2, 4);
    b[40] = 0xab;
    b[127] = 1;
    for (i, len) in [32, 8, 8, 8].iter().enumerate() {
        b.extend(filler(*len, i as u8 * 40));
    }
    b.extend_from_slice(b"mipmap 0\r\n\r\nblending  additive\r\n\0\0garbage\xff");
    let t = tpc_round_trip(&b);
    assert_eq!(t.reserved.len(), 114);
    assert_eq!(t.reserved[26], 0xab);
    assert_eq!(t.txi.as_ref().unwrap().value("blending"), Some("additive"));

    // A NaN with a payload, bit for bit.
    let mut b = tpc_header(0, 0.0, 2, 2, 4, 1);
    b[4..8].copy_from_slice(&0x7fc0_1234u32.to_le_bytes());
    b.extend(filler(16, 5));
    let t = tpc_round_trip(&b);
    assert_eq!(t.alpha_mean.to_bits(), 0x7fc0_1234);
    assert!(t.txi.is_none());
}

#[test]
fn tpc_uncompressed_cube_and_sheet() {
    for (enc, bpp) in [(1u8, 1usize), (2, 3), (4, 4)] {
        let mut b = tpc_header(0, 1.0, 4, 2, enc, 3);
        b.extend(filler((8 + 2 + 1) * bpp, enc));
        tpc_round_trip(&b);
    }

    let mut b = tpc_header(16, 1.0, 4, 24, 4, 3);
    b.extend(filler(6 * 48, 9));
    b.extend_from_slice(b"cube 1\r\n");
    assert_eq!(tpc_round_trip(&b).layout, Layout::Cube);

    // Four 8×8 DXT1 frames, each 8×8, 4×4, 2×2 and 1×1: 56 bytes.
    let mut b = tpc_header(4 * 56, 1.0, 16, 16, 2, 1);
    b.extend(filler(4 * 56, 2));
    b.extend_from_slice(b"proceduretype cycle\r\nnumx 2\r\nnumy 2\r\nfps 8");
    let t = tpc_round_trip(&b);
    assert_eq!(t.layout, Layout::Flipbook { numx: 2, numy: 2 });
}

#[test]
fn tpc_txi_edited_is_written_in_the_written_form() {
    let mut b = tpc_header(16, 1.0, 4, 4, 2, 1);
    b.extend(filler(8, 1));
    let body = b.clone();
    b.extend_from_slice(b"mipmap 0");
    let mut t = Tpc::parse(&b).unwrap();
    t.txi.as_mut().unwrap().entries.push(TxiEntry {
        key: "decal".into(),
        value: "1".into(),
        rows: vec![],
    });
    let mut want = body.clone();
    want.extend_from_slice(b"mipmap 0\r\ndecal 1");
    assert_eq!(t.to_bytes().unwrap(), want);

    t.txi = None;
    assert_eq!(t.to_bytes().unwrap(), body);

    let long = [0u8; 115];
    t.reserved = &long;
    assert_eq!(t.to_bytes().unwrap_err().code(), codes::TPC_UNWRITABLE);
    t.reserved = &[];
    assert_eq!(t.to_bytes().unwrap(), body);
}

#[test]
fn txi_round_trips_and_is_made_from_nothing() {
    for text in [
        &b"mipmap 0\r\nblending additive\r\n"[..],
        b"mipmap 0\nblending additive",
        b"\r\n3 grass 5.0 1.5\r\n",
        b"numchars 2\r\nupperleftcoords 2\r\n0.0 1.0 0\r\n0.5 1.0 0\r\n\r\n\r\n",
        b"Bumpyshinytexture   CM_Bright \t\r\n\0",
    ] {
        let t = Txi::parse(seed(text));
        let written = t.to_bytes().unwrap();
        assert_eq!(written, text);
        assert_eq!(Txi::parse(&written), t);
    }

    let made = Txi {
        preamble: vec!["3 grass".into()],
        entries: vec![
            TxiEntry {
                key: "mipmap".into(),
                value: "0".into(),
                rows: vec![],
            },
            TxiEntry {
                key: "channelscale".into(),
                value: "2".into(),
                rows: vec!["1.0".into(), "-0.5".into()],
            },
        ],
        ..Txi::default()
    };
    let written = made.to_bytes().unwrap();
    assert_eq!(
        written,
        b"3 grass\r\nmipmap 0\r\nchannelscale 2\r\n1.0\r\n-0.5\r\n"
    );
    assert_eq!(Txi::parse(&written), made);
}

#[test]
fn tga_every_variant_round_trips() {
    // 32-bit bottom-left with a footer, as the lightmaps are.
    let mut b = tga_header(2, 2, 2, 32, 8);
    b.extend(filler(16, 3));
    b.extend_from_slice(b"\0\0\0\0\0\0\0\0TRUEVISION-XFILE.\0");
    let t = tga_round_trip(&b);
    assert_eq!(t.layout.tail.len(), 26);

    // Each origin, 24-bit.
    for desc in [0x00, 0x10, 0x20, 0x30] {
        let mut b = tga_header(2, 3, 2, 24, desc);
        b.extend(filler(18, desc));
        tga_round_trip(&b);
    }

    // An image ID, a colour map, origin fields and descriptor bits 6 and 7.
    let mut b = tga_header(2, 1, 1, 24, 0xe0);
    b[0] = 3;
    b[1] = 1;
    b[3] = 4;
    b[5] = 2;
    b[7] = 24;
    b[8] = 7;
    b[11] = 1;
    b.extend_from_slice(b"abc");
    b.extend_from_slice(&[9; 6]);
    b.extend_from_slice(&[255, 0, 0]);
    b.extend_from_slice(b"not a footer");
    let t = tga_round_trip(&b);
    assert_eq!(t.layout.image_id, b"abc");
    assert_eq!(t.layout.color_map, [9; 6]);
    assert_eq!((t.layout.x_origin, t.layout.y_origin), (7, 256));
    assert_eq!(t.layout.descriptor_high, 0xc0);

    // 16-bit, with an alpha bit declared and without: the top bit
    // survives either way.
    for desc in [0x20, 0x21] {
        let mut b = tga_header(2, 2, 1, 16, desc);
        b.extend_from_slice(&0xfc00u16.to_le_bytes());
        b.extend_from_slice(&0x03e0u16.to_le_bytes());
        tga_round_trip(&b);
    }

    // Grey, 8 and 16 bits.
    let mut b = tga_header(3, 2, 1, 8, 0x08);
    b.extend_from_slice(&[0, 200]);
    tga_round_trip(&b);
    let mut b = tga_header(3, 1, 1, 16, 0x28);
    b.extend_from_slice(&[90, 30]);
    tga_round_trip(&b);
}

#[test]
fn tga_rle_keeps_its_packets() {
    // A run of two, then two raw pixels that could have been a run: the
    // packets as stored, not as a packer would choose.
    let mut b = tga_header(10, 4, 1, 24, 0x20);
    b.extend_from_slice(&[0x81, 1, 2, 3, 0x01, 4, 5, 6, 4, 5, 6]);
    let t = tga_round_trip(&b);
    assert_eq!(t.layout.packets, [0x81, 0x01]);

    // A packet across rows, a 32-bit run, a grey run, then a footer.
    let mut b = tga_header(10, 2, 2, 32, 8);
    b.extend_from_slice(&[0x82, 1, 2, 3, 4, 0x00, 9, 9, 9, 9]);
    b.extend_from_slice(b"\0\0\0\0\0\0\0\0TRUEVISION-XFILE.\0");
    tga_round_trip(&b);
    let mut b = tga_header(11, 3, 1, 8, 0x20);
    b.extend_from_slice(&[0x81, 77, 0x00, 78]);
    tga_round_trip(&b);
}

#[test]
fn tga_edited_rle_is_packed_anew() {
    let mut b = tga_header(10, 4, 1, 24, 0x20);
    b.extend_from_slice(&[0x83, 0, 0, 255]);
    let mut t = Tga::parse(&b).unwrap();
    // Red, red, green, red: the recorded run of four no longer fits.
    t.rgba[8..12].copy_from_slice(&[0, 255, 0, 255]);
    let written = t.to_bytes().unwrap();
    let mut want = tga_header(10, 4, 1, 24, 0x20);
    want.extend_from_slice(&[0x81, 0, 0, 255, 0x01, 0, 255, 0, 0, 0, 255]);
    assert_eq!(written, want);
    assert_eq!(Tga::parse(&written).unwrap(), t);
}

#[test]
fn tga_made_from_nothing_reads_back() {
    let rgba: Vec<u8> = (0..4 * 4 * 4).map(|i| (i * 7) as u8).collect();
    for (image_type, bits, alpha_bits) in [
        (ImageType::TrueColor, 32, 8),
        (ImageType::TrueColorRle, 32, 8),
        (ImageType::TrueColor, 24, 0),
        (ImageType::TrueColorRle, 24, 0),
    ] {
        for origin in [Origin::BottomLeft, Origin::TopLeft, Origin::TopRight] {
            let mut t = Tga {
                width: 4,
                height: 4,
                image_type,
                bits,
                alpha_bits,
                origin,
                rgba: rgba.clone(),
                layout: TgaLayout::default(),
            };
            if bits == 24 {
                for px in t.rgba.as_chunks_mut::<4>().0 {
                    px[3] = 255;
                }
            }
            let back = Tga::parse(&t.to_bytes().unwrap()).unwrap();
            assert_eq!(back, t, "{image_type:?} {bits} {origin:?}");
        }
    }
}

#[test]
fn tga_writer_refusals() {
    let base = Tga {
        width: 2,
        height: 2,
        image_type: ImageType::TrueColor,
        bits: 32,
        alpha_bits: 8,
        origin: Origin::BottomLeft,
        rgba: vec![0; 16],
        layout: TgaLayout::default(),
    };
    let mut t = base.clone();
    t.rgba.truncate(12);
    assert_eq!(t.to_bytes().unwrap_err().code(), codes::TGA_UNWRITABLE);
    let mut t = base.clone();
    t.layout.image_id = vec![0; 256];
    assert_eq!(t.to_bytes().unwrap_err().code(), codes::TGA_UNWRITABLE);
    let mut t = base.clone();
    t.alpha_bits = 16;
    assert_eq!(t.to_bytes().unwrap_err().code(), codes::TGA_UNWRITABLE);
    let mut t = base.clone();
    t.layout.descriptor_high = 0x20;
    assert_eq!(t.to_bytes().unwrap_err().code(), codes::TGA_UNWRITABLE);
    let mut t = base.clone();
    t.bits = 8;
    assert_eq!(t.to_bytes().unwrap_err().code(), codes::TGA_UNSUPPORTED);
}

#[test]
fn a_dxt_level_stays_borrowed_through_a_write() {
    let mut b = tpc_header(64, 0.5, 8, 8, 4, 1);
    b.extend(filler(64, 0));
    let t = Tpc::parse(&b).unwrap();
    assert_eq!(t.format, PixelFormat::Dxt5);
    assert!(std::ptr::eq(t.pixels.as_ptr(), b[128..].as_ptr()));
    assert_eq!(t.to_bytes().unwrap(), b);
}
