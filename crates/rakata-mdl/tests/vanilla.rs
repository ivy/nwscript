//! The census over the game's own models: every MDL in a KotOR 1 install
//! (the BIFs the KEY names, the capsules under `modules/` and `rims/` with
//! their `dx` partners, `patch.erf`, and loose files in `Override/`) is
//! read with its MDX, and what the layout leaves open is counted: node
//! kinds, classifications, the MDX attribute layouts, the flags, the
//! animations and their controllers by type, the compressed orientation
//! keys, and the skins. A census of the light, emitter and saber headers
//! counts every field of each, confirms their lengths from where the next
//! structure begins, counts the controllers lights and emitters carry, and
//! resolves the names emitters hold. Further tests resolve every
//! supermodel chain, read the four character skeletons (`s_male01`,
//! `s_male02`, `s_female01`, `s_female02`) through theirs, and read every
//! room, door and placeable model the Endar Spire's first area,
//! `end_m01aa`, uses.
//! Run with `--nocapture` to see the counts.
//!
//! Reads the user's own install from `$RAKATA_GAME_K1` (the folder holding
//! `chitin.key`) and skips, naming the variable, when it is unset. No game
//! bytes are committed.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use rakata_2da::TwoDa;
use rakata_archive::{Bif, Erf, Key, Rim};
use rakata_game::{Install, ModuleName, Profile};
use rakata_gff::{Gff, Value};
use rakata_lyt::Lyt;
use rakata_mdl::{
    Body, Chain, Controller, Lineage, MeshKind, Model, ModelFile, Node, NodeKind, Quaternion,
    controller_types, mdx_flags,
};
use rakata_res::{ResId, ResRef, ResType};

const GAME_VAR: &str = "RAKATA_GAME_K1";

fn game() -> Option<PathBuf> {
    match std::env::var(GAME_VAR) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${GAME_VAR} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

/// A child of `dir` named `name` in any case, as the game finds it.
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// Files in `dir` whose extension is one of `exts`, sorted by name.
fn files_with(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
        })
        .collect();
    files.sort();
    files
}

fn file_name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

/// The MDL and MDX members of a capsule, by lower-case resref.
type Members = (BTreeMap<String, Vec<u8>>, BTreeMap<String, Vec<u8>>);

fn capsule_models(path: &Path) -> Members {
    let bytes = fs::read(path).unwrap();
    let mut out: Members = Default::default();
    let mut put = |id: ResId, data: &[u8]| {
        let name = id.name.to_string().to_ascii_lowercase();
        match id.ty {
            ResType::Mdl => {
                out.0.insert(name, data.to_vec());
            }
            ResType::Mdx => {
                out.1.insert(name, data.to_vec());
            }
            _ => {}
        }
    };
    if bytes.starts_with(b"RIM ") {
        for m in Rim::parse(&bytes).unwrap().members() {
            put(m.id, m.data());
        }
    } else {
        for m in Erf::parse(&bytes).unwrap().members() {
            put(m.id, m.data());
        }
    }
    out
}

/// Every MDL in the install with its MDX (empty when none is found), by a
/// name saying where it lives.
fn each_model(root: &Path, mut f: impl FnMut(&str, &[u8], &[u8])) {
    // The BIFs, through the KEY's names.
    let key_path = child_ci(root, "chitin.key").expect("chitin.key");
    let key = Key::parse(&fs::read(&key_path).unwrap()).unwrap();
    let bifs: Vec<(String, Vec<u8>)> = key
        .bifs()
        .iter()
        .map(|bif_ref| {
            let mut path = root.to_path_buf();
            for part in bif_ref.components() {
                path = child_ci(&path, &part).unwrap_or_else(|| panic!("{}", bif_ref.name_str()));
            }
            (file_name(&path), fs::read(&path).unwrap())
        })
        .collect();
    let parsed: Vec<Bif> = bifs.iter().map(|(_, b)| Bif::parse(b).unwrap()).collect();
    let member = |e: &rakata_archive::KeyEntry| -> &[u8] {
        parsed[e.bif as usize]
            .get(e.index as usize)
            .expect("member the KEY names")
    };
    let mdx: HashMap<String, &rakata_archive::KeyEntry> = key
        .entries()
        .iter()
        .filter(|e| e.id.ty == ResType::Mdx)
        .map(|e| (e.id.name.to_string().to_ascii_lowercase(), e))
        .collect();
    for e in key.entries().iter().filter(|e| e.id.ty == ResType::Mdl) {
        let name = e.id.name.to_string().to_ascii_lowercase();
        let x = mdx.get(&name).map(|x| member(x)).unwrap_or(&[]);
        f(
            &format!("{}/{}", bifs[e.bif as usize].0, e.id),
            member(e),
            x,
        );
    }
    // Capsules: a RIM's MDX lives in its `dx` partner; an ERF or MOD keeps
    // both in itself.
    let mut capsules: Vec<PathBuf> = Vec::new();
    if let Some(p) = child_ci(root, "patch.erf") {
        capsules.push(p);
    }
    for dir in ["modules", "rims"] {
        if let Some(dir) = child_ci(root, dir) {
            capsules.extend(files_with(&dir, &["erf", "mod", "rim"]));
        }
    }
    for path in &capsules {
        let name = file_name(path);
        let stem = name.rsplit_once('.').unwrap().0.to_ascii_lowercase();
        if stem.ends_with("dx") && name.to_ascii_lowercase().ends_with(".rim") {
            continue;
        }
        let (mdls, mut mdxs) = capsule_models(path);
        if mdls.is_empty() {
            continue;
        }
        if name.to_ascii_lowercase().ends_with(".rim")
            && let Some(dx) = child_ci(path.parent().unwrap(), &format!("{stem}dx.rim"))
        {
            mdxs.extend(capsule_models(&dx).1);
        }
        for (resref, mdl) in &mdls {
            let x = mdxs.get(resref).map(Vec::as_slice).unwrap_or(&[]);
            f(&format!("{name}/{resref}.mdl"), mdl, x);
        }
    }
    // Loose files.
    if let Some(dir) = child_ci(root, "Override") {
        for path in files_with(&dir, &["mdl"]) {
            let x = path.with_extension("mdx");
            let x = fs::read(&x).unwrap_or_default();
            f(
                &format!("Override/{}", file_name(&path)),
                &fs::read(&path).unwrap(),
                &x,
            );
        }
    }
}

fn kind_name(k: NodeKind) -> String {
    match k {
        NodeKind::Other(f) => format!("other {f:#x}"),
        k => format!("{k:?}"),
    }
}

#[derive(Default)]
struct Census {
    models: usize,
    nodes: usize,
    failed: Vec<String>,
    by_where: BTreeMap<String, usize>,
    kinds: BTreeMap<String, usize>,
    classes: BTreeMap<String, usize>,
    layouts: BTreeMap<String, usize>,
    flags_disagree: Vec<String>,
    index_list_not_faces: Vec<String>,
    no_index_list: BTreeMap<String, usize>,
    render: [usize; 2],
    shadow: [usize; 2],
    lightmapped: usize,
    transparency: BTreeMap<u32, usize>,
    with_supermodel: usize,
    with_animations: usize,
    not_unit: usize,
    supermodels: BTreeSet<String>,
    animations: usize,
    animation_nodes: usize,
    events: BTreeMap<String, usize>,
    controllers: BTreeMap<String, usize>,
    compressed_keys: usize,
    keys_past_length: Vec<String>,
    rest_disagrees: Vec<String>,
    first_orientation_at_rest: [usize; 2],
    first_position_nearer: [usize; 2],
    animation_nodes_unnamed_in_model: usize,
    animation_roots_named_as_model: [usize; 2],
    animation_trees_rooted_at_model_name: [usize; 2],
    animation_node_kinds: BTreeMap<String, usize>,
    fields: BTreeMap<String, usize>,
    skins: usize,
    bones: BTreeMap<usize, usize>,
    weights_not_one: usize,
    bad_influences: Vec<String>,
    bind_disagrees: BTreeMap<String, usize>,
    bind_agrees: usize,
}

