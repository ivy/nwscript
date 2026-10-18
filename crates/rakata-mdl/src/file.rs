//! A binary model as stored, owned: every structure of the MDL with every
//! field it holds, and every MDX block with the bytes after it, so that
//! [`ModelFile::to_bytes`] writes the pair back byte for byte.
//!
//! The reader [`Model`](crate::Model) decodes what a renderer needs and
//! borrows; this form decodes only the structure (which records there are,
//! which arrays they own, which node is whose child) and keeps each fixed
//! record as its bytes. The fields the layout sets (offsets, the counts of
//! arrays, the MDX's length) are zero in those bytes, and the writer fills
//! them: two models read from files that differ only in layout compare
//! equal. The layout the writer chooses is BioWare's, as measured over the
//! vanilla files (`docs/formats/mdl.md`).

use std::collections::HashSet;

use crate::animation::{ANIMATION_HEADER_LEN, EVENT_LEN};
use crate::controller::CONTROLLER_LEN;
use crate::error::{Error, Result};
use crate::model::{
    Data, FILE_HEADER_LEN, GEOM_ROOT, MESH_HEADER_LEN, NODE_HAS_MESH, NODE_HEADER_LEN,
};
use crate::skin::SKIN_HEADER_LEN;
use crate::view::FACE_LEN;

/// The model header's length.
pub const MODEL_HEADER_LEN: usize = 0xC4;
/// The light header's length, following the node header.
pub const LIGHT_HEADER_LEN: usize = 92;
/// The emitter header's length, following the node header.
pub const EMITTER_HEADER_LEN: usize = 224;
/// The reference header's length, following the node header.
pub const REFERENCE_HEADER_LEN: usize = 36;
/// The dangly header's length, following the mesh header.
pub const DANGLY_HEADER_LEN: usize = 28;
/// The AABB header's length, following the mesh header: the offset of the
/// tree's root.
pub const AABB_HEADER_LEN: usize = 4;
/// The length of one entry of an AABB tree.
pub const AABB_ENTRY_LEN: usize = 40;
/// The saber header's length, following the mesh header.
pub const SABER_HEADER_LEN: usize = 20;

/// The deepest node tree read; deeper is taken for a damaged file.
const MAX_DEPTH: usize = 1024;

// The layout's fields in each record: (offset, length) runs the reader
// zeroes and the writer fills.
pub(crate) const MODEL_LAYOUT: &[(usize, usize)] =
    &[(0x28, 4), (0x58, 12), (0xA8, 4), (0xB0, 4), (0xB8, 12)];
pub(crate) const ANIMATION_LAYOUT: &[(usize, usize)] = &[(0x28, 4), (0x78, 12)];
pub(crate) const NODE_LAYOUT: &[(usize, usize)] = &[(0x0C, 4), (0x2C, 36)];
pub(crate) const MESH_LAYOUT: &[(usize, usize)] = &[(8, 12), (176, 36), (324, 8)];
pub(crate) const SKIN_LAYOUT: &[(usize, usize)] = &[(20, 44)];
pub(crate) const DANGLY_LAYOUT: &[(usize, usize)] = &[(0, 12), (24, 4)];
pub(crate) const AABB_ENTRY_LAYOUT: &[(usize, usize)] = &[(24, 8)];
pub(crate) const SABER_LAYOUT: &[(usize, usize)] = &[(0, 12)];
pub(crate) const LIGHT_LAYOUT: &[(usize, usize)] = &[(4, 60)];

/// A binary model's MDL and MDX as stored, owned.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFile {
    /// The model header, [`MODEL_HEADER_LEN`] bytes from the start of the
    /// model data, with the layout's fields zero: the root's offset (0x28),
    /// the animation array (0x58), the node at 0xA8 (see
    /// [`head_node`](Self::head_node)), the MDX's length (0xB0) and the
    /// name array (0xB8).
    pub header: [u8; MODEL_HEADER_LEN],
    /// The node the model header's word at 0xA8 points to: the root in
    /// most models, `neck_g` in the heads.
    pub head_node: NodePointer,
    /// The name array's names, without their NULs, by node number.
    pub names: Vec<Vec<u8>>,
    /// The animations, in stored order.
    pub animations: Vec<AnimationFile>,
    /// The geometry's root node, holding the tree.
    pub root: NodeFile,
}

