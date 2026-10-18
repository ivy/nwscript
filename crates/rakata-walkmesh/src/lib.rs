//! The walkmeshes of BioWare's Aurora-family games, KotOR 1 and 2 first:
//! an area room's WOK, a placeable's PWK and a door's DWK, which share one
//! binary layout, `BWM V1.0`.
//!
//! [`Walkmesh::parse`] (or [`parse`]) reads one to its vertices, its faces
//! with their surface material and plane, the adjacency of its walkable
//! faces, the perimeter edges around them with the room each leads into,
//! and the AABB tree over its faces. Coordinates are as stored: metres,
//! Z up; a WOK's vertices are already in the area's frame. Whether a face
//! is walkable is a column of `surfacemat.2da`, read by
//! [`SurfaceMaterials::from_2da`] from the table the caller loads.
//!
//! [`Walkmesh::to_bytes`] writes one back in the layout BioWare's files
//! use, every stored value as read: the AABB tree, adjacency and perimeter
//! edges are kept, not derived, so a vanilla file writes back byte for
//! byte.
//!
//! The vertex array borrows from the input ([`Vertices`]), so the input
//! must outlive the [`Walkmesh`]; the faces, adjacency, edges and tree are
//! owned, since each face gathers four arrays.
//!
//! The layout is `docs/formats/walkmesh.md` of this repository, measured
//! over the vanilla files; this crate was written without sight of any
//! other implementation.

mod bwm;
mod error;
mod surface;
mod write;

pub use bwm::{
    AabbNode, EdgeRef, Face, HEADER_LEN, Kind, PerimeterEdge, SIGNATURE, Vec3, Vertices, Walkmesh,
};
pub use error::{Error, Result, codes};
pub use surface::{SurfaceMaterial, SurfaceMaterials};

/// Reads a WOK, PWK or DWK; the same as [`Walkmesh::parse`].
pub fn parse(bytes: &[u8]) -> Result<Walkmesh<'_>> {
    Walkmesh::parse(bytes)
}
