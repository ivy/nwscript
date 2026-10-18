//! Walkmeshes built here, byte by byte, in the layout of
//! `docs/formats/walkmesh.md`: an area room with adjacency, a perimeter and
//! an AABB tree; an empty placeable; a door with its points; and each way
//! a file can fail to read. Each writes back byte for byte, and a walkmesh
//! built in memory writes and reads back the same.

use rakata_2da::TwoDa;
use rakata_walkmesh::{
    AabbNode, EdgeRef, Error, Face, HEADER_LEN, Kind, PerimeterEdge, SurfaceMaterials, Vertices,
    Walkmesh, codes,
};

/// A walkmesh's arrays, laid out as the vanilla files lay them out.
#[derive(Default, Clone)]
struct Build {
    kind: u32,
    points: [[f32; 3]; 5],
    vertices: Vec<[f32; 3]>,
    faces: Vec<[u32; 3]>,
    materials: Vec<u32>,
    normals: Vec<[f32; 3]>,
    distances: Vec<f32>,
    aabb: Vec<([f32; 6], [i32; 5])>,
    adjacency: Vec<[i32; 3]>,
    edges: Vec<(u32, i32)>,
    perimeters: Vec<u32>,
    /// The offsets of empty arrays after the planes are 0, as in PWK and
    /// DWK files, rather than where the array would start, as in WOKs.
    zero_empty_offsets: bool,
}

