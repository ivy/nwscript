//! The writer: a [`Walkmesh`] to the `BWM V1.0` layout as BioWare's files
//! lay it out (`docs/formats/walkmesh.md`), and the comparison of two
//! readings with their floats bit for bit.

use crate::bwm::{AabbNode, EdgeRef, Face, HEADER_LEN, Kind, PerimeterEdge, SIGNATURE, Walkmesh};
use crate::{Error, Result};

/// The largest value the layout's `i32` index fields hold.
const I32_LIMIT: u64 = i32::MAX as u64;

fn too_large(what: &'static str, value: u64, limit: u64) -> Error {
    Error::TooLarge { what, value, limit }
}

/// A count or offset as the header's `u32`.
fn word(what: &'static str, value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| too_large(what, value as u64, u64::from(u32::MAX)))
}

/// An index stored as `i32`, -1 for none.
fn optional(what: &'static str, value: Option<u32>) -> Result<i32> {
    match value {
        None => Ok(-1),
        Some(v) => i32::try_from(v).map_err(|_| too_large(what, u64::from(v), I32_LIMIT)),
    }
}

/// An edge's stored form, `face * 3 + edge`.
fn edge_word(what: &'static str, e: EdgeRef) -> Result<u32> {
    if e.edge > 2 {
        return Err(too_large(
            "edge's number within its face",
            u64::from(e.edge),
            2,
        ));
    }
    e.face
        .checked_mul(3)
        .and_then(|f| f.checked_add(u32::from(e.edge)))
        .ok_or_else(|| too_large(what, u64::from(e.face), u64::from(u32::MAX / 3)))
}

/// An edge where -1 is none, as adjacency stores it.
fn optional_edge(e: Option<EdgeRef>) -> Result<i32> {
    match e {
        None => Ok(-1),
        Some(e) => {
            let raw = edge_word("adjacent edge's face", e)?;
            i32::try_from(raw).map_err(|_| too_large("adjacent edge", u64::from(raw), I32_LIMIT))
        }
    }
}

fn put_vec3(out: &mut Vec<u8>, v: [f32; 3]) {
    for c in v {
        out.extend_from_slice(&c.to_le_bytes());
    }
}

fn bits3(v: [f32; 3]) -> [u32; 3] {
    v.map(f32::to_bits)
}

fn face_bits(f: &Face) -> ([u32; 3], u32, [u32; 3], u32) {
    (
        f.vertices,
        f.material,
        bits3(f.normal),
        f.distance.to_bits(),
    )
}

fn node_bits(n: &AabbNode) -> ([u32; 3], [u32; 3], AabbNode) {
    (
        bits3(n.min),
        bits3(n.max),
        AabbNode {
            min: [0.0; 3],
            max: [0.0; 3],
            ..*n
        },
    )
}

