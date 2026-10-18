//! The diff over textures, layouts, walkmeshes and models built here: each
//! kind of change each format shows, a layout another writer could choose
//! read as the same data, and a model's MDL and MDX compared as one.

use std::fs;
use std::path::{Path, PathBuf};

use rakata_lyt::{DoorHook, Lyt, Placement, Quaternion, Vis, VisRoom};
use rakata_mdl::{
    AnimationFile, Body, MdxBlock, MeshFile, MeshKind, ModelFile, NodeFile, NodePointer,
    controller_types, mdx_flags,
};
use rakata_walkmesh::{AabbNode, EdgeRef, Face, Kind, PerimeterEdge, Vertices, Walkmesh};

use super::*;

fn resources(a: (&str, &[u8]), b: (&str, &[u8])) -> Diff {
    diff_resources(a, b, &Options::default())
}

/// The one entry of a diff, and its sites.
fn only(d: &Diff) -> &Entry {
    assert_eq!(d.entries.len(), 1, "{}", d.text());
    let e = &d.entries[0];
    assert!(e.compared, "{}", d.text());
    e
}

/// Whether `sites` has a part of this kind, name and field changed so.
fn has_part(sites: &[Site], change: Change, kind: &str, name: &str, field: Option<&str>) -> bool {
    sites.iter().any(|s| {
        matches!(s, Site::Part { change: c, part, name: n, field: f, .. }
            if *c == change && part == kind && n == name && f.as_deref() == field)
    })
}

fn has_header(sites: &[Site], field: &str, before: &str, after: &str) -> bool {
    sites.iter().any(|s| {
        matches!(s, Site::Header { field: f, before: Some(b), after: Some(a) }
            if f == field && b == before && a == after)
    })
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rakata-diff-asset-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, data: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, data).unwrap();
}

// Textures.

/// An uncompressed RGBA TPC of `size` × `size` pixels, one level, every
/// byte `fill`, with `txi` after the pixels.
fn tpc(size: u16, fill: u8, txi: &str) -> Vec<u8> {
    let mut b = vec![0u8; 128];
    b[4..8].copy_from_slice(&1.0f32.to_le_bytes());
    b[8..10].copy_from_slice(&size.to_le_bytes());
    b[10..12].copy_from_slice(&size.to_le_bytes());
    b[12] = 4;
    b[13] = 1;
    b.extend(std::iter::repeat_n(
        fill,
        usize::from(size) * usize::from(size) * 4,
    ));
    b.extend_from_slice(txi.as_bytes());
    b
}

#[test]
fn a_texture_differs_by_header_level_and_txi_instruction() {
    let d = resources(
        ("t.tpc", &tpc(4, 0x10, "mipmap 0\r\nclamp 1\r\n")),
        ("t.tpc", &tpc(4, 0x20, "mipmap 1\r\nblending additive\r\n")),
    );
    let e = only(&d);
    assert_eq!(e.format.as_deref(), Some("tpc"));
    assert!(
        has_part(&e.sites, Change::Changed, "pixels", "level 0", None),
        "{}",
        d.text()
    );
    assert!(has_part(&e.sites, Change::Changed, "txi", "mipmap", None));
    assert!(has_part(&e.sites, Change::Removed, "txi", "clamp", None));
    assert!(has_part(&e.sites, Change::Added, "txi", "blending", None));
    assert!(d.text().contains("~ txi `mipmap`: 0 -> 1"), "{}", d.text());

    let d = resources(("t.tpc", &tpc(4, 0, "")), ("t.tpc", &tpc(8, 0, "")));
    let e = only(&d);
    assert!(has_header(&e.sites, "width", "4", "8"), "{}", d.text());
    assert!(has_header(&e.sites, "height", "4", "8"));
}

#[test]
fn a_txi_spelled_another_way_is_the_same_data() {
    let d = resources(
        ("t.tpc", &tpc(4, 0, "mipmap 0\r\nclamp 1\r\n")),
        ("t.tpc", &tpc(4, 0, "mipmap 0\nclamp 1\n")),
    );
    assert!(d.is_same(), "{}", d.text());
    assert_eq!(d.same_data, 1);
    let d = resources(
        ("t.txi", b"Mipmap 0\r\ndecal 1"),
        ("t.txi", b"Mipmap 0\r\ndecal 2\r\n"),
    );
    let e = only(&d);
    assert_eq!(e.format.as_deref(), Some("txi"));
    assert!(
        has_part(&e.sites, Change::Changed, "txi", "decal", None),
        "{}",
        d.text()
    );
    assert_eq!(e.sites.len(), 1, "{}", d.text());
}

