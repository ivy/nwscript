//! Hand-made models, built here, one for each variant the crate decodes
//! and each error it reports.

use rakata_mdl::{Classification, Error, Model, NodeKind, codes, mdx_flags};

const NONE: u32 = u32::MAX;

/// A model's MDL and MDX under construction. Offsets in the model data
/// count from its start, as the format's do.
struct Builder {
    d: Vec<u8>,
    mdx: Vec<u8>,
}

/// One mesh's contents: positions, normals, UV1 and UV2 per vertex (each
/// written to the MDX when the flag asks), and triangles.
struct MeshSpec {
    mdx_flags: u32,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uv1: Vec<[f32; 2]>,
    uv2: Vec<[f32; 2]>,
    faces: Vec<[u16; 3]>,
    texture: &'static str,
    lightmap: &'static str,
}

impl Builder {
    fn new(name: &str, classification: u8, supermodel: &str, names: &[&str]) -> Self {
        let mut b = Builder {
            d: vec![0; 0xC4],
            mdx: Vec::new(),
        };
        b.str32(0x08, name);
        b.d[0x4C] = 2;
        b.d[0x50] = classification;
        b.d[0x53] = 1;
        b.put_f32s(0x68, &[-5.0, -5.0, -1.0, 5.0, 5.0, 10.0, 7.0, 1.0]);
        b.str32(0x88, supermodel);
        // The name array: pointers, then the names back to back.
        b.put_array(0xB8, 0xC4, names.len());
        let ptrs = b.d.len();
        b.d.resize(ptrs + 4 * names.len(), 0);
        for (i, n) in names.iter().enumerate() {
            let at = b.d.len() as u32;
            b.put_u32(ptrs + 4 * i, at);
            b.d.extend_from_slice(n.as_bytes());
            b.d.push(0);
        }
        b
    }

