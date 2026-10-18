//! The `BWM V1.0` layout shared by WOK, PWK and DWK: the 136-byte header
//! and the nine arrays it places. The layout is `docs/formats/walkmesh.md`.

use crate::{Error, Result};

/// The header's length.
pub const HEADER_LEN: usize = 136;

/// The eight bytes every walkmesh begins with.
pub const SIGNATURE: &[u8; 8] = b"BWM V1.0";

/// A point or direction: X, Y, Z in metres, Z up, as stored.
pub type Vec3 = [f32; 3];

/// Which kind of walkmesh the header's type field says this is. The
/// extension tells a placeable's PWK from a door's DWK; the field does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Type 0: a placeable's PWK or a door's DWK, in the object's own frame.
    Object,
    /// Type 1: an area room's WOK, in the area's frame.
    Area,
}

impl Kind {
    /// The type field's value.
    pub fn to_raw(self) -> u32 {
        match self {
            Kind::Object => 0,
            Kind::Area => 1,
        }
    }
}

/// The vertex array, borrowed from the input: X, Y, Z as little-endian
/// `f32`, twelve bytes a vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertices<'a> {
    bytes: &'a [u8],
}

impl<'a> Vertices<'a> {
    /// The number of vertices.
    pub fn len(&self) -> usize {
        self.bytes.len() / 12
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The vertex at `index`, if there is one.
    pub fn get(&self, index: usize) -> Option<Vec3> {
        let b = self.bytes.get(index * 12..index * 12 + 12)?;
        Some([f32_at(b, 0), f32_at(b, 4), f32_at(b, 8)])
    }

    /// Every vertex in file order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Vec3> + 'a {
        self.bytes
            .as_chunks::<12>()
            .0
            .iter()
            .map(|b| [f32_at(b, 0), f32_at(b, 4), f32_at(b, 8)])
    }

    /// The array as stored: `len() * 12` bytes of little-endian `f32`
    /// triples, for a caller that copies them whole.
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Every vertex, copied.
    pub fn to_vec(&self) -> Vec<Vec3> {
        self.iter().collect()
    }

    /// The vertices stored in `bytes`, twelve a vertex as [`as_bytes`]
    /// returns them; `None` when the length is not a multiple of twelve.
    ///
    /// [`as_bytes`]: Vertices::as_bytes
    pub fn from_bytes(bytes: &'a [u8]) -> Option<Vertices<'a>> {
        bytes.len().is_multiple_of(12).then_some(Vertices { bytes })
    }
}

/// One edge of one face: edge `edge` of face `face` runs from the face's
/// vertex `edge` to its vertex `(edge + 1) % 3`. The file stores it as
/// `face * 3 + edge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeRef {
    /// The face's index.
    pub face: u32,
    /// The edge of the face, 0, 1 or 2.
    pub edge: u8,
}

impl EdgeRef {
    /// The edge a stored `face * 3 + edge` names.
    pub fn from_raw(raw: u32) -> EdgeRef {
        EdgeRef {
            face: raw / 3,
            edge: (raw % 3) as u8,
        }
    }

    /// The stored form, `face * 3 + edge`.
    pub fn to_raw(self) -> u32 {
        self.face * 3 + u32::from(self.edge)
    }

    /// The indices into the face's vertex triple of the edge's two ends,
    /// in the order the edge runs.
    pub fn corners(self) -> (usize, usize) {
        let e = usize::from(self.edge);
        (e, (e + 1) % 3)
    }
}

/// A triangle: its vertices, its surface material, and its plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    /// Indices into the vertex array. Seen from the side the normal points
    /// to, they run counter-clockwise.
    pub vertices: [u32; 3],
    /// The face's row of `surfacemat.2da`.
    pub material: u32,
    /// The unit normal as stored. Every PWK stores zero here.
    pub normal: Vec3,
    /// The plane's distance as stored: `-dot(normal, v)` for any vertex `v`
    /// of the face, in WOK and DWK files. In PWK files the value carries no
    /// meaning.
    pub distance: f32,
}

impl Face {
    /// Whether the face's material is walkable in `table`.
    pub fn is_walkable(&self, table: &crate::SurfaceMaterials) -> bool {
        table.is_walkable(self.material)
    }
}