/// A 2 × 2 true-colour TGA with alpha, every byte `fill`.
fn tga(fill: u8) -> Vec<u8> {
    let mut b = vec![0u8; 18];
    b[2] = 2;
    b[12] = 2;
    b[14] = 2;
    b[16] = 32;
    b[17] = 8;
    b.extend([fill; 16]);
    b
}

#[test]
fn an_image_differs_by_its_pixels() {
    let d = resources(("t.tga", &tga(1)), ("t.tga", &tga(2)));
    let e = only(&d);
    assert_eq!(e.format.as_deref(), Some("tga"));
    assert!(
        has_part(&e.sites, Change::Changed, "pixels", "level 0", None),
        "{}",
        d.text()
    );
    assert_eq!(e.sites.len(), 1, "{}", d.text());
}

// Layouts.

fn layout(rooms: &[(&str, [f32; 3])], hook_at: f32) -> Vec<u8> {
    let mut l = Lyt::new(Some("m01aa.max".into()));
    l.rooms = rooms.iter().map(|(m, p)| Placement::new(*m, *p)).collect();
    l.door_hooks = vec![DoorHook {
        room: "m01aa_01a".into(),
        name: "door_01".into(),
        unknown: 0,
        position: [hook_at, 0.0, 0.0],
        orientation: Quaternion::from_array([1.0, 0.0, 0.0, 0.0]),
    }];
    l.to_bytes().unwrap()
}

#[test]
fn a_layout_differs_by_room_and_door_hook() {
    let a = layout(
        &[("m01aa_01a", [0.0; 3]), ("m01aa_02a", [10.0, 0.0, 0.0])],
        1.0,
    );
    let b = layout(
        &[("M01AA_01A", [0.0; 3]), ("m01aa_03a", [20.0, 0.0, 0.0])],
        2.0,
    );
    let d = resources(("m01aa.lyt", &a), ("m01aa.lyt", &b));
    let e = only(&d);
    assert_eq!(e.format.as_deref(), Some("lyt"));
    assert!(
        has_part(&e.sites, Change::Removed, "room", "m01aa_02a", None),
        "{}",
        d.text()
    );
    assert!(has_part(&e.sites, Change::Added, "room", "m01aa_03a", None));
    assert!(has_part(
        &e.sites,
        Change::Changed,
        "door hook",
        "m01aa_01a/door_01",
        Some("placement")
    ));
    // A room's case is spelling, not data.
    assert!(!d.text().contains("01A"), "{}", d.text());

    let moved = layout(
        &[("m01aa_01a", [0.0; 3]), ("m01aa_02a", [11.0, 0.0, 0.0])],
        1.0,
    );
    let d = resources(("m01aa.lyt", &a), ("m01aa.lyt", &moved));
    assert!(
        d.text()
            .contains("~ room `m01aa_02a` position: (10, 0, 0) -> (11, 0, 0)"),
        "{}",
        d.text()
    );

    let swapped = layout(
        &[("m01aa_02a", [10.0, 0.0, 0.0]), ("m01aa_01a", [0.0; 3])],
        1.0,
    );
    let d = resources(("m01aa.lyt", &a), ("m01aa.lyt", &swapped));
    assert!(
        has_header(
            &only(&d).sites,
            "room order",
            "m01aa_01a, m01aa_02a",
            "m01aa_02a, m01aa_01a"
        ),
        "{}",
        d.text()
    );
}

fn vis(rooms: &[(&str, &[&str])]) -> Vec<u8> {
    Vis::new(
        rooms
            .iter()
            .map(|(r, v)| VisRoom {
                room: (*r).into(),
                visible: v.iter().map(|s| (*s).to_owned()).collect(),
            })
            .collect(),
    )
    .to_bytes()
    .unwrap()
}

#[test]
fn visibility_differs_by_room_and_by_room_seen() {
    let a = vis(&[("r1", &["r2"]), ("r2", &["r1"])]);
    let b = vis(&[("r1", &["r2", "r3"]), ("r3", &["r1"])]);
    let d = resources(("m.vis", &a), ("m.vis", &b));
    let e = only(&d);
    assert_eq!(e.format.as_deref(), Some("vis"));
    assert!(
        has_part(&e.sites, Change::Added, "sees", "r1 -> r3", None),
        "{}",
        d.text()
    );
    assert!(has_part(&e.sites, Change::Removed, "room", "r2", None));
    assert!(has_part(&e.sites, Change::Added, "room", "r3", None));
    // Case is spelling: the engine compares rooms without it.
    let upper = vis(&[("R1", &["R2"]), ("r2", &["r1"])]);
    let d = resources(("m.vis", &a), ("m.vis", &upper));
    assert!(d.is_same(), "{}", d.text());
    assert_eq!(d.same_data, 1);
}