fn same_rotation(a: Quaternion, b: Quaternion, tol: f32) -> bool {
    let d = |s: f32| {
        (a.w - s * b.w)
            .abs()
            .max((a.x - s * b.x).abs())
            .max((a.y - s * b.y).abs())
            .max((a.z - s * b.z).abs())
    };
    d(1.0).min(d(-1.0)) < tol
}

fn conj(q: Quaternion) -> Quaternion {
    Quaternion {
        w: q.w,
        x: -q.x,
        y: -q.y,
        z: -q.z,
    }
}

fn controller_label(c: &Controller<'_>) -> String {
    format!(
        "type {} columns {}{}",
        c.kind,
        c.columns,
        if c.is_compressed() {
            " (compressed)"
        } else if c.is_bezier() {
            " (bezier)"
        } else {
            ""
        }
    )
}

impl Census {
    fn add(&mut self, name: &str, mdl: &[u8], mdx: &[u8]) {
        self.models += 1;
        *self
            .by_where
            .entry(name.split('/').next().unwrap().to_owned())
            .or_default() += 1;
        let m = match Model::parse(mdl, mdx) {
            Ok(m) => m,
            Err(e) => return self.failed.push(format!("{name}: {e}")),
        };
        *self
            .classes
            .entry(format!("{:?}", m.classification))
            .or_default() += 1;
        self.with_supermodel += m.supermodel.is_some() as usize;
        self.with_animations += !m.animations.is_empty() as usize;
        if m.supermodel.is_some() {
            let resref = name.rsplit('/').next().unwrap();
            let resref = resref.split('.').next().unwrap().to_ascii_lowercase();
            self.supermodels.insert(resref);
        }
        self.animations_of(name, &m);
        self.skins_of(name, &m);
        for n in &m.nodes {
            self.nodes += 1;
            for c in &n.controllers {
                *self
                    .controllers
                    .entry(format!("geometry {}", controller_label(c)))
                    .or_default() += 1;
                if c.field != 0xFFFF {
                    *self
                        .fields
                        .entry(format!("geometry type {} field {:#x}", c.kind, c.field))
                        .or_default() += 1;
                }
            }
            // The position and orientation controllers repeat the header.
            if let Some(k) = n.controller(controller_types::POSITION) {
                let k = k.vector_keys().unwrap().next().unwrap();
                if k.value != n.position {
                    self.rest_disagrees
                        .push(format!("{name}/{}: position", n.name));
                }
            }
            if let Some(k) = n.controller(controller_types::ORIENTATION) {
                let k = k.orientation(0).unwrap();
                if !same_rotation(k.value, n.orientation, 1e-5) {
                    self.rest_disagrees
                        .push(format!("{name}/{}: orientation", n.name));
                }
            }
            *self.kinds.entry(kind_name(n.kind)).or_default() += 1;
            let q = n.orientation;
            if ((q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z) - 1.0).abs() > 1e-3 {
                self.not_unit += 1;
            }
            let Some(mesh) = &n.mesh else { continue };
            let mut parts = Vec::new();
            for (bit, present, label) in [
                (mdx_flags::POSITION, mesh.stride > 0, "position"),
                (mdx_flags::NORMAL, mesh.normals.is_some(), "normal"),
                (mdx_flags::UV1, mesh.uv1.is_some(), "uv1"),
                (mdx_flags::UV2, mesh.uv2.is_some(), "uv2"),
            ] {
                if present {
                    parts.push(label);
                }
                if mesh.vertex_count > 0 && (mesh.mdx_flags & bit != 0) != present {
                    self.flags_disagree
                        .push(format!("{name}/{}: {label} {:#x}", n.name, mesh.mdx_flags));
                }
            }
            if mesh.mdx_flags & mdx_flags::TANGENT_SPACE != 0 {
                parts.push("tangent space");
            }
            *self
                .layouts
                .entry(format!(
                    "{:#04x} stride {}: {}",
                    mesh.mdx_flags,
                    mesh.stride,
                    if parts.is_empty() {
                        "none".to_owned()
                    } else {
                        parts.join(", ")
                    }
                ))
                .or_default() += 1;
            match &mesh.indices {
                Some(list) => {
                    if !list.iter().eq(mesh.faces.iter().flat_map(|f| f.vertices)) {
                        self.index_list_not_faces.push(format!("{name}/{}", n.name));
                    }
                }
                None => {
                    *self
                        .no_index_list
                        .entry(format!(
                            "{} with {} vertices",
                            kind_name(n.kind),
                            if mesh.vertex_count == 0 { "no" } else { "some" }
                        ))
                        .or_default() += 1
                }
            }
            self.render[mesh.render as usize] += 1;
            self.shadow[mesh.shadow as usize] += 1;
            self.lightmapped += mesh.lightmapped as usize;
            *self.transparency.entry(mesh.transparency_hint).or_default() += 1;
        }
    }
}

impl Census {
    fn animations_of(&mut self, name: &str, m: &Model<'_>) {
        for a in &m.animations {
            self.animations += 1;
            self.animation_roots_named_as_model[a.root.eq_ignore_ascii_case(m.name) as usize] += 1;
            self.animation_trees_rooted_at_model_name
                [a.nodes[0].name.eq_ignore_ascii_case(m.name) as usize] += 1;
            for e in &a.events {
                *self.events.entry(e.name.to_ascii_lowercase()).or_default() += 1;
            }
            for n in &a.nodes {
                self.animation_nodes += 1;
                *self
                    .animation_node_kinds
                    .entry(kind_name(n.kind))
                    .or_default() += 1;
                let rest: Option<&Node<'_>> = m.node(n.name);
                if rest.is_none() {
                    self.animation_nodes_unnamed_in_model += 1;
                }
                for c in &n.controllers {
                    *self
                        .controllers
                        .entry(format!("animation {}", controller_label(c)))
                        .or_default() += 1;
                    *self
                        .fields
                        .entry(format!("animation type {} field {:#x}", c.kind, c.field))
                        .or_default() += 1;
                    if c.times().any(|t| t > a.length + 1e-4) {
                        self.keys_past_length
                            .push(format!("{name}/{}/{}", a.name, n.name));
                    }
                    if c.kind == controller_types::ORIENTATION {
                        let keys: Vec<_> = c.orientation_keys().unwrap().collect();
                        if c.is_compressed() {
                            self.compressed_keys += keys.len();
                            if let (Some(r), Some(k)) = (rest, keys.first()) {
                                self.first_orientation_at_rest
                                    [same_rotation(k.value, r.orientation, 3e-3) as usize] += 1;
                            }
                        }
                    }
                    if c.kind == controller_types::POSITION
                        && let (Some(r), Some(k)) = (rest, c.vector_keys().unwrap().next())
                        && r.position.iter().any(|x| x.abs() > 0.01)
                    {
                        let d0: f32 = k.value.iter().map(|x| x * x).sum();
                        let dr: f32 = k
                            .value
                            .iter()
                            .zip(r.position)
                            .map(|(x, y)| (x - y) * (x - y))
                            .sum();
                        self.first_position_nearer[(d0 < dr) as usize] += 1;
                    }
                }
            }
        }
    }