/// An edge on the walkable region's boundary, and the room beyond it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerimeterEdge {
    /// The edge.
    pub edge: EdgeRef,
    /// The index, in the area's LYT room list, of the room this edge leads
    /// into, or `None` (stored as -1) where it leads nowhere.
    pub transition: Option<u32>,
}

/// A node of the AABB tree, in the array's order: the root first, an inner
/// node's left child just after it (pre-order). An inner node's box is
/// its children's together; a leaf's is its face's grown by about 0.01 on
/// every side, except 87 vanilla leaves whose box misses part of the face
/// they name (`docs/formats/walkmesh.md` lists the files).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AabbNode {
    /// The box's lower corner.
    pub min: Vec3,
    /// The box's upper corner.
    pub max: Vec3,
    /// At a leaf, the face it holds; `None` (stored as -1) at an inner node.
    pub face: Option<u32>,
    /// The field after the face, 4 in every vanilla node. What it means is
    /// not established.
    pub unknown: u32,
    /// At an inner node, the direction from its left child's box to its
    /// right child's: 1 +X, 2 +Y, 4 +Z, 8 −X, 16 −Y, 32 −Z. 0 at a leaf.
    pub plane: u32,
    /// The left child's index, `None` (stored as -1) at a leaf.
    pub left: Option<u32>,
    /// The right child's index, `None` (stored as -1) at a leaf.
    pub right: Option<u32>,
}

/// A walkmesh: a WOK, PWK or DWK.
///
/// The vertices borrow from the input; everything else is owned.
#[derive(Debug, Clone, PartialEq)]
pub struct Walkmesh<'a> {
    /// What the header's type field says this is.
    pub kind: Kind,
    /// Two points in the object's frame (header offsets 12 and 24). Zero in
    /// every WOK.
    pub relative_points: [Vec3; 2],
    /// The same two points with `position` added (offsets 36 and 48), in
    /// every DWK; zero in every PWK and WOK.
    pub absolute_points: [Vec3; 2],
    /// A point at offset 60. In a DWK, `absolute_points` less
    /// `relative_points`. What it is in a WOK or PWK is not established.
    pub position: Vec3,
    /// The vertices, as stored. A WOK's are in the area's frame, its room's
    /// LYT offset already applied; a PWK's or DWK's are the object's own.
    pub vertices: Vertices<'a>,
    /// The faces, in file order. Where the file stores adjacency, its
    /// walkable faces come first.
    pub faces: Vec<Face>,
    /// For each of the first `adjacency.len()` faces (the walkable ones, in
    /// every vanilla WOK), the edge of another face across each of its three
    /// edges, or `None` where no walkable face is. Empty in every PWK and
    /// DWK.
    pub adjacency: Vec<[Option<EdgeRef>; 3]>,
    /// The edges of the faces in `adjacency` with no face across them,
    /// loop by loop.
    pub perimeter_edges: Vec<PerimeterEdge>,
    /// Where each loop of `perimeter_edges` ends: loop `i` is the edges from
    /// `perimeters[i - 1]` (0 for the first) up to `perimeters[i]`.
    pub perimeters: Vec<u32>,
    /// The AABB tree over the faces, root first; empty in every PWK and DWK
    /// and in WOKs with no faces.
    pub aabb: Vec<AabbNode>,
    /// The header's field at offset 108, kept as read: 0 in every PWK and
    /// DWK and in most WOKs. What it means is not established.
    pub unknown: u32,
    /// The bytes after the last array, kept as read and written back after
    /// it. Empty in every vanilla file.
    pub trailer: &'a [u8],
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn i32_at(b: &[u8], at: usize) -> i32 {
    u32_at(b, at) as i32
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(b, at))
}

fn vec3_at(b: &[u8], at: usize) -> Vec3 {
    [f32_at(b, at), f32_at(b, at + 4), f32_at(b, at + 8)]
}

