//! `rakata texture`, `lyt`, `walkmesh` and `mdl` over fixtures built here,
//! and one vanilla instance of each read by name when `$RAKATA_GAME_K1`
//! names an install.

use assert_cmd::Command;
use rakata::lyt::{DoorHook, Lyt, Placement, Quaternion, Vis, VisRoom};
use rakata::mdl::{
    AnimationFile, Body, MdxBlock, MeshFile, MeshKind, ModelFile, NodeFile, NodePointer,
    controller_types, mdx_flags,
};
use rakata::twoda::TwoDa;
use rakata::walkmesh::{AabbNode, EdgeRef, Face, Kind, PerimeterEdge, Vertices, Walkmesh};
use rakata_archive::Key;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// The text a command prints, after checking its exit code.
fn text(args: &[&str], code: i32) -> String {
    let out = rakata().args(args).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(code),
        "{args:?}\n{stdout}\n{stderr}"
    );
    format!("{stdout}{stderr}")
}

fn json(args: &[&str]) -> (Option<i32>, Json) {
    let out = rakata()
        .arg("--format")
        .arg("json")
        .args(args)
        .output()
        .unwrap();
    let doc: Json = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stderr)));
    (out.status.code(), doc)
}

fn codes(doc: &Json) -> Vec<String> {
    doc["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap().to_owned())
        .collect()
}

// Textures.

/// A DXT1 TPC of 8 × 8 pixels with its full chain of four levels, and
/// `txi` after the pixels.
fn tpc(txi: &str) -> Vec<u8> {
    let mut b = vec![0u8; 128];
    b[0..4].copy_from_slice(&32u32.to_le_bytes());
    b[4..8].copy_from_slice(&1.0f32.to_le_bytes());
    b[8..10].copy_from_slice(&8u16.to_le_bytes());
    b[10..12].copy_from_slice(&8u16.to_le_bytes());
    b[12] = 2;
    b[13] = 4;
    // 8x8 is four blocks; 4x4, 2x2 and 1x1 are one each.
    b.extend([0x11; 32 + 8 + 8 + 8]);
    b.extend_from_slice(txi.as_bytes());
    b
}

/// A 2 × 2 true-colour TGA with eight alpha bits, origin top left.
fn tga() -> Vec<u8> {
    let mut b = vec![0u8; 18];
    b[2] = 2;
    b[12] = 2;
    b[14] = 2;
    b[16] = 32;
    b[17] = 0x28;
    b.extend([7; 16]);
    b
}

// Layouts.

fn lyt() -> Vec<u8> {
    let mut l = Lyt::new(Some("area.max".into()));
    l.rooms = vec![
        Placement::new("area_01a", [0.0, 0.0, 0.0]),
        Placement::new("area_02a", [10.5, -2.0, 0.0]),
    ];
    l.door_hooks = vec![DoorHook {
        room: "area_01a".into(),
        name: "door_01".into(),
        unknown: 0,
        position: [1.0, 2.0, 0.0],
        orientation: Quaternion::from_array([1.0, 0.0, 0.0, 0.0]),
    }];
    l.to_bytes().unwrap()
}

fn vis() -> Vec<u8> {
    Vis::new(vec![
        VisRoom {
            room: "area_01a".into(),
            visible: vec!["area_02a".into()],
        },
        VisRoom {
            room: "area_02a".into(),
            visible: vec!["area_01a".into()],
        },
    ])
    .to_bytes()
    .unwrap()
}

// Walkmeshes.

/// A WOK of one walkable face of material 4, its second edge leading into
/// room 3; its vertices span (0, 0, 0) to (2, 2, 0).
fn wok() -> Vec<u8> {
    let v: Vec<u8> = [[0.0f32, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]]
        .iter()
        .flatten()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    Walkmesh {
        kind: Kind::Area,
        relative_points: [[0.0; 3]; 2],
        absolute_points: [[0.0; 3]; 2],
        position: [0.0; 3],
        vertices: Vertices::from_bytes(&v).unwrap(),
        faces: vec![Face {
            vertices: [0, 1, 2],
            material: 4,
            normal: [0.0, 0.0, 1.0],
            distance: 0.0,
        }],
        adjacency: vec![[None; 3]],
        perimeter_edges: (0..3)
            .map(|e| PerimeterEdge {
                edge: EdgeRef { face: 0, edge: e },
                transition: (e == 1).then_some(3),
            })
            .collect(),
        perimeters: vec![3],
        aabb: vec![AabbNode {
            min: [0.0; 3],
            max: [2.0, 2.0, 0.0],
            face: Some(0),
            unknown: 4,
            plane: 0,
            left: None,
            right: None,
        }],
        unknown: 0,
        trailer: &[],
    }
    .to_bytes()
    .unwrap()
}

fn surfacemat() -> Vec<u8> {
    let mut t = TwoDa::new(["label", "walk"]);
    for (i, (label, walk)) in [
        ("Undefined", "0"),
        ("Dirt", "1"),
        ("Obscuring", "0"),
        ("Grass", "1"),
        ("Stone", "1"),
    ]
    .iter()
    .enumerate()
    {
        t.add_row(i.to_string(), &[("label", label), ("walk", walk)])
            .unwrap();
    }
    t.to_bytes().unwrap()
}

// Models.

fn put_u16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_f32s(b: &mut [u8], at: usize, v: &[f32]) {
    for (i, x) in v.iter().enumerate() {
        put_u32(b, at + 4 * i, x.to_bits());
    }
}

fn put_str(b: &mut [u8], at: usize, s: &str) {
    b[at..at + 32].fill(0);
    b[at..at + s.len()].copy_from_slice(s.as_bytes());
}

fn floats(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A node of `flags` and `number`, with position and orientation
/// controllers repeating its transform, as the geometry's nodes have.
fn node(flags: u16, number: u16, body: Body) -> NodeFile {
    let mut header = [0u8; 80];
    put_u16(&mut header, 0, flags);
    put_u16(&mut header, 2, number);
    put_u16(&mut header, 4, number);
    put_f32s(&mut header, 28, &[1.0, 0.0, 0.0, 0.0]);
    let mut controllers = Vec::new();
    for (kind, first, columns) in [
        (controller_types::POSITION, 0u16, 3u8),
        (controller_types::ORIENTATION, 4, 4),
    ] {
        let mut c = [0u8; 16];
        put_u32(&mut c, 0, kind);
        put_u16(&mut c, 4, 0xFFFF);
        put_u16(&mut c, 6, 1);
        put_u16(&mut c, 8, first);
        put_u16(&mut c, 10, first + 1);
        c[12] = columns;
        controllers.push(c);
    }
    NodeFile {
        header,
        body,
        children: Vec::new(),
        controllers,
        controller_data: floats(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
    }
}

/// A triangle mesh of four vertices and one face, textured `texture`.
fn triangle(texture: &str) -> Body {
    let mut h = [0u8; 332];
    put_u32(&mut h, 0, 0x0040_5750);
    put_u32(&mut h, 4, 0x0040_5760);
    put_f32s(&mut h, 60, &[0.8, 0.8, 0.8, 0.2, 0.2, 0.2]);
    put_str(&mut h, 88, texture);
    put_u32(&mut h, 212, u32::MAX);
    put_u32(&mut h, 216, u32::MAX);
    put_u32(&mut h, 224, 3);
    put_u32(&mut h, 252, 24);
    put_u32(&mut h, 256, mdx_flags::POSITION | mdx_flags::NORMAL);
    h[260..304].fill(0xFF);
    put_u32(&mut h, 260, 0);
    put_u32(&mut h, 264, 12);
    put_u16(&mut h, 304, 4);
    put_u16(&mut h, 306, 1);
    h[313] = 1;
    let positions = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let mut face = [0u8; 32];
    put_f32s(&mut face, 0, &[0.0, 0.0, 1.0, 0.0]);
    for k in 0..3 {
        put_u16(&mut face, 20 + 2 * k, u16::MAX);
        put_u16(&mut face, 26 + 2 * k, k as u16);
    }
    let mut vertices = Vec::new();
    for p in positions {
        vertices.extend(floats(&p));
        vertices.extend(floats(&[0.0, 0.0, 1.0]));
    }
    Body::Mesh(Box::new(MeshFile {
        header: h,
        kind: MeshKind::Trimesh,
        faces: face.to_vec(),
        index_counts: vec![3],
        index_lists: vec![[0u16, 1, 2].iter().flat_map(|i| i.to_le_bytes()).collect()],
        index_unknown: vec![1],
        positions: floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0]),
        mdx: Some(MdxBlock {
            vertices,
            terminator: floats(&[1.0e7, 1.0e7, 1.0e7, 0.0, 0.0, 0.0]),
            padding: Vec::new(),
        }),
    }))
}

/// A model `name` naming `supermodel`: a root with a mesh `body` textured
/// `tex_box` and a dummy `hook`, or with `wave` a root alone that has an
/// animation `wave` of 1.5 seconds.
fn model(name: &str, supermodel: &str, wave: bool) -> (Vec<u8>, Vec<u8>) {
    let names: Vec<&str> = if wave {
        vec![name]
    } else {
        vec![name, "body", "hook"]
    };
    let mut header = [0u8; 0xC4];
    put_u32(&mut header, 0, 0x0041_3670);
    put_u32(&mut header, 4, 0x0040_5520);
    put_str(&mut header, 8, name);
    put_u32(&mut header, 0x2C, names.len() as u32);
    header[0x4C] = 2;
    header[0x50] = 4;
    header[0x53] = 1;
    put_f32s(
        &mut header,
        0x68,
        &[-5.0, -5.0, -1.0, 5.0, 5.0, 10.0, 7.0, 1.0],
    );
    put_str(&mut header, 0x88, supermodel);
    let mut root = node(0x001, 0, Body::Dummy);
    let mut animations = Vec::new();
    if wave {
        let mut h = [0u8; 0x88];
        put_u32(&mut h, 0, 0x0041_34F0);
        put_u32(&mut h, 4, 0x0043_ECE0);
        put_str(&mut h, 8, "wave");
        put_u32(&mut h, 0x2C, 1);
        h[0x4C] = 5;
        put_f32s(&mut h, 0x50, &[1.5, 0.25]);
        put_str(&mut h, 0x58, name);
        let mut anim_root = node(0x001, 0, Body::Dummy);
        anim_root.controllers.clear();
        anim_root.controller_data.clear();
        animations.push(AnimationFile {
            header: h,
            events: Vec::new(),
            root: anim_root,
        });
    } else {
        root.children = vec![
            node(0x021, 1, triangle("tex_box")),
            node(0x001, 2, Body::Dummy),
        ];
    }
    ModelFile {
        header,
        head_node: NodePointer::Node(0),
        names: names.iter().map(|n| n.as_bytes().to_vec()).collect(),
        animations,
        root,
    }
    .to_bytes()
    .unwrap()
}

/// A temporary install whose Override holds one of each format, and a
/// folder `loose/` beside it holding `box.mdl` and `base.mdl` with their
/// MDXs, and `bare.mdl` without one.
struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let game = root.join("game");
        std::fs::create_dir_all(game.join("Override")).unwrap();
        std::fs::create_dir_all(root.join("loose")).unwrap();
        std::fs::write(game.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
        let (box_mdl, box_mdx) = model("box", "base", false);
        let (base_mdl, base_mdx) = model("base", "NULL", true);
        let files: [(&str, Vec<u8>); 12] = [
            ("t.tpc", tpc("envmaptexture cm_bright\r\nmipmap 0\r\n")),
            (
                "t.txi",
                b"blending additive\r\nchannelscale 2\r\n0.5\r\n1\r\n".to_vec(),
            ),
            ("lm.tga", tga()),
            ("area.lyt", lyt()),
            ("area.vis", vis()),
            ("room.wok", wok()),
            ("surfacemat.2da", surfacemat()),
            ("box.mdl", box_mdl.clone()),
            ("box.mdx", box_mdx.clone()),
            ("base.mdl", base_mdl.clone()),
            ("base.mdx", base_mdx.clone()),
            ("odd.utc", b"UTC V3.2".to_vec()),
        ];
        for (n, b) in files {
            std::fs::write(game.join("Override").join(n), b).unwrap();
        }
        for (n, b) in [
            ("box.mdl", &box_mdl),
            ("box.mdx", &box_mdx),
            ("base.mdl", &base_mdl),
            ("base.mdx", &base_mdx),
            ("bare.mdl", &box_mdl),
        ] {
            std::fs::write(root.join("loose").join(n), b).unwrap();
        }
        Fixture { dir }
    }

    fn game(&self) -> PathBuf {
        self.dir.path().join("game")
    }

    fn path(&self, p: &str) -> String {
        self.dir.path().join(p).display().to_string()
    }
}