    fn skins_of(&mut self, name: &str, m: &Model<'_>) {
        let xf = m.model_transforms();
        for (i, n) in m.nodes.iter().enumerate() {
            let Some(skin) = &n.skin else { continue };
            self.skins += 1;
            *self.bones.entry(skin.bones.len()).or_default() += 1;
            let mesh = n.mesh.as_ref().unwrap();
            for v in 0..skin.weights.len() {
                let w = skin.weights.get(v).unwrap();
                if (w.iter().sum::<f32>() - 1.0).abs() > 1e-3 {
                    self.weights_not_one += 1;
                }
                let ix = skin.bone_indices.get(v).unwrap();
                let inf = skin.influences(v).unwrap();
                for k in 0..4 {
                    if (ix[k] >= 0.0) != inf[k].is_some() {
                        self.bad_influences
                            .push(format!("{name}/{} vertex {v}", n.name));
                    }
                }
            }
            assert_eq!(skin.weights.len(), mesh.vertex_count);
            // The bind transforms: from the skin node's coordinates to the
            // bone's, as the geometry's rest pose has them.
            let (mq, mt) = xf[i];
            for &b in &skin.bones {
                let Some(bi) = m.nodes.iter().position(|x| x.number == b) else {
                    *self
                        .bind_disagrees
                        .entry(format!("{name}: bone not in the tree"))
                        .or_default() += 1;
                    continue;
                };
                let (bq, bt) = xf[bi];
                let want_q = conj(bq) * mq;
                let want_t = conj(bq).rotate([mt[0] - bt[0], mt[1] - bt[1], mt[2] - bt[2]]);
                let q = skin.qbone(b).unwrap();
                let t = skin.tbone(b).unwrap();
                let t_ok = t.iter().zip(want_t).all(|(a, b)| (a - b).abs() < 1e-3);
                if same_rotation(q, want_q, 1e-3) && t_ok {
                    self.bind_agrees += 1;
                } else {
                    *self.bind_disagrees.entry(name.to_owned()).or_default() += 1;
                }
            }
        }
    }
}

fn show(label: &str, v: &[String]) {
    println!("{label}: {}", v.len());
    for s in v.iter().take(8) {
        println!("    {s}");
    }
    if v.len() > 8 {
        println!("    ... and {} more", v.len() - 8);
    }
}

fn table<K: std::fmt::Display>(label: &str, m: &BTreeMap<K, usize>) {
    println!("{label}:");
    for (k, n) in m {
        println!("    {k}: {n}");
    }
}

#[test]
fn every_vanilla_model_parses() {
    let Some(root) = game() else { return };
    let mut c = Census::default();
    each_model(&root, |name, mdl, mdx| c.add(name, mdl, mdx));

    println!("{} models, {} nodes", c.models, c.nodes);
    table("by location", &c.by_where);
    table("classification", &c.classes);
    table("node kinds", &c.kinds);
    table(
        "MDX layouts (flags, stride: attributes) per mesh",
        &c.layouts,
    );
    table("meshes without an index list", &c.no_index_list);
    table("transparency hint per mesh", &c.transparency);
    println!(
        "render: {} on, {} off; shadow: {} on, {} off; lightmapped: {}",
        c.render[1], c.render[0], c.shadow[1], c.shadow[0], c.lightmapped
    );
    println!(
        "with a supermodel: {}; with animations: {}",
        c.with_supermodel, c.with_animations
    );
    show("MDX flags disagreeing with the offsets", &c.flags_disagree);
    show(
        "index lists that are not the faces' vertices",
        &c.index_list_not_faces,
    );
    println!(
        "animations: {}, with {} nodes ({} named by no node of their own model)",
        c.animations, c.animation_nodes, c.animation_nodes_unnamed_in_model
    );
    table(
        "controllers (where, type, columns) per controller",
        &c.controllers,
    );
    table("animation node kinds", &c.animation_node_kinds);
    println!(
        "animation root field naming the model: {} of {}; animation trees whose root is named as the model: {} of {}",
        c.animation_roots_named_as_model[1],
        c.animations,
        c.animation_trees_rooted_at_model_name[1],
        c.animations
    );
    let fields: BTreeMap<String, usize> = c
        .fields
        .iter()
        .filter(|(k, _)| {
            k.contains("type 8 ") || k.contains("type 20 ") || k.starts_with("geometry")
        })
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    table(
        "the u16 after the controller type, position and orientation (and any not 0xFFFF in geometry)",
        &fields,
    );
    println!(
        "other animation controllers with the u16 0xFFFF: {} of {}",
        c.fields
            .iter()
            .filter(|(k, _)| k.ends_with("0xffff")
                && !k.contains("type 8 ")
                && !k.contains("type 20 "))
            .map(|(_, v)| v)
            .sum::<usize>(),
        c.fields
            .iter()
            .filter(|(k, _)| k.starts_with("animation")
                && !k.contains("type 8 ")
                && !k.contains("type 20 "))
            .map(|(_, v)| v)
            .sum::<usize>()
    );
    println!("compressed orientation keys: {}", c.compressed_keys);
    println!(
        "first compressed key within 0.003 of the node's rest orientation: {} of {}",
        c.first_orientation_at_rest[1],
        c.first_orientation_at_rest[0] + c.first_orientation_at_rest[1]
    );
    println!(
        "animation position controllers whose first key is nearer the origin than the rest position: {} of {}",
        c.first_position_nearer[1],
        c.first_position_nearer[0] + c.first_position_nearer[1]
    );
    let top_events: Vec<String> = {
        let mut v: Vec<_> = c.events.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1));
        v.iter().take(8).map(|(k, n)| format!("{k} {n}")).collect()
    };
    println!(
        "events: {} ({} names); most: {}",
        c.events.values().sum::<usize>(),
        c.events.len(),
        top_events.join(", ")
    );
    println!(
        "skins: {}; weights not summing to one: {}; bind transforms agreeing with the rest pose: {}",
        c.skins, c.weights_not_one, c.bind_agrees
    );
    table("skins by bone count", &c.bones);
    table("bind transforms disagreeing, by model", &c.bind_disagrees);
    show(
        "geometry controllers disagreeing with the node header",
        &c.rest_disagrees,
    );
    show("keys past their animation's length", &c.keys_past_length);
    show(
        "vertex slots whose bone the bone map lacks",
        &c.bad_influences,
    );
    show("failed", &c.failed);

    assert!(c.models > 3000, "only {} models found", c.models);
    assert!(c.rest_disagrees.is_empty());
    assert!(c.keys_past_length.is_empty());
    assert!(c.bad_influences.is_empty());
    assert_eq!(c.weights_not_one, 0);
    // The main menu's model is posed away from its bind pose; in four
    // others eight bones of one mesh hold transforms that fit other bones.
    let odd = ["mainmenu_model", "pfbcl", "pfbcm", "pfbcs", "p_bastilabb"];
    assert!(
        c.bind_disagrees
            .keys()
            .all(|k| odd.iter().any(|o| k.ends_with(&format!("/{o}.mdl")))),
        "{:?}",
        c.bind_disagrees
    );
    assert!(c.failed.is_empty(), "models refused: {:#?}", c.failed);
    assert!(c.flags_disagree.is_empty());
    assert!(c.index_list_not_faces.is_empty());
    assert_eq!(c.not_unit, 0);
}