// Walkmeshes.

fn walkmesh(bytes: &[u8], material: u32, transition: Option<u32>) -> Vec<u8> {
    let leaf = AabbNode {
        min: [-0.01, -0.01, -0.01],
        max: [2.01, 2.01, 0.01],
        face: Some(0),
        unknown: 4,
        plane: 0,
        left: None,
        right: None,
    };
    Walkmesh {
        kind: Kind::Area,
        relative_points: [[0.0; 3]; 2],
        absolute_points: [[0.0; 3]; 2],
        position: [0.0; 3],
        vertices: Vertices::from_bytes(bytes).unwrap(),
        faces: vec![Face {
            vertices: [0, 1, 2],
            material,
            normal: [0.0, 0.0, 1.0],
            distance: 0.0,
        }],
        adjacency: vec![[None; 3]],
        perimeter_edges: (0..3)
            .map(|e| PerimeterEdge {
                edge: EdgeRef { face: 0, edge: e },
                transition: if e == 1 { transition } else { None },
            })
            .collect(),
        perimeters: vec![3],
        aabb: vec![leaf],
        unknown: 0,
        trailer: &[],
    }
    .to_bytes()
    .unwrap()
}

fn vertices(far: f32) -> Vec<u8> {
    [[0.0f32, 0.0, 0.0], [far, 0.0, 0.0], [0.0, 2.0, 0.0]]
        .iter()
        .flatten()
        .flat_map(|c| c.to_le_bytes())
        .collect()
}

#[test]
fn a_walkmesh_differs_by_material_transition_and_vertex() {
    let a = walkmesh(&vertices(2.0), 4, Some(3));
    let b = walkmesh(&vertices(2.5), 7, Some(5));
    let d = resources(("r.wok", &a), ("r.wok", &b));
    let e = only(&d);
    assert_eq!(e.format.as_deref(), Some("walkmesh"));
    let t = d.text();
    assert!(
        has_part(&e.sites, Change::Removed, "material", "4", None),
        "{t}"
    );
    assert!(
        has_part(&e.sites, Change::Added, "material", "7", None),
        "{t}"
    );
    assert!(
        has_part(&e.sites, Change::Removed, "transition to room", "3", None),
        "{t}"
    );
    assert!(
        has_part(&e.sites, Change::Added, "transition to room", "5", None),
        "{t}"
    );
    assert!(t.contains("~ array `vertices`: 1 of 3 moved"), "{t}");
    assert!(
        t.contains("~ array `materials`: 1 of 1 faces of another material"),
        "{t}"
    );
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

/// A triangle mesh with `texture`, its first vertex at `x`.
fn triangle(texture: &str, x: f32) -> Body {
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
    // Four vertices, so a block with its terminator is not a multiple of
    // 16 bytes and the next block's padding is the writer's to keep.
    let positions = [
        [x, 0.0, 0.0],
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
        // The MDL's own copy of the positions, which the MDX's override:
        // `x` moves only the MDX's.
        positions: floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0]),
        mdx: Some(MdxBlock {
            vertices,
            terminator: floats(&[1.0e7, 1.0e7, 1.0e7, 0.0, 0.0, 0.0]),
            padding: Vec::new(),
        }),
    }))
}