impl Walkmesh<'_> {
    /// The walkmesh as a `BWM V1.0` file, laid out as BioWare's files lay
    /// it out: the header, then the nine arrays in the header's order, each
    /// starting where the one before ends, then [`trailer`]. An empty
    /// array's offset is where it would start, except that an object's
    /// (PWK or DWK) empty AABB, adjacency, perimeter edge and perimeter
    /// arrays have offset 0. Every value is written as stored, the
    /// header's [`unknown`] field and the AABB nodes' among them; nothing
    /// is derived.
    ///
    /// Fails only where a count, offset or index does not fit its field.
    ///
    /// [`trailer`]: Walkmesh::trailer
    /// [`unknown`]: Walkmesh::unknown
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let nv = self.vertices.len();
        let nf = self.faces.len();
        let sizes = [
            nv * 12,
            nf * 12,
            nf * 4,
            nf * 12,
            nf * 4,
            self.aabb.len() * 44,
            self.adjacency.len() * 12,
            self.perimeter_edges.len() * 8,
            self.perimeters.len() * 4,
        ];
        let mut offsets = [0u32; 9];
        let mut at = HEADER_LEN;
        for (k, size) in sizes.iter().enumerate() {
            let zero_when_empty = self.kind == Kind::Object && k >= 5;
            offsets[k] = if *size == 0 && zero_when_empty {
                0
            } else {
                word("array offset", at)?
            };
            at += size;
        }
        word("file length", at)?;

        let mut out = Vec::with_capacity(at + self.trailer.len());
        out.extend_from_slice(SIGNATURE);
        out.extend_from_slice(&self.kind.to_raw().to_le_bytes());
        put_vec3(&mut out, self.relative_points[0]);
        put_vec3(&mut out, self.relative_points[1]);
        put_vec3(&mut out, self.absolute_points[0]);
        put_vec3(&mut out, self.absolute_points[1]);
        put_vec3(&mut out, self.position);
        let words = [
            word("vertex count", nv)?,
            offsets[0],
            word("face count", nf)?,
            offsets[1],
            offsets[2],
            offsets[3],
            offsets[4],
            word("AABB node count", self.aabb.len())?,
            offsets[5],
            self.unknown,
            word("adjacency count", self.adjacency.len())?,
            offsets[6],
            word("perimeter edge count", self.perimeter_edges.len())?,
            offsets[7],
            word("perimeter count", self.perimeters.len())?,
            offsets[8],
        ];
        for w in words {
            out.extend_from_slice(&w.to_le_bytes());
        }
        debug_assert_eq!(out.len(), HEADER_LEN);

        out.extend_from_slice(self.vertices.as_bytes());
        for f in &self.faces {
            for v in f.vertices {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        for f in &self.faces {
            out.extend_from_slice(&f.material.to_le_bytes());
        }
        for f in &self.faces {
            put_vec3(&mut out, f.normal);
        }
        for f in &self.faces {
            out.extend_from_slice(&f.distance.to_le_bytes());
        }
        for n in &self.aabb {
            put_vec3(&mut out, n.min);
            put_vec3(&mut out, n.max);
            out.extend_from_slice(&optional("AABB node's face", n.face)?.to_le_bytes());
            out.extend_from_slice(&n.unknown.to_le_bytes());
            out.extend_from_slice(&n.plane.to_le_bytes());
            out.extend_from_slice(&optional("AABB node's left child", n.left)?.to_le_bytes());
            out.extend_from_slice(&optional("AABB node's right child", n.right)?.to_le_bytes());
        }
        for row in &self.adjacency {
            for e in row {
                out.extend_from_slice(&optional_edge(*e)?.to_le_bytes());
            }
        }
        for PerimeterEdge { edge, transition } in &self.perimeter_edges {
            out.extend_from_slice(&edge_word("perimeter edge's face", *edge)?.to_le_bytes());
            let t = optional("perimeter edge's transition", *transition)?;
            out.extend_from_slice(&t.to_le_bytes());
        }
        for p in &self.perimeters {
            out.extend_from_slice(&p.to_le_bytes());
        }
        debug_assert_eq!(out.len(), at);
        out.extend_from_slice(self.trailer);
        Ok(out)
    }

    /// Whether two walkmeshes hold the same data, every float compared bit
    /// for bit (so a NaN equals itself, and 0.0 differs from -0.0), the
    /// header's unknown field and the trailer included. Where the header
    /// placed the arrays is not compared.
    pub fn semantic_eq(&self, other: &Walkmesh<'_>) -> bool {
        self.kind == other.kind
            && self.relative_points.map(bits3) == other.relative_points.map(bits3)
            && self.absolute_points.map(bits3) == other.absolute_points.map(bits3)
            && bits3(self.position) == bits3(other.position)
            && self.vertices.as_bytes() == other.vertices.as_bytes()
            && self
                .faces
                .iter()
                .map(face_bits)
                .eq(other.faces.iter().map(face_bits))
            && self.adjacency == other.adjacency
            && self.perimeter_edges == other.perimeter_edges
            && self.perimeters == other.perimeters
            && self.aabb.len() == other.aabb.len()
            && self
                .aabb
                .iter()
                .zip(&other.aabb)
                .all(|(a, b)| node_bits(a) == node_bits(b))
            && self.unknown == other.unknown
            && self.trailer == other.trailer
    }
}