/// Every vanilla model, read as a [`ModelFile`] and written again, gives
/// back its MDL and its MDX byte for byte.
#[test]
fn every_vanilla_model_writes_back_byte_for_byte() {
    let Some(root) = game() else { return };
    let (mut models, mut failed) = (0, Vec::new());
    each_model(&root, |name, mdl, mdx| {
        models += 1;
        let outcome = ModelFile::parse(mdl, mdx).and_then(|m| m.to_bytes());
        match outcome {
            Ok((w_mdl, w_mdx)) if w_mdl == mdl && w_mdx == mdx => {}
            Ok((w_mdl, w_mdx)) => {
                let first = |a: &[u8], b: &[u8]| {
                    a.iter()
                        .zip(b)
                        .position(|(x, y)| x != y)
                        .unwrap_or(a.len().min(b.len()))
                };
                failed.push(format!(
                    "{name}: MDL {} read, {} written, first difference at {}; MDX {} read, {} written, first difference at {}",
                    mdl.len(),
                    w_mdl.len(),
                    first(mdl, &w_mdl),
                    mdx.len(),
                    w_mdx.len(),
                    first(mdx, &w_mdx)
                ));
            }
            Err(e) => failed.push(format!("{name}: {e}")),
        }
    });
    println!("{models} models written, {} not identical", failed.len());
    show("not identical", &failed);
    assert!(failed.is_empty(), "{} models differ", failed.len());
}

fn install() -> Option<Install> {
    game().map(|root| Install::open(root, Profile::K1).unwrap())
}

fn read(install: &Install, module: &ModuleName, name: &str, ty: ResType) -> Option<Vec<u8>> {
    let id = ResId::new(ResRef::new(name).unwrap(), ty);
    install.read(id, Some(module)).unwrap().map(|(_, b)| b)
}

fn table_2da(install: &Install, name: &str) -> TwoDa {
    let id = ResId::new(ResRef::new(name).unwrap(), ResType::TwoDa);
    TwoDa::parse(&install.read(id, None).unwrap().unwrap().1).unwrap()
}

fn resref(s: &rakata_gff::Struct, label: &str) -> String {
    match s.get(label) {
        Some(Value::ResRef(r)) => r.as_str().unwrap().to_owned(),
        v => panic!("{label}: {v:?}"),
    }
}

fn int(s: &rakata_gff::Struct, label: &str) -> usize {
    match s.get(label) {
        Some(Value::Byte(v)) => *v as usize,
        Some(Value::Word(v)) => *v as usize,
        Some(Value::Dword(v)) => *v as usize,
        Some(Value::Int(v)) => *v as usize,
        v => panic!("{label}: {v:?}"),
    }
}

/// The models a module's area draws: its layout's rooms, and the models
/// of the doors and placeables its GIT places, through their blueprints'
/// appearance rows.
fn module_models(
    install: &Install,
    module: &str,
    area: &str,
) -> BTreeMap<String, BTreeSet<String>> {
    let m = ModuleName::new(module);
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let lyt = Lyt::parse(&read(install, &m, area, ResType::Lyt).unwrap()).unwrap();
    for r in &lyt.rooms {
        out.entry("room".into())
            .or_default()
            .insert(r.model.to_ascii_lowercase());
    }
    let git = Gff::parse(&read(install, &m, area, ResType::Git).unwrap()).unwrap();
    let doors = table_2da(install, "genericdoors");
    let placeables = table_2da(install, "placeables");
    for (list, ty, field, table, kind) in [
        ("Door List", ResType::Utd, "GenericType", &doors, "door"),
        (
            "Placeable List",
            ResType::Utp,
            "Appearance",
            &placeables,
            "placeable",
        ),
    ] {
        for inst in git.root.get(list).and_then(Value::as_list).unwrap_or(&[]) {
            let template = resref(inst, "TemplateResRef");
            let blueprint = Gff::parse(&read(install, &m, &template, ty).unwrap()).unwrap();
            let row = int(&blueprint.root, field);
            let model = table.get(row, "modelname").unwrap().to_ascii_lowercase();
            out.entry(kind.into()).or_default().insert(model);
        }
    }
    out
}

#[test]
fn every_model_of_the_endar_spire_first_area_parses() {
    let Some(install) = install() else { return };
    let m = ModuleName::new("end_m01aa");
    let used = module_models(&install, "end_m01aa", "m01aa");
    let mut failed = Vec::new();
    let mut missing_lightmaps = BTreeSet::new();
    for (kind, models) in &used {
        println!("{kind}: {} models", models.len());
        for name in models {
            let mdl = read(&install, &m, name, ResType::Mdl);
            let mdx = read(&install, &m, name, ResType::Mdx).unwrap_or_default();
            let Some(mdl) = mdl else {
                failed.push(format!("{kind} {name}: no MDL"));
                continue;
            };
            let model = match Model::parse(&mdl, &mdx) {
                Ok(model) => model,
                Err(e) => {
                    failed.push(format!("{kind} {name}: {e}"));
                    continue;
                }
            };
            let meshes: Vec<_> = model.meshes().filter(|(_, m)| m.render).collect();
            let (verts, tris) = meshes.iter().fold((0, 0), |(v, t), (_, m)| {
                (v + m.vertex_count, t + m.faces.len())
            });
            let textures: BTreeSet<&str> = meshes.iter().filter_map(|(_, m)| m.texture).collect();
            let lightmaps: BTreeSet<&str> = meshes.iter().filter_map(|(_, m)| m.lightmap).collect();
            for lm in &lightmaps {
                let id = ResId::new(ResRef::new(lm).unwrap(), ResType::Tga);
                if install.read(id, None).unwrap().is_none() {
                    missing_lightmaps.insert(lm.to_string());
                }
            }
            let b = model.bounds();
            println!(
                "    {name}: {:?}, {} nodes, {} rendered meshes, {verts} vertices, {tris} faces, {} textures, {} lightmaps, bounds {:?}",
                model.classification,
                model.nodes.len(),
                meshes.len(),
                textures.len(),
                lightmaps.len(),
                b.map(|b| (b.min, b.max))
            );
        }
    }
    show("failed", &failed);
    let missing: Vec<String> = missing_lightmaps.into_iter().collect();
    // Two names in vanilla (`M01aa_04a_a0002t`, `DOR_LHR02_a00004`) name a
    // lightmap the install does not ship.
    show("lightmap names no TGA answers", &missing);
    assert!(used.get("room").is_some_and(|r| !r.is_empty()));
    assert!(failed.is_empty(), "{failed:#?}");
}

fn load_model(install: &Install, name: &str) -> rakata_game::Result<Option<(Vec<u8>, Vec<u8>)>> {
    let Ok(r) = ResRef::new(name) else {
        return Ok(None);
    };
    let Some((_, mdl)) = install.read(ResId::new(r, ResType::Mdl), None)? else {
        return Ok(None);
    };
    let mdx = install
        .read(ResId::new(r, ResType::Mdx), None)?
        .map(|(_, b)| b)
        .unwrap_or_default();
    Ok(Some((mdl, mdx)))
}