/// A model `box`: a root with a mesh `body` (textured `texture`, its first
/// vertex at `x`) and a dummy `hook`, and an animation `wave` when asked.
fn model(texture: &str, x: f32, wave: bool) -> ModelFile {
    let names = ["box", "body", "hook"];
    let mut header = [0u8; 0xC4];
    put_u32(&mut header, 0, 0x0041_3670);
    put_u32(&mut header, 4, 0x0040_5520);
    put_str(&mut header, 8, "box");
    put_u32(&mut header, 0x2C, names.len() as u32);
    header[0x4C] = 2;
    header[0x50] = 32;
    header[0x53] = 1;
    put_f32s(
        &mut header,
        0x68,
        &[-5.0, -5.0, -1.0, 5.0, 5.0, 10.0, 7.0, 1.0],
    );
    put_str(&mut header, 0x88, "NULL");
    let mut root = node(0x001, 0, Body::Dummy);
    root.children = vec![
        node(0x021, 1, triangle(texture, x)),
        node(0x001, 2, Body::Dummy),
    ];
    let mut animations = Vec::new();
    if wave {
        let mut h = [0u8; 0x88];
        put_u32(&mut h, 0, 0x0041_34F0);
        put_u32(&mut h, 4, 0x0043_ECE0);
        put_str(&mut h, 8, "wave");
        put_u32(&mut h, 0x2C, 1);
        h[0x4C] = 5;
        put_f32s(&mut h, 0x50, &[1.5, 0.25]);
        put_str(&mut h, 0x58, "box");
        let mut anim_root = node(0x001, 0, Body::Dummy);
        anim_root.controllers.clear();
        anim_root.controller_data.clear();
        animations.push(AnimationFile {
            header: h,
            events: Vec::new(),
            root: anim_root,
        });
    }
    ModelFile {
        header,
        head_node: NodePointer::Node(0),
        names: names.iter().map(|n| n.as_bytes().to_vec()).collect(),
        animations,
        root,
    }
}

fn halves(m: &ModelFile) -> (Vec<u8>, Vec<u8>) {
    m.to_bytes().unwrap()
}

/// Two trees, each `models/box.mdl` and `models/box.mdx`.
fn model_trees(name: &str, a: &ModelFile, b: &ModelFile) -> Diff {
    let root = scratch(name);
    for (side, m) in [("a", a), ("b", b)] {
        let (mdl, mdx) = halves(m);
        write(&root.join(side).join("models/box.mdl"), &mdl);
        write(&root.join(side).join("models/box.mdx"), &mdx);
    }
    let d = diff_trees(&root.join("a"), &root.join("b"), &Options::default()).unwrap();
    let _ = fs::remove_dir_all(&root);
    d
}

#[test]
fn a_model_differs_by_node_texture_and_animation() {
    let a = model("tex_a", 0.0, true);
    let mut b = model("tex_b", 0.0, false);
    // `hook` moves under `body`; a node `light` is added.
    let hook = b.root.children.pop().unwrap();
    b.root.children[0].children.push(hook);
    b.names.push(b"light".to_vec());
    put_u32(&mut b.header, 0x2C, 4);
    b.root.children.push(node(0x001, 3, Body::Dummy));
    let d = model_trees("node", &a, &b);
    // The texture is the MDL's: the MDX is the same, and the one entry is
    // the MDL's.
    let e = only(&d);
    assert_eq!(e.path, "models/box.mdl");
    assert_eq!(e.format.as_deref(), Some("mdl"));
    let t = d.text();
    assert!(t.contains("~ node `body` texture: tex_a -> tex_b"), "{t}");
    assert!(t.contains("~ node `hook` parent: box -> body"), "{t}");
    assert!(
        has_part(&e.sites, Change::Added, "node", "light", None),
        "{t}"
    );
    assert!(
        has_part(&e.sites, Change::Removed, "animation", "wave", None),
        "{t}"
    );
    assert!(has_header(&e.sites, "nodes", "3", "4"), "{t}");

    // The other way round: the node removed and the animation added.
    let d = model_trees("node-back", &b, &a);
    let e = only(&d);
    let t = d.text();
    assert!(
        has_part(&e.sites, Change::Removed, "node", "light", None),
        "{t}"
    );
    assert!(
        has_part(&e.sites, Change::Added, "animation", "wave", None),
        "{t}"
    );
}