impl Build {
    fn bytes(&self) -> Vec<u8> {
        let mut head = Vec::new();
        head.extend_from_slice(b"BWM V1.0");
        head.extend_from_slice(&self.kind.to_le_bytes());
        for p in &self.points {
            for c in p {
                head.extend_from_slice(&c.to_le_bytes());
            }
        }
        let mut body = Vec::new();
        let mut at = |len: usize, count: usize, zero: bool| {
            let off = if count == 0 && zero {
                0
            } else {
                HEADER_LEN + body.len()
            };
            body.resize(body.len() + len, 0);
            off as u32
        };
        let nv = self.vertices.len();
        let nf = self.faces.len();
        let ov = at(nv * 12, nv, false);
        let of = at(nf * 12, nf, false);
        let om = at(nf * 4, nf, false);
        let on = at(nf * 12, nf, false);
        let od = at(nf * 4, nf, false);
        let z = self.zero_empty_offsets;
        let oa = at(self.aabb.len() * 44, self.aabb.len(), z);
        let oj = at(self.adjacency.len() * 12, self.adjacency.len(), z);
        let oe = at(self.edges.len() * 8, self.edges.len(), z);
        let op = at(self.perimeters.len() * 4, self.perimeters.len(), z);
        for v in [
            nv as u32,
            ov,
            nf as u32,
            of,
            om,
            on,
            od,
            self.aabb.len() as u32,
            oa,
            0,
            self.adjacency.len() as u32,
            oj,
            self.edges.len() as u32,
            oe,
            self.perimeters.len() as u32,
            op,
        ] {
            head.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(head.len(), HEADER_LEN);
        let mut out = head;
        let mut put = |b: &[u8]| out.extend_from_slice(b);
        for v in &self.vertices {
            v.iter().for_each(|c| put(&c.to_le_bytes()));
        }
        for f in &self.faces {
            f.iter().for_each(|c| put(&c.to_le_bytes()));
        }
        self.materials.iter().for_each(|m| put(&m.to_le_bytes()));
        for n in &self.normals {
            n.iter().for_each(|c| put(&c.to_le_bytes()));
        }
        self.distances.iter().for_each(|d| put(&d.to_le_bytes()));
        for (b, i) in &self.aabb {
            b.iter().for_each(|c| put(&c.to_le_bytes()));
            i.iter().for_each(|c| put(&c.to_le_bytes()));
        }
        for a in &self.adjacency {
            a.iter().for_each(|c| put(&c.to_le_bytes()));
        }
        for (e, t) in &self.edges {
            put(&e.to_le_bytes());
            put(&t.to_le_bytes());
        }
        self.perimeters.iter().for_each(|p| put(&p.to_le_bytes()));
        out
    }
}

/// A room of two walkable triangles making a unit square at z = 1 and one
/// non-walkable triangle standing on its far edge: the shape of a vanilla
/// WOK, walkable faces first.
fn room() -> Build {
    let up = [0.0, 0.0, 1.0];
    Build {
        kind: 1,
        points: [[0.0; 3], [0.0; 3], [0.0; 3], [0.0; 3], [0.5, -0.25, 0.25]],
        vertices: vec![
            [10.0, 20.0, 1.0],
            [11.0, 20.0, 1.0],
            [11.0, 21.0, 1.0],
            [10.0, 21.0, 1.0],
            [10.5, 21.0, 2.0],
        ],
        // Face 0: (0, 1, 2); face 1: (0, 2, 3); they share the edge 0–2,
        // face 0's edge 2 (2 → 0) and face 1's edge 0 (0 → 2).
        faces: vec![[0, 1, 2], [0, 2, 3], [3, 2, 4]],
        materials: vec![4, 1, 7],
        normals: vec![up, up, [0.0, -1.0, 0.0]],
        distances: vec![-1.0, -1.0, 21.0],
        // Root over all three faces, split along X; then its children.
        aabb: vec![
            ([9.99, 19.99, 0.99, 11.01, 21.01, 2.01], [-1, 4, 1, 1, 2]),
            ([9.99, 19.99, 0.99, 11.01, 21.01, 1.01], [0, 4, 0, -1, -1]),
            ([9.99, 19.99, 0.99, 11.01, 21.01, 2.01], [-1, 4, 2, 3, 4]),
            ([9.99, 19.99, 0.99, 11.01, 21.01, 1.01], [1, 4, 0, -1, -1]),
            ([9.99, 20.99, 0.99, 11.01, 21.01, 2.01], [2, 4, 0, -1, -1]),
        ],
        adjacency: vec![[-1, -1, 3], [2, -1, -1]],
        // One loop round the square: 0→1, 1→2 (into room 7), 2→3, 3→0.
        edges: vec![(0, -1), (1, 7), (4, -1), (5, -1)],
        perimeters: vec![4],
        zero_empty_offsets: false,
    }
}

#[test]
fn an_area_room_reads_to_its_faces_adjacency_perimeter_and_tree() {
    let bytes = room().bytes();
    let w = Walkmesh::parse(&bytes).unwrap();
    assert_eq!(w.kind, Kind::Area);
    assert_eq!(w.position, [0.5, -0.25, 0.25]);
    assert_eq!(w.vertices.len(), 5);
    assert_eq!(w.vertices.get(4), Some([10.5, 21.0, 2.0]));
    assert_eq!(w.vertices.get(5), None);
    assert_eq!(w.vertices.as_bytes().len(), 60);
    assert_eq!(w.vertices.to_vec()[1], [11.0, 20.0, 1.0]);
    assert_eq!(w.faces.len(), 3);
    assert_eq!(w.faces[1].vertices, [0, 2, 3]);
    assert_eq!(w.faces[2].material, 7);
    assert_eq!(w.faces[2].normal, [0.0, -1.0, 0.0]);
    assert_eq!(w.faces[0].distance, -1.0);

    let across = |face, edge| EdgeRef { face, edge };
    assert_eq!(w.adjacency.len(), 2);
    assert_eq!(w.adjacency[0], [None, None, Some(across(1, 0))]);
    assert_eq!(w.adjacency[1], [Some(across(0, 2)), None, None]);
    assert_eq!(across(1, 0).to_raw(), 3);
    assert_eq!(EdgeRef::from_raw(5), across(1, 2));
    assert_eq!(across(1, 2).corners(), (2, 0));

    assert_eq!(
        w.perimeter_edges[1],
        PerimeterEdge {
            edge: across(0, 1),
            transition: Some(7)
        }
    );
    assert_eq!(w.perimeter_edges[0].transition, None);
    let loops: Vec<_> = w.perimeter_loops().collect();
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].len(), 4);
    // The loop is closed: each edge ends where the next begins.
    for k in 0..4 {
        let (_, end) = w.edge_points(loops[0][k].edge).unwrap();
        let (start, _) = w.edge_points(loops[0][(k + 1) % 4].edge).unwrap();
        assert_eq!(end, start);
    }

    assert_eq!(w.aabb.len(), 5);
    assert_eq!(w.aabb[0].face, None);
    assert_eq!((w.aabb[0].left, w.aabb[0].right), (Some(1), Some(2)));
    assert_eq!(w.aabb[0].plane, 1);
    assert_eq!(w.aabb[0].unknown, 4);
    assert_eq!(w.aabb[4].face, Some(2));
    assert_eq!((w.aabb[4].left, w.aabb[4].right), (None, None));
    assert_eq!(w.aabb[1].min, [9.99, 19.99, 0.99]);
    assert_eq!(w.aabb[1].max, [11.01, 21.01, 1.01]);
}