#[test]
fn every_supermodel_chain_resolves() {
    let Some(root) = game() else { return };
    let mut supered = BTreeSet::new();
    each_model(&root, |name, mdl, _| {
        if rakata_mdl::supermodel_name(mdl).unwrap().is_some() {
            let resref = name.rsplit('/').next().unwrap();
            supered.insert(resref.split('.').next().unwrap().to_ascii_lowercase());
        }
    });
    let install = Install::open(&root, Profile::K1).unwrap();
    let mut depths: BTreeMap<usize, usize> = BTreeMap::new();
    let mut missing = Vec::new();
    let mut failed = Vec::new();
    let mut unbound = 0usize;
    let mut inherited = 0usize;
    for name in &supered {
        let chain = match Chain::load(name, |n| load_model(&install, n)) {
            Ok(c) => c,
            Err(e) => {
                failed.push(format!("{name}: {e}"));
                continue;
            }
        };
        if let Some(m) = &chain.missing {
            missing.push(format!("{name}: supermodel {m}"));
        }
        let models = match chain.parse() {
            Ok(m) => m,
            Err(e) => {
                failed.push(format!("{name}: {e}"));
                continue;
            }
        };
        let lineage = match Lineage::new(&models) {
            Ok(l) => l,
            Err(e) => {
                failed.push(format!("{name}: {e}"));
                continue;
            }
        };
        *depths.entry(models.len() - 1).or_default() += 1;
        for (from, a) in lineage.animations() {
            inherited += (from > 0) as usize;
            unbound += lineage.bind(a).iter().filter(|b| b.is_none()).count();
        }
    }
    println!(
        "{} models with a supermodel (by resref, as the install resolves them)",
        supered.len()
    );
    table("supermodels above each (chain depth)", &depths);
    println!(
        "animations played from a supermodel: {inherited}; their nodes naming no node of the model: {unbound}"
    );
    show("chains ending at a supermodel the install lacks", &missing);
    show("failed", &failed);
    assert!(failed.is_empty(), "{failed:#?}");
}

#[test]
fn the_character_skeletons_resolve_through_their_supermodels() {
    let Some(install) = install() else { return };
    for (name, depth) in [
        ("s_male01", 0),
        ("s_male02", 1),
        ("s_female01", 2),
        ("s_female02", 3),
    ] {
        let chain = Chain::load(name, |n| load_model(&install, n)).unwrap();
        assert_eq!(chain.missing, None);
        let models = chain.parse().unwrap();
        let lineage = Lineage::new(&models).unwrap();
        let names: Vec<&str> = models.iter().map(|m| m.name).collect();
        assert_eq!(names.len(), depth + 1, "{names:?}");
        let m = &models[0];
        let animations = lineage.animations();
        let mut keys = [0usize; 3];
        for (_, a) in &animations {
            for n in &a.nodes {
                for c in &n.controllers {
                    if let Some(k) = c.orientation_keys() {
                        keys[c.is_compressed() as usize] += k.len();
                    } else if c.kind == controller_types::POSITION {
                        keys[2] += c.vector_keys().unwrap().len();
                    }
                }
            }
        }
        println!(
            "{name}: chain {}; {} nodes ({} with a mesh); {} animations of its own, {} played; orientation keys {} plain, {} compressed; position keys {}",
            names.join(" -> "),
            m.nodes.len(),
            m.meshes().count(),
            m.animations.len(),
            animations.len(),
            keys[0],
            keys[1],
            keys[2]
        );
        // `s_male01` holds twelve combat animations; the rest come down
        // from `s_male02`, which holds the walk.
        let name = if depth == 0 { "c2d1" } else { "walk" };
        let (from, a) = lineage.animation(name).expect(name);
        assert_eq!(from, depth.saturating_sub(1));
        let bound = lineage.bind(a);
        let unbound = bound.iter().filter(|b| b.is_none()).count();
        println!(
            "    {name} from {}: {} nodes, {unbound} the model lacks",
            models[from].name,
            a.nodes.len()
        );
        assert!(bound.iter().filter(|b| b.is_some()).count() > 10);
    }

    // A player body skinned to the skeleton it inherits.
    let chain = Chain::load("pmbam", |n| load_model(&install, n)).unwrap();
    let models = chain.parse().unwrap();
    let lineage = Lineage::new(&models).unwrap();
    let names: Vec<&str> = models.iter().map(|m| m.name).collect();
    assert_eq!(
        names[1..],
        ["S_Female02", "S_Female01", "S_Male02", "S_Male01"]
    );
    let body = &models[0];
    let skins: Vec<_> = body
        .nodes
        .iter()
        .filter_map(|n| n.skin.as_ref().map(|s| (n.name, s)))
        .collect();
    for (name, skin) in &skins {
        println!("    pmbam {name}: {} bones", skin.bones.len());
        for &b in &skin.bones {
            assert!(body.nodes.iter().any(|n| n.number == b), "{name} bone {b}");
        }
    }
    assert!(!skins.is_empty());
    let (from, walk) = lineage.animation("walk").unwrap();
    assert_eq!(models[from].name, "S_Male02");
    let bound = lineage.bind(walk);
    let lacks: Vec<&str> = walk
        .nodes
        .iter()
        .zip(&bound)
        .filter(|(_, b)| b.is_none())
        .map(|(n, _)| n.name)
        .collect();
    println!(
        "    pmbam plays walk from S_Male02: {} of its {} nodes bound; the body lacks {lacks:?}",
        bound.len() - lacks.len(),
        bound.len()
    );
    // The head and face bones are the head model's.
    assert!(
        lacks
            .iter()
            .all(|n| !n.contains("thigh") && !n.contains("bicep"))
    );
}

/// A NUL-ended name in a fixed field, and whether any byte after its NUL
/// is not zero.
fn field_name(b: &[u8]) -> (String, bool) {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let rest = b.get(end + 1..).unwrap_or(&[]);
    (
        String::from_utf8_lossy(&b[..end]).into_owned(),
        rest.iter().any(|&c| c != 0),
    )
}

