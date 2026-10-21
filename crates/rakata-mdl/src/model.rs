//! The model, its node tree and each mesh's trimesh part, read from the
//! MDL's model data with the vertices borrowed from the MDX.

use std::collections::HashSet;

use crate::animation::{Animation, read_animation};
use crate::controller::{Controller, read_controllers};
use crate::effects::{Emitter, Light, Saber, read_emitter, read_light, read_saber};
use crate::error::{Error, Result};
use crate::skin::{Skin, read_skin};
use crate::view::{Attribute, FACE_LEN, Faces, Indices, Vec2s, Vec3s};

/// The file header's length; the model data begins here, and every offset
/// inside it counts from here.
pub const FILE_HEADER_LEN: usize = 12;
/// The node header's length.
pub const NODE_HEADER_LEN: usize = 80;
/// The mesh header's length in KotOR 1, following the node header.
pub const MESH_HEADER_LEN: usize = 332;

// The model data, from its start.
pub(crate) const GEOM_NAME: usize = 0x08;
pub(crate) const GEOM_ROOT: usize = 0x28;
const MODEL_CLASS: usize = 0x50;
const MODEL_FOG: usize = 0x53;
const MODEL_ANIMS: usize = 0x58;
const MODEL_BBOX: usize = 0x68;
const MODEL_RADIUS: usize = 0x80;
const MODEL_ANIM_SCALE: usize = 0x84;
pub(crate) const MODEL_SUPER: usize = 0x88;
const NAMES: usize = 0xB8;
const HEADER_END: usize = 0xC4;

// A node, from its start.
pub(crate) const NODE_NUMBER: usize = 4;
const NODE_POSITION: usize = 16;
const NODE_ORIENTATION: usize = 28;
pub(crate) const NODE_CHILDREN: usize = 44;

// A mesh header, from its start (the node's start plus 80).
const MESH_FACES: usize = 8;
const MESH_BBOX: usize = 20;
const MESH_RADIUS: usize = 44;
const MESH_AVERAGE: usize = 48;
const MESH_DIFFUSE: usize = 60;
const MESH_AMBIENT: usize = 72;
const MESH_TRANSPARENCY: usize = 84;
const MESH_TEXTURE_MOTION: usize = 232;
const MESH_AREA: usize = 316;
pub(crate) const MESH_TEXTURE: usize = 88;
pub(crate) const MESH_LIGHTMAP: usize = 120;
const MESH_INDEX_COUNTS: usize = 176;
const MESH_INDEX_OFFSETS: usize = 188;
const MESH_STRIDE: usize = 252;
const MESH_MDX_FLAGS: usize = 256;
const MESH_OFF_POSITION: usize = 260;
const MESH_OFF_NORMAL: usize = 264;
const MESH_OFF_UV1: usize = 272;
const MESH_OFF_UV2: usize = 276;
const MESH_VERTEX_COUNT: usize = 304;
const MESH_TEXTURE_COUNT: usize = 306;
const MESH_LIGHTMAPPED: usize = 308;
const MESH_SHADOW: usize = 311;
const MESH_RENDER: usize = 313;
const MESH_MDX_OFFSET: usize = 324;
const MESH_VERTICES: usize = 328;

/// The bits of a mesh's MDX flags word, naming the attributes each vertex
/// of its MDX block holds.
pub mod mdx_flags {
    /// Positions, three floats.
    pub const POSITION: u32 = 0x01;
    /// The first texture coordinates (the diffuse texture), two floats.
    pub const UV1: u32 = 0x02;
    /// The second texture coordinates (the lightmap), two floats.
    pub const UV2: u32 = 0x04;
    /// Normals, three floats.
    pub const NORMAL: u32 = 0x20;
    /// Tangent space, nine floats; not decoded by this crate.
    pub const TANGENT_SPACE: u32 = 0x80;
}

/// An axis-aligned box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// The least corner.
    pub min: [f32; 3],
    /// The greatest corner.
    pub max: [f32; 3],
}

