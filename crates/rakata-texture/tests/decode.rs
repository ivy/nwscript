//! Textures decoded to RGBA8 and encoded as PNG, over hand-made files:
//! where each image of a cube or an animation sheet lands, which levels
//! are written, and the errors for a level or an image the texture lacks.

use rakata_texture::{Levels, Tga, Tpc, codes};

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

/// A DXT1 block whose every texel is the RGB565 colour `c`.
fn solid_dxt1(c: u16) -> [u8; 8] {
    let [lo, hi] = c.to_le_bytes();
    [lo, hi, 0, 0, 0, 0, 0, 0]
}

/// A DXT5 block of colour `c` and alpha `a` at every texel.
fn solid_dxt5(c: u16, a: u8) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0] = a;
    b[8..].copy_from_slice(&solid_dxt1(c));
    b
}

/// `n` blocks of `block`.
fn blocks<const N: usize>(block: [u8; N], n: usize) -> Vec<u8> {
    block.iter().copied().cycle().take(N * n).collect()
}

/// The PNG's size and RGBA8 pixels.
fn read_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

fn pixel(px: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    px[at..at + 4].try_into().unwrap()
}

/// RGB565 colours with their 8-bit forms: red, green, blue, white, and
/// two greys, one per cube face.
const FACES: [(u16, [u8; 3]); 6] = [
    (0xf800, [255, 0, 0]),
    (0x07e0, [0, 255, 0]),
    (0x001f, [0, 0, 255]),
    (0xffff, [255, 255, 255]),
    (0x8410, [132, 130, 132]),
    (0x4208, [66, 65, 66]),
];

#[test]
fn a_dxt1_texture_exports_each_level_at_its_size() {
    // 8×4, two levels: 8×4 (two blocks) and 4×2 (one).
    let mut b = tpc_header(16, 1.0, 8, 4, 2, 2);
    b.extend(solid_dxt1(0xf800));
    b.extend(solid_dxt1(0x001f));
    b.extend(solid_dxt1(0x07e0));
    let t = Tpc::parse(&b).unwrap();

    let largest = t.to_pngs(Levels::Largest, None).unwrap();
    assert_eq!(largest.len(), 1);
    let (w, h, px) = read_png(&largest[0].bytes);
    assert_eq!((w, h), (8, 4));
    assert_eq!(pixel(&px, w, 0, 3), [255, 0, 0, 255]);
    assert_eq!(pixel(&px, w, 7, 0), [0, 0, 255, 255]);

    let all = t.to_pngs(Levels::All, None).unwrap();
    assert_eq!(
        all.iter()
            .map(|p| (p.level, p.width, p.height))
            .collect::<Vec<_>>(),
        [(0, 8, 4), (1, 4, 2)]
    );
    let (w, h, px) = read_png(&all[1].bytes);
    assert_eq!((w, h), (4, 2));
    assert!(px.chunks(4).all(|p| p == [0, 255, 0, 255]));

    let one = t.to_pngs(Levels::One(1), None).unwrap();
    assert_eq!(one[0].bytes, all[1].bytes);

    let e = t.to_pngs(Levels::One(2), None).unwrap_err();
    assert_eq!(e.code(), codes::LEVEL_MISSING);
    assert_eq!(
        e.to_string(),
        "RE2210: mip level 2 asked for; the texture has 2, 0 to 1"
    );
    let e = t.to_pngs(Levels::Largest, Some(1)).unwrap_err();
    assert_eq!(e.code(), codes::IMAGE_MISSING);
}

#[test]
fn a_dxt5_texture_keeps_its_alpha() {
    let mut b = tpc_header(16, 0.5, 4, 4, 4, 1);
    b.extend(solid_dxt5(0xffff, 128));
    let t = Tpc::parse(&b).unwrap();
    let (_, _, px) = read_png(&t.to_pngs(Levels::Largest, None).unwrap()[0].bytes);
    assert!(px.chunks(4).all(|p| p == [255, 255, 255, 128]));
}