fn word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn float(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// A controller record's type and column byte, and the first value of
/// its first row.
fn controller_head(n: &rakata_mdl::NodeFile, c: &[u8; 16]) -> (u32, u8, f32) {
    let first = u16::from_le_bytes([c[10], c[11]]) as usize;
    (word(c, 0), c[12], float(&n.controller_data, 4 * first))
}

/// Where the first structure after a node's own headers begins, from the
/// raw model data: the node at `at` (an offset in the model data) is
/// followed by its kind's header of `len` bytes, then the first array
/// any of them points to.
fn after_headers(d: &[u8], at: usize, flags: u16) -> (usize, usize) {
    let h = at + 80;
    match flags {
        // A light's arrays, then its children's array.
        0x003 => {
            let arrays = [4, 16, 28, 40, 52].map(|o| word(d, h + o) as usize);
            let next = arrays
                .iter()
                .copied()
                .filter(|&o| o != 0)
                .chain([word(d, at + 0x2C) as usize])
                .min()
                .unwrap();
            (h + rakata_mdl::LIGHT_HEADER_LEN, next)
        }
        // An emitter points nowhere: its children's array follows.
        0x005 => (
            h + rakata_mdl::EMITTER_HEADER_LEN,
            word(d, at + 0x2C) as usize,
        ),
        // A saber's header follows the mesh header; its first array, at
        // 0, follows it.
        0x821 => {
            let s = h + rakata_mdl::MESH_HEADER_LEN;
            (s + rakata_mdl::SABER_HEADER_LEN, word(d, s) as usize)
        }
        _ => unreachable!(),
    }
}

/// The geometry's nodes of one model, raw: (offset, flags) for every node
/// the tree reaches.
fn raw_nodes(mdl: &[u8]) -> Vec<(usize, u16)> {
    let d = &mdl[12..];
    let mut out = Vec::new();
    let mut stack = vec![word(d, 0x28) as usize];
    while let Some(at) = stack.pop() {
        out.push((at, u16::from_le_bytes([d[at], d[at + 1]])));
        let (off, n) = (word(d, at + 0x2C) as usize, word(d, at + 0x30) as usize);
        stack.extend((0..n).map(|i| word(d, off + 4 * i) as usize));
    }
    out
}

/// What resolves a name in the install: the first of `types` that does,
/// outside the modules or else in some module's capsules (`in_modules`).
fn held_as(
    install: &Install,
    in_modules: &BTreeSet<(String, ResType)>,
    name: &str,
    types: &[ResType],
) -> &'static str {
    let Ok(r) = ResRef::new(name) else {
        return "nothing";
    };
    for &ty in types {
        let res = install.resolve(ResId::new(r, ty), None).unwrap();
        if res.winner().is_some() {
            return match ty {
                ResType::Tpc => "TPC",
                ResType::Tga => "TGA",
                ResType::Mdl => "MDL",
                _ => "other",
            };
        }
    }
    if types
        .iter()
        .any(|&ty| in_modules.contains(&(name.to_ascii_lowercase(), ty)))
    {
        return "a module's capsule only";
    }
    "nothing"
}