/// A rotation, stored `w, x, y, z` in the node header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    /// The scalar part.
    pub w: f32,
    /// The vector part's x.
    pub x: f32,
    /// The vector part's y.
    pub y: f32,
    /// The vector part's z.
    pub z: f32,
}

impl Quaternion {
    /// No rotation.
    pub const IDENTITY: Quaternion = Quaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// `v` rotated by this (unit) quaternion.
    pub fn rotate(self, v: [f32; 3]) -> [f32; 3] {
        let p = Quaternion {
            w: 0.0,
            x: v[0],
            y: v[1],
            z: v[2],
        };
        let conj = Quaternion {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        };
        let r = self * p * conj;
        [r.x, r.y, r.z]
    }
}

impl std::ops::Mul for Quaternion {
    type Output = Quaternion;

    /// The product `self × o`: `o`'s rotation, then `self`'s.
    fn mul(self, o: Quaternion) -> Quaternion {
        Quaternion {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }
}

/// What the model is, from the classification byte. The values seen on
/// the vanilla files and what the files bearing them are is
/// `docs/formats/mdl.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// 0: rooms, and other models of no class.
    Other,
    /// 1: visual effects.
    Effect,
    /// 2: the NWN documents' tile; in KotOR 1, force-power effects.
    Tile,
    /// 4: creatures and heads, bodies and some items.
    Character,
    /// 8: doors.
    Door,
    /// 16: lightsabers.
    Lightsaber,
    /// 32: placeables, and items and weapons.
    Placeable,
    /// Any other value.
    Unknown(u8),
}

impl Classification {
    /// From the byte as stored.
    pub fn from_byte(b: u8) -> Self {
        match b {
            0 => Classification::Other,
            1 => Classification::Effect,
            2 => Classification::Tile,
            4 => Classification::Character,
            8 => Classification::Door,
            16 => Classification::Lightsaber,
            32 => Classification::Placeable,
            b => Classification::Unknown(b),
        }
    }

    /// The byte as stored.
    pub fn to_byte(self) -> u8 {
        match self {
            Classification::Other => 0,
            Classification::Effect => 1,
            Classification::Tile => 2,
            Classification::Character => 4,
            Classification::Door => 8,
            Classification::Lightsaber => 16,
            Classification::Placeable => 32,
            Classification::Unknown(b) => b,
        }
    }
}

/// The kind of a node, from its flags word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// `0x001`: a node header only.
    Dummy,
    /// `0x003`: a light.
    Light,
    /// `0x005`: a particle emitter.
    Emitter,
    /// `0x011`: a reference to another model.
    Reference,
    /// `0x021`: a triangle mesh.
    Trimesh,
    /// `0x061`: a mesh skinned to bones.
    Skin,
    /// `0x0A1`: a mesh with vertex animation.
    AnimMesh,
    /// `0x121`: a mesh whose vertices sway.
    Dangly,
    /// `0x221`: a mesh with an AABB tree, the walkmesh of a room.
    Aabb,
    /// `0x821`: a lightsaber blade.
    Saber,
    /// Any other flags word.
    Other(u16),
}

impl NodeKind {
    /// From the flags word.
    pub fn from_flags(flags: u16) -> Self {
        match flags {
            0x001 => NodeKind::Dummy,
            0x003 => NodeKind::Light,
            0x005 => NodeKind::Emitter,
            0x011 => NodeKind::Reference,
            0x021 => NodeKind::Trimesh,
            0x061 => NodeKind::Skin,
            0x0A1 => NodeKind::AnimMesh,
            0x121 => NodeKind::Dangly,
            0x221 => NodeKind::Aabb,
            0x821 => NodeKind::Saber,
            f => NodeKind::Other(f),
        }
    }
}

/// The flag of a node with a mesh header.
pub const NODE_HAS_MESH: u16 = 0x020;