#[test]
fn walkability_comes_from_the_tables_walk_column() {
    let mut t = TwoDa::new(["label", "walk", "walkcheck"]);
    for (label, walk) in [
        ("NotDefined", "0"),
        ("Dirt", "1"),
        ("Obscuring", "0"),
        ("Grass", "1"),
        ("Stone", "1"),
        ("Wood", ""),
        ("Water", "1"),
        ("Nonwalk", "0"),
    ] {
        let row = t.add_row(t.rows.len().to_string(), &[]).unwrap();
        t.rows[row].cells = vec![label.into(), walk.into(), "1".into()];
    }
    let table = SurfaceMaterials::from_2da(&t).unwrap();
    assert_eq!(table.materials.len(), 8);
    assert_eq!(table.get(4).unwrap().label, "Stone");
    assert!(table.is_walkable(1) && table.is_walkable(4));
    assert!(!table.is_walkable(5), "a blank walk cell is not walkable");
    assert!(!table.is_walkable(7));
    assert!(!table.is_walkable(99), "a material past the table");

    let bytes = room().bytes();
    let w = Walkmesh::parse(&bytes).unwrap();
    assert_eq!(w.walkable_faces(&table).collect::<Vec<_>>(), [0, 1]);
    assert!(!w.faces[2].is_walkable(&table));

    let reread = SurfaceMaterials::parse(&t.to_bytes().unwrap()).unwrap();
    assert_eq!(reread, table);
}

#[test]
fn a_surface_table_without_a_walk_column_or_with_a_word_in_it_is_refused() {
    let t = TwoDa::new(["label"]);
    let e = SurfaceMaterials::from_2da(&t).unwrap_err();
    assert_eq!(e.code(), codes::BAD_SURFACE_TABLE);

    let mut t = TwoDa::new(["label", "walk"]);
    t.add_row("0", &[("label", "Dirt"), ("walk", "yes")])
        .unwrap();
    let e = SurfaceMaterials::from_2da(&t).unwrap_err();
    assert_eq!(e.code(), codes::BAD_SURFACE_TABLE);
    assert!(e.to_string().contains("\"yes\""), "{e}");

    let e = SurfaceMaterials::parse(b"not a table").unwrap_err();
    assert_eq!(e.code(), codes::BAD_SURFACE_TABLE);
}

#[test]
fn an_empty_placeable_reads_as_no_faces() {
    // 74 vanilla PWKs are a bare header: counts 0, the vertex and face
    // offsets 136, the rest 0.
    let b = Build {
        zero_empty_offsets: true,
        ..Build::default()
    };
    let bytes = b.bytes();
    assert_eq!(bytes.len(), HEADER_LEN);
    let w = Walkmesh::parse(&bytes).unwrap();
    assert_eq!(w.kind, Kind::Object);
    assert!(w.vertices.is_empty() && w.faces.is_empty());
    assert!(w.adjacency.is_empty() && w.perimeter_edges.is_empty() && w.aabb.is_empty());
    assert_eq!(w.perimeter_loops().count(), 0);
}

#[test]
fn a_door_reads_its_points_and_faces_without_a_tree() {
    let rel = [[0.05, -0.56, 0.0], [0.05, 0.89, 0.0]];
    let pos = [-0.05, -0.14, 0.0];
    let abs = [
        [rel[0][0] + pos[0], rel[0][1] + pos[1], 0.0],
        [rel[1][0] + pos[0], rel[1][1] + pos[1], 0.0],
    ];
    let b = Build {
        kind: 0,
        points: [rel[0], rel[1], abs[0], abs[1], pos],
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 2.0]],
        faces: vec![[0, 1, 2]],
        materials: vec![7],
        normals: vec![[0.0, -1.0, 0.0]],
        distances: vec![0.0],
        zero_empty_offsets: true,
        ..Build::default()
    };
    let bytes = b.bytes();
    let w = Walkmesh::parse(&bytes).unwrap();
    assert_eq!(w.kind, Kind::Object);
    assert_eq!(w.relative_points, rel);
    assert_eq!(w.absolute_points, abs);
    assert_eq!(w.position, pos);
    assert_eq!(w.faces.len(), 1);
    assert!(w.aabb.is_empty() && w.adjacency.is_empty());
}

fn code(bytes: &[u8]) -> &'static str {
    Walkmesh::parse(bytes).unwrap_err().code()
}

