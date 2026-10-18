//! Hand-made models, built here, one for each variant the crate decodes
//! and each error it reports.

use std::collections::HashMap;

use rakata_mdl::{
    Chain, ChainError, Classification, Error, Lineage, Model, NodeKind, Quaternion, codes,
    controller_types, decompress_quaternion, mdx_flags,
};

/// `mdl`, kept with `mdx` after it as a seed for the fuzz targets in the
/// directory `$RAKATA_FUZZ_SEEDS` names, when it is set.
fn seed<'a>(mdl: &'a [u8], mdx: &[u8]) -> &'a [u8] {
    if let Some(dir) = std::env::var_os("RAKATA_FUZZ_SEEDS") {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let bytes = [mdl, mdx].concat();
        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{:016x}", h.finish())), bytes).unwrap();
    }
    mdl
}

const NONE: u32 = u32::MAX;

/// A model's MDL and MDX under construction. Offsets in the model data
/// count from its start, as the format's do.
struct Builder {
    d: Vec<u8>,
    mdx: Vec<u8>,
    anims: Vec<u32>,
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
    /// A skin mesh's weights and local bone indices per vertex, written
    /// after the other attributes.
    skin: Vec<([f32; 4], [f32; 4])>,
}

/// A controller to write: type, the u16 after it, the column byte, the
/// times, and each row's values as 32-bit words.
struct Ctl {
    kind: u32,
    field: u16,
    columns: u8,
    times: Vec<f32>,
    rows: Vec<Vec<u32>>,
}