/// A binary model: the model header and the geometry's node tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Model<'a> {
    /// The model's name.
    pub name: &'a str,
    /// What the model is.
    pub classification: Classification,
    /// The byte after the classification; its meaning is not established.
    pub subclassification: u8,
    /// The byte at 0x53 of the model data, where the NWN layout keeps its
    /// fog flag; zero on 89 vanilla models, mostly rooms.
    pub fog: bool,
    /// The model whose nodes and animations this one inherits, if any.
    pub supermodel: Option<&'a str>,
    /// The model header's box. Every vanilla model but one stores the
    /// default (-5, -5, -1) to (5, 5, 10); [`Model::bounds`] computes one.
    pub bounding_box: BoundingBox,
    /// The model header's radius.
    pub radius: f32,
    /// The animation scale.
    pub animation_scale: f32,
    /// The model's own animations, in stored order. A model with a
    /// supermodel also plays the supermodel's; see [`Lineage`](crate::Lineage).
    pub animations: Vec<Animation<'a>>,
    /// Every node the geometry's tree reaches, depth first in child
    /// order: `nodes[0]` is the root.
    pub nodes: Vec<Node<'a>>,
}

/// One node of the geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct Node<'a> {
    /// Its name.
    pub name: &'a str,
    /// Its kind.
    pub kind: NodeKind,
    /// The flags word as stored.
    pub flags: u16,
    /// Its number, the index of its name in the model's name array.
    pub number: u16,
    /// The u16 at offset 2: the number of the supermodel's node it stands
    /// for, equal to `number` in a model without a supermodel.
    pub supernode: u16,
    /// Its parent's index in [`Model::nodes`]; `None` for the root.
    pub parent: Option<usize>,
    /// Its children's indices in [`Model::nodes`], in stored order.
    pub children: Vec<usize>,
    /// Its position relative to its parent.
    pub position: [f32; 3],
    /// Its orientation relative to its parent.
    pub orientation: Quaternion,
    /// The trimesh part of a node with a mesh header.
    pub mesh: Option<Mesh<'a>>,
    /// The skin part of a skin node: its bones and its vertices' weights.
    pub skin: Option<Skin<'a>>,
    /// A light's header.
    pub light: Option<Light<'a>>,
    /// An emitter's header.
    pub emitter: Option<Emitter<'a>>,
    /// A saber blade's vertices and, through
    /// [`Saber::blade_triangles`](crate::Saber::blade_triangles), its surface.
    pub saber: Option<Saber<'a>>,
    /// The node's controllers, in stored order. In a model's geometry each
    /// holds one key, the node's resting value (the position and
    /// orientation controllers repeat the header's); in an animation they
    /// hold its keys.
    pub controllers: Vec<Controller<'a>>,
}

impl<'a> Node<'a> {
    /// The first controller of type `kind` (see
    /// [`controller_types`](crate::controller_types)).
    pub fn controller(&self, kind: u32) -> Option<&Controller<'a>> {
        self.controllers.iter().find(|c| c.kind == kind)
    }
}