/// Where a pointer to a geometry node leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodePointer {
    /// The geometry node at this index, counting in pre-order (children in
    /// order) from the root at 0.
    Node(usize),
    /// A value that is the offset of no geometry node, kept as read.
    Raw(u32),
}

/// One animation: its header, its events and its tree of nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationFile {
    /// The animation header, [`ANIMATION_HEADER_LEN`] bytes, with the root
    /// node's offset (0x28) and the event array (0x78) zero.
    pub header: [u8; ANIMATION_HEADER_LEN],
    /// The event records, [`EVENT_LEN`] bytes each: a time and a name.
    pub events: Vec<[u8; EVENT_LEN]>,
    /// The animation's root node.
    pub root: NodeFile,
}

/// One node: its header, what its flags say follows the header, its
/// children and its controllers.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeFile {
    /// The node header, [`NODE_HEADER_LEN`] bytes, with the parent's offset
    /// (0x0C) and the child, controller and controller data arrays (0x2C to
    /// 0x50) zero.
    pub header: [u8; NODE_HEADER_LEN],
    /// The structures after the header.
    pub body: Body,
    /// The children, in stored order.
    pub children: Vec<NodeFile>,
    /// The controller records, [`CONTROLLER_LEN`] bytes each.
    pub controllers: Vec<[u8; CONTROLLER_LEN]>,
    /// The controller data: four bytes per float.
    pub controller_data: Vec<u8>,
}

/// What follows a node's header, by its flags.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    /// A dummy (0x001): nothing.
    Dummy,
    /// A light (0x003).
    Light(Box<LightFile>),
    /// An emitter (0x005): its header, which points to nothing else.
    Emitter(Box<[u8; EMITTER_HEADER_LEN]>),
    /// A reference (0x011): a model name and a word.
    Reference(Box<[u8; REFERENCE_HEADER_LEN]>),
    /// Any node with a mesh header (bit 0x020).
    Mesh(Box<MeshFile>),
}

/// A light node's header and the flare arrays it points to.
#[derive(Debug, Clone, PartialEq)]
pub struct LightFile {
    /// The light header, [`LIGHT_HEADER_LEN`] bytes, with its five arrays
    /// (4 to 64) zero.
    pub header: [u8; LIGHT_HEADER_LEN],
    /// The array at 4, four bytes per element; empty in every vanilla
    /// light.
    pub unknown: Vec<u8>,
    /// The flare sizes (the array at 16), four bytes each.
    pub flare_sizes: Vec<u8>,
    /// The flare positions (the array at 28), four bytes each.
    pub flare_positions: Vec<u8>,
    /// The flare colour shifts (the array at 40), twelve bytes each.
    pub flare_colors: Vec<u8>,
    /// The flare textures' names (the array of pointers at 52), without
    /// their NULs.
    pub flare_textures: Vec<Vec<u8>>,
}

/// A mesh node's mesh header, the header of its kind, and the arrays they
/// point to, with its MDX block.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshFile {
    /// The mesh header, [`MESH_HEADER_LEN`] bytes, with the face array (8),
    /// the three index arrays (176 to 212), the MDX offset (324) and the
    /// position array's offset (328) zero.
    pub header: [u8; MESH_HEADER_LEN],
    /// The header after the mesh header, by the node's kind.
    pub kind: MeshKind,
    /// The faces, [`FACE_LEN`] bytes each.
    pub faces: Vec<u8>,
    /// The index counts (the array at 176): one per index list.
    pub index_counts: Vec<u32>,
    /// The index lists the array at 188 points to, two bytes per index.
    pub index_lists: Vec<Vec<u8>>,
    /// The array at 200, one word per element; its meaning is not
    /// established.
    pub index_unknown: Vec<u32>,
    /// The positions in the model data (the offset at 328), twelve bytes
    /// per vertex; empty when the mesh stores none there.
    pub positions: Vec<u8>,
    /// The mesh's MDX block, when it has one.
    pub mdx: Option<MdxBlock>,
}

