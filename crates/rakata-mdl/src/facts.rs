//! What a model contributes to an index: the [`Ref`]s its MDL makes to
//! other resources, read from the MDL alone (no MDX is needed).
//!
//! - The supermodel the model header names; the site is `supermodel`.
//! - Each mesh node's texture and second texture (the lightmap on a
//!   lightmapped mesh); the sites are `NODE.texture` and `NODE.lightmap`.
//!   Every mesh kind is read, the AABB node of a room's walkmesh too (93
//!   of the 1,102 vanilla AABB nodes name a texture).
//! - Each light node's flare textures; the site is `NODE.flare[N]`.
//! - The model a reference node names; the site is `NODE.model`.
//!
//! `NODE` is the node's name. An empty name and `NULL` name nothing. Only
//! the geometry's tree is read: an animation's nodes are dummies in every
//! vanilla model and name nothing. Emitters' names (a texture, a chunk
//! model) are not read.

use std::collections::HashSet;

use rakata_res::Ref;

use crate::error::{Error, Result};
use crate::model::{
    GEOM_ROOT, MESH_LIGHTMAP, MESH_TEXTURE, MODEL_SUPER, NODE_CHILDREN, NODE_HAS_MESH,
    NODE_HEADER_LEN, NODE_NUMBER, model_data, read_names, texture_name,
};

/// The light header's array of flare texture name offsets.
const LIGHT_FLARE_TEXTURES: usize = 52;

/// Every reference the MDL `mdl` makes, in tree order (the supermodel
/// first, then each node in pre-order).
pub fn refs(mdl: &[u8]) -> Result<Vec<Ref>> {
    let d = model_data(mdl)?;
    let names = read_names(&d)?;
    let mut out = Vec::new();
    if let Some(s) = texture_name(d.fixed_str(MODEL_SUPER, "supermodel name")?) {
        out.push(Ref::resref(s, "supermodel"));
    }
    let mut seen = HashSet::new();
    let mut stack = vec![d.u32(GEOM_ROOT, "root")? as usize];
    while let Some(at) = stack.pop() {
        if !seen.insert(at) {
            return Err(Error::BadTree {
                reason: format!("the node at model offset {at} is reached twice"),
            });
        }
        d.slice(at, NODE_HEADER_LEN, "node header")?;
        let flags = d.u16(at)?;
        let number = d.u16(at + NODE_NUMBER)?;
        let node = *names.get(number as usize).ok_or_else(|| Error::BadTree {
            reason: format!(
                "the node at model offset {at} is number {number} and there are {} names",
                names.len()
            ),
        })?;
        let h = at + NODE_HEADER_LEN;
        let mut name = |site: &str, s: &str| {
            if let Some(s) = texture_name(s) {
                out.push(Ref::resref(s, format!("{node}.{site}")));
            }
        };
        if flags & NODE_HAS_MESH != 0 {
            name("texture", d.fixed_str(h + MESH_TEXTURE, "texture name")?);
            name("lightmap", d.fixed_str(h + MESH_LIGHTMAP, "lightmap name")?);
        } else if flags == 0x011 {
            name("model", d.fixed_str(h, "reference model name")?);
        } else if flags == 0x003 {
            let (off, count) = d.array(h + LIGHT_FLARE_TEXTURES, "flare texture array")?;
            d.slice(off, count * 4, "flare texture array")?;
            for i in 0..count {
                let p = d.u32(off + 4 * i, "flare texture pointer")? as usize;
                name(&format!("flare[{i}]"), d.c_str(p, "flare texture")?);
            }
        }
        let (ch_off, ch_count) = d.array(at + NODE_CHILDREN, "child array")?;
        d.slice(ch_off, ch_count * 4, "child array")?;
        for i in (0..ch_count).rev() {
            stack.push(d.u32(ch_off + 4 * i, "child pointer")? as usize);
        }
    }
    Ok(out)
}