/// The trimesh part of a mesh node: what a renderer needs to draw it.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh<'a> {
    /// The diffuse texture's name; `None` when the field is empty or
    /// `NULL`.
    pub texture: Option<&'a str>,
    /// The second texture's name, the lightmap on a lightmapped mesh.
    pub lightmap: Option<&'a str>,
    /// The texture count field.
    pub texture_count: u16,
    /// The byte at 308 of the mesh header: set exactly when the mesh has
    /// a lightmap and second texture coordinates, in all vanilla meshes but one.
    pub lightmapped: bool,
    /// Whether the mesh is drawn.
    pub render: bool,
    /// Whether the mesh casts a shadow.
    pub shadow: bool,
    /// The transparency hint.
    pub transparency_hint: u32,
    /// The diffuse colour.
    pub diffuse: [f32; 3],
    /// The ambient colour.
    pub ambient: [f32; 3],
    /// The mesh header's box, in the node's coordinates.
    pub bounding_box: BoundingBox,
    /// The mesh header's radius.
    pub radius: f32,
    /// The mesh header's average of the vertices.
    pub average: [f32; 3],
    /// The number of vertices.
    pub vertex_count: usize,
    /// The MDX flags word: the bits of [`mdx_flags`] name what each vertex
    /// of the MDX block holds.
    pub mdx_flags: u32,
    /// The mesh's interleaved MDX block, `vertex_count × stride` bytes;
    /// empty when the mesh has no MDX block.
    pub vertex_buffer: &'a [u8],
    /// The bytes from one vertex to the next in `vertex_buffer`.
    pub stride: usize,
    /// The positions: from the MDX block when it holds them, else from the
    /// MDL's own position array.
    pub positions: Vec3s<'a>,
    /// The normals, when the MDX block holds them.
    pub normals: Option<Vec3s<'a>>,
    /// The first texture coordinates (the diffuse texture's).
    pub uv1: Option<Vec2s<'a>>,
    /// The second texture coordinates (the lightmap's).
    pub uv2: Option<Vec2s<'a>>,
    /// The faces.
    pub faces: Faces<'a>,
    /// The triangle index list, when the mesh stores one.
    pub indices: Option<Indices<'a>>,
    /// How the texture coordinates move, on a mesh whose texture-motion
    /// word (232) is set: water, clouds, planets, energy effects.
    pub texture_motion: Option<TextureMotion>,
    /// The float at 316: the sum of the faces' areas in the node's
    /// coordinates, on a trimesh or AABB mesh; zero on a skin, dangly mesh
    /// or saber.
    pub area: f32,
}

/// How a mesh's texture coordinates move (`docs/formats/mdl.md`, "Texture
/// motion"). The rates' unit and the direction a positive rate moves are
/// not established.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureMotion {
    /// The rate the first texture coordinate (U) moves at (236).
    pub u_rate: f32,
    /// The rate the second texture coordinate (V) moves at (240).
    pub v_rate: f32,
    /// The floats at 244 and 248: a second motion's two values, the first
    /// never zero and the second from 0 to 1 in vanilla; which is which,
    /// and what they drive, is not established.
    pub floats_244_248: [f32; 2],
}

impl<'a> Model<'a> {
    /// Read a binary model from its MDL and MDX. The MDX may be empty for
    /// a model whose meshes have no MDX blocks.
    pub fn parse(mdl: &'a [u8], mdx: &'a [u8]) -> Result<Self> {
        parse(mdl, mdx)
    }

    /// The root node.
    pub fn root(&self) -> &Node<'a> {
        &self.nodes[0]
    }

    /// The first node named `name`, compared without case.
    pub fn node(&self, name: &str) -> Option<&Node<'a>> {
        self.nodes
            .iter()
            .find(|n| n.name.eq_ignore_ascii_case(name))
    }

    /// The model's own animation named `name`, compared without case.
    pub fn animation(&self, name: &str) -> Option<&Animation<'a>> {
        self.animations
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
    }

    /// Each node with a mesh, with its mesh.
    pub fn meshes(&self) -> impl Iterator<Item = (&Node<'a>, &Mesh<'a>)> {
        self.nodes
            .iter()
            .filter_map(|n| n.mesh.as_ref().map(|m| (n, m)))
    }

    /// Each node's transform to the model's coordinates, by index into
    /// [`Model::nodes`]: the rotation, then the translation.
    pub fn model_transforms(&self) -> Vec<(Quaternion, [f32; 3])> {
        let mut out: Vec<(Quaternion, [f32; 3])> = Vec::with_capacity(self.nodes.len());
        for n in &self.nodes {
            let t = match n.parent {
                None => (n.orientation, n.position),
                Some(p) => {
                    let (pq, pt) = out[p];
                    let r = pq.rotate(n.position);
                    (
                        pq * n.orientation,
                        [pt[0] + r[0], pt[1] + r[1], pt[2] + r[2]],
                    )
                }
            };
            out.push(t);
        }
        out
    }

    /// The box around every vertex of every rendered mesh, in the model's
    /// coordinates; `None` when no rendered mesh has a vertex.
    pub fn bounds(&self) -> Option<BoundingBox> {
        let xf = self.model_transforms();
        let mut b: Option<BoundingBox> = None;
        for (i, n) in self.nodes.iter().enumerate() {
            let Some(m) = n.mesh.as_ref().filter(|m| m.render) else {
                continue;
            };
            let (q, t) = xf[i];
            for v in m.positions.iter() {
                let r = q.rotate(v);
                let p = [r[0] + t[0], r[1] + t[1], r[2] + t[2]];
                let bb = b.get_or_insert(BoundingBox { min: p, max: p });
                for (k, x) in p.into_iter().enumerate() {
                    bb.min[k] = bb.min[k].min(x);
                    bb.max[k] = bb.max[k].max(x);
                }
            }
        }
        b
    }
}