/// One mesh's block of the MDX.
#[derive(Debug, Clone, PartialEq)]
pub struct MdxBlock {
    /// The vertices, `vertex count × stride` bytes.
    pub vertices: Vec<u8>,
    /// The vertex-sized row after them, its first float 10,000,000 (1,000,000
    /// after a skin's).
    pub terminator: Vec<u8>,
    /// The bytes after the terminator up to the next block, uninitialised
    /// memory in BioWare's files. The writer keeps them where its layout
    /// puts the same number of bytes there, and writes zeros otherwise;
    /// [`ModelFile::semantic_eq`] does not compare them.
    pub padding: Vec<u8>,
}

/// The header that follows a mesh header, by the node's kind.
#[derive(Debug, Clone, PartialEq)]
pub enum MeshKind {
    /// A trimesh (0x021): nothing.
    Trimesh,
    /// A skin (0x061).
    Skin(Box<SkinFile>),
    /// A dangly mesh (0x121).
    Dangly(Box<DanglyFile>),
    /// An AABB mesh (0x221): the tree's entries.
    Aabb(Vec<AabbEntry>),
    /// A lightsaber blade (0x821).
    Saber(Box<SaberFile>),
}

/// A skin node's header and the arrays it points to.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinFile {
    /// The skin header, [`SKIN_HEADER_LEN`] bytes, with the bone map's
    /// offset and count (20, 24) and the three arrays (28 to 64) zero.
    pub header: [u8; SKIN_HEADER_LEN],
    /// The bone map, four bytes per entry.
    pub bone_map: Vec<u8>,
    /// The qbones (the array at 28), sixteen bytes each.
    pub qbones: Vec<u8>,
    /// The tbones (the array at 40), twelve bytes each.
    pub tbones: Vec<u8>,
    /// The array at 52, four bytes each; not established.
    pub unknown: Vec<u8>,
}

/// A dangly node's header and the arrays it points to.
#[derive(Debug, Clone, PartialEq)]
pub struct DanglyFile {
    /// The dangly header, [`DANGLY_HEADER_LEN`] bytes, with the constraint
    /// array (0) and the offset at 24 zero.
    pub header: [u8; DANGLY_HEADER_LEN],
    /// The constraints, four bytes per vertex.
    pub constraints: Vec<u8>,
    /// The array the offset at 24 points to, twelve bytes per vertex.
    pub positions: Vec<u8>,
}

/// One entry of an AABB tree.
#[derive(Debug, Clone, PartialEq)]
pub struct AabbEntry {
    /// The entry, [`AABB_ENTRY_LEN`] bytes, with its two child offsets (24,
    /// 28) zero.
    pub bytes: [u8; AABB_ENTRY_LEN],
    /// The index of the child the word at 24 points to, if any.
    pub left: Option<usize>,
    /// The index of the child the word at 28 points to, if any.
    pub right: Option<usize>,
}

/// A saber node's header and the three per-vertex arrays it points to.
#[derive(Debug, Clone, PartialEq)]
pub struct SaberFile {
    /// The saber header, [`SABER_HEADER_LEN`] bytes, with its three offsets
    /// (0, 4, 8) zero.
    pub header: [u8; SABER_HEADER_LEN],
    /// The arrays the offsets at 0, 4 and 8 point to: twelve, eight and
    /// twelve bytes per vertex.
    pub arrays: [Vec<u8>; 3],
}

impl ModelFile {
    /// Read a binary model's MDL and MDX. The MDX may be empty for a model
    /// whose meshes have no MDX blocks.
    pub fn parse(mdl: &[u8], mdx: &[u8]) -> Result<Self> {
        Reader::new(mdl, mdx)?.model()
    }

    /// Whether two models hold the same data: equal but for the
    /// [`MdxBlock::padding`], which is layout.
    pub fn semantic_eq(&self, other: &ModelFile) -> bool {
        let mut a = self.clone();
        let mut b = other.clone();
        a.clear_padding();
        b.clear_padding();
        a == b
    }

