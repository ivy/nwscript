//! The binary models of BioWare's Aurora-family games, KotOR 1 first: a
//! model's MDL (the header, the node tree and each node's structures) and
//! its MDX (the interleaved vertex data of its meshes), read to what a
//! renderer needs to draw a room, a door or a placeable.
//!
//! [`parse`] reads a [`Model`]: its name, [`Classification`], supermodel
//! and bounding box, and every [`Node`] the geometry's tree reaches with
//! its name, parent, children, position and orientation. A node with a
//! mesh header (a trimesh, and the trimesh part of a skin, dangly, AABB or
//! saber node) carries a [`Mesh`]: its texture and lightmap names, render
//! and shadow flags, transparency hint, colours, faces and triangle list,
//! and its vertices. Skin weights, dangly constraints, AABB trees,
//! emitters, lights, controllers and animations are not decoded; a model
//! that has them reads all the same.
//!
//! A [`Model`] borrows. Names are slices of the MDL; the faces and the
//! triangle list are views of the MDL's records; a mesh's vertices are
//! its MDX block as stored, `vertex_count × stride` bytes, with each
//! attribute an [`Attribute`] view at an offset within the stride, so a
//! caller may hand the block to a GPU and describe it without a copy. The
//! inputs must outlive the model. Coordinates are as stored: metres, Z up,
//! each node's position and orientation relative to its parent.
//!
//! The layout is `docs/formats/mdl.md` of this repository, measured from
//! the game's files; this crate was written without sight of any other
//! implementation.

mod error;
mod model;
mod view;

pub use error::{Error, Result, codes};
pub use model::{
    BoundingBox, Classification, FILE_HEADER_LEN, MESH_HEADER_LEN, Mesh, Model, NODE_HAS_MESH,
    NODE_HEADER_LEN, Node, NodeKind, Quaternion, mdx_flags, parse,
};
pub use view::{Attribute, FACE_LEN, Face, Faces, Indices, Vec2s, Vec3s};