/// The bytes of an array of `count` elements of `size` bytes at `offset`;
/// empty, whatever the offset, when `count` is 0.
fn array<'a>(
    bytes: &'a [u8],
    what: &'static str,
    offset: u32,
    count: u32,
    size: usize,
) -> Result<&'a [u8]> {
    if count == 0 {
        return Ok(&[]);
    }
    let start = offset as usize;
    let needed = (count as usize)
        .checked_mul(size)
        .and_then(|n| n.checked_add(start))
        .unwrap_or(usize::MAX);
    bytes.get(start..needed).ok_or(Error::Truncated {
        what,
        needed,
        available: bytes.len(),
    })
}

/// An index stored as `i32` where -1 is none, checked against `limit`.
fn optional(what: &'static str, at: usize, raw: i32, limit: usize) -> Result<Option<u32>> {
    match raw {
        -1 => Ok(None),
        v if v >= 0 && (v as usize) < limit => Ok(Some(v as u32)),
        v => Err(Error::BadIndex {
            what,
            at,
            value: i64::from(v),
            limit,
        }),
    }
}

impl<'a> Walkmesh<'a> {
    /// Reads a WOK, PWK or DWK. Every index the file holds is checked
    /// against the array it names.
    pub fn parse(bytes: &'a [u8]) -> Result<Walkmesh<'a>> {
        if bytes.len() < SIGNATURE.len() || &bytes[..8] != SIGNATURE {
            return Err(Error::NotAWalkmesh);
        }
        if bytes.len() < HEADER_LEN {
            return Err(Error::Truncated {
                what: "header",
                needed: HEADER_LEN,
                available: bytes.len(),
            });
        }
        let kind = match u32_at(bytes, 8) {
            0 => Kind::Object,
            1 => Kind::Area,
            kind => return Err(Error::UnknownKind { kind }),
        };
        let h = |at| u32_at(bytes, at);
        let (vertex_count, face_count) = (h(72), h(80));
        let vertex_bytes = array(bytes, "vertex array", h(76), vertex_count, 12)?;
        let face_bytes = array(bytes, "face array", h(84), face_count, 12)?;
        let material_bytes = array(bytes, "material array", h(88), face_count, 4)?;
        let normal_bytes = array(bytes, "normal array", h(92), face_count, 12)?;
        let distance_bytes = array(bytes, "plane distance array", h(96), face_count, 4)?;
        let aabb_bytes = array(bytes, "AABB array", h(104), h(100), 44)?;
        let adjacency_bytes = array(bytes, "adjacency array", h(116), h(112), 12)?;
        let edge_bytes = array(bytes, "perimeter edge array", h(124), h(120), 8)?;
        let perimeter_bytes = array(bytes, "perimeter array", h(132), h(128), 4)?;
        // The trailer starts where the last array to end does, whatever
        // order the header placed them in.
        let end = [
            (h(76), vertex_bytes),
            (h(84), face_bytes),
            (h(88), material_bytes),
            (h(92), normal_bytes),
            (h(96), distance_bytes),
            (h(104), aabb_bytes),
            (h(116), adjacency_bytes),
            (h(124), edge_bytes),
            (h(132), perimeter_bytes),
        ]
        .iter()
        .filter(|(_, b)| !b.is_empty())
        .map(|&(at, b)| at as usize + b.len())
        .fold(HEADER_LEN, usize::max);

        let vertices = Vertices {
            bytes: vertex_bytes,
        };
        let nv = vertices.len();
        let nf = face_count as usize;
        let mut faces = Vec::with_capacity(nf);
        for i in 0..nf {
            let mut v = [0u32; 3];
            for (k, slot) in v.iter_mut().enumerate() {
                let raw = u32_at(face_bytes, i * 12 + k * 4);
                if raw as usize >= nv {
                    return Err(Error::BadIndex {
                        what: "face vertex",
                        at: i,
                        value: i64::from(raw),
                        limit: nv,
                    });
                }
                *slot = raw;
            }
            faces.push(Face {
                vertices: v,
                material: u32_at(material_bytes, i * 4),
                normal: vec3_at(normal_bytes, i * 12),
                distance: f32_at(distance_bytes, i * 4),
            });
        }

        let edges = nf * 3;
        let adjacency_count = adjacency_bytes.len() / 12;
        if adjacency_count > nf {
            return Err(Error::BadIndex {
                what: "adjacency count",
                at: 0,
                value: adjacency_count as i64,
                limit: nf,
            });
        }
        let adjacency = adjacency_bytes
            .as_chunks::<12>()
            .0
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let mut row = [None; 3];
                for (k, slot) in row.iter_mut().enumerate() {
                    *slot = optional("adjacency of face", i, i32_at(b, k * 4), edges)?
                        .map(EdgeRef::from_raw);
                }
                Ok(row)
            })
            .collect::<Result<Vec<_>>>()?;