/// Read a binary model from its MDL and MDX; see [`Model::parse`].
pub fn parse<'a>(mdl: &'a [u8], mdx: &'a [u8]) -> Result<Model<'a>> {
    let d = model_data(mdl)?;
    let data = d.b;

    let names = read_names(&d)?;
    let name = d.fixed_str(GEOM_NAME, "model name")?;
    let supermodel = d.fixed_str(MODEL_SUPER, "supermodel name")?;
    let supermodel = (!supermodel.is_empty() && supermodel != "NULL").then_some(supermodel);

    let (anim_off, anim_count) = d.array(MODEL_ANIMS, "animation array")?;
    d.slice(anim_off, anim_count * 4, "animation array")?;
    let mut animations = Vec::with_capacity(anim_count);
    for i in 0..anim_count {
        let a = d.u32(anim_off + 4 * i, "animation pointer")? as usize;
        animations.push(read_animation(&d, &names, a)?);
    }

    let nodes = read_tree(&d, mdx, &names, d.u32(GEOM_ROOT, "root")? as usize)?;
    Ok(Model {
        name,
        classification: Classification::from_byte(data[MODEL_CLASS]),
        subclassification: data[MODEL_CLASS + 1],
        fog: data[MODEL_FOG] != 0,
        supermodel,
        bounding_box: d.bbox(MODEL_BBOX)?,
        radius: d.f32(MODEL_RADIUS)?,
        animation_scale: d.f32(MODEL_ANIM_SCALE)?,
        animations,
        nodes,
    })
}

/// The model data of a binary MDL, past its file header, with the model
/// header's extent checked.
pub(crate) fn model_data(mdl: &[u8]) -> Result<Data<'_>> {
    if mdl.len() < FILE_HEADER_LEN {
        return Err(Error::Truncated {
            what: "file header",
            offset: 0,
            needed: FILE_HEADER_LEN,
            available: mdl.len(),
        });
    }
    let word = |i: usize| u32::from_le_bytes([mdl[i], mdl[i + 1], mdl[i + 2], mdl[i + 3]]);
    if word(0) != 0 {
        return Err(Error::NotBinary { first: word(0) });
    }
    let size = word(4) as usize;
    let data = &mdl[FILE_HEADER_LEN..];
    // The size field is the model data's length; trust the bytes when it
    // claims more than there is, so a short file fails on what it lacks.
    let d = Data {
        b: &data[..size.min(data.len())],
    };
    d.slice(0, HEADER_END, "model header")?;
    Ok(d)
}

/// The name array: pointers to NUL-ended names, in node-number order.
pub(crate) fn read_names<'a>(d: &Data<'a>) -> Result<Vec<&'a str>> {
    let (off, count) = d.array(NAMES, "name array")?;
    d.slice(off, count * 4, "name array")?;
    (0..count)
        .map(|i| {
            let p = d.u32(off + 4 * i, "name pointer")? as usize;
            d.c_str(p, "node name")
        })
        .collect()
}

