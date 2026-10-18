//! A skin node's binding to its bones: the bone map, the bind transforms
//! (qbones and tbones), and each vertex's weights and bone indices in the
//! MDX.

use crate::error::{Error, Result};
use crate::model::{Data, Quaternion};
use crate::view::Attribute;

/// The skin header's length in KotOR 1, following the mesh header.
pub const SKIN_HEADER_LEN: usize = 100;

/// Four floats per vertex: a skin's weights or bone indices.
pub type Vec4s<'a> = Attribute<'a, 4>;

/// The skin part of a skin node: which nodes its vertices follow and how.
///
/// A vertex names up to four bones by local index, with a weight each;
/// [`bones`](Self::bones) turns a local index into the node number of the
/// bone (the index of its name in the model's name array, the same number
/// the node of that name carries). The bind transforms are kept by node
/// number.
#[derive(Debug, Clone, PartialEq)]
pub struct Skin<'a> {
    /// The node numbers of the bones, by local bone index: the inverse of
    /// the bone map.
    pub bones: Vec<u16>,
    /// Each vertex's four weights, from the mesh's MDX block. They sum to
    /// one in every vanilla vertex.
    pub weights: Vec4s<'a>,
    /// Each vertex's four local bone indices, from the mesh's MDX block,
    /// stored as floats; -1 where a slot names no bone.
    pub bone_indices: Vec4s<'a>,
    bone_map: &'a [u8],
    qbones: &'a [u8],
    tbones: &'a [u8],
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

impl<'a> Skin<'a> {
    /// The number of entries in the bone map, the qbones and the tbones:
    /// one per name of the model in every vanilla skin but one.
    pub fn bone_map_len(&self) -> usize {
        self.bone_map.len() / 4
    }

    /// The bone map as stored: one float per node number, the node's local
    /// bone index or -1.
    pub fn bone_map_bytes(&self) -> &'a [u8] {
        self.bone_map
    }

    /// The local bone index of node number `node`, if it is a bone of
    /// this skin.
    pub fn bone_index(&self, node: u16) -> Option<u16> {
        let at = 4 * node as usize;
        if at + 4 > self.bone_map.len() {
            return None;
        }
        let v = f32_at(self.bone_map, at);
        (v >= 0.0).then_some(v as u16)
    }

    /// The bind rotation stored for node number `node`: the rotation from
    /// the skin node's coordinates to the bone's at the bind pose.
    pub fn qbone(&self, node: u16) -> Option<Quaternion> {
        let at = 16 * node as usize;
        (at + 16 <= self.qbones.len()).then(|| Quaternion {
            w: f32_at(self.qbones, at),
            x: f32_at(self.qbones, at + 4),
            y: f32_at(self.qbones, at + 8),
            z: f32_at(self.qbones, at + 12),
        })
    }

    /// The bind translation stored for node number `node`, applied after
    /// [`qbone`](Self::qbone).
    pub fn tbone(&self, node: u16) -> Option<[f32; 3]> {
        let at = 12 * node as usize;
        (at + 12 <= self.tbones.len()).then(|| {
            [
                f32_at(self.tbones, at),
                f32_at(self.tbones, at + 4),
                f32_at(self.tbones, at + 8),
            ]
        })
    }

    /// Vertex `v`'s bones: for each of its four slots, the bone's node
    /// number and the weight, `None` for a slot that names no bone.
    pub fn influences(&self, v: usize) -> Option<[Option<(u16, f32)>; 4]> {
        let w = self.weights.get(v)?;
        let ix = self.bone_indices.get(v)?;
        let mut out = [None; 4];
        for k in 0..4 {
            if ix[k] >= 0.0 {
                out[k] = self.bones.get(ix[k] as usize).map(|&n| (n, w[k]));
            }
        }
        Some(out)
    }
}

/// The skin header at `h`, after the mesh header, with the per-vertex data
/// from the mesh's MDX block.
pub(crate) fn read_skin<'a>(
    d: &Data<'a>,
    h: usize,
    node: &str,
    vertex_buffer: &'a [u8],
    stride: usize,
    vertices: usize,
) -> Result<Skin<'a>> {
    d.slice(h, SKIN_HEADER_LEN, "skin header")?;
    let bad = |reason: String| Error::BadSkin {
        node: node.to_owned(),
        reason,
    };
    let attr = |field: usize| -> Result<Vec4s<'a>> {
        let off = d.u32(h + field, "skin MDX offset")? as usize;
        if vertex_buffer.is_empty() {
            return Ok(Attribute::new(&[], 0, 0, 0));
        }
        if off + 16 > stride {
            return Err(Error::BadVertexLayout {
                node: node.to_owned(),
                reason: format!("skin data of 4 floats at {off} passes the stride of {stride}"),
            });
        }
        Ok(Attribute::new(vertex_buffer, stride, off, vertices))
    };
    let weights = attr(12)?;
    let bone_indices = attr(16)?;

    let bm_off = d.u32(h + 20, "bone map")? as usize;
    let bm_len = d.u32(h + 24, "bone map")? as usize;
    let bone_map = d.slice(bm_off, 4 * bm_len, "bone map")?;
    let (q_off, q_len) = d.array(h + 28, "qbone array")?;
    let (t_off, t_len) = d.array(h + 40, "tbone array")?;
    if q_len != bm_len || t_len != bm_len {
        return Err(bad(format!(
            "the bone map has {bm_len} entries, the qbones {q_len} and the tbones {t_len}"
        )));
    }
    let qbones = d.slice(q_off, 16 * q_len, "qbone array")?;
    let tbones = d.slice(t_off, 12 * t_len, "tbone array")?;

    // The bone map's entries name local indices 0, 1, … each once.
    let mut by_index: Vec<Option<u16>> = Vec::new();
    for n in 0..bm_len {
        let v = f32_at(bone_map, 4 * n);
        if v < 0.0 {
            continue;
        }
        if v != v.trunc() || v > u16::MAX as f32 {
            return Err(bad(format!("node {n}'s bone map entry is {v}")));
        }
        let i = v as usize;
        if by_index.len() <= i {
            by_index.resize(i + 1, None);
        }
        if by_index[i].replace(n as u16).is_some() {
            return Err(bad(format!("two nodes have bone index {i}")));
        }
    }
    let bones = by_index
        .into_iter()
        .enumerate()
        .map(|(i, n)| n.ok_or_else(|| bad(format!("no node has bone index {i}"))))
        .collect::<Result<Vec<u16>>>()?;

    Ok(Skin {
        bones,
        weights,
        bone_indices,
        bone_map,
        qbones,
        tbones,
    })
}
