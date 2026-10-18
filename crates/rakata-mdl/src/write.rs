//! The writer: a [`ModelFile`] to its MDL and MDX, laid out as BioWare's
//! compiler lays them out.
//!
//! The model data is written in one pass: the model header, the name
//! array and the names, the animation array, each animation (its header,
//! its events, its tree), then the geometry's tree. A node is its header,
//! the header of its kind and the arrays that kind owns, its child array,
//! its children in order, and last its controllers and their data. The
//! MDX holds the geometry's mesh blocks, every non-skin mesh by node
//! number and then every skin by node number, each followed by its
//! terminator row and, before the next block, as many bytes as the MDX's
//! length then is past a multiple of 16. An empty array points where the
//! next structure begins, except those the vanilla files leave at zero.
//! `docs/formats/mdl.md` records the measurements.

use crate::error::{Error, Result};
use crate::file::{
    AABB_ENTRY_LEN, AnimationFile, Body, LightFile, MeshFile, MeshKind, ModelFile, NodeFile,
    NodePointer, has_block,
};

/// Where an empty array points.
#[derive(Clone, Copy)]
enum Empty {
    /// At the next structure.
    Cursor,
    /// At zero.
    Zero,
}

struct Writer<'m> {
    d: Vec<u8>,
    /// The geometry's node offsets in pre-order.
    geometry: Vec<usize>,
    /// Each geometry mesh with an MDX block: its mesh header's offset and
    /// the mesh.
    blocks: Vec<(usize, &'m MeshFile)>,
}

fn too_large(what: &str) -> Error {
    Error::TooLarge {
        what: what.to_owned(),
    }
}

fn u32_of(n: usize, what: &str) -> Result<u32> {
    u32::try_from(n).map_err(|_| too_large(what))
}