pub(crate) fn read_tree<'a>(
    d: &Data<'a>,
    mdx: &'a [u8],
    names: &[&'a str],
    root: usize,
) -> Result<Vec<Node<'a>>> {
    let mut nodes: Vec<Node<'a>> = Vec::new();
    let mut seen = HashSet::new();
    // (offset, parent index), popped in pre-order with children in order.
    let mut stack = vec![(root, None::<usize>)];
    while let Some((at, parent)) = stack.pop() {
        if !seen.insert(at) {
            return Err(Error::BadTree {
                reason: format!("the node at model offset {at} is reached twice"),
            });
        }
        d.slice(at, NODE_HEADER_LEN, "node header")?;
        let flags = d.u16(at)?;
        let supernode = d.u16(at + 2)?;
        let number = d.u16(at + NODE_NUMBER)?;
        let name = *names.get(number as usize).ok_or_else(|| Error::BadTree {
            reason: format!(
                "the node at model offset {at} is number {number} and there are {} names",
                names.len()
            ),
        })?;
        let p = d.f32s::<3>(at + NODE_POSITION)?;
        let q = d.f32s::<4>(at + NODE_ORIENTATION)?;
        let mesh = if flags & NODE_HAS_MESH != 0 {
            Some(read_mesh(d, mdx, at + NODE_HEADER_LEN, name)?)
        } else {
            None
        };
        let skin = match (&mesh, NodeKind::from_flags(flags)) {
            (Some(m), NodeKind::Skin) => Some(read_skin(
                d,
                at + NODE_HEADER_LEN + MESH_HEADER_LEN,
                name,
                m.vertex_buffer,
                m.stride,
                if m.vertex_buffer.is_empty() {
                    0
                } else {
                    m.vertex_count
                },
            )?),
            _ => None,
        };
        let h = at + NODE_HEADER_LEN;
        let light = match flags {
            0x003 => Some(read_light(d, h)?),
            _ => None,
        };
        let emitter = match flags {
            0x005 => Some(read_emitter(d, h)?),
            _ => None,
        };
        let saber = match (&mesh, NodeKind::from_flags(flags)) {
            (Some(m), NodeKind::Saber) => Some(read_saber(d, h + MESH_HEADER_LEN, m.vertex_count)?),
            _ => None,
        };
        let controllers = read_controllers(d, at, name)?;
        let (ch_off, ch_count) = d.array(at + NODE_CHILDREN, "child array")?;
        d.slice(ch_off, ch_count * 4, "child array")?;
        let index = nodes.len();
        if let Some(p) = parent {
            nodes[p].children.push(index);
        }
        nodes.push(Node {
            name,
            kind: NodeKind::from_flags(flags),
            flags,
            number,
            supernode,
            parent,
            children: Vec::new(),
            position: p,
            orientation: Quaternion {
                w: q[0],
                x: q[1],
                y: q[2],
                z: q[3],
            },
            mesh,
            skin,
            light,
            emitter,
            saber,
            controllers,
        });
        for i in (0..ch_count).rev() {
            stack.push((
                d.u32(ch_off + 4 * i, "child pointer")? as usize,
                Some(index),
            ));
        }
    }
    Ok(nodes)
}

const ABSENT: u32 = u32::MAX;