    fn clear_padding(&mut self) {
        fn go(n: &mut NodeFile) {
            if let Body::Mesh(m) = &mut n.body
                && let Some(b) = &mut m.mdx
            {
                b.padding.clear();
            }
            n.children.iter_mut().for_each(go);
        }
        go(&mut self.root);
        for a in &mut self.animations {
            go(&mut a.root);
        }
    }

    /// The geometry's nodes in pre-order, children in order: the order
    /// [`NodePointer::Node`] counts in.
    pub fn nodes(&self) -> Vec<&NodeFile> {
        let mut out = Vec::new();
        let mut stack = vec![&self.root];
        while let Some(n) = stack.pop() {
            out.push(n);
            stack.extend(n.children.iter().rev());
        }
        out
    }
}

impl NodeFile {
    /// The flags word.
    pub fn flags(&self) -> u16 {
        u16::from_le_bytes([self.header[0], self.header[1]])
    }

    /// The node number: the index of its name.
    pub fn number(&self) -> u16 {
        u16::from_le_bytes([self.header[4], self.header[5]])
    }
}

impl MeshFile {
    /// The vertex count (the u16 at 304).
    pub fn vertex_count(&self) -> usize {
        u16::from_le_bytes([self.header[304], self.header[305]]) as usize
    }

    /// The MDX stride (the word at 252).
    pub fn stride(&self) -> u32 {
        u32::from_le_bytes(self.header[252..256].try_into().expect("four bytes"))
    }
}

/// Zero the layout's runs of a record.
pub(crate) fn mask<const N: usize>(mut bytes: [u8; N], runs: &[(usize, usize)]) -> [u8; N] {
    for &(at, len) in runs {
        bytes[at..at + len].fill(0);
    }
    bytes
}

/// Whether a mesh header's stride names an MDX block.
pub(crate) fn has_block(stride: u32) -> bool {
    stride != 0 && stride != u32::MAX
}

struct Reader<'a> {
    d: Data<'a>,
    mdx: &'a [u8],
    /// Every structure's offset reached, to refuse a node reached twice.
    seen: HashSet<usize>,
    /// The geometry's node offsets in pre-order, for the pointer at 0xA8.
    geometry: Vec<usize>,
    /// The MDX blocks' starts and ends, for the padding after each.
    blocks: Vec<(usize, usize)>,
}

impl<'a> Reader<'a> {
    fn new(mdl: &'a [u8], mdx: &'a [u8]) -> Result<Self> {
        if mdl.len() < FILE_HEADER_LEN {
            return Err(Error::Truncated {
                what: "file header",
                offset: 0,
                needed: FILE_HEADER_LEN,
                available: mdl.len(),
            });
        }
        let word = |i: usize| u32::from_le_bytes(mdl[i..i + 4].try_into().expect("four bytes"));
        if word(0) != 0 {
            return Err(Error::NotBinary { first: word(0) });
        }
        let data = &mdl[FILE_HEADER_LEN..];
        let data = &data[..(word(4) as usize).min(data.len())];
        Ok(Reader {
            d: Data { b: data },
            mdx,
            seen: HashSet::new(),
            geometry: Vec::new(),
            blocks: Vec::new(),
        })
    }

    fn fixed<const N: usize>(&self, at: usize, what: &'static str) -> Result<[u8; N]> {
        Ok(self.d.slice(at, N, what)?.try_into().expect("N bytes"))
    }

    /// An array's elements of `size` bytes each, copied.
    fn array(&self, at: usize, size: usize, what: &'static str) -> Result<(usize, Vec<u8>)> {
        let (off, n) = self.d.array(at, what)?;
        let len = n.checked_mul(size).ok_or(Error::Truncated {
            what,
            offset: off,
            needed: usize::MAX,
            available: self.d.b.len(),
        })?;
        Ok((n, self.d.slice(off, len, what)?.to_vec()))
    }

    fn words(&self, at: usize, what: &'static str) -> Result<Vec<u32>> {
        let (_, b) = self.array(at, 4, what)?;
        Ok(b.as_chunks::<4>()
            .0
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect())
    }