impl<'m> Writer<'m> {
    fn put(&mut self, at: usize, v: u32) {
        self.d[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn put_offset(&mut self, at: usize, off: usize) -> Result<()> {
        let v = u32_of(off, "an offset in the model data")?;
        self.put(at, v);
        Ok(())
    }

    /// Reserve `n` zero bytes and return their offset.
    fn alloc(&mut self, n: usize) -> usize {
        let at = self.d.len();
        self.d.resize(at + n, 0);
        at
    }

    fn append(&mut self, bytes: &[u8]) -> usize {
        let at = self.d.len();
        self.d.extend_from_slice(bytes);
        at
    }

    /// Write `bytes` as the array whose header is at `at`, of `size`-byte
    /// elements.
    fn array(&mut self, at: usize, bytes: &[u8], size: usize, empty: Empty) -> Result<usize> {
        let n = u32_of(bytes.len() / size, "an array's count")?;
        let off = match (bytes.is_empty(), empty) {
            (true, Empty::Zero) => 0,
            _ => self.append(bytes),
        };
        self.put_offset(at, off)?;
        self.put(at + 4, n);
        self.put(at + 8, n);
        Ok(off)
    }

    /// An array of offsets to NUL-ended strings, then the strings.
    fn strings(&mut self, at: usize, strings: &[Vec<u8>], empty: Empty) -> Result<()> {
        let table = self.array(at, &vec![0; 4 * strings.len()], 4, empty)?;
        for (i, s) in strings.iter().enumerate() {
            let p = self.append(s);
            self.d.push(0);
            self.put_offset(table + 4 * i, p)?;
        }
        Ok(())
    }

    /// `bytes` at the offset stored at `at`, or zero there when empty.
    fn pointed(&mut self, at: usize, bytes: &[u8]) -> Result<()> {
        if !bytes.is_empty() {
            let off = self.append(bytes);
            self.put_offset(at, off)?;
        }
        Ok(())
    }

    fn node(&mut self, n: &'m NodeFile, parent: usize, geometry: bool) -> Result<usize> {
        let at = self.append(&n.header);
        if geometry {
            self.geometry.push(at);
        }
        self.put_offset(at + 12, parent)?;
        let h = self.d.len();
        match &n.body {
            Body::Dummy => {}
            Body::Light(l) => self.light(h, l)?,
            Body::Emitter(b) => {
                self.append(&b[..]);
            }
            Body::Reference(b) => {
                self.append(&b[..]);
            }
            Body::Mesh(m) => self.mesh(h, m, geometry)?,
        }
        let table = self.array(at + 44, &vec![0; 4 * n.children.len()], 4, Empty::Cursor)?;
        for (i, c) in n.children.iter().enumerate() {
            let off = self.node(c, at, geometry)?;
            self.put_offset(table + 4 * i, off)?;
        }
        let records: Vec<u8> = n.controllers.iter().flatten().copied().collect();
        self.array(at + 56, &records, 16, Empty::Zero)?;
        if !n.controller_data.len().is_multiple_of(4) {
            return Err(Error::Inconsistent {
                what: format!(
                    "node {}'s controller data is {} bytes, not whole floats",
                    n.number(),
                    n.controller_data.len()
                ),
            });
        }
        self.array(at + 68, &n.controller_data, 4, Empty::Zero)?;
        Ok(at)
    }

    fn light(&mut self, h: usize, l: &LightFile) -> Result<()> {
        self.append(&l.header);
        self.array(h + 4, &l.unknown, 4, Empty::Zero)?;
        self.strings(h + 52, &l.flare_textures, Empty::Zero)?;
        self.array(h + 16, &l.flare_sizes, 4, Empty::Zero)?;
        self.array(h + 28, &l.flare_positions, 4, Empty::Zero)?;
        self.array(h + 40, &l.flare_colors, 12, Empty::Zero)?;
        Ok(())
    }

    fn mesh(&mut self, h: usize, m: &'m MeshFile, geometry: bool) -> Result<()> {
        let nv = m.vertex_count();
        let disagree = |what: String| Error::Inconsistent { what };
        let check = |what: &str, bytes: &[u8], per: usize| -> Result<()> {
            if !bytes.is_empty() && bytes.len() != per * nv {
                return Err(disagree(format!(
                    "a mesh of {nv} vertices has {} bytes of {what}, not {per} per vertex",
                    bytes.len()
                )));
            }
            Ok(())
        };
        self.append(&m.header);
        let e = self.d.len();
        match &m.kind {
            MeshKind::Trimesh => {}
            MeshKind::Skin(s) => {
                self.append(&s.header);
                if !s.bone_map.is_empty() {
                    let off = self.append(&s.bone_map);
                    self.put_offset(e + 20, off)?;
                }
                let n = u32_of(s.bone_map.len() / 4, "a bone map's count")?;
                self.put(e + 24, n);
                self.array(e + 28, &s.qbones, 16, Empty::Zero)?;
                self.array(e + 40, &s.tbones, 12, Empty::Zero)?;
                self.array(e + 52, &s.unknown, 4, Empty::Zero)?;
            }
            MeshKind::Dangly(g) => {
                check("dangly positions", &g.positions, 12)?;
                self.append(&g.header);
                self.array(e, &g.constraints, 4, Empty::Zero)?;
                self.pointed(e + 24, &g.positions)?;
            }
            MeshKind::Aabb(entries) => {
                self.alloc(4);
                if !entries.is_empty() {
                    let base = self.d.len();
                    self.put_offset(e, base)?;
                    let at = |i: Option<usize>| -> Result<usize> {
                        match i {
                            None => Ok(0),
                            Some(i) if i < entries.len() => Ok(base + AABB_ENTRY_LEN * i),
                            Some(i) => Err(Error::Inconsistent {
                                what: format!(
                                    "an AABB entry names entry {i} of a tree of {}",
                                    entries.len()
                                ),
                            }),
                        }
                    };
                    for a in entries {
                        let p = self.append(&a.bytes);
                        let (l, r) = (at(a.left)?, at(a.right)?);
                        self.put_offset(p + 24, l)?;
                        self.put_offset(p + 28, r)?;
                    }
                }
            }
            MeshKind::Saber(s) => {
                check("saber positions", &s.arrays[0], 12)?;
                check("saber coordinates", &s.arrays[1], 8)?;
                check("saber vectors", &s.arrays[2], 12)?;
                self.append(&s.header);
                self.pointed(e, &s.arrays[0])?;
                self.pointed(e + 8, &s.arrays[2])?;
                self.pointed(e + 4, &s.arrays[1])?;
            }
        }
        self.array(h + 8, &m.faces, 32, Empty::Zero)?;
        let counts: Vec<u8> = m
            .index_counts
            .iter()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        self.array(h + 176, &counts, 4, Empty::Cursor)?;
        check("positions", &m.positions, 12)?;
        self.pointed(h + 328, &m.positions)?;
        if m.index_lists.len() != m.index_counts.len()
            || m.index_lists
                .iter()
                .zip(&m.index_counts)
                .any(|(l, &c)| l.len() != 2 * c as usize)
        {
            return Err(disagree(format!(
                "a mesh's {} index lists do not match its index counts {:?}",
                m.index_lists.len(),
                m.index_counts
            )));
        }
        // A mesh without vertices leaves these two at zero; one with
        // vertices and no lists (a saber) points them at the cursor.
        let empty = if nv == 0 { Empty::Zero } else { Empty::Cursor };
        let table = self.array(h + 188, &vec![0; 4 * m.index_lists.len()], 4, empty)?;
        let unknown: Vec<u8> = m
            .index_unknown
            .iter()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        self.array(h + 200, &unknown, 4, empty)?;
        for (i, list) in m.index_lists.iter().enumerate() {
            let p = self.append(list);
            self.put_offset(table + 4 * i, p)?;
        }
        if let Some(b) = &m.mdx {
            if !geometry {
                return Err(Error::Unsupported {
                    what: "an MDX block on a mesh of an animation".to_owned(),
                });
            }
            let stride = m.stride();
            if !has_block(stride)
                || b.vertices.len() != nv * stride as usize
                || b.terminator.len() != stride as usize
            {
                return Err(disagree(format!(
                    "an MDX block of {} bytes of vertices and a {}-byte terminator for {nv} vertices of stride {stride}",
                    b.vertices.len(),
                    b.terminator.len()
                )));
            }
            self.blocks.push((h, m));
        }
        Ok(())
    }

    fn animation(&mut self, a: &'m AnimationFile) -> Result<usize> {
        let at = self.append(&a.header);
        let events: Vec<u8> = a.events.iter().flatten().copied().collect();
        self.array(at + 0x78, &events, 36, Empty::Zero)?;
        let root = self.node(&a.root, 0, false)?;
        self.put_offset(at + 0x28, root)?;
        Ok(at)
    }
}

impl ModelFile {
    /// Write the model: its MDL and its MDX.
    pub fn to_bytes(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut w = Writer {
            d: Vec::new(),
            geometry: Vec::new(),
            blocks: Vec::new(),
        };
        w.append(&self.header);
        w.strings(0xB8, &self.names, Empty::Cursor)?;
        let table = w.array(0x58, &vec![0; 4 * self.animations.len()], 4, Empty::Cursor)?;
        for (i, a) in self.animations.iter().enumerate() {
            let off = w.animation(a)?;
            w.put_offset(table + 4 * i, off)?;
        }
        let root = w.node(&self.root, 0, true)?;
        w.put_offset(0x28, root)?;
        let head = match self.head_node {
            NodePointer::Node(i) => *w.geometry.get(i).ok_or_else(|| Error::Inconsistent {
                what: format!(
                    "the header names node {i} of a geometry of {}",
                    w.geometry.len()
                ),
            })? as u32,
            // Kept as read, it must still name no node where this layout
            // puts them, or it would read back as that node.
            NodePointer::Raw(v) => match w.geometry.iter().position(|&o| o == v as usize) {
                Some(i) => {
                    return Err(Error::Inconsistent {
                        what: format!(
                            "the header's raw head-node value {v} is where node {i} is written"
                        ),
                    });
                }
                None => v,
            },
        };
        w.put(0xA8, head);

        let mut blocks = std::mem::take(&mut w.blocks);
        blocks.sort_by_key(|(h, _)| {
            let n = *h - crate::model::NODE_HEADER_LEN;
            let flags = u16::from_le_bytes([w.d[n], w.d[n + 1]]);
            let number = u16::from_le_bytes([w.d[n + 4], w.d[n + 5]]);
            (flags == 0x061, number)
        });
        let mut mdx = Vec::new();
        let last = blocks.len().saturating_sub(1);
        for (i, (h, m)) in blocks.iter().enumerate() {
            let b = m.mdx.as_ref().expect("a mesh with a block");
            let off = u32_of(mdx.len(), "an offset in the MDX")?;
            w.put(h + 324, off);
            mdx.extend_from_slice(&b.vertices);
            mdx.extend_from_slice(&b.terminator);
            if i < last {
                let pad = mdx.len() % 16;
                if b.padding.len() == pad {
                    mdx.extend_from_slice(&b.padding);
                } else {
                    mdx.resize(mdx.len() + pad, 0);
                }
            }
        }
        let mdx_len = u32_of(mdx.len(), "the MDX's length")?;
        w.put(0xB0, mdx_len);

        let data_len = u32_of(w.d.len(), "the model data's length")?;
        let mut mdl = Vec::with_capacity(12 + w.d.len());
        mdl.extend_from_slice(&0u32.to_le_bytes());
        mdl.extend_from_slice(&data_len.to_le_bytes());
        mdl.extend_from_slice(&mdx_len.to_le_bytes());
        mdl.extend_from_slice(&w.d);
        Ok((mdl, mdx))
    }
}