#[test]
fn every_asset_help_runs_and_has_an_example() {
    for cmd in ["texture", "lyt", "walkmesh", "mdl"] {
        for args in [&[cmd, "--help"][..], &[cmd, "show", "--help"]] {
            let out = rakata().args(args).output().unwrap();
            assert_eq!(out.status.code(), Some(0), "{args:?}");
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(text.contains("Example"), "{args:?} --help has no example");
        }
    }
}

#[test]
fn texture_show_prints_a_tpcs_levels_and_txi_a_txi_and_a_tga() {
    let f = Fixture::new();
    let g = f.game();
    let t = text(&["--game", s(&g), "texture", "show", "t.tpc"], 0);
    assert!(t.starts_with("tpc 8x8 dxt1, one image, 4 levels\n"), "{t}");
    assert!(t.contains("\n  0  8x8  32 bytes  at 128\n"), "{t}");
    assert!(t.contains("\n  3  1x1  8 bytes  at 176\n"), "{t}");
    assert!(
        t.contains("\ntxi\n  envmaptexture cm_bright\n  mipmap 0"),
        "{t}"
    );

    let (code, doc) = json(&["--game", s(&g), "texture", "show", "t.tpc"]);
    assert_eq!(code, Some(0));
    let r = &doc["result"];
    assert_eq!(r["source"], "Override/t.tpc");
    assert_eq!(r["pixel_format"], "dxt1");
    assert_eq!(r["layout"]["kind"], "single");
    assert_eq!(r["images"][0].as_array().unwrap().len(), 4);
    assert_eq!(r["images"][0][1]["width"], 4);
    assert_eq!(r["txi"]["entries"][0]["value"], "cm_bright");
    assert_eq!(doc["inputs"][0]["role"], "resource");

    let t = text(&["--game", s(&g), "texture", "show", "t.txi"], 0);
    assert_eq!(
        t,
        "txi, 2 instructions\nblending additive\nchannelscale 2  (2 rows)\n"
    );
    let (_, doc) = json(&["--game", s(&g), "texture", "show", "t.txi"]);
    assert_eq!(doc["result"]["entries"][1]["rows"][0], "0.5");

    let t = text(&["--game", s(&g), "texture", "show", "lm.tga"], 0);
    assert_eq!(
        t,
        "tga 2x2 true colour, 32 bits, 8 alpha bits, origin top left\n"
    );
    let (_, doc) = json(&["--game", s(&g), "texture", "show", "lm.tga"]);
    assert_eq!(doc["result"]["image_type"], 2);
    assert_eq!(doc["result"]["origin"], "top left");
}