    /// A NUL-ended string anywhere in the model data, without its NUL.
    fn c_bytes(&self, at: usize, what: &'static str) -> Result<Vec<u8>> {
        let s = self.d.b.get(at..).ok_or(Error::Truncated {
            what,
            offset: at,
            needed: 1,
            available: self.d.b.len(),
        })?;
        let end = s
            .iter()
            .position(|&c| c == 0)
            .ok_or(Error::BadName { what, offset: at })?;
        Ok(s[..end].to_vec())
    }

    /// `n` bytes at the offset stored at `at`, or none when `n` is zero.
    fn pointed(&self, at: usize, n: usize, what: &'static str) -> Result<Vec<u8>> {
        if n == 0 {
            return Ok(Vec::new());
        }
        let off = self.d.u32(at, what)? as usize;
        Ok(self.d.slice(off, n, what)?.to_vec())
    }

    fn model(mut self) -> Result<ModelFile> {
        let header: [u8; MODEL_HEADER_LEN] = self.fixed(0, "model header")?;
        let (names_off, names_n) = self.d.array(0xB8, "name array")?;
        self.d
            .slice(names_off, names_n.saturating_mul(4), "name array")?;
        let names = (0..names_n)
            .map(|i| {
                let p = self.d.u32(names_off + 4 * i, "name pointer")? as usize;
                self.c_bytes(p, "node name")
            })
            .collect::<Result<Vec<_>>>()?;

        let (anim_off, anim_n) = self.d.array(0x58, "animation array")?;
        self.d
            .slice(anim_off, anim_n.saturating_mul(4), "animation array")?;
        let mut animations = Vec::with_capacity(anim_n.min(4096));
        for i in 0..anim_n {
            let a = self.d.u32(anim_off + 4 * i, "animation pointer")? as usize;
            let header: [u8; ANIMATION_HEADER_LEN] = self.fixed(a, "animation header")?;
            let (_, ev) = self.array(a + 0x78, EVENT_LEN, "event array")?;
            let events = ev.as_chunks::<EVENT_LEN>().0.to_vec();
            let root_at = self.d.u32(a + GEOM_ROOT, "animation root node")? as usize;
            let root = self.node(root_at, 0, false)?;
            animations.push(AnimationFile {
                header: mask(header, ANIMATION_LAYOUT),
                events,
                root,
            });
        }

        let root_at = self.d.u32(GEOM_ROOT, "root")? as usize;
        let root = self.node(root_at, 0, true)?;
        let head = self.d.u32(0xA8, "head node")?;
        let head_node = self
            .geometry
            .iter()
            .position(|&o| o == head as usize)
            .map_or(NodePointer::Raw(head), NodePointer::Node);

        let mut model = ModelFile {
            header: mask(header, MODEL_LAYOUT),
            head_node,
            names,
            animations,
            root,
        };
        self.padding(&mut model);
        Ok(model)
    }

    fn node(&mut self, at: usize, depth: usize, geometry: bool) -> Result<NodeFile> {
        if depth > MAX_DEPTH {
            return Err(Error::BadTree {
                reason: format!("the tree is deeper than {MAX_DEPTH} nodes"),
            });
        }
        if !self.seen.insert(at) {
            return Err(Error::BadTree {
                reason: format!("the node at model offset {at} is reached twice"),
            });
        }
        if geometry {
            self.geometry.push(at);
        }
        let header: [u8; NODE_HEADER_LEN] = self.fixed(at, "node header")?;
        let flags = u16::from_le_bytes([header[0], header[1]]);
        let h = at + NODE_HEADER_LEN;
        let body = match flags {
            0x001 => Body::Dummy,
            0x003 => Body::Light(Box::new(self.light(h)?)),
            0x005 => Body::Emitter(Box::new(self.fixed(h, "emitter header")?)),
            0x011 => Body::Reference(Box::new(self.fixed(h, "reference header")?)),
            f if f & NODE_HAS_MESH != 0 => Body::Mesh(Box::new(self.mesh(at, flags, geometry)?)),
            _ => {
                return Err(Error::Unsupported {
                    what: format!("node flags {flags:#05x} at model offset {at}"),
                });
            }
        };
        let (_, controllers) = self.array(at + 56, CONTROLLER_LEN, "controller array")?;
        let controllers = controllers.as_chunks::<CONTROLLER_LEN>().0.to_vec();
        let (_, controller_data) = self.array(at + 68, 4, "controller data")?;
        let kids = self.words(at + 44, "child array")?;
        let mut children = Vec::with_capacity(kids.len());
        for k in kids {
            children.push(self.node(k as usize, depth + 1, geometry)?);
        }
        Ok(NodeFile {
            header: mask(header, NODE_LAYOUT),
            body,
            children,
            controllers,
            controller_data,
        })
    }