fn floats(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// A compressed orientation word as the vanilla files are measured to
/// round: each of `x, y, z` scaled from -1..1 to 0..2047 (0..1023 for
/// `z`) and truncated.
fn compress(q: [f32; 4]) -> u32 {
    let f = |v: f32, max: f32| ((v + 1.0) / 2.0 * max) as u32;
    f(q[0], 2047.0) | (f(q[1], 2047.0) << 11) | (f(q[2], 1023.0) << 22)
}

impl Builder {
    fn new(name: &str, classification: u8, supermodel: &str, names: &[&str]) -> Self {
        let mut b = Builder {
            d: vec![0; 0xC4],
            mdx: Vec::new(),
            anims: Vec::new(),
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

    /// The node's controller records and their data array.
    fn controllers(&mut self, node: usize, ctls: &[Ctl]) {
        let mut records = Vec::new();
        let mut data: Vec<u32> = Vec::new();
        for c in ctls {
            let tk = data.len() as u16;
            data.extend(floats(&c.times));
            let vk = data.len() as u16;
            for r in &c.rows {
                data.extend(r);
            }
            records.extend(c.kind.to_le_bytes());
            records.extend(c.field.to_le_bytes());
            records.extend((c.times.len() as u16).to_le_bytes());
            records.extend(tk.to_le_bytes());
            records.extend(vk.to_le_bytes());
            records.extend([c.columns, 0, 0, 0]);
        }
        let at = self.append(&records);
        self.put_array(node + 56, at, ctls.len());
        let bytes: Vec<u8> = data.iter().flat_map(|w| w.to_le_bytes()).collect();
        let at = self.append(&bytes);
        self.put_array(node + 68, at, data.len());
    }

    /// The skin header of the skin node at `node`: a bone map (one float
    /// per name) and a qbone and tbone per name.
    fn skin_header(&mut self, node: usize, bone_map: &[f32], q: &[[f32; 4]], t: &[[f32; 3]]) {
        let h = node + 80 + 332;
        let bm: Vec<u8> = bone_map.iter().flat_map(|x| x.to_le_bytes()).collect();
        let bm_at = self.append(&bm);
        self.put_u32(h + 20, bm_at as u32);
        self.put_u32(h + 24, bone_map.len() as u32);
        let qb: Vec<u8> = q.iter().flatten().flat_map(|x| x.to_le_bytes()).collect();
        let q_at = self.append(&qb);
        self.put_array(h + 28, q_at, q.len());
        let tb: Vec<u8> = t.iter().flatten().flat_map(|x| x.to_le_bytes()).collect();
        let t_at = self.append(&tb);
        self.put_array(h + 40, t_at, t.len());
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
        let skin_at = stride;
        if !m.skin.is_empty() {
            stride += 32;
            self.put_u32(h + 332 + 12, skin_at);
            self.put_u32(h + 332 + 16, skin_at + 16);
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
            if let Some((w, b)) = m.skin.get(i) {
                v.extend(w);
                v.extend(b);
            }
            self.mdx.extend(v.iter().flat_map(|x| x.to_le_bytes()));
        }
        // The terminator row the vanilla files end each block with.
        let term = if m.skin.is_empty() { 1.0e7f32 } else { 1.0e6 };
        self.mdx
            .extend((0..stride / 4).flat_map(|_| term.to_le_bytes()));
    }

    /// An animation header whose tree is rooted at `root`, with events;
    /// added to the model's animation array.
    fn animation(&mut self, name: &str, length: f32, root: usize, events: &[(f32, &str)]) {
        let at = self.append(&[0; 0x88]);
        self.str32(at + 0x08, name);
        self.put_u32(at + 0x28, root as u32);
        self.d[at + 0x4C] = 5;
        self.put_f32s(at + 0x50, &[length, 0.25]);
        let model = String::from_utf8_lossy(&self.d[8..40])
            .trim_end_matches('\0')
            .to_owned();
        self.str32(at + 0x58, &model);
        let mut ev = Vec::new();
        for (t, n) in events {
            ev.extend(t.to_le_bytes());
            let mut name = [0u8; 32];
            name[..n.len()].copy_from_slice(n.as_bytes());
            ev.extend(name);
        }
        let ev_at = self.append(&ev);
        self.put_array(at + 0x78, ev_at, events.len());
        self.anims.push(at as u32);
        let ptrs: Vec<u8> = self.anims.iter().flat_map(|a| a.to_le_bytes()).collect();
        let ptrs_at = self.append(&ptrs);
        self.put_array(0x58, ptrs_at, self.anims.len());
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
        skin: Vec::new(),
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
    // The floor's resting position and orientation as controllers too.
    b.controllers(
        floor,
        &[
            Ctl {
                kind: 8,
                field: 0xFFFF,
                columns: 3,
                times: vec![0.0],
                rows: vec![floats(&[1.0, 0.0, 0.0])],
            },
            Ctl {
                kind: 20,
                field: 0xFFFF,
                columns: 4,
                times: vec![0.0],
                rows: vec![floats(&[0.0, 0.0, s, s])],
            },
        ],
    );
    // An animation: the floor slides and turns (compressed keys), the
    // cloth moves on a bezier path and scales.
    let a_root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    let a_floor = b.node(0x001, 1, [0.0; 3], IDENTITY, 0);
    let a_cloth = b.node(0x001, 2, [0.0; 3], IDENTITY, 0);
    b.children(a_root, &[a_floor, a_cloth]);
    b.controllers(
        a_floor,
        &[
            Ctl {
                kind: 8,
                field: 16,
                columns: 3,
                times: vec![0.0, 1.0],
                rows: vec![floats(&[0.0, 0.0, 0.0]), floats(&[0.5, 0.0, 0.0])],
            },
            Ctl {
                kind: 20,
                field: 28,
                columns: 2,
                times: vec![0.0, 0.5, 1.0],
                rows: vec![
                    vec![compress([0.0, 0.0, s, s])],
                    vec![compress([0.0, 0.0, 0.0, 1.0])],
                    vec![compress([0.0, 0.6, 0.0, 0.8])],
                ],
            },
        ],
    );
    b.controllers(
        a_cloth,
        &[
            Ctl {
                kind: 8,
                field: 16,
                columns: 3 | 0x10,
                times: vec![0.0, 2.0],
                rows: vec![
                    floats(&[0.0, 0.0, 0.0, 0.1, 0.0, 0.0, -0.1, 0.0, 0.0]),
                    floats(&[0.0, 0.0, 1.0, 0.0, 0.2, 0.0, 0.0, -0.2, 0.0]),
                ],
            },
            Ctl {
                kind: 36,
                field: 0xFFFF,
                columns: 1,
                times: vec![0.0, 2.0],
                rows: vec![floats(&[1.0]), floats(&[1.5])],
            },
        ],
    );
    b.animation(
        "animloop1",
        2.0,
        a_root,
        &[(0.5, "snd_footstep"), (1.5, "hit")],
    );
    b.finish()
}

#[test]
fn a_room_with_meshes_of_two_kinds_and_other_nodes() {
    let (mdl, mdx) = room();
    let m = Model::parse(seed(&mdl, &mdx), &mdx).unwrap();
    assert_eq!(m.name, "m01aa_01a");
    assert_eq!(m.classification, Classification::Other);
    assert_eq!(m.supermodel, None);
    assert!(m.fog);
    assert_eq!(m.bounding_box.min, [-5.0, -5.0, -1.0]);
    assert_eq!(m.radius, 7.0);
    let names: Vec<&str> = m.animations.iter().map(|a| a.name).collect();
    assert_eq!(names, ["animloop1"]);
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
    let m = Model::parse(seed(&mdl, &[]), &[]).unwrap();
    assert_eq!(m.classification, Classification::Door);
    assert_eq!(m.supermodel, Some("dor_base"));
    assert!(m.animations.is_empty());
    assert!(
        m.nodes
            .iter()
            .all(|n| n.controllers.is_empty() && n.skin.is_none())
    );
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
    let e = Model::parse(seed(&mdl, &mdx), &mdx).unwrap_err();
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
    let e = Model::parse(seed(&mdl, &mdx), &mdx).unwrap_err();
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
    let e = Model::parse(seed(&mdl, &mdx), &mdx).unwrap_err();
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

fn close(a: [f32; 4], b: Quaternion, tol: f32) -> bool {
    let d = |s: f32| {
        (a[0] - s * b.x)
            .abs()
            .max((a[1] - s * b.y).abs())
            .max((a[2] - s * b.z).abs())
            .max((a[3] - s * b.w).abs())
    };
    d(1.0).min(d(-1.0)) < tol
}

#[test]
fn geometry_controllers_repeat_the_node_header() {
    let (mdl, mdx) = room();
    let m = Model::parse(seed(&mdl, &mdx), &mdx).unwrap();
    let floor = m.node("floor").unwrap();
    assert_eq!(floor.controllers.len(), 2);
    let p = floor.controller(controller_types::POSITION).unwrap();
    assert_eq!((p.field, p.len(), p.is_bezier()), (0xFFFF, 1, false));
    let k = p.vector_keys().unwrap().next().unwrap();
    assert_eq!((k.time, k.value, k.bezier), (0.0, floor.position, None));
    let o = floor.controller(controller_types::ORIENTATION).unwrap();
    assert!(!o.is_compressed());
    assert_eq!(o.orientation(0).unwrap().value, floor.orientation);
    assert!(o.scalar_keys().is_none());
}

#[test]
fn an_animation_with_events_and_linear_compressed_and_bezier_keys() {
    let (mdl, mdx) = room();
    let m = Model::parse(seed(&mdl, &mdx), &mdx).unwrap();
    let a = m.animation("ANIMLOOP1").unwrap();
    assert_eq!((a.length, a.transition, a.root), (2.0, 0.25, "m01aa_01a"));
    let events: Vec<_> = a.events.iter().map(|e| (e.time, e.name)).collect();
    assert_eq!(events, [(0.5, "snd_footstep"), (1.5, "hit")]);
    let tree: Vec<_> = a.nodes.iter().map(|n| (n.name, n.parent)).collect();
    assert_eq!(
        tree,
        [("m01aa_01a", None), ("floor", Some(0)), ("cloth", Some(0))]
    );

    let floor = a.node("floor").unwrap();
    let p = floor.controller(controller_types::POSITION).unwrap();
    assert_eq!(p.field, 16);
    let keys: Vec<_> = p
        .vector_keys()
        .unwrap()
        .map(|k| (k.time, k.value))
        .collect();
    assert_eq!(keys, [(0.0, [0.0; 3]), (1.0, [0.5, 0.0, 0.0])]);
    // The keys are views of the MDL's bytes.
    let at = p.time_bytes().as_ptr() as usize - mdl.as_ptr() as usize;
    assert!(at < mdl.len());

    let o = floor.controller(controller_types::ORIENTATION).unwrap();
    assert!(o.is_compressed());
    assert_eq!(
        (o.field, o.columns, o.row_len(), o.value_len()),
        (28, 2, 1, 4)
    );
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let keys: Vec<_> = o.orientation_keys().unwrap().collect();
    assert_eq!(
        keys.iter().map(|k| k.time).collect::<Vec<_>>(),
        [0.0, 0.5, 1.0]
    );
    // A step of the 11- and 10-bit fields is 2/2047 and 2/1023.
    let want = [[0.0, 0.0, s, s], [0.0, 0.0, 0.0, 1.0], [0.0, 0.6, 0.0, 0.8]];
    for (k, want) in keys.iter().zip(want) {
        assert!(close(want, k.value, 2.0 / 1023.0), "{:?} {want:?}", k.value);
        let q = k.value;
        let n = q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z;
        assert!((n - 1.0).abs() < 1e-5);
    }
    assert!(o.vector_keys().is_none());

    let cloth = a.node("cloth").unwrap();
    let b = cloth.controller(controller_types::POSITION).unwrap();
    assert!(b.is_bezier());
    assert_eq!((b.row_len(), b.value_len()), (9, 3));
    let last = b.vector_keys().unwrap().last().unwrap();
    assert_eq!(last.time, 2.0);
    assert_eq!(last.value, [0.0, 0.0, 1.0]);
    assert_eq!(last.bezier, Some([[0.0, 0.2, 0.0], [0.0, -0.2, 0.0]]));
    let scale = cloth.controller(controller_types::SCALE).unwrap();
    let k: Vec<_> = scale.scalar_keys().unwrap().map(|k| k.value).collect();
    assert_eq!(k, [1.0, 1.5]);
    assert_eq!(scale.floats(1).unwrap().collect::<Vec<_>>(), [1.5]);
    assert!(scale.orientation_keys().is_none());
}

#[test]
fn compressed_words_decode_to_their_fields() {
    // All fields zero: x = y = z = -1, past the unit sphere, so w is 0.
    let q = decompress_quaternion(0);
    assert_eq!((q.x, q.y, q.z, q.w), (-1.0, -1.0, -1.0, 0.0));
    // x at its greatest, y and z at their middles.
    let q = decompress_quaternion(0x7FF | (1023 << 11) | (511 << 22));
    assert_eq!(q.x, 1.0);
    assert!(q.y.abs() < 1e-3 && q.z.abs() < 2e-3 && q.w < 0.05);
    // z alone at its greatest.
    let q = decompress_quaternion(1023 << 22);
    assert_eq!(q.z, 1.0);
}

#[test]
fn a_controller_past_its_data_is_refused() {
    let mut b = Builder::new("bad", 0, "NULL", &["bad"]);
    let root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    b.root(root);
    b.controllers(
        root,
        &[Ctl {
            kind: 8,
            field: 0xFFFF,
            columns: 3,
            times: vec![0.0],
            rows: vec![floats(&[0.0; 3])],
        }],
    );
    // Claim three rows.
    let records = u32::from_le_bytes(b.d[root + 56..root + 60].try_into().unwrap()) as usize;
    b.put_u16(records + 6, 3);
    let e = Model::parse(&b.finish().0, &[]).unwrap_err();
    assert_eq!(e.code(), codes::BAD_CONTROLLER, "{e}");
}

/// A body: a root, two bones under it, and a skin mesh bound to both.
fn body(bone_map: [f32; 4]) -> (Vec<u8>, Vec<u8>) {
    let names = ["body", "pelvis", "thigh", "skin"];
    let mut b = Builder::new("body", 4, "NULL", &names);
    let root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    b.root(root);
    let pelvis = b.node(0x001, 1, [0.0, 0.0, 1.0], IDENTITY, 0);
    let thigh = b.node(0x001, 2, [0.2, 0.0, -0.5], IDENTITY, 0);
    let skin = b.node(0x061, 3, [0.0; 3], IDENTITY, 100);
    let mut q = quad(0x23);
    q.lightmap = "";
    q.skin = vec![
        ([1.0, 0.0, 0.0, 0.0], [0.0, -1.0, -1.0, -1.0]),
        ([0.5, 0.5, 0.0, 0.0], [0.0, 1.0, -1.0, -1.0]),
        ([0.25, 0.75, 0.0, 0.0], [1.0, 0.0, -1.0, -1.0]),
        ([1.0, 0.0, 0.0, 0.0], [1.0, -1.0, -1.0, -1.0]),
    ];
    b.mesh(skin, &q);
    b.skin_header(
        skin,
        &bone_map,
        &[[1.0, 0.0, 0.0, 0.0]; 4],
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [-0.2, 0.0, -0.5],
            [0.0; 3],
        ],
    );
    b.children(root, &[pelvis, skin]);
    b.children(pelvis, &[thigh]);
    b.finish()
}

#[test]
fn a_skin_with_its_bones_and_weights() {
    let (mdl, mdx) = body([-1.0, 0.0, 1.0, -1.0]);
    let m = Model::parse(seed(&mdl, &mdx), &mdx).unwrap();
    let n = m.node("skin").unwrap();
    assert_eq!(n.kind, NodeKind::Skin);
    let mesh = n.mesh.as_ref().unwrap();
    assert_eq!(mesh.stride, 64);
    let skin = n.skin.as_ref().unwrap();
    assert_eq!(skin.bones, [1, 2]);
    assert_eq!(skin.bone_map_len(), 4);
    assert_eq!((skin.bone_index(1), skin.bone_index(0)), (Some(0), None));
    assert_eq!(
        (skin.weights.offset(), skin.bone_indices.offset()),
        (32, 48)
    );
    assert!(std::ptr::eq(skin.weights.buffer().as_ptr(), mdx.as_ptr()));
    assert_eq!(skin.weights.get(2), Some([0.25, 0.75, 0.0, 0.0]));
    assert_eq!(skin.bone_indices.get(1), Some([0.0, 1.0, -1.0, -1.0]));
    assert_eq!(
        skin.influences(2).unwrap(),
        [Some((2, 0.25)), Some((1, 0.75)), None, None]
    );
    assert_eq!(skin.qbone(2), Some(Quaternion::IDENTITY));
    assert_eq!(skin.tbone(2), Some([-0.2, 0.0, -0.5]));
    assert_eq!(skin.tbone(4), None);
    // The MDX block ends with the skin's terminator row.
    assert_eq!(&mdx[4 * 64..4 * 64 + 4], &1.0e6f32.to_le_bytes());
}

#[test]
fn a_skin_naming_a_bone_index_twice_is_refused() {
    let (mdl, mdx) = body([-1.0, 0.0, 0.0, -1.0]);
    let e = Model::parse(seed(&mdl, &mdx), &mdx).unwrap_err();
    assert_eq!(e.code(), codes::BAD_SKIN, "{e}");
    let (mdl, mdx) = body([-1.0, 0.0, 2.0, -1.0]);
    let e = Model::parse(seed(&mdl, &mdx), &mdx).unwrap_err();
    assert_eq!(e.code(), codes::BAD_SKIN, "{e}");
}

/// A model with a torso, and animations of the given names that hold the
/// root and the torso.
fn animated(model: &str, supermodel: &str, anims: &[&str]) -> (Vec<u8>, Vec<u8>) {
    let mut b = Builder::new(model, 4, supermodel, &[model, "torso"]);
    let root = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
    b.root(root);
    let torso = b.node(0x001, 1, [0.0, 0.0, 1.0], IDENTITY, 0);
    b.children(root, &[torso]);
    for a in anims {
        let r = b.node(0x001, 0, [0.0; 3], IDENTITY, 0);
        let t = b.node(0x001, 1, [0.0; 3], IDENTITY, 0);
        b.children(r, &[t]);
        b.animation(a, 1.0, r, &[]);
    }
    b.finish()
}

type Files = HashMap<&'static str, (Vec<u8>, Vec<u8>)>;
type Loaded = Result<Option<(Vec<u8>, Vec<u8>)>, String>;

fn loader(files: &Files) -> impl FnMut(&str) -> Loaded + '_ {
    |n: &str| {
        if n == "broken" {
            return Err("disk on fire".to_owned());
        }
        Ok(files.get(n.to_ascii_lowercase().as_str()).cloned())
    }
}

#[test]
fn animations_resolve_through_the_supermodel_chain() {
    let mut files: Files = HashMap::new();
    files.insert("base", animated("base", "NULL", &["walk", "run"]));
    files.insert("middle", animated("middle", "Base", &["run"]));
    files.insert("child", animated("child", "middle", &["talk"]));
    let chain = Chain::load("child", loader(&files)).unwrap();
    assert_eq!(chain.missing, None);
    let names: Vec<&str> = chain.files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["child", "middle", "Base"]);
    let models = chain.parse().unwrap();
    let l = Lineage::new(&models).unwrap();
    let played: Vec<_> = l.animations().iter().map(|(i, a)| (*i, a.name)).collect();
    assert_eq!(played, [(0, "talk"), (1, "run"), (2, "walk")]);
    let (from, walk) = l.animation("WALK").unwrap();
    assert_eq!(from, 2);
    assert_eq!(l.bind(walk), [Some(0), Some(1)]);
    let torso = l.node("torso").map(|(i, n)| (i, n.position));
    assert_eq!(torso, Some((0, [0.0, 0.0, 1.0])));
    assert!(l.animation("fly").is_none());

    // Out of order, the lineage is refused.
    let wrong = [models[0].clone(), models[2].clone()];
    let e = Lineage::new(&wrong).unwrap_err();
    assert_eq!(e.code(), codes::BAD_LINEAGE, "{e}");
}

#[test]
fn a_chain_with_a_missing_supermodel_stops_there() {
    let mut files: Files = HashMap::new();
    files.insert("child", animated("child", "gone", &[]));
    let chain = Chain::load("child", loader(&files)).unwrap();
    assert_eq!(chain.files.len(), 1);
    assert_eq!(chain.missing.as_deref(), Some("gone"));
    let models = chain.parse().unwrap();
    assert!(Lineage::new(&models).unwrap().animations().is_empty());
}

#[test]
fn chains_that_cannot_be_loaded_are_refused() {
    let mut files: Files = HashMap::new();
    files.insert("a", animated("a", "b", &[]));
    files.insert("b", animated("b", "A", &[]));
    files.insert("c", animated("c", "broken", &[]));
    let e = Chain::load("a", loader(&files)).unwrap_err();
    assert_eq!(e.code(), codes::SUPERMODEL_CYCLE, "{e}");
    let e = Chain::load("nothing", loader(&files)).unwrap_err();
    assert_eq!(e.code(), codes::MODEL_NOT_FOUND, "{e}");
    let e = Chain::load("c", loader(&files)).unwrap_err();
    assert!(matches!(&e, ChainError::Load { name, .. } if name == "broken"));
    assert_eq!(e.code(), codes::SUPERMODEL_LOAD);
    assert!(e.to_string().contains("disk on fire"));
    assert_eq!(
        rakata_mdl::supermodel_name(&files["c"].0).unwrap(),
        Some("broken")
    );
    assert!(matches!(
        rakata_mdl::supermodel_name(b"text"),
        Err(Error::Truncated { .. })
    ));
}