    fn put_u32(&mut self, at: usize, v: u32) {
        self.d[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn put_u16(&mut self, at: usize, v: u16) {
        self.d[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }

    fn put_f32s(&mut self, at: usize, v: &[f32]) {
        for (i, x) in v.iter().enumerate() {
            self.put_u32(at + 4 * i, x.to_bits());
        }
    }

    fn put_array(&mut self, at: usize, off: usize, n: usize) {
        self.put_u32(at, off as u32);
        self.put_u32(at + 4, n as u32);
        self.put_u32(at + 8, n as u32);
    }

    fn str32(&mut self, at: usize, s: &str) {
        self.d[at..at + 32].fill(0);
        self.d[at..at + s.len()].copy_from_slice(s.as_bytes());
    }

    /// Append `bytes`, four-aligned, and return their offset.
    fn append(&mut self, bytes: &[u8]) -> usize {
        while !self.d.len().is_multiple_of(4) {
            self.d.push(0);
        }
        let at = self.d.len();
        self.d.extend_from_slice(bytes);
        at
    }

    /// A node header (and a zeroed mesh header when the flags ask, and
    /// `extra` zero bytes for a derived type's own header).
    fn node(&mut self, flags: u16, number: u16, pos: [f32; 3], q: [f32; 4], extra: usize) -> usize {
        let len = 80 + if flags & 0x20 != 0 { 332 } else { 0 } + extra;
        let at = self.append(&vec![0; len]);
        self.put_u16(at, flags);
        self.put_u16(at + 2, number);
        self.put_u16(at + 4, number);
        self.put_f32s(at + 16, &pos);
        self.put_f32s(at + 28, &q);
        at
    }

    fn root(&mut self, node: usize) {
        self.put_u32(0x28, node as u32);
        self.put_u32(0xA8, node as u32);
    }

    fn children(&mut self, parent: usize, kids: &[usize]) {
        let bytes: Vec<u8> = kids
            .iter()
            .flat_map(|k| (*k as u32).to_le_bytes())
            .collect();
        let at = self.append(&bytes);
        self.put_array(parent + 44, at, kids.len());
        for k in kids {
            self.put_u32(k + 12, parent as u32);
        }
    }

    fn mesh(&mut self, node: usize, m: &MeshSpec) {
        let h = node + 80;
        let nv = m.positions.len();
        self.put_f32s(h + 60, &[0.8, 0.8, 0.8, 0.2, 0.2, 0.2]);
        self.put_u32(h + 84, 3);
        self.str32(h + 88, m.texture);
        self.str32(h + 120, m.lightmap);
        // Faces: normal, distance, surface, adjacency, vertices.
        let mut faces = Vec::new();
        for f in &m.faces {
            for x in [0.0f32, 0.0, 1.0, 0.0] {
                faces.extend(x.to_le_bytes());
            }
            faces.extend(1u32.to_le_bytes());
            for a in [-1i16, -1, -1] {
                faces.extend(a.to_le_bytes());
            }
            for v in f {
                faces.extend(v.to_le_bytes());
            }
        }
        let fo = self.append(&faces);
        self.put_array(h + 8, fo, m.faces.len());
        if !m.faces.is_empty() {
            let list: Vec<u8> = m
                .faces
                .iter()
                .flatten()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let list_at = self.append(&list);
            let count_at = self.append(&(3 * m.faces.len() as u32).to_le_bytes());
            let ptr_at = self.append(&(list_at as u32).to_le_bytes());
            self.put_array(h + 176, count_at, 1);
            self.put_array(h + 188, ptr_at, 1);
        }
        // The MDL's own positions.
        let pos: Vec<u8> = m
            .positions
            .iter()
            .flatten()
            .flat_map(|x| x.to_le_bytes())
            .collect();
        let pos_at = self.append(&pos);
        self.put_u32(h + 328, pos_at as u32);
        // The MDX block: the attributes the flags name, in order.
        let mut offs = [NONE; 11];
        let mut stride = 0u32;
        for (bit, slot, floats) in [
            (mdx_flags::POSITION, 0, 3),
            (mdx_flags::NORMAL, 1, 3),
            (mdx_flags::UV1, 3, 2),
            (mdx_flags::UV2, 4, 2),
        ] {
            if m.mdx_flags & bit != 0 {
                offs[slot] = stride;
                stride += 4 * floats;
            }
        }
        self.put_u32(h + 252, stride);
        self.put_u32(h + 256, m.mdx_flags);
        for (i, o) in offs.iter().enumerate() {
            self.put_u32(h + 260 + 4 * i, *o);
        }
        self.put_u16(h + 304, nv as u16);
        self.put_u16(
            h + 306,
            (!m.texture.is_empty()) as u16 + (!m.lightmap.is_empty()) as u16,
        );
        self.d[h + 308] = (!m.lightmap.is_empty()) as u8;
        self.d[h + 311] = 1;
        self.d[h + 313] = 1;
        if m.mdx_flags == 0 {
            self.put_u32(h + 324, 0);
            return;
        }
        self.put_u32(h + 324, self.mdx.len() as u32);
        for i in 0..nv {
            let mut v: Vec<f32> = Vec::new();
            if m.mdx_flags & mdx_flags::POSITION != 0 {
                v.extend(m.positions[i]);
            }
            if m.mdx_flags & mdx_flags::NORMAL != 0 {
                v.extend(m.normals[i]);
            }
            if m.mdx_flags & mdx_flags::UV1 != 0 {
                v.extend(m.uv1[i]);
            }
            if m.mdx_flags & mdx_flags::UV2 != 0 {
                v.extend(m.uv2[i]);
            }
            self.mdx.extend(v.iter().flat_map(|x| x.to_le_bytes()));
        }
        // The terminator row the vanilla files end each block with.
        self.mdx
            .extend((0..stride / 4).flat_map(|_| 1.0e7f32.to_le_bytes()));
    }

    fn animation(&mut self, name: &str) {
        let at = self.append(&[0; 0x88]);
        self.str32(at + 0x08, name);
        self.d[at + 0x4C] = 5;
        let ptrs = self.append(&(at as u32).to_le_bytes());
        self.put_array(0x58, ptrs, 1);
    }

    fn finish(&self) -> (Vec<u8>, Vec<u8>) {
        let mut mdl = Vec::new();
        mdl.extend(0u32.to_le_bytes());
        mdl.extend((self.d.len() as u32).to_le_bytes());
        mdl.extend((self.mdx.len() as u32).to_le_bytes());
        mdl.extend(&self.d);
        (mdl, self.mdx.clone())
    }
}

const IDENTITY: [f32; 4] = [1.0, 0.0, 0.0, 0.0];

fn quad(mdx_flags: u32) -> MeshSpec {
    MeshSpec {
        mdx_flags,
        positions: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        normals: vec![[0.0, 0.0, 1.0]; 4],
        uv1: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        uv2: vec![[0.5, 0.5], [0.75, 0.5], [0.75, 0.75], [0.5, 0.75]],
        faces: vec![[0, 1, 2], [0, 2, 3]],
        texture: "lhr_floor01",
        lightmap: "m01aa_01a_lm0",
    }
}

/// A room: a root dummy with a lit trimesh, a dangly mesh, an emitter and
/// a light under it, and one animation.
fn room() -> (Vec<u8>, Vec<u8>) {
    let names = ["m01aa_01a", "floor", "cloth", "sparks", "lamp"];
    let mut b = Builder::new("m01aa_01a", 0, "NULL", &names);
    let root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    b.root(root);
    // A quarter turn about Z, then one metre along X.
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let floor = b.node(0x021, 1, [1.0, 0.0, 0.0], [s, 0.0, 0.0, s], 0);
    b.mesh(floor, &quad(0x27));
    let cloth = b.node(0x121, 2, [0.0, 0.0, 2.0], IDENTITY, 28);
    let mut c = quad(0x23);
    c.lightmap = "";
    c.texture = "NULL";
    b.mesh(cloth, &c);
    let sparks = b.node(0x005, 3, [0.0; 3], IDENTITY, 0x150);
    let lamp = b.node(0x003, 4, [0.0, 0.0, 3.0], IDENTITY, 0x5C);
    b.children(root, &[floor, cloth, sparks]);
    b.children(cloth, &[lamp]);
    b.animation("animloop1");
    b.finish()
}

#[test]
fn a_room_with_meshes_of_two_kinds_and_other_nodes() {
    let (mdl, mdx) = room();
    let m = Model::parse(&mdl, &mdx).unwrap();
    assert_eq!(m.name, "m01aa_01a");
    assert_eq!(m.classification, Classification::Other);
    assert_eq!(m.supermodel, None);
    assert!(m.fog);
    assert_eq!(m.bounding_box.min, [-5.0, -5.0, -1.0]);
    assert_eq!(m.radius, 7.0);
    assert_eq!(m.animations, ["animloop1"]);
    let names: Vec<_> = m.nodes.iter().map(|n| (n.name, n.kind, n.parent)).collect();
    assert_eq!(
        names,
        [
            ("m01aa_01a", NodeKind::Dummy, None),
            ("floor", NodeKind::Trimesh, Some(0)),
            ("cloth", NodeKind::Dangly, Some(0)),
            ("lamp", NodeKind::Light, Some(2)),
            ("sparks", NodeKind::Emitter, Some(0)),
        ]
    );
    assert_eq!(m.root().children, [1, 2, 4]);

    let floor = m.node("FLOOR").unwrap();
    assert_eq!(floor.position, [1.0, 0.0, 0.0]);
    assert_eq!(floor.orientation.z, std::f32::consts::FRAC_1_SQRT_2);
    let mesh = floor.mesh.as_ref().unwrap();
    assert_eq!(mesh.texture, Some("lhr_floor01"));
    assert_eq!(mesh.lightmap, Some("m01aa_01a_lm0"));
    assert!(mesh.lightmapped && mesh.render && mesh.shadow);
    assert_eq!(mesh.transparency_hint, 3);
    assert_eq!(mesh.diffuse, [0.8; 3]);
    assert_eq!(mesh.vertex_count, 4);
    assert_eq!(mesh.stride, 40);
    assert_eq!(mesh.vertex_buffer.len(), 160);
    // The vertices are the MDX's bytes, not copies.
    assert!(std::ptr::eq(mesh.vertex_buffer.as_ptr(), mdx.as_ptr()));
    assert!(std::ptr::eq(mesh.positions.buffer().as_ptr(), mdx.as_ptr()));
    assert_eq!(mesh.positions.get(2), Some([1.0, 1.0, 0.0]));
    let n = mesh.normals.unwrap();
    assert_eq!((n.offset(), n.stride()), (12, 40));
    assert_eq!(n.get(0), Some([0.0, 0.0, 1.0]));
    assert_eq!(mesh.uv1.unwrap().offset(), 24);
    assert_eq!(
        mesh.uv2.unwrap().iter().collect::<Vec<_>>(),
        [[0.5, 0.5], [0.75, 0.5], [0.75, 0.75], [0.5, 0.75]]
    );
    assert_eq!(mesh.faces.len(), 2);
    let f = mesh.faces.get(1).unwrap();
    assert_eq!((f.vertices, f.adjacent, f.surface), ([0, 2, 3], [-1; 3], 1));
    let idx = mesh.indices.unwrap();
    assert_eq!(idx.iter().collect::<Vec<_>>(), [0, 1, 2, 0, 2, 3]);
    assert_eq!(idx.bytes().len(), 12);

    let cloth = m.node("cloth").unwrap().mesh.as_ref().unwrap();
    assert_eq!((cloth.texture, cloth.lightmap), (None, None));
    assert_eq!(cloth.stride, 32);
    assert!(cloth.uv2.is_none());
    assert_eq!(cloth.uv1.unwrap().get(3), Some([0.0, 1.0]));
    assert!(std::ptr::eq(
        cloth.vertex_buffer.as_ptr(),
        mdx[200..].as_ptr()
    ));

    // The floor turned a quarter about Z and moved along X; the cloth
    // raised two metres.
    let b = m.bounds().unwrap();
    for (got, want) in b.min.iter().zip([0.0, 0.0, 0.0]) {
        assert!((got - want).abs() < 1e-6, "{:?}", b);
    }
    for (got, want) in b.max.iter().zip([1.0, 1.0, 2.0]) {
        assert!((got - want).abs() < 1e-6, "{:?}", b);
    }
}

#[test]
fn a_door_with_a_supermodel_and_a_mesh_with_its_positions_in_the_mdl() {
    let mut b = Builder::new("dor_lhr01", 8, "dor_base", &["dor_lhr01", "blade"]);
    let root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    b.root(root);
    let blade = b.node(0x821, 1, [0.0; 3], IDENTITY, 40);
    let mut q = quad(0);
    q.lightmap = "";
    b.mesh(blade, &q);
    b.children(root, &[blade]);
    let (mdl, _) = b.finish();
    let m = Model::parse(&mdl, &[]).unwrap();
    assert_eq!(m.classification, Classification::Door);
    assert_eq!(m.supermodel, Some("dor_base"));
    assert!(m.animations.is_empty());
    let mesh = m.nodes[1].mesh.as_ref().unwrap();
    assert_eq!(m.nodes[1].kind, NodeKind::Saber);
    assert!(mesh.vertex_buffer.is_empty());
    assert_eq!((mesh.positions.stride(), mesh.positions.len()), (12, 4));
    assert_eq!(mesh.positions.get(1), Some([1.0, 0.0, 0.0]));
    assert!(mesh.normals.is_none() && mesh.uv1.is_none());
}

#[test]
fn an_ascii_model_is_refused() {
    let e = Model::parse(b"# MAXMODEL ASCII\nnewmodel x\n", &[]).unwrap_err();
    assert!(matches!(e, Error::NotBinary { .. }));
    assert_eq!(e.code(), codes::NOT_BINARY);
}

#[test]
fn a_model_cut_short_is_refused() {
    let (mdl, mdx) = room();
    for len in [8, 12 + 0x40, mdl.len() - 40] {
        let e = Model::parse(&mdl[..len], &mdx).unwrap_err();
        assert_eq!(e.code(), codes::TRUNCATED, "{len}: {e}");
    }
}

#[test]
fn an_mdx_cut_short_is_refused() {
    let (mdl, mdx) = room();
    let e = Model::parse(&mdl, &mdx[..100]).unwrap_err();
    assert_eq!(e.code(), codes::MDX_OUT_OF_RANGE, "{e}");
}

#[test]
fn a_node_reached_twice_is_refused() {
    let mut b = Builder::new("loop", 0, "NULL", &["loop", "child"]);
    let root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    b.root(root);
    let child = b.node(0x001, 1, [0.0; 3], IDENTITY, 0);
    b.children(root, &[child]);
    b.children(child, &[root]);
    let e = Model::parse(&b.finish().0, &[]).unwrap_err();
    assert_eq!(e.code(), codes::BAD_TREE, "{e}");

    let mut b = Builder::new("short", 0, "NULL", &["short"]);
    let root = b.node(0x001, 7, [0.0; 3], IDENTITY, 0);
    b.root(root);
    let e = Model::parse(&b.finish().0, &[]).unwrap_err();
    assert_eq!(e.code(), codes::BAD_TREE, "{e}");
}

#[test]
fn a_name_not_ascii_is_refused() {
    let (mut mdl, mdx) = room();
    mdl[12 + 0x08] = 0xE9;
    let e = Model::parse(&mdl, &mdx).unwrap_err();
    assert_eq!(e.code(), codes::BAD_NAME, "{e}");
}

#[test]
fn an_attribute_past_the_stride_is_refused() {
    let mut b = Builder::new("bad", 0, "NULL", &["bad", "mesh"]);
    let root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    b.root(root);
    let mesh = b.node(0x021, 1, [0.0; 3], IDENTITY, 0);
    b.mesh(mesh, &quad(0x23));
    b.put_u32(mesh + 80 + 272, 28);
    b.children(root, &[mesh]);
    let (mdl, mdx) = b.finish();
    let e = Model::parse(&mdl, &mdx).unwrap_err();
    assert_eq!(e.code(), codes::BAD_VERTEX_LAYOUT, "{e}");
}

#[test]
fn a_face_naming_a_missing_vertex_is_refused() {
    let mut b = Builder::new("bad", 0, "NULL", &["bad", "mesh"]);
    let root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    b.root(root);
    let mesh = b.node(0x021, 1, [0.0; 3], IDENTITY, 0);
    let mut q = quad(0x23);
    q.faces[1] = [0, 2, 4];
    b.mesh(mesh, &q);
    b.children(root, &[mesh]);
    let (mdl, mdx) = b.finish();
    let e = Model::parse(&mdl, &mdx).unwrap_err();
    assert_eq!(
        e,
        Error::BadIndex {
            node: "mesh".into(),
            index: 4,
            vertices: 4
        }
    );
    assert_eq!(e.code(), codes::BAD_INDEX);
}