#[test]
fn bytes_that_are_not_a_walkmesh_are_refused() {
    assert_eq!(code(b""), codes::NOT_A_WALKMESH);
    assert_eq!(code(b"BWM V2.0 and more"), codes::NOT_A_WALKMESH);
    assert_eq!(code(b"MDL"), codes::NOT_A_WALKMESH);
}

#[test]
fn a_header_or_array_cut_short_is_refused() {
    let bytes = room().bytes();
    let e = Walkmesh::parse(&bytes[..100]).unwrap_err();
    assert_eq!(
        e,
        Error::Truncated {
            what: "header",
            needed: HEADER_LEN,
            available: 100
        }
    );
    let e = Walkmesh::parse(&bytes[..bytes.len() - 1]).unwrap_err();
    assert_eq!(e.code(), codes::TRUNCATED);
    assert!(e.to_string().contains("perimeter array"), "{e}");
    // A count so large its length overflows.
    let mut b = bytes.clone();
    b[72..76].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(code(&b), codes::TRUNCATED);
}

#[test]
fn a_type_other_than_object_or_area_is_refused() {
    let mut b = room();
    b.kind = 2;
    let e = Walkmesh::parse(&b.bytes()).unwrap_err();
    assert_eq!(e, Error::UnknownKind { kind: 2 });
    assert_eq!(e.code(), codes::UNKNOWN_KIND);
}

#[test]
fn an_index_naming_nothing_is_refused() {
    type Edit = Box<dyn Fn(&mut Build)>;
    let cases: Vec<(&str, Edit)> = vec![
        ("face vertex", Box::new(|b| b.faces[1][2] = 5)),
        ("adjacency of face", Box::new(|b| b.adjacency[0][1] = 9)),
        ("adjacency of face", Box::new(|b| b.adjacency[0][1] = -2)),
        (
            "adjacency count",
            Box::new(|b| b.adjacency.extend([[-1; 3]; 2])),
        ),
        ("perimeter edge", Box::new(|b| b.edges[0].0 = 9)),
        (
            "perimeter edge's transition",
            Box::new(|b| b.edges[0].1 = -5),
        ),
        ("perimeter end", Box::new(|b| b.perimeters = vec![5])),
        ("perimeter end", Box::new(|b| b.perimeters = vec![3, 2])),
        ("AABB node's face", Box::new(|b| b.aabb[1].1[0] = 3)),
        ("AABB node's left child", Box::new(|b| b.aabb[0].1[3] = 5)),
        ("AABB node's right child", Box::new(|b| b.aabb[0].1[4] = -3)),
    ];
    for (what, edit) in cases {
        let mut b = room();
        edit(&mut b);
        match Walkmesh::parse(&b.bytes()).unwrap_err() {
            e @ Error::BadIndex { what: got, .. } => {
                assert_eq!(got, what, "{e}");
                assert_eq!(e.code(), codes::BAD_INDEX);
            }
            e => panic!("{what}: {e}"),
        }
    }
}

#[test]
fn files_in_the_vanilla_layout_write_back_byte_for_byte() {
    let placeable = Build {
        kind: 0,
        points: [
            [0.1, 0.2, 0.0],
            [0.3, 0.4, 0.0],
            [0.0; 3],
            [0.0; 3],
            [0.5; 3],
        ],
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 2.0]],
        faces: vec![[0, 1, 2]],
        materials: vec![7],
        // A PWK's normals are zero and its plane distances arbitrary bits.
        normals: vec![[0.0; 3]],
        distances: vec![f32::from_bits(0x7fc0_1234)],
        zero_empty_offsets: true,
        ..Build::default()
    };
    let bare = Build {
        zero_empty_offsets: true,
        ..Build::default()
    };
    let empty_room = Build {
        kind: 1,
        ..Build::default()
    };
    for (what, b) in [
        ("room", room()),
        ("placeable", placeable),
        ("bare", bare),
        ("empty room", empty_room),
    ] {
        let bytes = b.bytes();
        let w = Walkmesh::parse(&bytes).unwrap();
        assert_eq!(w.to_bytes().unwrap(), bytes, "{what}");
        assert!(w.semantic_eq(&w), "{what}");
    }
}

#[test]
fn the_unknown_header_field_and_bytes_after_the_last_array_are_kept() {
    let mut bytes = room().bytes();
    bytes[108..112].copy_from_slice(&0xdead_beef_u32.to_le_bytes());
    bytes.extend_from_slice(b"left by another tool");
    let w = Walkmesh::parse(&bytes).unwrap();
    assert_eq!(w.unknown, 0xdead_beef);
    assert_eq!(w.trailer, b"left by another tool");
    assert_eq!(w.to_bytes().unwrap(), bytes);

    let clean = Walkmesh {
        trailer: &[],
        ..w.clone()
    };
    assert!(!clean.semantic_eq(&w));
    assert!(w.semantic_eq(&Walkmesh::parse(&bytes).unwrap()));
}