#[test]
fn a_resource_of_another_format_is_re1307_naming_its_command() {
    let f = Fixture::new();
    let g = f.game();
    let (code, doc) = json(&["--game", s(&g), "texture", "show", "area.lyt"]);
    assert_eq!(code, Some(1));
    assert_eq!(codes(&doc), ["RE1307"]);
    let help = doc["diagnostics"][0]["help"].as_str().unwrap();
    assert!(help.contains("`rakata lyt show`"), "{help}");
    for (cmd, name) in [
        ("lyt", "t.tpc"),
        ("walkmesh", "box.mdl"),
        ("mdl", "odd.utc"),
    ] {
        let (code, doc) = json(&["--game", s(&g), cmd, "show", name]);
        assert_eq!(code, Some(1), "{cmd}");
        assert_eq!(codes(&doc), ["RE1307"], "{cmd}");
    }
}

/// A PNG's size and its RGBA8 pixels.
fn read_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let file = std::io::BufReader::new(std::fs::File::open(path).unwrap());
    let mut reader = png::Decoder::new(file).read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

#[test]
fn texture_export_writes_a_png_and_the_txi_beside_it() {
    let f = Fixture::new();
    let g = f.game();
    let out = f.path("out");
    std::fs::create_dir(&out).unwrap();
    let png = format!("{out}/t.png");
    let t = text(
        &["--game", s(&g), "texture", "export", "t.tpc", "-o", &png],
        0,
    );
    assert_eq!(
        t,
        format!("wrote {out}/t.png  8x8, level 0 of 4\nwrote {out}/t.txi  from the TPC\n")
    );
    // Every block is 0x1111 twice: RGB565 (2, 8, 17) in the three-colour
    // mode, codes 0 and 1 only.
    let (w, h, px) = read_png(Path::new(&png));
    assert_eq!((w, h), (8, 8));
    assert!(px.chunks(4).all(|p| p == [16, 32, 140, 255]), "{px:?}");
    assert_eq!(
        std::fs::read(format!("{out}/t.txi")).unwrap(),
        b"envmaptexture cm_bright\r\nmipmap 0\r\n"
    );

    let (code, doc) = json(&[
        "--game",
        s(&g),
        "texture",
        "export",
        "t.tpc",
        "-o",
        &format!("{out}/all.png"),
        "--all-levels",
    ]);
    assert_eq!(code, Some(0), "{doc}");
    let r = &doc["result"];
    let pngs = r["pngs"].as_array().unwrap();
    assert_eq!(pngs.len(), 4);
    assert_eq!(pngs[3]["path"], format!("{out}/all.3.png"));
    assert_eq!(pngs[3]["width"], 1);
    assert_eq!(r["txi"]["embedded"], true);
    assert_eq!(read_png(Path::new(&format!("{out}/all.2.png"))).0, 2);
    assert_eq!(doc["outputs"].as_array().unwrap().len(), 5);

    // A TGA takes its TXI from beside it in the install.
    std::fs::write(g.join("Override/lm.txi"), b"islightmap 1\r\n").unwrap();
    let (code, doc) = json(&[
        "--game",
        s(&g),
        "texture",
        "export",
        "lm.tga",
        "-o",
        &format!("{out}/lm.png"),
    ]);
    assert_eq!(code, Some(0), "{doc}");
    assert_eq!(doc["result"]["txi"]["source"], "Override/lm.txi");
    assert_eq!(doc["result"]["txi"]["embedded"], false);
    let (w, h, px) = read_png(Path::new(&format!("{out}/lm.png")));
    assert_eq!((w, h, px), (2, 2, vec![7; 16]));
    assert_eq!(
        std::fs::read(format!("{out}/lm.txi")).unwrap(),
        b"islightmap 1\r\n"
    );

    for (args, code) in [
        (&["t.tpc", "--level", "4"][..], "RE2210"),
        (&["t.tpc", "--image", "1"], "RE2211"),
        (&["lm.tga", "--level", "1"], "RE2210"),
        (&["t.txi"], "RE1307"),
    ] {
        let mut all = vec!["--game", s(&g), "texture", "export"];
        all.extend_from_slice(args);
        all.extend(["-o", &png]);
        let (exit, doc) = json(&all);
        assert_eq!(exit, Some(1), "{args:?}");
        assert_eq!(codes(&doc), [code], "{args:?}");
    }
    let (exit, doc) = json(&[
        "--game",
        s(&g),
        "texture",
        "export",
        "t.tpc",
        "-o",
        &f.path("missing/t.png"),
    ]);
    assert_eq!(exit, Some(1));
    assert_eq!(codes(&doc), ["RE1306"]);
}