#[test]
fn a_node_turned_shows_its_orientation_not_its_controller() {
    let a = model("tex_a", 0.0, false);
    let mut b = a.clone();
    let hook = &mut b.root.children[1];
    put_f32s(&mut hook.header, 28, &[0.0, 0.0, 0.0, 1.0]);
    // The orientation controller's value repeats the header's.
    hook.controller_data = floats(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    let d = model_trees("turned", &a, &b);
    let t = d.text();
    assert!(
        t.contains("~ node `hook` orientation: (1, 0, 0, 0) -> (0, 0, 0, 1)"),
        "{t}"
    );
    assert!(!t.contains("controllers"), "{t}");
}

#[test]
fn a_model_whose_mdx_alone_differs_is_listed_by_its_mdx_read_with_the_mdl() {
    let d = model_trees(
        "mdx",
        &model("tex_a", 0.0, false),
        &model("tex_a", 0.5, false),
    );
    let e = only(&d);
    assert_eq!(e.path, "models/box.mdx");
    assert_eq!(e.format.as_deref(), Some("mdl"));
    assert!(
        d.text()
            .contains("~ node `body` vertex data: positions changed"),
        "{}",
        d.text()
    );
}

#[test]
fn a_model_whose_halves_both_differ_is_one_entry() {
    let d = model_trees(
        "both",
        &model("tex_a", 0.0, false),
        &model("tex_b", 0.5, false),
    );
    let e = only(&d);
    assert_eq!(e.path, "models/box.mdl");
    let t = d.text();
    assert!(t.contains("texture: tex_a -> tex_b"), "{t}");
    assert!(t.contains("vertex data: positions changed"), "{t}");
    assert_eq!(d.compared, 2);
}

#[test]
fn a_model_padded_another_way_is_the_same_data() {
    // A second mesh, so the first block is followed by padding.
    let mut a = model("tex_a", 0.0, false);
    a.names.push(b"lid".to_vec());
    put_u32(&mut a.header, 0x2C, 4);
    a.root.children.push(node(0x021, 3, triangle("tex_c", 0.0)));
    let (mdl, mdx) = halves(&a);
    let mut b = ModelFile::parse(&mdl, &mdx).unwrap();
    let mut padded = false;
    if let Body::Mesh(m) = &mut b.root.children[0].body
        && let Some(block) = &mut m.mdx
        && !block.padding.is_empty()
    {
        block.padding.fill(0xAB);
        padded = true;
    }
    assert!(padded, "the block is padded");
    let d = model_trees("padding", &a, &b);
    assert!(d.is_same(), "{}", d.text());
    assert_eq!(d.same_data, 1);
}

#[test]
fn a_model_named_as_a_resource_is_read_with_the_mdx_beside_it() {
    let root = scratch("named");
    for (side, m) in [
        ("a", model("tex_a", 0.0, false)),
        ("b", model("tex_a", 0.5, false)),
    ] {
        let (mdl, mdx) = halves(&m);
        write(&root.join(side).join("box.mdl"), &mdl);
        write(&root.join(side).join("box.mdx"), &mdx);
    }
    let side = |s: &str| root.join(s).join("box.mdx").display().to_string();
    let d = diff(&side("a"), &side("b"), &Options::default()).unwrap();
    let _ = fs::remove_dir_all(&root);
    assert!(
        d.text().contains("vertex data: positions changed"),
        "{}",
        d.text()
    );
}

/// An install with an empty `chitin.key` and a module whose RIM holds the
/// model `box`.
fn install_with_model(root: &Path, m: &ModelFile) {
    use rakata_archive::{Key, Rim};
    write(&root.join("chitin.key"), &Key::new().to_bytes().unwrap());
    let (mdl, mdx) = halves(m);
    let mut rim = Rim::new();
    rim.add(rakata_res::ResId::parse("box.mdl").unwrap(), mdl)
        .unwrap();
    rim.add(rakata_res::ResId::parse("box.mdx").unwrap(), mdx)
        .unwrap();
    write(&root.join("modules/danm14ac.rim"), &rim.to_bytes().unwrap());
}

#[test]
fn a_model_half_in_override_is_read_with_the_other_half_the_install_loads() {
    let dir = scratch("override");
    let (a, b) = (dir.join("a"), dir.join("b"));
    let vanilla = model("tex_a", 0.0, false);
    install_with_model(&a, &vanilla);
    install_with_model(&b, &vanilla);
    // The MDX alone in Override: read with the module's MDL.
    let (_, mdx) = halves(&model("tex_a", 0.5, false));
    write(&b.join("Override/box.mdx"), &mdx);
    let d = diff_trees(&a, &b, &Options::default()).unwrap();
    let e = only(&d);
    assert_eq!(e.path, "Override/box.mdx");
    assert_eq!(e.against, ["modules/danm14ac.rim!box.mdx"]);
    assert!(
        d.text().contains("vertex data: positions changed"),
        "{}",
        d.text()
    );

    // Both halves in Override: one entry, the MDL's, for the model whole.
    let (mdl, mdx) = halves(&model("tex_b", 0.5, false));
    write(&b.join("Override/box.mdl"), &mdl);
    write(&b.join("Override/box.mdx"), &mdx);
    let d = diff_trees(&a, &b, &Options::default()).unwrap();
    let e = only(&d);
    assert_eq!(e.path, "Override/box.mdl");
    let t = d.text();
    assert!(t.contains("texture: tex_a -> tex_b"), "{t}");
    assert!(t.contains("vertex data: positions changed"), "{t}");
    let _ = fs::remove_dir_all(&dir);
}