#[test]
fn a_walkmesh_built_in_memory_writes_and_reads_back_the_same() {
    let vertex_bytes: Vec<u8> = [[0.0f32, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]]
        .iter()
        .flatten()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let vertices = Vertices::from_bytes(&vertex_bytes).unwrap();
    assert!(Vertices::from_bytes(&vertex_bytes[..11]).is_none());
    let leaf = AabbNode {
        min: [-0.01, -0.01, -0.01],
        max: [2.01, 2.01, 0.01],
        face: Some(0),
        unknown: 4,
        plane: 0,
        left: None,
        right: None,
    };
    let edge = |e| EdgeRef { face: 0, edge: e };
    let w = Walkmesh {
        kind: Kind::Area,
        relative_points: [[0.0; 3]; 2],
        absolute_points: [[0.0; 3]; 2],
        position: [1.0, -0.0, 0.5],
        vertices,
        faces: vec![Face {
            vertices: [0, 1, 2],
            material: 4,
            normal: [0.0, 0.0, 1.0],
            distance: -0.0,
        }],
        adjacency: vec![[None; 3]],
        perimeter_edges: (0..3)
            .map(|e| PerimeterEdge {
                edge: edge(e),
                transition: (e == 1).then_some(3),
            })
            .collect(),
        perimeters: vec![3],
        aabb: vec![leaf],
        unknown: 0,
        trailer: &[],
    };
    let bytes = w.to_bytes().unwrap();
    assert_eq!(&bytes[..8], b"BWM V1.0");
    // The header, three vertices, one face's five arrays, one node, one
    // adjacency row, three edges and one perimeter.
    assert_eq!(bytes.len(), HEADER_LEN + 36 + 32 + 44 + 12 + 24 + 4);
    let again = Walkmesh::parse(&bytes).unwrap();
    assert!(again.semantic_eq(&w));
    assert_eq!(again, w);
    assert_eq!(again.to_bytes().unwrap(), bytes);
    // -0.0 is kept, and compared bit for bit.
    assert_eq!(again.faces[0].distance.to_bits(), (-0.0f32).to_bits());
    let mut positive = w.clone();
    positive.faces[0].distance = 0.0;
    assert!(!positive.semantic_eq(&w));

    // An object's empty tree, adjacency and perimeter arrays sit at 0.
    let object = Walkmesh {
        kind: Kind::Object,
        adjacency: vec![],
        perimeter_edges: vec![],
        perimeters: vec![],
        aabb: vec![],
        ..w.clone()
    };
    let bytes = object.to_bytes().unwrap();
    for at in [104, 116, 124, 132] {
        assert_eq!(bytes[at..at + 4], [0; 4], "offset field at {at}");
    }
    assert!(Walkmesh::parse(&bytes).unwrap().semantic_eq(&object));
}

#[test]
fn a_value_too_large_for_its_field_is_refused_by_the_writer() {
    let bytes = room().bytes();
    let w = Walkmesh::parse(&bytes).unwrap();
    type Edit = Box<dyn Fn(&mut Walkmesh)>;
    let cases: Vec<(&str, Edit)> = vec![
        (
            "perimeter edge's transition",
            Box::new(|w| w.perimeter_edges[0].transition = Some(3_000_000_000)),
        ),
        (
            "edge's number within its face",
            Box::new(|w| w.perimeter_edges[0].edge.edge = 3),
        ),
        (
            "perimeter edge's face",
            Box::new(|w| w.perimeter_edges[0].edge.face = u32::MAX),
        ),
        (
            "adjacent edge",
            Box::new(|w| {
                w.adjacency[0][0] = Some(EdgeRef {
                    face: 1 << 30,
                    edge: 0,
                })
            }),
        ),
        (
            "AABB node's left child",
            Box::new(|w| w.aabb[0].left = Some(u32::MAX)),
        ),
    ];
    for (what, edit) in cases {
        let mut w = w.clone();
        edit(&mut w);
        match w.to_bytes().unwrap_err() {
            e @ Error::TooLarge { what: got, .. } => {
                assert_eq!(got, what, "{e}");
                assert_eq!(e.code(), codes::TOO_LARGE);
            }
            e => panic!("{what}: {e}"),
        }
    }
}