/// The census of the light, emitter and saber headers (docs/formats/mdl.md,
/// "Light header", "Emitter header", "Saber header"), and of the
/// controllers lights and emitters carry: every byte of each header is
/// counted over every vanilla model, the headers' lengths are confirmed by
/// where the next structure begins, and the names emitters hold are
/// resolved through the install.
#[test]
fn the_light_emitter_and_saber_headers() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let Some(root) = game() else { return };
    let install = Install::open(&root, Profile::K1).unwrap();
    let in_modules: BTreeSet<(String, ResType)> = install
        .modules()
        .iter()
        .flat_map(|m| install.module_ids(m).unwrap())
        .map(|id| (id.name.to_string().to_ascii_lowercase(), id.ty))
        .collect();

    let mut light_words: BTreeMap<String, usize> = BTreeMap::new();
    let mut emitter_fields: BTreeMap<String, usize> = BTreeMap::new();
    let mut saber_fields: BTreeMap<String, usize> = BTreeMap::new();
    let mut controllers: BTreeMap<String, usize> = BTreeMap::new();
    let mut checks: BTreeMap<String, usize> = BTreeMap::new();
    let mut namings: BTreeMap<String, usize> = BTreeMap::new();
    let mut names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut contents = BTreeSet::new();
    let mut failed = Vec::new();
    let (mut lights, mut emitters, mut sabers) = (0, 0, 0);

    each_model(&root, |label, mdl, mdx| {
        let m = ModelFile::parse(mdl, mdx).unwrap();
        let d = &mdl[12..];
        let mut h = DefaultHasher::new();
        mdl.hash(&mut h);
        let distinct = contents.insert(h.finish());

        // The header lengths, from where the next structure begins.
        for (at, flags) in raw_nodes(mdl) {
            if matches!(flags, 0x003 | 0x005 | 0x821) {
                let (end, next) = after_headers(d, at, flags);
                let ok = if end == next {
                    "follows"
                } else {
                    "does not follow"
                };
                *checks
                    .entry(format!("{flags:#05x}: next structure {ok} the header"))
                    .or_default() += 1;
                if end != next {
                    failed.push(format!(
                        "{label}: node at {at}: header ends {end}, next {next}"
                    ));
                }
            }
        }

        let kind_of: HashMap<u16, u16> =
            m.nodes().iter().map(|n| (n.number(), n.flags())).collect();
        let count = |map: &mut BTreeMap<String, usize>, k: String| *map.entry(k).or_default() += 1;

        for n in m.nodes() {
            for c in &n.controllers {
                let (ty, cols, _) = controller_head(n, c);
                let kind = match n.flags() {
                    0x003 => "light",
                    0x005 => "emitter",
                    _ => continue,
                };
                count(
                    &mut controllers,
                    format!("{kind} {ty:3} cols {cols:2} geometry"),
                );
            }
            match &n.body {
                Body::Light(l) => {
                    lights += 1;
                    let b = &l.header;
                    let flares = l.flare_textures.len();
                    count(
                        &mut light_words,
                        format!(
                            "0 flare radius > 0: {}, flares: {}",
                            float(b, 0) > 0.0,
                            flares
                        ),
                    );
                    for (o, what) in [
                        (64, "64"),
                        (68, "68"),
                        (72, "72"),
                        (76, "76"),
                        (80, "80"),
                        (84, "84"),
                        (88, "88"),
                    ] {
                        count(&mut light_words, format!("{what}: {}", word(b, o) as i32));
                    }
                    let same = l.flare_sizes.len() / 4 == flares
                        && l.flare_positions.len() / 4 == flares
                        && l.flare_colors.len() / 12 == flares;
                    count(
                        &mut light_words,
                        format!("flare arrays of one count: {same}"),
                    );
                    count(
                        &mut light_words,
                        format!("array at 4 empty: {}", l.unknown.is_empty()),
                    );
                }
                Body::Emitter(e) => {
                    emitters += 1;
                    let b = &e[..];
                    for (o, what) in [(0, "0 f32"), (4, "4 f32"), (8, "8 f32")] {
                        count(&mut emitter_fields, format!("{what}: {}", float(b, o)));
                    }
                    let update = field_name(&b[32..64]).0;
                    for (o, what) in [(12, "12"), (16, "16"), (20, "20"), (24, "24"), (28, "28")] {
                        count(
                            &mut emitter_fields,
                            format!("{what}: {} ({update})", word(b, o)),
                        );
                    }
                    for (o, len, what) in [
                        (32, 32, "32 update"),
                        (64, 32, "64 render"),
                        (96, 32, "96 blend"),
                        (128, 32, "128 texture"),
                        (160, 16, "160 chunk"),
                        (187, 32, "187 name"),
                    ] {
                        let (s, junk) = field_name(&b[o..o + len]);
                        count(&mut emitter_fields, format!("{what} longest {}", s.len()));
                        count(
                            &mut emitter_fields,
                            format!("{what} bytes after the NUL not zero: {junk}"),
                        );
                        if o != 128 || !s.is_empty() {
                            names
                                .entry(what.to_string())
                                .or_default()
                                .insert(s.to_ascii_lowercase());
                        }
                        if matches!(o, 32 | 64 | 96 | 187) {
                            count(&mut emitter_fields, format!("{what}: {s}"));
                        }
                    }
                    count(&mut emitter_fields, format!("176: {}", word(b, 176)));
                    count(&mut emitter_fields, format!("180: {}", word(b, 180)));
                    count(
                        &mut emitter_fields,
                        format!("184 u16: {}", u16::from_le_bytes([b[184], b[185]])),
                    );
                    count(&mut emitter_fields, format!("186: {}", b[186]));
                    count(&mut emitter_fields, format!("219 zero: {}", b[219] == 0));
                    let flags = word(b, 220);
                    for bit in 0..32 {
                        if flags >> bit & 1 != 0 {
                            count(&mut emitter_fields, format!("220 bit {bit:2}"));
                        }
                    }
                    // The controllers the header's fields go with.
                    let value = |ty: u32| {
                        n.controllers
                            .iter()
                            .map(|c| controller_head(n, c))
                            .find(|(t, _, _)| *t == ty)
                            .map(|(_, _, v)| v)
                    };
                    let extended = value(284).is_some();
                    let named = !field_name(&b[187..219]).0.is_empty();
                    count(
                        &mut checks,
                        format!("emitter: controller 284 {extended}, name at 187 {named}"),
                    );
                    if let (Some(start), Some(end)) = (value(112), value(108)) {
                        count(
                            &mut checks,
                            format!(
                                "emitter: frame start (112) <= frame end (108): {}",
                                start <= end
                            ),
                        );
                        let grid = word(b, 20) * word(b, 24);
                        count(
                            &mut checks,
                            format!(
                                "emitter: frame end within the grid: {}",
                                (end as u32) < grid.max(1)
                            ),
                        );
                    }
                    for (ty, needs) in [(184, "Lightning"), (188, "Lightning"), (192, "Lightning")]
                    {
                        if value(ty).is_some_and(|v| v != 0.0) {
                            count(
                                &mut checks,
                                format!(
                                    "emitter: {ty} non-zero on update {update} (expected {needs})"
                                ),
                            );
                        }
                    }
                    for ty in [128, 132] {
                        if value(ty).is_some_and(|v| v != 0.0) {
                            count(
                                &mut checks,
                                format!(
                                    "emitter: {ty} non-zero with flag bit 0: {}",
                                    flags & 1 != 0
                                ),
                            );
                        }
                    }
                    for ty in [124, 140, 168] {
                        if value(ty).is_some_and(|v| v != 0.0) {
                            count(
                                &mut checks,
                                format!("emitter: {ty} non-zero on update {update}"),
                            );
                        }
                    }
                    if distinct {
                        let tex = field_name(&b[128..160]).0;
                        let chunk = field_name(&b[160..176]).0;
                        let depth = field_name(&b[187..219]).0;
                        for (site, s, types) in [
                            ("emitter texture", tex, &[ResType::Tpc, ResType::Tga][..]),
                            ("emitter chunk", chunk, &[ResType::Mdl][..]),
                            (
                                "emitter name at 187",
                                depth,
                                &[ResType::Tpc, ResType::Tga][..],
                            ),
                        ] {
                            if s.is_empty() || s.eq_ignore_ascii_case("NULL") {
                                continue;
                            }
                            let held = held_as(&install, &in_modules, &s, types);
                            count(&mut namings, format!("{site}: held as {held}"));
                            names
                                .entry(format!("{site} held as {held}"))
                                .or_default()
                                .insert(s.to_ascii_lowercase());
                        }
                    }
                }
                Body::Mesh(me) => {
                    if let MeshKind::Saber(s) = &me.kind {
                        sabers += 1;
                        let b = &s.header;
                        count(
                            &mut saber_fields,
                            format!("vertices: {}", me.vertex_count()),
                        );
                        count(&mut saber_fields, format!("faces: {}", me.faces.len() / 32));
                        let (w12, w16) = (word(b, 12), word(b, 16));
                        count(
                            &mut saber_fields,
                            format!("words 12, 16 differ: {}", w12 != w16),
                        );
                        count(
                            &mut saber_fields,
                            format!(
                                "word 12 names a node number: {}",
                                kind_of.contains_key(&(w12 as u16))
                            ),
                        );
                        count(
                            &mut saber_fields,
                            format!(
                                "word 12 range: {}",
                                if (86..=98).contains(&w12) {
                                    "86..98"
                                } else {
                                    "other"
                                }
                            ),
                        );
                        count(
                            &mut saber_fields,
                            format!(
                                "word 16 range: {}",
                                if (86..=98).contains(&w16) {
                                    "86..98"
                                } else {
                                    "other"
                                }
                            ),
                        );
                        count(
                            &mut saber_fields,
                            format!("array 0 is the positions: {}", s.arrays[0] == me.positions),
                        );
                        let unit = s.arrays[2].chunks(12).all(|v| {
                            let l: f32 = (0..3).map(|i| float(v, 4 * i).powi(2)).sum();
                            (l.sqrt() - 1.0).abs() < 1e-3
                        });
                        count(&mut saber_fields, format!("array 8 unit vectors: {unit}"));
                        let in01 = s.arrays[1]
                            .chunks(4)
                            .all(|v| (0.0..=1.0).contains(&float(v, 0)));
                        count(&mut saber_fields, format!("array 4 in 0..1: {in01}"));
                    }
                }
                _ => {}
            }
        }
        // Controllers of an animation's nodes, by the kind of the geometry
        // node of the same number.
        for a in &m.animations {
            let mut stack = vec![&a.root];
            while let Some(n) = stack.pop() {
                stack.extend(n.children.iter());
                let kind = match kind_of.get(&n.number()) {
                    Some(0x003) => "light",
                    Some(0x005) => "emitter",
                    _ => continue,
                };
                for c in &n.controllers {
                    let (ty, cols, _) = controller_head(n, c);
                    count(
                        &mut controllers,
                        format!("{kind} {ty:3} cols {cols:2} animations"),
                    );
                }
            }
        }
    });

    println!(
        "{lights} lights, {emitters} emitters, {sabers} sabers, {} distinct MDL contents",
        contents.len()
    );
    table("header lengths and other checks", &checks);
    table("light header", &light_words);
    table("emitter header", &emitter_fields);
    table("saber header", &saber_fields);
    table("light and emitter controllers", &controllers);
    table(
        "names emitters hold, one per node and site over distinct contents",
        &namings,
    );
    for (k, v) in &names {
        show(k, &v.iter().cloned().collect::<Vec<_>>());
    }
    assert!(failed.is_empty(), "{failed:#?}");
    assert_eq!((lights, emitters, sabers), (5_599, 9_442, 68));
}