#[test]
fn a_cube_map_stacks_its_faces_top_to_bottom_in_stored_order() {
    // Six 4×4 DXT5 faces of two levels (4×4, 2×2): one block each.
    let mut b = tpc_header(16, 1.0, 4, 24, 4, 2);
    for (c, _) in FACES {
        b.extend(solid_dxt5(c, 255));
        b.extend(solid_dxt5(c, 255));
    }
    b.extend_from_slice(b"cube 1\r\n");
    let t = Tpc::parse(&b).unwrap();
    for (level, side) in [(0, 4), (1, 2)] {
        let img = t.decode_level(level).unwrap();
        assert_eq!((img.width, img.height), (side, 6 * side));
        for (face, (_, rgb)) in FACES.iter().enumerate() {
            let y = face as u32 * side + side - 1;
            let want = [rgb[0], rgb[1], rgb[2], 255];
            assert_eq!(pixel(&img.pixels, side, side - 1, y), want, "face {face}");
        }
    }
    let face = t.decode_image(4, 0).unwrap();
    assert_eq!((face.width, face.height), (4, 4));
    assert_eq!(&face.pixels[..4], &[132, 130, 132, 255]);
    let pngs = t.to_pngs(Levels::Largest, Some(5)).unwrap();
    assert_eq!((pngs[0].width, pngs[0].height), (4, 4));
    assert_eq!(
        t.decode_image(6, 0).unwrap_err().to_string(),
        "RE2211: image 6 asked for; the texture has 6, 0 to 5"
    );
}

#[test]
fn an_animation_sheet_lays_its_frames_out_across_then_down() {
    // 16×8 DXT1 as 2×1 frames of 8×8; each frame 8×8 (4 blocks), 4×4,
    // 2×2, 1×1 (one each).
    let per_frame = 4 + 1 + 1 + 1;
    let mut b = tpc_header(2 * per_frame * 8, 1.0, 16, 8, 2, 1);
    b.extend(blocks(solid_dxt1(0xf800), per_frame as usize));
    b.extend(blocks(solid_dxt1(0x001f), per_frame as usize));
    b.extend_from_slice(b"proceduretype cycle\r\nnumx 2\r\nnumy 1\r\n");
    let t = Tpc::parse(&b).unwrap();
    let sheet = t.decode_level(0).unwrap();
    assert_eq!((sheet.width, sheet.height), (16, 8));
    assert_eq!(pixel(&sheet.pixels, 16, 7, 7), [255, 0, 0, 255]);
    assert_eq!(pixel(&sheet.pixels, 16, 8, 0), [0, 0, 255, 255]);
    let small = t.decode_level(3).unwrap();
    assert_eq!((small.width, small.height), (2, 1));
    assert_eq!(t.to_pngs(Levels::All, None).unwrap().len(), 4);
}

#[test]
fn uncompressed_levels_export_as_stored() {
    // 2×1 RGB, one level.
    let mut b = tpc_header(0, 1.0, 2, 1, 2, 1);
    b.extend([10, 20, 30, 40, 50, 60]);
    let t = Tpc::parse(&b).unwrap();
    let (w, h, px) = read_png(&t.to_pngs(Levels::Largest, None).unwrap()[0].bytes);
    assert_eq!((w, h), (2, 1));
    assert_eq!(px, [10, 20, 30, 255, 40, 50, 60, 255]);
}

#[test]
fn a_tga_exports_its_rgba() {
    // 2 × 1, 32-bit, top left: BGRA pixels.
    let mut b = vec![0u8; 18];
    b[2] = 2;
    b[12..14].copy_from_slice(&2u16.to_le_bytes());
    b[14..16].copy_from_slice(&1u16.to_le_bytes());
    b[16] = 32;
    b[17] = 0x28;
    b.extend([3, 2, 1, 9, 30, 20, 10, 90]);
    let t = Tga::parse(&b).unwrap();
    let (w, h, px) = read_png(&t.to_png().unwrap());
    assert_eq!((w, h), (2, 1));
    assert_eq!(px, [1, 2, 3, 9, 10, 20, 30, 90]);
}