#[test]
fn lyt_show_prints_rooms_and_door_hooks_and_a_vis_the_rooms_seen() {
    let f = Fixture::new();
    let g = f.game();
    let t = text(&["--game", s(&g), "lyt", "show", "area.lyt"], 0);
    assert_eq!(
        t,
        "layout area.max: 2 rooms, 0 tracks, 0 obstacles, 1 door hook\n\
         rooms\n  area_01a  0.0 0.0 0.0\n  area_02a  10.5 -2.0 0.0\n\
         door hooks\n  area_01a door_01  1.0 2.0 0.0  orientation 1.0 0.0 0.0 0.0\n"
    );
    let (_, doc) = json(&["--game", s(&g), "lyt", "show", "area.lyt"]);
    let r = &doc["result"];
    assert_eq!(r["rooms"][1]["model"], "area_02a");
    assert_eq!(r["rooms"][1]["position"][0], 10.5);
    assert_eq!(r["door_hooks"][0]["name"], "door_01");
    assert_eq!(r["tracks"].as_array().unwrap().len(), 0);

    let t = text(&["--game", s(&g), "lyt", "show", "area.vis"], 0);
    assert_eq!(
        t,
        "visibility: 2 rooms\narea_01a  sees 1: area_02a\narea_02a  sees 1: area_01a\n"
    );
    let (_, doc) = json(&["--game", s(&g), "lyt", "show", "area.vis"]);
    assert_eq!(doc["result"]["rooms"][0]["visible"][0], "area_02a");
}