    fn light(&self, h: usize) -> Result<LightFile> {
        let header: [u8; LIGHT_HEADER_LEN] = self.fixed(h, "light header")?;
        let pointers = self.words(h + 52, "flare texture array")?;
        let flare_textures = pointers
            .iter()
            .map(|&p| self.c_bytes(p as usize, "flare texture"))
            .collect::<Result<Vec<_>>>()?;
        Ok(LightFile {
            header: mask(header, LIGHT_LAYOUT),
            unknown: self.array(h + 4, 4, "light array")?.1,
            flare_sizes: self.array(h + 16, 4, "flare sizes")?.1,
            flare_positions: self.array(h + 28, 4, "flare positions")?.1,
            flare_colors: self.array(h + 40, 12, "flare colours")?.1,
            flare_textures,
        })
    }

    fn mesh(&mut self, at: usize, flags: u16, geometry: bool) -> Result<MeshFile> {
        let h = at + NODE_HEADER_LEN;
        let header: [u8; MESH_HEADER_LEN] = self.fixed(h, "mesh header")?;
        let nv = u16::from_le_bytes([header[304], header[305]]) as usize;
        let e = h + MESH_HEADER_LEN;
        let kind = match flags {
            0x021 => MeshKind::Trimesh,
            0x061 => {
                let header: [u8; SKIN_HEADER_LEN] = self.fixed(e, "skin header")?;
                let n = self.d.u32(e + 24, "bone map count")? as usize;
                MeshKind::Skin(Box::new(SkinFile {
                    header: mask(header, SKIN_LAYOUT),
                    bone_map: self.pointed(e + 20, n.saturating_mul(4), "bone map")?,
                    qbones: self.array(e + 28, 16, "qbones")?.1,
                    tbones: self.array(e + 40, 12, "tbones")?.1,
                    unknown: self.array(e + 52, 4, "skin array")?.1,
                }))
            }
            0x121 => {
                let header: [u8; DANGLY_HEADER_LEN] = self.fixed(e, "dangly header")?;
                MeshKind::Dangly(Box::new(DanglyFile {
                    header: mask(header, DANGLY_LAYOUT),
                    constraints: self.array(e, 4, "constraints")?.1,
                    positions: self.pointed(e + 24, 12 * nv, "dangly positions")?,
                }))
            }
            0x221 => MeshKind::Aabb(self.aabb(e)?),
            0x821 => {
                let header: [u8; SABER_HEADER_LEN] = self.fixed(e, "saber header")?;
                MeshKind::Saber(Box::new(SaberFile {
                    header: mask(header, SABER_LAYOUT),
                    arrays: [
                        self.pointed(e, 12 * nv, "saber array")?,
                        self.pointed(e + 4, 8 * nv, "saber array")?,
                        self.pointed(e + 8, 12 * nv, "saber array")?,
                    ],
                }))
            }
            _ => {
                return Err(Error::Unsupported {
                    what: format!("node flags {flags:#05x} at model offset {at}"),
                });
            }
        };
        let (_, faces) = self.array(h + 8, FACE_LEN, "face array")?;
        let index_counts = self.words(h + 176, "index count array")?;
        let pointers = self.words(h + 188, "index offset array")?;
        let mut index_lists = Vec::with_capacity(pointers.len());
        for (i, &p) in pointers.iter().enumerate() {
            let n = *index_counts.get(i).ok_or_else(|| Error::Inconsistent {
                what: format!(
                    "the mesh at model offset {at} has {} index lists and {} counts",
                    pointers.len(),
                    index_counts.len()
                ),
            })? as usize;
            index_lists.push(
                self.d
                    .slice(p as usize, n.saturating_mul(2), "index list")?
                    .to_vec(),
            );
        }
        let index_unknown = self.words(h + 200, "mesh array")?;
        let pos_at = self.d.u32(h + 328, "vertex array pointer")?;
        let positions = if nv > 0 && pos_at != 0 && pos_at != u32::MAX {
            self.d
                .slice(pos_at as usize, 12 * nv, "vertex array")?
                .to_vec()
        } else {
            Vec::new()
        };
        let stride = u32::from_le_bytes(header[252..256].try_into().expect("four bytes"));
        let mdx = if geometry && has_block(stride) {
            let off = self.d.u32(h + 324, "MDX offset")? as usize;
            let stride = stride as usize;
            let len = nv.saturating_add(1).saturating_mul(stride);
            let block = off
                .checked_add(len)
                .and_then(|end| self.mdx.get(off..end))
                .ok_or_else(|| Error::MdxOutOfRange {
                    node: format!("at model offset {at}"),
                    offset: off,
                    needed: len,
                    available: self.mdx.len(),
                })?;
            self.blocks.push((off, off + len));
            let (vertices, terminator) = block.split_at(nv * stride);
            Some(MdxBlock {
                vertices: vertices.to_vec(),
                terminator: terminator.to_vec(),
                padding: Vec::new(),
            })
        } else {
            None
        };
        Ok(MeshFile {
            header: mask(header, MESH_LAYOUT),
            kind,
            faces,
            index_counts,
            index_lists,
            index_unknown,
            positions,
            mdx,
        })
    }

