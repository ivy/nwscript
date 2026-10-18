//! Hand-made models, built here as [`ModelFile`]s with every kind of node
//! the writer lays out, written, and read back: by the owned form (equal,
//! and written again to the same bytes) and by the reader [`Model`].

use rakata_mdl::{
    AabbEntry, AnimationFile, Body, DanglyFile, LightFile, MdxBlock, MeshFile, MeshKind, ModelFile,
    NodeFile, NodePointer, SaberFile, SkinFile, codes, controller_types, layout, mdx_flags,
};

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
    b[at..at + s.len()].copy_from_slice(s.as_bytes());
}

fn floats(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A node of `flags` and `number` at `position`, with a position and an
/// orientation controller repeating its transform.
fn node(flags: u16, number: u16, position: [f32; 3], body: Body) -> NodeFile {
    let mut header = [0u8; 80];
    put_u16(&mut header, 0, flags);
    put_u16(&mut header, 2, number);
    put_u16(&mut header, 4, number);
    put_f32s(&mut header, 16, &position);
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
    let mut data = vec![0.0];
    data.extend(position);
    data.extend([0.0, 0.0, 0.0, 0.0, 1.0]);
    NodeFile {
        header,
        body,
        children: Vec::new(),
        controllers,
        controller_data: floats(&data),
    }
}

/// A mesh header for `nv` vertices with the MDX `flags` laid out at
/// `stride` bytes per vertex, and a texture.
fn mesh_header(nv: u16, flags: u32, stride: u32, texture: &str) -> [u8; 332] {
    let mut h = [0u8; 332];
    put_u32(&mut h, 0, 0x0040_5750);
    put_u32(&mut h, 4, 0x0040_5760);
    put_f32s(&mut h, 60, &[0.8, 0.8, 0.8, 0.2, 0.2, 0.2]);
    put_str(&mut h, 88, texture);
    put_u32(&mut h, 212, u32::MAX);
    put_u32(&mut h, 216, u32::MAX);
    put_u32(&mut h, 224, 3);
    put_u32(&mut h, 252, stride);
    put_u32(&mut h, 256, flags);
    h[260..304].fill(0xFF);
    if flags & mdx_flags::POSITION != 0 {
        put_u32(&mut h, 260, 0);
    }
    if flags & mdx_flags::NORMAL != 0 {
        put_u32(&mut h, 264, 12);
    }
    if flags & mdx_flags::UV1 != 0 {
        put_u32(&mut h, 272, 24);
    }
    put_u16(&mut h, 304, nv);
    put_u16(&mut h, 306, 1);
    h[313] = 1;
    h
}

/// A triangle of three vertices: positions, its face and its index list.
fn triangle(header: [u8; 332], kind: MeshKind, stride: usize, skin: bool) -> MeshFile {
    let positions = [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let mut face = [0u8; 32];
    put_f32s(&mut face, 0, &[0.0, 0.0, 1.0, 0.0]);
    for k in 0..3 {
        put_u16(&mut face, 20 + 2 * k, u16::MAX);
        put_u16(&mut face, 26 + 2 * k, k as u16);
    }
    let mut vertices = Vec::new();
    for p in positions {
        let mut v = vec![0u8; stride];
        put_f32s(&mut v, 0, &p);
        put_f32s(&mut v, 12, &[0.0, 0.0, 1.0]);
        if stride >= 32 {
            put_f32s(&mut v, 24, &[p[0], p[1]]);
        }
        if skin {
            put_f32s(&mut v, 32, &[1.0, 0.0, 0.0, 0.0, 0.0, -1.0, -1.0, -1.0]);
        }
        vertices.extend(v);
    }
    let mut terminator = vec![0u8; stride];
    let t = if skin { 1_000_000.0 } else { 10_000_000.0 };
    put_f32s(&mut terminator, 0, &[t, t, t]);
    MeshFile {
        header,
        kind,
        faces: face.to_vec(),
        index_counts: vec![3],
        index_lists: vec![[0u16, 1, 2].iter().flat_map(|i| i.to_le_bytes()).collect()],
        index_unknown: vec![1],
        positions: floats(&positions.concat()),
        mdx: Some(MdxBlock {
            vertices,
            terminator,
            padding: Vec::new(),
        }),
    }
}

/// A model of every kind of node the writer lays out, and an animation.
fn every_kind() -> ModelFile {
    let names = [
        "every_kind",
        "trimesh",
        "skin",
        "dangly",
        "walkmesh",
        "blade",
        "lamp",
        "smoke",
        "hook",
        "empty",
    ];
    let mut header = [0u8; 0xC4];
    put_u32(&mut header, 0, 0x0041_3670);
    put_u32(&mut header, 4, 0x0040_5520);
    put_str(&mut header, 8, "every_kind");
    put_u32(&mut header, 0x2C, names.len() as u32);
    header[0x4C] = 2;
    header[0x50] = 4;
    header[0x53] = 1;
    put_f32s(
        &mut header,
        0x68,
        &[-5.0, -5.0, -1.0, 5.0, 5.0, 10.0, 7.0, 1.0],
    );
    put_str(&mut header, 0x88, "NULL");

    let trimesh = Body::Mesh(Box::new(triangle(
        mesh_header(3, 0x23, 32, "tex_a"),
        MeshKind::Trimesh,
        32,
        false,
    )));

    let mut skin_header = [0u8; 100];
    put_u32(&mut skin_header, 12, 32);
    put_u32(&mut skin_header, 16, 48);
    put_u16(&mut skin_header, 64, 0);
    let mut skin_mesh = mesh_header(3, 0x23, 64, "tex_b");
    put_u32(&mut skin_mesh, 0, 0x0040_5710);
    put_u32(&mut skin_mesh, 4, 0x0040_5720);
    let mut bone_map = vec![-1.0f32; names.len()];
    bone_map[0] = 0.0;
    let skin = Body::Mesh(Box::new(triangle(
        skin_mesh,
        MeshKind::Skin(Box::new(SkinFile {
            header: skin_header,
            bone_map: floats(&bone_map),
            qbones: floats(&[1.0, 0.0, 0.0, 0.0].repeat(names.len())),
            tbones: floats(&[0.0; 3].repeat(names.len())),
            unknown: floats(&[0.0; 1].repeat(names.len())),
        })),
        64,
        true,
    )));

    let mut dangly_header = [0u8; 28];
    put_f32s(&mut dangly_header, 12, &[0.025, 3.0, 8.0]);
    let dangly = Body::Mesh(Box::new(triangle(
        mesh_header(3, 0x21, 24, "tex_c"),
        MeshKind::Dangly(Box::new(DanglyFile {
            header: dangly_header,
            constraints: floats(&[0.0, 128.0, 255.0]),
            positions: floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        })),
        24,
        false,
    )));

    let aabb_entry = |leaf: i32, plane: u32| {
        let mut b = [0u8; 40];
        put_f32s(&mut b, 0, &[0.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
        put_u32(&mut b, 32, leaf as u32);
        put_u32(&mut b, 36, plane);
        b
    };
    let walkmesh = Body::Mesh(Box::new(triangle(
        mesh_header(3, 0x21, 24, "NULL"),
        MeshKind::Aabb(vec![
            AabbEntry {
                bytes: aabb_entry(-1, 1),
                left: Some(1),
                right: Some(2),
            },
            AabbEntry {
                bytes: aabb_entry(0, 0),
                left: None,
                right: None,
            },
            AabbEntry {
                bytes: aabb_entry(0, 0),
                left: None,
                right: None,
            },
        ]),
        24,
        false,
    )));

    // A saber: positions only in the model data, no MDX block, no index
    // lists, and its own three arrays.
    let mut blade = triangle(
        mesh_header(3, 0, 0, "fx_blade"),
        MeshKind::Saber(Box::new(SaberFile {
            header: {
                let mut h = [0u8; 20];
                put_u32(&mut h, 12, 88);
                put_u32(&mut h, 16, 87);
                h
            },
            arrays: [vec![1; 36], vec![2; 24], vec![3; 36]],
        })),
        24,
        false,
    );
    blade.mdx = None;
    blade.index_counts.clear();
    blade.index_lists.clear();
    blade.index_unknown.clear();

    let mut light_header = [0u8; 92];
    put_f32s(&mut light_header, 0, &[100.0]);
    put_u32(&mut light_header, 64, 1);
    let lamp = Body::Light(Box::new(LightFile {
        header: light_header,
        unknown: Vec::new(),
        flare_sizes: floats(&[0.5, 0.25]),
        flare_positions: floats(&[0.0, 1.0]),
        flare_colors: floats(&[1.0, 1.0, 1.0, 0.5, 0.5, 0.5]),
        flare_textures: vec![b"fx_flare01".to_vec(), b"fx_ring".to_vec()],
    }));

    let mut emitter = [0u8; 224];
    put_str(&mut emitter, 32, "Fountain");
    let mut reference = [0u8; 36];
    put_str(&mut reference, 0, "dor_lhr01");
    put_u32(&mut reference, 32, 1);

    // A mesh without vertices: no positions, no block.
    let mut empty = triangle(
        mesh_header(0, 0, u32::MAX, "NULL"),
        MeshKind::Trimesh,
        24,
        false,
    );
    empty.faces.clear();
    empty.index_counts.clear();
    empty.index_lists.clear();
    empty.index_unknown.clear();
    empty.positions.clear();
    empty.mdx = None;

    let mut root = node(0x001, 0, [0.0; 3], Body::Dummy);
    let mut trimesh = node(0x021, 1, [0.0, 0.0, 1.0], trimesh);
    trimesh.children.push(node(0x061, 2, [0.0, 1.0, 0.0], skin));
    root.children = vec![
        trimesh,
        node(0x121, 3, [1.0, 0.0, 0.0], dangly),
        node(0x221, 4, [0.0; 3], walkmesh),
        node(0x821, 5, [0.0; 3], Body::Mesh(Box::new(blade))),
        node(0x003, 6, [0.0, 0.0, 3.0], lamp),
        node(0x005, 7, [0.0; 3], Body::Emitter(Box::new(emitter))),
        node(0x011, 8, [0.0; 3], Body::Reference(Box::new(reference))),
        node(0x021, 9, [0.0; 3], Body::Mesh(Box::new(empty))),
    ];
    // The node of the empty mesh has no controllers.
    root.children[7].controllers.clear();
    root.children[7].controller_data.clear();

    let mut anim_header = [0u8; 0x88];
    put_u32(&mut anim_header, 0, 0x0041_34F0);
    put_u32(&mut anim_header, 4, 0x0043_ECE0);
    put_str(&mut anim_header, 8, "wave");
    put_u32(&mut anim_header, 0x2C, 2);
    anim_header[0x4C] = 5;
    put_f32s(&mut anim_header, 0x50, &[1.5, 0.25]);
    put_str(&mut anim_header, 0x58, "every_kind");
    let mut event = [0u8; 36];
    put_f32s(&mut event, 0, &[0.5]);
    put_str(&mut event, 4, "snd_footstep");
    let mut anim_root = node(0x001, 0, [0.0; 3], Body::Dummy);
    anim_root.controllers.clear();
    anim_root.controller_data.clear();
    anim_root
        .children
        .push(node(0x001, 1, [0.0, 0.0, 0.5], Body::Dummy));

    ModelFile {
        header,
        head_node: NodePointer::Node(0),
        names: names.iter().map(|n| n.as_bytes().to_vec()).collect(),
        animations: vec![AnimationFile {
            header: anim_header,
            events: vec![event],
            root: anim_root,
        }],
        root,
    }
}

#[test]
fn a_model_of_every_kind_writes_and_reads_back_equal() {
    let model = every_kind();
    let (mdl, mdx) = model.to_bytes().unwrap();
    let again = ModelFile::parse(&mdl, &mdx).unwrap();
    // The padding after each MDX block is what the writer's layout put
    // there; the rest is the model as built.
    assert!(again.semantic_eq(&model));
    // Written again, the same bytes.
    assert_eq!(again.to_bytes().unwrap(), (mdl.clone(), mdx.clone()));
    // And no departure from BioWare's layout.
    let l = layout(&mdl, &mdx).unwrap();
    assert!(l.in_order, "{l:?}");
    assert_eq!(l.unreferenced, 0);
    assert_eq!(l.mdx_trailing, 0);

    // The reader sees what was built.
    let m = rakata_mdl::parse(&mdl, &mdx).unwrap();
    assert_eq!(m.name, "every_kind");
    assert_eq!(m.nodes.len(), 10);
    let names: Vec<&str> = m.nodes.iter().map(|n| n.name).collect();
    assert_eq!(
        names,
        [
            "every_kind",
            "trimesh",
            "skin",
            "dangly",
            "walkmesh",
            "blade",
            "lamp",
            "smoke",
            "hook",
            "empty"
        ]
    );
    let tri = m.node("trimesh").unwrap().mesh.as_ref().unwrap();
    assert_eq!(tri.texture, Some("tex_a"));
    assert_eq!(tri.vertex_count, 3);
    assert_eq!(tri.positions.get(1), Some([1.0, 0.0, 0.0]));
    assert_eq!(tri.uv1.unwrap().get(2), Some([0.0, 1.0]));
    assert_eq!(tri.indices.unwrap().iter().collect::<Vec<_>>(), [0, 1, 2]);
    let skin = m.node("skin").unwrap().skin.as_ref().unwrap();
    assert_eq!(skin.bones, [0]);
    assert_eq!(skin.influences(0).unwrap()[0], Some((0, 1.0)));
    let blade = m.node("blade").unwrap().mesh.as_ref().unwrap();
    assert!(blade.vertex_buffer.is_empty());
    assert_eq!(blade.positions.get(2), Some([0.0, 1.0, 0.0]));
    assert_eq!(m.node("dangly").unwrap().position, [1.0, 0.0, 0.0]);
    // What it names, from the MDL alone: no supermodel (`NULL`), each
    // mesh's texture but the `NULL`s, the light's flares, the reference's
    // model.
    let refs: Vec<(String, String)> = rakata_mdl::facts::refs(&mdl)
        .unwrap()
        .into_iter()
        .map(|r| (r.site, r.value))
        .collect();
    let want = [
        ("trimesh.texture", "tex_a"),
        ("skin.texture", "tex_b"),
        ("dangly.texture", "tex_c"),
        ("blade.texture", "fx_blade"),
        ("lamp.flare[0]", "fx_flare01"),
        ("lamp.flare[1]", "fx_ring"),
        ("hook.model", "dor_lhr01"),
    ]
    .map(|(s, v)| (s.to_string(), v.to_string()));
    assert_eq!(refs, want);
    let wave = m.animation("wave").unwrap();
    assert_eq!(wave.length, 1.5);
    assert_eq!(wave.events[0].name, "snd_footstep");
    assert_eq!(wave.nodes.len(), 2);
    assert_eq!(
        wave.nodes[1]
            .controller(controller_types::POSITION)
            .unwrap()
            .vector_keys()
            .unwrap()
            .next()
            .unwrap()
            .value,
        [0.0, 0.0, 0.5]
    );
}

#[test]
fn the_mdx_holds_skins_last_and_pads_by_the_length_past_sixteen() {
    let (mdl, mdx) = every_kind().to_bytes().unwrap();
    let m = rakata_mdl::parse(&mdl, &mdx).unwrap();
    let start = |name: &str| {
        let mesh = m.node(name).unwrap().mesh.as_ref().unwrap();
        mesh.vertex_buffer.as_ptr() as usize - mdx.as_ptr() as usize
    };
    // trimesh (number 1, stride 32), dangly (3, 24), walkmesh (4, 24),
    // then the skin (2, 64): each block is four vertices with its
    // terminator, then as many bytes as the length is past 16.
    assert_eq!(start("trimesh"), 0);
    assert_eq!(start("dangly"), 128);
    // 128 + 96 = 224, a multiple of 16: no padding.
    assert_eq!(start("walkmesh"), 224);
    // 224 + 96 = 320.
    assert_eq!(start("skin"), 320);
    assert_eq!(mdx.len(), 320 + 4 * 64);
}

#[test]
fn a_layout_another_writer_chose_reads_equal_and_is_described() {
    let model = every_kind();
    let (mut mdl, mdx) = model.to_bytes().unwrap();
    // Zero the empty child array of the first leaf node, as some tools do.
    let m = rakata_mdl::parse(&mdl, &mdx).unwrap();
    let leaf = m.node("skin").unwrap();
    assert!(leaf.children.is_empty());
    let data = &mdl[12..];
    let skin_at = (0..data.len() - 2)
        .find(|&i| {
            data[i..i + 2] == 0x061u16.to_le_bytes()
                && data[i + 4..i + 6] == 2u16.to_le_bytes()
                && data[i + 2..i + 4] == 2u16.to_le_bytes()
        })
        .unwrap();
    put_u32(&mut mdl, 12 + skin_at + 44, 0);
    let read = ModelFile::parse(&mdl, &mdx).unwrap();
    assert!(read.semantic_eq(&model));
    assert_ne!(read.to_bytes().unwrap().0, mdl);
    let l = layout(&mdl, &mdx).unwrap();
    assert_eq!(l.empty_at_zero, (1, Some("child array")));
    assert!(
        l.mdl_line()
            .contains("1 empty arrays at zero that BioWare points on, first the child array"),
        "{}",
        l.mdl_line()
    );
    assert_eq!(
        l.mdx_line(),
        "mdx: blocks in BioWare's order; BioWare's padding; ends at its last block"
    );
}

#[test]
fn parts_that_disagree_are_refused_by_the_writer() {
    let mut model = every_kind();
    let Body::Mesh(m) = &mut model.root.children[0].body else {
        panic!("a mesh")
    };
    m.positions.pop();
    let e = model.to_bytes().unwrap_err();
    assert_eq!(e.code(), codes::INCONSISTENT);

    let mut model = every_kind();
    model.head_node = NodePointer::Node(99);
    assert_eq!(model.to_bytes().unwrap_err().code(), codes::INCONSISTENT);
}

#[test]
fn a_node_of_a_kind_the_owned_model_does_not_hold_is_refused() {
    let mut model = every_kind();
    // The NWN animmesh flags on a trimesh: its header's length is unknown.
    put_u16(&mut model.root.children[0].header, 0, 0x0A1);
    let (mdl, mdx) = model.to_bytes().unwrap();
    let e = ModelFile::parse(&mdl, &mdx).unwrap_err();
    assert_eq!(e.code(), codes::UNSUPPORTED);
}