#[test]
fn walkmesh_show_labels_materials_through_the_install_and_lists_transitions() {
    let f = Fixture::new();
    let g = f.game();
    let t = text(&["--game", s(&g), "walkmesh", "show", "room.wok"], 0);
    assert_eq!(
        t,
        "walkmesh wok (area room), 3 vertices, 1 face (1 walkable), 3 perimeter edges in 1 loop, 1 AABB node\n\
         bounds (0, 0, 0) to (2, 2, 0)\n\
         materials\n  4 Stone walkable  1 face\n\
         transitions\n  room 3  1 edge\n"
    );
    let (code, doc) = json(&["--game", s(&g), "walkmesh", "show", "room.wok"]);
    assert_eq!(code, Some(0));
    let r = &doc["result"];
    assert_eq!(r["kind"], "area");
    assert_eq!(r["walkable_faces"], 1);
    assert_eq!(r["materials"][0]["label"], "Stone");
    assert_eq!(r["materials"][0]["walkable"], true);
    assert_eq!(r["transitions"][0]["room"], 3);
    assert_eq!(r["bounds"]["max"][1], 2.0);
    assert_eq!(doc["inputs"][1]["role"], "surfacemat");

    // A path without an install: the materials by number.
    let wok = g.join("Override/room.wok");
    let (code, doc) = json(&["walkmesh", "show", s(&wok)]);
    assert_eq!(code, Some(0));
    assert!(doc["result"]["materials"][0]["label"].is_null());
    assert!(doc["result"]["walkable_faces"].is_null());

    // An install without the table: RW1301, and the rest shown.
    std::fs::remove_file(g.join("Override/surfacemat.2da")).unwrap();
    let (code, doc) = json(&["--game", s(&g), "walkmesh", "show", "room.wok"]);
    assert_eq!(code, Some(0));
    assert_eq!(codes(&doc), ["RW1301"]);
    assert_eq!(doc["result"]["faces"], 1);
}