fn read_mesh<'a>(d: &Data<'a>, mdx: &'a [u8], h: usize, node: &str) -> Result<Mesh<'a>> {
    d.slice(h, MESH_HEADER_LEN, "mesh header")?;
    let nv = d.u16(h + MESH_VERTEX_COUNT)? as usize;

    let (face_off, face_count) = d.array(h + MESH_FACES, "face array")?;
    let faces = Faces::new(d.slice(face_off, face_count * FACE_LEN, "face array")?);
    for f in faces.iter() {
        check_indices(node, nv, f.vertices.iter().copied())?;
    }

    let (counts_off, counts_n) = d.array(h + MESH_INDEX_COUNTS, "index count array")?;
    let (lists_off, lists_n) = d.array(h + MESH_INDEX_OFFSETS, "index offset array")?;
    let indices = if counts_n > 0 && lists_n > 0 {
        let n = d.u32(counts_off, "index count")? as usize;
        let p = d.u32(lists_off, "index list pointer")? as usize;
        let list = Indices::new(d.slice(p, n * 2, "index list")?);
        check_indices(node, nv, list.iter())?;
        Some(list)
    } else {
        None
    };

    let stride = d.u32(h + MESH_STRIDE, "MDX stride")? as usize;
    let flags = d.u32(h + MESH_MDX_FLAGS, "MDX flags")?;
    let attr = |field: usize, floats: usize| -> Result<Option<usize>> {
        let off = d.u32(h + field, "MDX attribute offset")?;
        if off == ABSENT {
            return Ok(None);
        }
        let off = off as usize;
        if off + 4 * floats > stride {
            return Err(Error::BadVertexLayout {
                node: node.to_owned(),
                reason: format!(
                    "an attribute of {floats} floats at {off} passes the stride of {stride}"
                ),
            });
        }
        Ok(Some(off))
    };
    let pos_at = attr(MESH_OFF_POSITION, 3)?;
    let normal_at = attr(MESH_OFF_NORMAL, 3)?;
    let uv1_at = attr(MESH_OFF_UV1, 2)?;
    let uv2_at = attr(MESH_OFF_UV2, 2)?;
    let any = [pos_at, normal_at, uv1_at, uv2_at]
        .iter()
        .any(Option::is_some);

    let vertex_buffer: &'a [u8] = if nv > 0 && any {
        let off = d.u32(h + MESH_MDX_OFFSET, "MDX offset")?;
        if off == ABSENT {
            return Err(Error::BadVertexLayout {
                node: node.to_owned(),
                reason: "attributes in the MDX and no MDX offset".to_owned(),
            });
        }
        let off = off as usize;
        let len = nv * stride;
        mdx.get(off..off + len)
            .ok_or_else(|| Error::MdxOutOfRange {
                node: node.to_owned(),
                offset: off,
                needed: len,
                available: mdx.len(),
            })?
    } else {
        &[]
    };
    let count = if vertex_buffer.is_empty() { 0 } else { nv };
    let view3 = |at: Option<usize>| at.map(|o| Attribute::new(vertex_buffer, stride, o, count));
    let view2 = |at: Option<usize>| at.map(|o| Attribute::new(vertex_buffer, stride, o, count));

    let positions = match view3(pos_at) {
        Some(p) if count == nv => p,
        _ => {
            let at = d.u32(h + MESH_VERTICES, "vertex array pointer")? as usize;
            let bytes = if nv == 0 {
                &[][..]
            } else {
                d.slice(at, nv * 12, "vertex array")?
            };
            Attribute::new(bytes, 12, 0, nv)
        }
    };

    Ok(Mesh {
        texture: texture_name(d.fixed_str(h + MESH_TEXTURE, "texture name")?),
        lightmap: texture_name(d.fixed_str(h + MESH_LIGHTMAP, "lightmap name")?),
        texture_count: d.u16(h + MESH_TEXTURE_COUNT)?,
        lightmapped: d.b[h + MESH_LIGHTMAPPED] != 0,
        render: d.b[h + MESH_RENDER] != 0,
        shadow: d.b[h + MESH_SHADOW] != 0,
        transparency_hint: d.u32(h + MESH_TRANSPARENCY, "transparency hint")?,
        diffuse: d.f32s::<3>(h + MESH_DIFFUSE)?,
        ambient: d.f32s::<3>(h + MESH_AMBIENT)?,
        bounding_box: d.bbox(h + MESH_BBOX)?,
        radius: d.f32(h + MESH_RADIUS)?,
        average: d.f32s::<3>(h + MESH_AVERAGE)?,
        vertex_count: nv,
        mdx_flags: flags,
        vertex_buffer,
        stride: if vertex_buffer.is_empty() { 0 } else { stride },
        positions,
        normals: view3(normal_at),
        uv1: view2(uv1_at),
        uv2: view2(uv2_at),
        faces,
        indices,
        texture_motion: match d.u32(h + MESH_TEXTURE_MOTION, "texture motion")? {
            0 => None,
            _ => Some(TextureMotion {
                u_rate: d.f32(h + MESH_TEXTURE_MOTION + 4)?,
                v_rate: d.f32(h + MESH_TEXTURE_MOTION + 8)?,
                floats_244_248: d.f32s::<2>(h + MESH_TEXTURE_MOTION + 12)?,
            }),
        },
        area: d.f32(h + MESH_AREA)?,
    })
}