        let perimeter_edges = edge_bytes
            .as_chunks::<8>()
            .0
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let raw = u32_at(b, 0);
                if raw as usize >= edges {
                    return Err(Error::BadIndex {
                        what: "perimeter edge",
                        at: i,
                        value: i64::from(raw),
                        limit: edges,
                    });
                }
                let transition = match i32_at(b, 4) {
                    -1 => None,
                    t if t >= 0 => Some(t as u32),
                    t => {
                        return Err(Error::BadIndex {
                            what: "perimeter edge's transition",
                            at: i,
                            value: i64::from(t),
                            limit: i32::MAX as usize,
                        });
                    }
                };
                Ok(PerimeterEdge {
                    edge: EdgeRef::from_raw(raw),
                    transition,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let ne = perimeter_edges.len();
        let mut perimeters = Vec::with_capacity(perimeter_bytes.len() / 4);
        let mut previous = 0u32;
        for (i, b) in perimeter_bytes.as_chunks::<4>().0.iter().enumerate() {
            let end = u32_at(b, 0);
            if end < previous || end as usize > ne {
                return Err(Error::BadIndex {
                    what: "perimeter end",
                    at: i,
                    value: i64::from(end),
                    limit: ne + 1,
                });
            }
            previous = end;
            perimeters.push(end);
        }

        let na = aabb_bytes.len() / 44;
        let aabb = aabb_bytes
            .as_chunks::<44>()
            .0
            .iter()
            .enumerate()
            .map(|(i, b)| {
                Ok(AabbNode {
                    min: vec3_at(b, 0),
                    max: vec3_at(b, 12),
                    face: optional("AABB node's face", i, i32_at(b, 24), nf)?,
                    unknown: u32_at(b, 28),
                    plane: u32_at(b, 32),
                    left: optional("AABB node's left child", i, i32_at(b, 36), na)?,
                    right: optional("AABB node's right child", i, i32_at(b, 40), na)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Walkmesh {
            kind,
            relative_points: [vec3_at(bytes, 12), vec3_at(bytes, 24)],
            absolute_points: [vec3_at(bytes, 36), vec3_at(bytes, 48)],
            position: vec3_at(bytes, 60),
            vertices,
            faces,
            adjacency,
            perimeter_edges,
            perimeters,
            aabb,
            unknown: h(108),
            trailer: bytes.get(end..).unwrap_or(&[]),
        })
    }

    /// The perimeter loops, each a run of `perimeter_edges`. Each vanilla
    /// loop is closed: an edge ends at the vertex the next one starts at,
    /// and the last ends where the first starts.
    pub fn perimeter_loops(&self) -> impl Iterator<Item = &[PerimeterEdge]> {
        let mut start = 0usize;
        self.perimeters.iter().map(move |&end| {
            let s = start;
            start = end as usize;
            &self.perimeter_edges[s..end as usize]
        })
    }

    /// The positions of an edge's two ends, in the order the edge runs, if
    /// it names a face of this walkmesh.
    pub fn edge_points(&self, edge: EdgeRef) -> Option<(Vec3, Vec3)> {
        let face = self.faces.get(edge.face as usize)?;
        let (a, b) = edge.corners();
        Some((
            self.vertices.get(face.vertices[a] as usize)?,
            self.vertices.get(face.vertices[b] as usize)?,
        ))
    }

    /// The indices of the faces whose material is walkable in `table`.
    pub fn walkable_faces<'t>(
        &'t self,
        table: &'t crate::SurfaceMaterials,
    ) -> impl Iterator<Item = usize> + 't {
        self.faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_walkable(table))
            .map(|(i, _)| i)
    }
}