#[test]
fn mdl_show_reads_the_mdx_and_resolves_the_supermodel_through_the_install() {
    let f = Fixture::new();
    let g = f.game();
    let t = text(&["--game", s(&g), "mdl", "show", "box.mdl"], 0);
    assert!(
        t.starts_with("model box: character, supermodel base, 3 nodes (1 mesh), 0 animations\n"),
        "{t}"
    );
    assert!(
        t.contains("\nnodes\n  box  dummy\n    body  trimesh  texture tex_box  4 vertices  1 face\n    hook  dummy\n"),
        "{t}"
    );
    assert!(
        t.contains("\nsupermodels\n  base  Override/base.mdl\n"),
        "{t}"
    );
    assert!(
        t.contains("\ninherited animations\n  wave  from base"),
        "{t}"
    );

    let (code, doc) = json(&["--game", s(&g), "mdl", "show", "box.mdl"]);
    assert_eq!(code, Some(0), "{doc}");
    let r = &doc["result"];
    assert_eq!(r["mdx"], "Override/box.mdx");
    assert_eq!(r["nodes"][1]["kind"], "trimesh");
    assert_eq!(r["nodes"][1]["parent"], 0);
    assert_eq!(r["nodes"][1]["mesh"]["texture"], "tex_box");
    assert_eq!(r["nodes"][1]["mesh"]["vertices"], 4);
    assert_eq!(r["supermodels"][0]["name"], "base");
    assert_eq!(r["inherited_animations"][0]["name"], "wave");
    let roles: Vec<&str> = doc["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["resource", "mdx", "supermodel", "supermodel mdx"]);

    // The supermodel's own animation, and the MDX named in place of the MDL.
    let t = text(&["--game", s(&g), "mdl", "show", "base.mdx"], 0);
    assert!(t.contains("\nanimations\n  wave  1.5 s  1 node"), "{t}");
}

#[test]
fn mdl_show_finds_the_mdx_and_supermodel_beside_a_path_and_warns_without() {
    let f = Fixture::new();
    let (code, doc) = json(&["mdl", "show", &f.path("loose/box.mdl")]);
    assert_eq!(code, Some(0), "{doc}");
    assert!(codes(&doc).is_empty(), "{doc}");
    assert!(
        doc["result"]["supermodels"][0]["source"]
            .as_str()
            .unwrap()
            .ends_with("loose/base.mdl")
    );

    // bare.mdl has no MDX beside it, and so no vertices to read.
    let (code, doc) = json(&["mdl", "show", &f.path("loose/bare.mdl")]);
    assert_eq!(code, Some(1));
    assert_eq!(codes(&doc), ["RW1302", "RE2403"]);

    // Without base beside it, the chain ends at the model.
    std::fs::remove_file(f.path("loose/base.mdl")).unwrap();
    let (code, doc) = json(&["mdl", "show", &f.path("loose/box.mdl")]);
    assert_eq!(code, Some(0));
    assert_eq!(codes(&doc), ["RW1303"]);
    assert_eq!(doc["result"]["missing_supermodel"], "base");
}

#[test]
fn every_new_code_is_explained() {
    for code in ["RE1307", "RW1301", "RW1302", "RW1303"] {
        let t = text(&["explain", code], 0);
        assert!(t.contains("```text"), "{code}: {t}");
    }
}

// The user's vanilla install.

fn vanilla() -> Option<PathBuf> {
    match std::env::var_os("RAKATA_GAME_K1") {
        Some(v) => Some(PathBuf::from(v)),
        None => {
            eprintln!("skipped: $RAKATA_GAME_K1 is not set; it names a vanilla KotOR 1 install");
            None
        }
    }
}

#[test]
fn the_vanilla_assets_of_end_m01aa_read_by_name() {
    let Some(g) = vanilla() else { return };
    let g = s(&g);

    let (code, doc) = json(&["--game", g, "texture", "show", "lhr_flr01.tpc"]);
    assert_eq!(code, Some(0), "{doc}");
    let r = &doc["result"];
    assert_eq!(r["format"], "tpc");
    assert!(r["images"][0].as_array().unwrap().len() > 1, "{r}");

    let (code, doc) = json(&["--game", g, "texture", "show", "m01aa_01a_lm0.tga"]);
    assert_eq!(code, Some(0), "{doc}");
    assert_eq!(doc["result"]["format"], "tga");

    let (code, doc) = json(&["--game", g, "lyt", "show", "m01aa.lyt"]);
    assert_eq!(code, Some(0), "{doc}");
    let rooms = doc["result"]["rooms"].as_array().unwrap();
    assert!(
        rooms.iter().any(|r| r["model"]
            .as_str()
            .unwrap()
            .eq_ignore_ascii_case("m01aa_01a")),
        "{rooms:?}"
    );

    let (code, doc) = json(&["--game", g, "lyt", "show", "m01aa.vis"]);
    assert_eq!(code, Some(0), "{doc}");
    assert_eq!(
        doc["result"]["rooms"].as_array().unwrap().len(),
        rooms.len()
    );

    let (code, doc) = json(&["--game", g, "walkmesh", "show", "m01aa_01a.wok"]);
    assert_eq!(code, Some(0), "{doc}");
    let r = &doc["result"];
    assert_eq!(r["kind"], "area");
    assert!(codes(&doc).is_empty(), "{doc}");
    // Every material is labelled through the vanilla surfacemat.2da.
    for m in r["materials"].as_array().unwrap() {
        assert!(m["label"].is_string(), "{m}");
    }
    assert!(r["walkable_faces"].as_u64().unwrap() > 0);

    let (code, doc) = json(&["--game", g, "mdl", "show", "m01aa_01a.mdl"]);
    assert_eq!(code, Some(0), "{doc}");
    assert!(codes(&doc).is_empty(), "{doc}");
    let r = &doc["result"];
    assert!(
        r["name"]
            .as_str()
            .unwrap()
            .eq_ignore_ascii_case("m01aa_01a")
    );
    assert!(r["mdx"].is_string());
    let nodes = r["nodes"].as_array().unwrap();
    assert!(
        nodes
            .iter()
            .any(|n| n["mesh"]["texture"].is_string() && n["mesh"]["lightmap"].is_string()),
        "no lightmapped mesh"
    );
    assert!(
        nodes.iter().any(|n| n["kind"] == "aabb"),
        "no walkmesh node"
    );
}

#[test]
fn mdl_show_prints_the_vanilla_emitters_lights_and_sabers() {
    let Some(g) = vanilla() else { return };
    let g = s(&g);

    // A muzzle flash: each emitter's update, textures and grid.
    let t = text(&["--game", g, "mdl", "show", "v_muzflash_01.mdl"], 0);
    let emitters: Vec<&str> = t.lines().filter(|l| l.contains("update ")).collect();
    assert_eq!(emitters.len(), 5, "{t}");
    assert!(emitters.iter().all(|l| l.contains("update Explosion")
        && l.contains("render Billboard_to_Local_Z")
        && l.contains("blend Lighten")
        && l.contains("two-sided")));
    assert!(
        t.contains("texture fx_muzflash  grid 1x1  birth rate 1"),
        "{t}"
    );
    assert!(t.contains("texture fx_flare02  grid 0x0"), "{t}");
    let full = text(
        &[
            "--game",
            g,
            "mdl",
            "show",
            "v_muzflash_01.mdl",
            "--controllers",
        ],
        0,
    );
    assert!(full.contains("        birth_rate  1"), "{full}");
    assert!(full.contains("        type 240  "), "{full}");
    let (code, doc) = json(&["--game", g, "mdl", "show", "v_muzflash_01.mdl"]);
    assert_eq!(code, Some(0), "{doc}");
    let node = &doc["result"]["nodes"][2];
    assert_eq!(node["name"], "muzflash01");
    assert_eq!(node["emitter"]["update"], "Explosion");
    assert_eq!(node["emitter"]["texture"], "fx_muzflash");
    assert_eq!(node["emitter"]["two_sided"], true);
    assert_eq!(node["emitter"]["name_187"], "NULL");
    let birth = node["controllers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "birth_rate")
        .unwrap();
    assert_eq!(birth["type"], 88);
    assert_eq!(birth["keys"][0]["value"][0], 1.0);

    // A light model: colour, radius, priority and flags.
    let t = text(&["--game", g, "mdl", "show", "gidy_sun.mdl"], 0);
    assert!(
        t.contains(
            "color 0.313726 0.352941 0.431373  radius 100000  priority 1  ambient only  fading"
        ),
        "{t}"
    );
    assert!(
        t.contains("color 1 1 1  radius 100000  priority 1  shadow  fading"),
        "{t}"
    );
    let t = text(&["--game", g, "mdl", "show", "fx_explode_03.mdl"], 0);
    assert!(t.contains("flares fx_flare01, "), "{t}");
    let (_, doc) = json(&["--game", g, "mdl", "show", "gidy_sun.mdl"]);
    let amb = &doc["result"]["nodes"][1];
    assert_eq!(amb["light"]["ambient_only"], true);
    assert_eq!(amb["controllers"][2]["name"], "radius");

    // A saber model: each blade's vertices and words.
    let t = text(&["--game", g, "mdl", "show", "w_dblsbr_001.mdl"], 0);
    assert!(
        t.contains("blade 176 vertices along z -0.0432471 to 0.980982  words 98 97"),
        "{t}"
    );
    let (_, doc) = json(&["--game", g, "mdl", "show", "w_dblsbr_001.mdl"]);
    let blade = doc["result"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "plane269")
        .unwrap();
    assert_eq!(blade["saber"]["positions"].as_array().unwrap().len(), 176);
    assert_eq!(blade["saber"]["words"], serde_json::json!([98, 97]));
}

/// Export `name` from the vanilla install into `dir`, then the size and
/// the content hash of the PNG's pixels, and the TXI written, if any.
fn export_vanilla(g: &str, dir: &Path, name: &str) -> (u32, u32, String, Option<Vec<u8>>) {
    let stem = name.rsplit_once('.').unwrap().0;
    let png = dir.join(format!("{stem}.png"));
    let (code, doc) = json(&["--game", g, "texture", "export", name, "-o", s(&png)]);
    assert_eq!(code, Some(0), "{doc}");
    assert!(codes(&doc).is_empty(), "{doc}");
    let (w, h, px) = read_png(&png);
    let hash = rakata_res::ContentHash::of(&px).to_hex();
    let txi = std::fs::read(dir.join(format!("{stem}.txi"))).ok();
    (w, h, hash, txi)
}

#[test]
fn vanilla_textures_export_as_png() {
    let Some(g) = vanilla() else { return };
    let g = s(&g);
    let dir = tempfile::tempdir().unwrap();

    // DXT5 with alpha, from the high-quality texture pack.
    let (w, h, hash, txi) = export_vanilla(g, dir.path(), "lhr_flr01.tpc");
    assert_eq!((w, h), (512, 512));
    assert_eq!(
        hash,
        "18365933ff0140bfb358c48c958c849ce93e96da4161c79f42154730151e0034"
    );
    assert_eq!(txi.as_deref(), Some(&b"envmaptexture CM_Endar\r\n\r\n"[..]));

    // DXT1.
    let (w, h, hash, _) = export_vanilla(g, dir.path(), "lhr_flr02.tpc");
    assert_eq!((w, h), (256, 256));
    assert_eq!(
        hash,
        "5d7b06c610608fb7282406540ad7b0914d8a43bc83bf1de94768579b441761f5"
    );

    // A font: DXT5, its glyph coordinates in the TXI after the pixels.
    let (w, h, hash, txi) = export_vanilla(g, dir.path(), "fnt_d16x16.tpc");
    assert_eq!((w, h), (512, 512));
    assert_eq!(
        hash,
        "4dc36b877dc792b2d7e78bba564fe9d2fefc8898b61da9cd1b7e014995d1ae73"
    );
    let txi = String::from_utf8(txi.expect("the font's TXI")).unwrap();
    assert!(txi.contains("upperleftcoords"), "{txi}");

    // A lightmap TGA, its TXI a separate resource.
    let (w, h, hash, txi) = export_vanilla(g, dir.path(), "m01aa_01a_lm0.tga");
    assert_eq!((w, h), (64, 64));
    assert_eq!(
        hash,
        "6bc0cedd682b734e0ef3a8d402f563d24e3607ae55033f05c6addd33f500cc2a"
    );
    assert!(txi.is_some());
}
