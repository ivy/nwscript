//! The binary models of BioWare's Aurora-family games, KotOR 1 first: a
//! model's MDL (the header, the node tree and each node's structures) and
//! its MDX (the interleaved vertex data of its meshes), read to what a
//! renderer needs to draw a room, a door, a placeable or a character.
//!
//! [`parse`] reads a [`Model`]: its name, [`Classification`], supermodel
//! and bounding box, every [`Node`] the geometry's tree reaches with its
//! name, parent, children, position, orientation and [`Controller`]s, and
//! its [`Animation`]s. A node with a mesh header (a trimesh, and the
//! trimesh part of a skin, dangly, AABB or saber node) carries a
//! [`Mesh`]: its texture and lightmap names, render and shadow flags,
//! transparency hint, colours, faces and triangle list, and its vertices.
//! A skin node also carries a [`Skin`]: its bones, their bind transforms
//! and each vertex's weights. An animation is a tree of nodes whose
//! controllers hold the keys, positions and orientations among them
//! (compressed orientations decoded by [`decompress_quaternion`]).
//! Dangly constraints, AABB trees, emitters and lights are not decoded
//! beyond their controllers; a model that has them reads all the same.
//!
//! A model inherits its supermodel's animations. [`Chain::load`] fetches a
//! model and its supermodels through a caller's loader, and [`Lineage`]
//! answers which animation and which node a name resolves to.
//!
//! A [`Model`] borrows. Names are slices of the MDL; the faces, the
//! triangle list and the controllers' keys are views of the MDL's records;
//! a mesh's vertices are its MDX block as stored, `vertex_count × stride`
//! bytes, with each attribute an [`Attribute`] view at an offset within
//! the stride, so a caller may hand the block to a GPU and describe it
//! without a copy. The inputs must outlive the model. Coordinates are as
//! stored: metres, Z up, each node's position and orientation relative to
//! its parent.
//!
//! The layout is `docs/formats/mdl.md` of this repository, measured from
//! the game's files; this crate was written without sight of any other
//! implementation.

mod animation;
mod controller;
mod error;
mod file;
mod lineage;
mod model;
mod skin;
mod view;
mod write;

pub use animation::{ANIMATION_HEADER_LEN, Animation, EVENT_LEN, Event};
pub use controller::{
    BEZIER, CONTROLLER_LEN, Controller, Key, controller_types, decompress_quaternion,
};
pub use error::{ChainError, Error, Result, codes};
pub use file::{
    AABB_ENTRY_LEN, AABB_HEADER_LEN, AabbEntry, AnimationFile, Body, DANGLY_HEADER_LEN, DanglyFile,
    EMITTER_HEADER_LEN, LIGHT_HEADER_LEN, LightFile, MODEL_HEADER_LEN, MdxBlock, MeshFile,
    MeshKind, ModelFile, NodeFile, NodePointer, REFERENCE_HEADER_LEN, SABER_HEADER_LEN, SaberFile,
    SkinFile,
};
pub use lineage::{Chain, Lineage, ModelFiles, supermodel_name};
pub use model::{
    BoundingBox, Classification, FILE_HEADER_LEN, MESH_HEADER_LEN, Mesh, Model, NODE_HAS_MESH,
    NODE_HEADER_LEN, Node, NodeKind, Quaternion, mdx_flags, parse,
};
pub use skin::{SKIN_HEADER_LEN, Skin, Vec4s};
pub use view::{Attribute, FACE_LEN, Face, Faces, Indices, Vec2s, Vec3s};