pub(crate) fn texture_name(s: &str) -> Option<&str> {
    (!s.is_empty() && !s.eq_ignore_ascii_case("NULL")).then_some(s)
}

fn check_indices(node: &str, nv: usize, it: impl Iterator<Item = u16>) -> Result<()> {
    for i in it {
        if i as usize >= nv {
            return Err(Error::BadIndex {
                node: node.to_owned(),
                index: i,
                vertices: nv,
            });
        }
    }
    Ok(())
}

/// The model data, read with every offset checked.
pub(crate) struct Data<'a> {
    pub(crate) b: &'a [u8],
}

impl<'a> Data<'a> {
    pub(crate) fn slice(&self, at: usize, len: usize, what: &'static str) -> Result<&'a [u8]> {
        at.checked_add(len)
            .and_then(|end| self.b.get(at..end))
            .ok_or(Error::Truncated {
                what,
                offset: at,
                needed: len,
                available: self.b.len(),
            })
    }

    pub(crate) fn u32(&self, at: usize, what: &'static str) -> Result<u32> {
        let s = self.slice(at, 4, what)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    pub(crate) fn u16(&self, at: usize) -> Result<u16> {
        let s = self.slice(at, 2, "u16")?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }

    pub(crate) fn f32(&self, at: usize) -> Result<f32> {
        Ok(f32::from_bits(self.u32(at, "float")?))
    }

    pub(crate) fn f32s<const N: usize>(&self, at: usize) -> Result<[f32; N]> {
        let mut out = [0.0; N];
        for (i, v) in out.iter_mut().enumerate() {
            *v = self.f32(at + 4 * i)?;
        }
        Ok(out)
    }

    pub(crate) fn bbox(&self, at: usize) -> Result<BoundingBox> {
        Ok(BoundingBox {
            min: self.f32s::<3>(at)?,
            max: self.f32s::<3>(at + 12)?,
        })
    }

    /// An array header: the offset and the used count (the allocated count
    /// that follows is ignored).
    pub(crate) fn array(&self, at: usize, what: &'static str) -> Result<(usize, usize)> {
        Ok((
            self.u32(at, what)? as usize,
            self.u32(at + 4, what)? as usize,
        ))
    }

    /// A 32-byte name field: ASCII up to a NUL within the field.
    pub(crate) fn fixed_str(&self, at: usize, what: &'static str) -> Result<&'a str> {
        let s = self.slice(at, 32, what)?;
        let end = s
            .iter()
            .position(|&c| c == 0)
            .ok_or(Error::BadName { what, offset: at })?;
        ascii(&s[..end], what, at)
    }

    /// A name field of `len` bytes: ASCII up to a NUL, or the whole field
    /// when it holds none.
    pub(crate) fn field_str(&self, at: usize, len: usize, what: &'static str) -> Result<&'a str> {
        let s = self.slice(at, len, what)?;
        let end = s.iter().position(|&c| c == 0).unwrap_or(len);
        ascii(&s[..end], what, at)
    }

    /// A NUL-ended name anywhere in the data.
    pub(crate) fn c_str(&self, at: usize, what: &'static str) -> Result<&'a str> {
        let s = self.b.get(at..).ok_or(Error::Truncated {
            what,
            offset: at,
            needed: 1,
            available: self.b.len(),
        })?;
        let end = s
            .iter()
            .position(|&c| c == 0)
            .ok_or(Error::BadName { what, offset: at })?;
        ascii(&s[..end], what, at)
    }
}

fn ascii<'a>(s: &'a [u8], what: &'static str, at: usize) -> Result<&'a str> {
    if !s.is_ascii() {
        return Err(Error::BadName { what, offset: at });
    }
    Ok(std::str::from_utf8(s).expect("ASCII"))
}
