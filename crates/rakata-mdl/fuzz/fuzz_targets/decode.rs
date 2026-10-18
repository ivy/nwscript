//! `parse` and `ModelFile::parse` on arbitrary bytes: each returns a model
//! or an error, never panics, and what `parse` returns answers its
//! accessors and every view over its records.
//!
//! The input is an MDL with its MDX after it, split where the MDL's file
//! header says its model data ends.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_mdl::{Controller, Lineage, ModelFile, Node};

fn split(data: &[u8]) -> (&[u8], &[u8]) {
    let size = data
        .get(4..8)
        .map_or(0, |w| u32::from_le_bytes(w.try_into().unwrap()) as usize);
    data.split_at(size.saturating_add(12).min(data.len()))
}

fn controllers(cs: &[Controller<'_>]) {
    for c in cs {
        let _ = (c.len(), c.is_bezier(), c.is_compressed(), c.row_len());
        let _ = (c.time_bytes(), c.value_bytes(), c.times().count());
        for i in 0..c.len() {
            let _ = (c.time(i), c.row(i), c.floats(i).map(Iterator::count));
            let _ = c.orientation(i);
        }
        let _ = c.vector_keys().map(Iterator::count);
        let _ = c.scalar_keys().map(Iterator::count);
        let _ = c.orientation_keys().map(Iterator::count);
    }
}

fn nodes(ns: &[Node<'_>]) {
    for n in ns {
        controllers(&n.controllers);
        if let Some(m) = &n.mesh {
            let _ = m.positions.iter().count();
            let _ = m.normals.map(|a| a.iter().count());
            let _ = (
                m.uv1.map(|a| a.iter().count()),
                m.uv2.map(|a| a.iter().count()),
            );
            let _ = m.faces.iter().count();
            let _ = m.indices.map(|i| i.iter().count());
        }
        if let Some(s) = &n.skin {
            let _ = (s.bone_map_len(), s.bone_map_bytes());
            for b in 0..s.bone_map_len().min(256) as u16 {
                let _ = (s.bone_index(b), s.qbone(b), s.tbone(b));
            }
            let count = n.mesh.as_ref().map_or(0, |m| m.vertex_count);
            for v in 0..=count {
                let _ = s.influences(v);
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let (mdl, mdx) = split(data);
    let _ = rakata_mdl::supermodel_name(mdl);
    let _ = rakata_mdl::layout(mdl, mdx);
    let _ = ModelFile::parse(mdl, mdx);
    if let Ok(m) = rakata_mdl::parse(mdl, mdx) {
        let _ = (
            m.root(),
            m.model_transforms(),
            m.bounds(),
            m.meshes().count(),
        );
        for n in &m.nodes {
            assert!(m.node(n.name).is_some());
        }
        nodes(&m.nodes);
        for a in &m.animations {
            nodes(&a.nodes);
            let _ = (m.animation(a.name), a.node(a.root));
        }
        let models = [m];
        if let Ok(l) = Lineage::new(&models) {
            for a in l.animations() {
                let _ = l.bind(a.1);
            }
        }
    }
});
