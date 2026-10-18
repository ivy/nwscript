//! The census over the game's own models: every MDL in a KotOR 1 install
//! (the BIFs the KEY names, the capsules under `modules/` and `rims/` with
//! their `dx` partners, `patch.erf`, and loose files in `Override/`) is
//! read with its MDX, and what the layout leaves open is counted: node
//! kinds, classifications, the MDX attribute layouts, the flags. A second
//! test reads every room, door and placeable model the Endar Spire's first
//! area, `end_m01aa`, uses. Run with `--nocapture` to see the counts.
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
use rakata_mdl::{Model, NodeKind, mdx_flags};
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
        for n in &m.nodes {
            self.nodes += 1;
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
    show("failed", &c.failed);

    assert!(c.models > 3000, "only {} models found", c.models);
    assert!(c.failed.is_empty(), "models refused: {:#?}", c.failed);
    assert!(c.flags_disagree.is_empty());
    assert!(c.index_list_not_faces.is_empty());
    assert_eq!(c.not_unit, 0);
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