/// Every light, emitter and saber of the install read as typed values,
/// with values `docs/formats/mdl.md` states; and an animation node that
/// keys neither position nor orientation kept with its keys.
#[test]
fn every_light_emitter_and_saber_reads_as_the_spec_says() {
    use rakata_mdl::controller_types::{self, emitter, light, mesh};
    use rakata_mdl::emitter_flags;

    let Some(root) = game() else { return };
    let (mut lights, mut emitters, mut sabers) = (0, 0, 0);
    each_model(&root, |label, mdl, mdx| {
        let m = Model::parse(mdl, mdx).unwrap_or_else(|e| panic!("{label}: {e}"));
        for n in &m.nodes {
            assert_eq!(n.light.is_some(), n.kind == NodeKind::Light, "{label}");
            assert_eq!(n.emitter.is_some(), n.kind == NodeKind::Emitter, "{label}");
            assert_eq!(n.saber.is_some(), n.kind == NodeKind::Saber, "{label}");
            if let Some(l) = &n.light {
                lights += 1;
                assert_eq!(l.flare_radius > 0.0, !l.flares.is_empty(), "{label}");
                assert!(!l.generate_flare, "{label}");
            }
            if let Some(e) = &n.emitter {
                emitters += 1;
                assert!(
                    ["Single", "Fountain", "Explosion", "Lightning"].contains(&e.update),
                    "{label}: {}",
                    e.update
                );
                // The mid-life controllers come with the name at 187.
                assert_eq!(
                    n.controller(emitter::COLOR_MID).is_some(),
                    !e.name_187.is_empty(),
                    "{label}"
                );
            }
            if let Some(s) = &n.saber {
                sabers += 1;
                let mesh = n.mesh.as_ref().unwrap();
                assert_eq!(s.positions.len(), 176);
                assert!(s.positions.iter().eq(mesh.positions.iter()), "{label}");
                assert_eq!(mesh.faces.len(), 12);
                assert_ne!(s.words[0], s.words[1]);
                // The arrangement docs/formats/mdl.md records: 44 columns of
                // four rows; the columns at 0 and 88 the centre line, at 4
                // and 92 the right and left edges, the rest the centre
                // line again; one normal; the stored faces sequential.
                let p: Vec<[f32; 3]> = s.positions.iter().collect();
                let column = |k: usize| &p[4 * k..4 * k + 4];
                let near = |a: &[[f32; 3]], b: &[[f32; 3]]| {
                    a.iter()
                        .zip(b)
                        .all(|(x, y)| (0..3).all(|i| (x[i] - y[i]).abs() < 1e-4))
                };
                let apart: Vec<usize> = (0..44).filter(|&k| !near(column(k), column(0))).collect();
                assert_eq!(apart, [1, 23], "{label}");
                assert!(column(0).windows(2).all(|r| r[0][2] < r[1][2]), "{label}");
                let u = |v: usize| s.uv.get(v).unwrap()[0];
                assert_eq!(
                    [u(0), u(4), u(88), u(92)].map(|x| (x * 2.0).round() / 2.0),
                    [0.5, 1.0, 0.5, 0.0]
                );
                let n0 = s.normals.get(0).unwrap();
                assert!(
                    s.normals
                        .iter()
                        .all(|n| (0..3).all(|i| (n[i] - n0[i]).abs() < 1e-2))
                );
                let stored: Vec<[u16; 3]> = mesh.faces.iter().map(|f| f.vertices).collect();
                let mut seq: Vec<[u16; 3]> =
                    (0..11).map(|i| [3 * i, 3 * i + 1, 3 * i + 2]).collect();
                seq.push([33, 32, 31]);
                assert_eq!(stored, seq, "{label}");
                // The derived triangles face the stored normal, as a
                // mesh's faces do theirs.
                for [a, b, c] in s.blade_triangles() {
                    let (a, b, c) = (p[a as usize], p[b as usize], p[c as usize]);
                    let (e, f) = (
                        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
                    );
                    let x = [
                        e[1] * f[2] - e[2] * f[1],
                        e[2] * f[0] - e[0] * f[2],
                        e[0] * f[1] - e[1] * f[0],
                    ];
                    assert!(x[0] * n0[0] + x[1] * n0[1] + x[2] * n0[2] > 0.0, "{label}");
                }
            }
        }
    });
    assert_eq!((lights, emitters, sabers), (5_599, 9_442, 68));

    let install = install().unwrap();
    let model = |name: &str| load_model(&install, name).unwrap().unwrap();

    // `v_muzflash_01`'s emitters: two-sided `Explosion` billboards.
    let (mdl, mdx) = model("v_muzflash_01");
    let m = Model::parse(&mdl, &mdx).unwrap();
    let n = m.node("muzflash01").unwrap();
    let e = n.emitter.as_ref().unwrap();
    assert_eq!(
        (e.update, e.render, e.blend),
        ("Explosion", "Billboard_to_Local_Z", "Lighten")
    );
    assert_eq!(e.texture, Some("fx_muzflash"));
    assert_eq!((e.x_grid, e.y_grid, e.chunk), (1, 1, None));
    assert!(e.two_sided);
    assert_eq!(e.name_187, "NULL");
    assert_eq!(e.flags, emitter_flags::P2P_SEL | emitter_flags::INHERIT);
    assert!(n.controller(emitter::BIRTH_RATE).is_some());
    assert_eq!(controller_types::name(n.kind, 88), Some("birth_rate"));

    // `v_grnfrag_fnf`'s `Explode05`: a blast, drawn fifth.
    let (mdl, mdx) = model("v_grnfrag_fnf");
    let m = Model::parse(&mdl, &mdx).unwrap();
    let e = m.node("Explode05").unwrap().emitter.as_ref().unwrap();
    assert_eq!((e.blast_radius, e.blast_length), (5.0, 1.0));
    assert_eq!(e.render_order, 5);
    assert_eq!(e.texture, Some("fx_chunks"));

    // `gidy_sun`: the ambient light and the shadow-casting diffuse one.
    let (mdl, mdx) = model("gidy_sun");
    let m = Model::parse(&mdl, &mdx).unwrap();
    let amb = m.node("gidy_sun_amb").unwrap();
    let diff = m.node("gidy_sun_diff").unwrap().light.as_ref().unwrap();
    assert!(amb.light.as_ref().unwrap().ambient_only && !diff.ambient_only);
    assert!(diff.shadow && !amb.light.as_ref().unwrap().shadow);
    assert_eq!(controller_types::name(amb.kind, 88), Some("radius"));
    let radius = amb
        .controller(light::RADIUS)
        .unwrap()
        .scalar_keys()
        .unwrap();
    assert_eq!(radius.map(|k| k.value).collect::<Vec<_>>(), [100_000.0]);

    // `fx_explode_03`'s light has six flares.
    let (mdl, mdx) = model("fx_explode_03");
    let m = Model::parse(&mdl, &mdx).unwrap();
    let l = m.node("AuroraLight04").unwrap().light.as_ref().unwrap();
    assert_eq!(l.flares.len(), 6);
    assert_eq!(l.flares[0].texture, "fx_flare01");
    assert_eq!(l.priority, 1);

    // A double-bladed saber's first blade.
    let (mdl, mdx) = model("w_dblsbr_001");
    let m = Model::parse(&mdl, &mdx).unwrap();
    let n = m.node("plane269").unwrap();
    let s = n.saber.as_ref().unwrap();
    assert_eq!(s.words, [98, 97]);
    let normal = s.normals.get(0).unwrap();
    assert!((normal[1] + 1.0).abs() < 1e-3, "{normal:?}");
    let uv = s.uv.get(3).unwrap();
    assert!(
        (uv[0] - 0.5).abs() < 1e-3 && (uv[1] - 1.0).abs() < 1e-3,
        "{uv:?}"
    );
    assert_eq!(controller_types::name(n.kind, mesh::ALPHA), Some("alpha"));

    // An animation node keyed by self-illumination alone is kept.
    let (mdl, mdx) = model("c_drdspyder");
    let m = Model::parse(&mdl, &mdx).unwrap();
    let a = m.animation("cdead").unwrap().node("Object01").unwrap();
    assert!(a.controller(controller_types::POSITION).is_none());
    assert!(a.controller(controller_types::ORIENTATION).is_none());
    let glow = a.controller(mesh::SELF_ILLUM_COLOR).unwrap();
    assert_eq!(glow.len(), 2);
    assert!(glow.vector_keys().is_some());
}