    /// The AABB tree from the root the header at `e` points to, in
    /// pre-order, left before right.
    fn aabb(&mut self, e: usize) -> Result<Vec<AabbEntry>> {
        let root = self.d.u32(e, "AABB root")? as usize;
        let mut out: Vec<AabbEntry> = Vec::new();
        if root == 0 {
            return Ok(out);
        }
        // (offset, parent index and whether this is its right child)
        let mut stack = vec![(root, None::<(usize, bool)>)];
        while let Some((at, parent)) = stack.pop() {
            if !self.seen.insert(at) {
                return Err(Error::BadTree {
                    reason: format!("the AABB entry at model offset {at} is reached twice"),
                });
            }
            let bytes: [u8; AABB_ENTRY_LEN] = self.fixed(at, "AABB entry")?;
            let index = out.len();
            if let Some((p, right)) = parent {
                if right {
                    out[p].right = Some(index);
                } else {
                    out[p].left = Some(index);
                }
            }
            let word = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("four"));
            let (l, r) = (word(24) as usize, word(28) as usize);
            out.push(AabbEntry {
                bytes: mask(bytes, AABB_ENTRY_LAYOUT),
                left: None,
                right: None,
            });
            if r != 0 {
                stack.push((r, Some((index, true))));
            }
            if l != 0 {
                stack.push((l, Some((index, false))));
            }
        }
        Ok(out)
    }

    /// Give each MDX block the bytes after its terminator up to the next
    /// block, or to the MDX's end after the last.
    fn padding(&self, model: &mut ModelFile) {
        let mut starts: Vec<usize> = self.blocks.iter().map(|b| b.0).collect();
        starts.sort_unstable();
        let mdx = self.mdx;
        let mut blocks = self.blocks.iter();
        fn go(n: &mut NodeFile, f: &mut dyn FnMut(&mut MdxBlock)) {
            if let Body::Mesh(m) = &mut n.body
                && let Some(b) = &mut m.mdx
            {
                f(b);
            }
            for c in &mut n.children {
                go(c, f);
            }
        }
        // Blocks were recorded in the order the reader met the meshes,
        // which is this walk's order.
        go(&mut model.root, &mut |b| {
            let &(_, end) = blocks.next().expect("one record per block");
            let next = starts
                .iter()
                .copied()
                .find(|&s| s >= end)
                .unwrap_or(mdx.len());
            b.padding = mdx[end..next].to_vec();
        });
    }
}
