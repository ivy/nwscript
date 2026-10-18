//! The census over the game's own walkmeshes: every WOK, PWK and DWK the
//! KEY names, and any a module's capsules hold, is read, and the facts
//! `docs/formats/walkmesh.md` states are checked against each: walkable
//! faces first and exactly the adjacency's faces, adjacency paired both
//! ways, the perimeter edges exactly the edges with no face across them in
//! closed loops, the AABB tree's leaves one per face. Walkability is read
//! from the install's own `surfacemat.2da`. Each file writes back byte for
//! byte. Run with `--nocapture` to see
//! the counts.
//!
//! Reads the user's own install from `$RAKATA_GAME_K1` (the folder holding
//! `chitin.key`) and skips, naming the variable, when it is unset. No game
//! bytes are committed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use rakata_game::{Install, Location, ModuleName, Profile};
use rakata_res::{ResId, ResRef, ResType};
use rakata_walkmesh::{EdgeRef, Kind, SurfaceMaterials, Walkmesh};

fn install() -> Option<Install> {
    match std::env::var("RAKATA_GAME_K1") {
        Ok(root) => Some(Install::open(PathBuf::from(root), Profile::K1).unwrap()),
        Err(_) => {
            eprintln!("skipped: $RAKATA_GAME_K1 is unset; set it to a KotOR 1 install");
            None
        }
    }
}

/// Every copy of a type the install holds outside `Override/`: the KEY's
/// entries, then each module's capsule members, by name.
fn every(install: &Install, ty: ResType) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for e in install.key().entries().iter().filter(|e| e.id.ty == ty) {
        let (location, bytes) = install.read(e.id, None).unwrap().unwrap();
        assert!(
            matches!(location, Location::Bif { .. }),
            "{} is overridden in the vanilla install",
            e.id
        );
        out.push((e.id.to_string(), bytes));
    }
    for m in install.modules() {
        for id in install.module_ids(m).unwrap() {
            if id.ty == ty {
                let (_, bytes) = install.read(id, Some(m)).unwrap().unwrap();
                out.push((format!("{m}/{id}"), bytes));
            }
        }
    }
    out
}

fn surface_materials(install: &Install) -> SurfaceMaterials {
    let id = ResId::new(ResRef::new("surfacemat").unwrap(), ResType::TwoDa);
    let (_, bytes) = install.read(id, None).unwrap().expect("surfacemat.2da");
    SurfaceMaterials::parse(&bytes).unwrap()
}

/// The edge's two ends as positions, so edges of different faces that meet
/// at coincident vertices compare equal.
fn ends(w: &Walkmesh, e: EdgeRef) -> [[u32; 3]; 2] {
    let (a, b) = w.edge_points(e).unwrap();
    [a.map(f32::to_bits), b.map(f32::to_bits)]
}

#[derive(Default)]
struct Census {
    files: usize,
    failures: Vec<String>,
    empty: usize,
    faces: usize,
    walkable: usize,
    with_tree: usize,
    with_adjacency: usize,
    edges: usize,
    transitions: usize,
    loops: usize,
    leaves_missing_face: usize,
    materials: BTreeMap<u32, usize>,
    exceptions: BTreeMap<&'static str, BTreeSet<String>>,
}

impl Census {
    fn except(&mut self, what: &'static str, name: &str) {
        self.exceptions
            .entry(what)
            .or_default()
            .insert(name.to_string());
    }

    fn print(&self, ty: &str) {
        println!(
            "{ty}: {} files, {} failures, {} without faces; {} faces, {} walkable ({:.1}%); \
             {} with an AABB tree, {} with adjacency; {} perimeter edges in {} loops, {} with a transition",
            self.files,
            self.failures.len(),
            self.empty,
            self.faces,
            self.walkable,
            100.0 * self.walkable as f64 / self.faces.max(1) as f64,
            self.with_tree,
            self.with_adjacency,
            self.edges,
            self.loops,
            self.transitions,
        );
        println!("  materials: {:?}", self.materials);
        for (what, names) in &self.exceptions {
            println!("  {what}: {} {:?}", names.len(), names);
        }
        for f in &self.failures {
            println!("  FAILED {f}");
        }
    }
}

fn census(name: &str, w: &Walkmesh, table: &SurfaceMaterials, c: &mut Census) {
    let nf = w.faces.len();
    if nf == 0 {
        c.empty += 1;
    }
    c.faces += nf;
    let walkable: Vec<usize> = w.walkable_faces(table).collect();
    c.walkable += walkable.len();
    for f in &w.faces {
        *c.materials.entry(f.material).or_default() += 1;
    }
    if walkable != (0..walkable.len()).collect::<Vec<_>>() {
        c.except("walkable faces not first", name);
    }
    if !w.adjacency.is_empty() {
        c.with_adjacency += 1;
    }
    if w.adjacency.len() != walkable.len() {
        c.except("adjacency not one row per walkable face", name);
    }
    // Adjacency pairs both ways, across coincident edges.
    let mut asymmetric = false;
    for (f, row) in w.adjacency.iter().enumerate() {
        for (e, across) in row.iter().enumerate() {
            let Some(other) = across else { continue };
            let here = EdgeRef {
                face: f as u32,
                edge: e as u8,
            };
            let back = w.adjacency[other.face as usize][usize::from(other.edge)];
            asymmetric |= back != Some(here);
            let [a, b] = ends(w, here);
            assert_eq!(ends(w, *other), [b, a], "{name}: {here:?} and {other:?}");
        }
    }
    if asymmetric {
        c.except("adjacency not paired both ways", name);
    }
    // The perimeter is the adjacency's open edges, in closed loops.
    let open: BTreeSet<EdgeRef> = w
        .adjacency
        .iter()
        .enumerate()
        .flat_map(|(f, row)| {
            row.iter().enumerate().filter_map(move |(e, a)| {
                a.is_none().then_some(EdgeRef {
                    face: f as u32,
                    edge: e as u8,
                })
            })
        })
        .collect();
    let listed: BTreeSet<EdgeRef> = w.perimeter_edges.iter().map(|p| p.edge).collect();
    assert_eq!(open, listed, "{name}: perimeter edges");
    assert_eq!(
        listed.len(),
        w.perimeter_edges.len(),
        "{name}: an edge twice"
    );
    c.edges += w.perimeter_edges.len();
    c.transitions += w
        .perimeter_edges
        .iter()
        .filter(|p| p.transition.is_some())
        .count();
    if let Some(&last) = w.perimeters.last() {
        assert_eq!(last as usize, w.perimeter_edges.len(), "{name}");
    }
    for l in w.perimeter_loops() {
        c.loops += 1;
        for k in 0..l.len() {
            let [_, end] = ends(w, l[k].edge);
            let [start, _] = ends(w, l[(k + 1) % l.len()].edge);
            assert_eq!(end, start, "{name}: a loop broken after edge {k}");
        }
    }
    // The tree: pre-order, one leaf per face.
    if !w.aabb.is_empty() {
        c.with_tree += 1;
        let mut leaves: Vec<u32> = w.aabb.iter().filter_map(|n| n.face).collect();
        leaves.sort();
        if leaves != (0..nf as u32).collect::<Vec<_>>() {
            c.except("AABB leaves not one per face", name);
        }
        for (i, n) in w.aabb.iter().enumerate() {
            assert_eq!(n.unknown, 4, "{name}: node {i}");
            match n.face {
                Some(f) => {
                    assert!(n.left.is_none() && n.right.is_none() && n.plane == 0);
                    // Whether the leaf's box holds the face it names.
                    let corners = w.faces[f as usize].vertices;
                    let holds = corners.iter().all(|&v| {
                        let p = w.vertices.get(v as usize).unwrap();
                        (0..3).all(|a| n.min[a] - 1e-4 <= p[a] && p[a] <= n.max[a] + 1e-4)
                    });
                    if !holds {
                        c.leaves_missing_face += 1;
                        c.except("AABB leaf box not holding its face", name);
                    }
                }
                None => {
                    // An inner node's box is its children's, exactly.
                    let (l, r) = (n.left.unwrap() as usize, n.right.unwrap() as usize);
                    for a in 0..3 {
                        assert_eq!(n.min[a], w.aabb[l].min[a].min(w.aabb[r].min[a]), "{name}");
                        assert_eq!(n.max[a], w.aabb[l].max[a].max(w.aabb[r].max[a]), "{name}");
                    }
                    assert_eq!(n.left, Some(i as u32 + 1), "{name}: node {i}");
                    assert!(n.right.is_some_and(|r| r as usize > i));
                    assert!([1, 2, 4, 8, 16, 32].contains(&n.plane), "{name}: {n:?}");
                }
            }
        }
    }
}

#[test]
fn every_vanilla_walkmesh_reads() {
    let Some(install) = install() else {
        return;
    };
    let table = surface_materials(&install);
    for (ty, ext) in [
        (ResType::Wok, "WOK"),
        (ResType::Pwk, "PWK"),
        (ResType::Dwk, "DWK"),
    ] {
        let mut c = Census::default();
        for (name, bytes) in every(&install, ty) {
            c.files += 1;
            match Walkmesh::parse(&bytes) {
                Ok(w) => {
                    let want = if ty == ResType::Wok {
                        Kind::Area
                    } else {
                        Kind::Object
                    };
                    assert_eq!(w.kind, want, "{name}");
                    assert!(w.trailer.is_empty(), "{name}");
                    assert!(w.to_bytes().unwrap() == bytes, "{name} writes back changed");
                    census(&name, &w, &table, &mut c);
                    if ty == ResType::Dwk {
                        for k in 0..2 {
                            for a in 0..3 {
                                let sum = w.relative_points[k][a] + w.position[a];
                                assert!((w.absolute_points[k][a] - sum).abs() < 1e-3, "{name}");
                            }
                        }
                    }
                    if ty == ResType::Pwk {
                        assert!(w.faces.iter().all(|f| f.normal == [0.0; 3]), "{name}");
                    }
                }
                Err(e) => c.failures.push(format!("{name}: {e}")),
            }
        }
        c.print(ext);
        assert!(c.files > 0, "no {ext} in the install");
        assert!(c.failures.is_empty(), "{ext}: {:?}", c.failures);
        if ty == ResType::Wok {
            let files = c.exceptions.remove("AABB leaf box not holding its face");
            assert_eq!(c.leaves_missing_face, 87);
            assert_eq!(files.map_or(0, |f| f.len()), 24);
        }
        let exceptions: BTreeMap<&str, Vec<&str>> = c
            .exceptions
            .iter()
            .map(|(k, v)| (*k, v.iter().map(String::as_str).collect()))
            .collect();
        // The exceptions docs/formats/walkmesh.md names, and no others.
        let expected: BTreeMap<&str, Vec<&str>> = match ty {
            ResType::Wok => BTreeMap::from([
                ("AABB leaves not one per face", vec!["m02af_01a.wok"]),
                ("adjacency not paired both ways", vec!["m38aa_03.wok"]),
            ]),
            // Placeables and doors with walkable faces store no adjacency.
            ResType::Pwk => BTreeMap::from([(
                "adjacency not one row per walkable face",
                vec!["plc_chwdnblc.pwk", "plc_lockerlg.pwk"],
            )]),
            _ => BTreeMap::from([
                (
                    "adjacency not one row per walkable face",
                    vec![
                        "dor_lda031.dwk",
                        "dor_lda032.dwk",
                        "dor_lda041.dwk",
                        "dor_lda042.dwk",
                        "dor_lma051.dwk",
                        "dor_lma052.dwk",
                        "dor_lts051.dwk",
                        "dor_lts052.dwk",
                        "dor_lts091.dwk",
                        "dor_lts092.dwk",
                        "dor_lts101.dwk",
                        "dor_lts102.dwk",
                        "dor_lyv011.dwk",
                        "dor_lyv012.dwk",
                    ],
                ),
                (
                    "walkable faces not first",
                    vec![
                        "dor_lts051.dwk",
                        "dor_lts052.dwk",
                        "dor_lts101.dwk",
                        "dor_lts102.dwk",
                    ],
                ),
            ]),
        };
        assert_eq!(exceptions, expected, "{ext}");
        if ty != ResType::Wok {
            assert_eq!(c.with_tree + c.with_adjacency + c.edges, 0, "{ext}");
        }
    }
}

#[test]
fn the_endar_spire_bridge_rooms_walkmeshes_read() {
    let Some(install) = install() else {
        return;
    };
    let table = surface_materials(&install);
    let m = ModuleName::new("end_m01aa");
    let rooms = [
        "m01aa_01a",
        "m01aa_02a",
        "m01aa_03a",
        "m01aa_04a",
        "m01aa_05a",
        "m01aa_06a",
        "m01aa_06b",
        "m01aa_07a",
        "m01aa_07b",
        "m01aa_08a",
        "m01aa_08b",
        "m01aa_08c",
        "m01aa_08d",
        "m01aa_09",
        "m01aa_10",
    ];
    let mut with_faces = 0;
    for room in rooms {
        let id = ResId::new(ResRef::new(room).unwrap(), ResType::Wok);
        let (_, bytes) = install.read(id, Some(&m)).unwrap().unwrap();
        let w = Walkmesh::parse(&bytes).unwrap();
        let transitions: BTreeSet<u32> = w
            .perimeter_edges
            .iter()
            .filter_map(|p| p.transition)
            .collect();
        println!(
            "{room}: {} vertices, {} faces, {} walkable, {} AABB nodes, {} perimeter edges in {} loops, \
             transitions to rooms {:?}",
            w.vertices.len(),
            w.faces.len(),
            w.walkable_faces(&table).count(),
            w.aabb.len(),
            w.perimeter_edges.len(),
            w.perimeters.len(),
            transitions,
        );
        assert_eq!(w.kind, Kind::Area);
        if !w.faces.is_empty() {
            with_faces += 1;
            assert_eq!(w.aabb.len(), 2 * w.faces.len() - 1, "{room}");
        }
    }
    // One room, M01aa_07b, has an empty walkmesh.
    assert_eq!(with_faces, 14);
}

#[test]
fn perimeter_transitions_name_rooms_of_the_areas_layout_and_lead_back() {
    let Some(install) = install() else {
        return;
    };
    let (mut areas, mut links, mut back, mut missing) = (0, 0, 0, 0);
    let (mut past, mut to_no_wok, mut one_way) = (Vec::new(), Vec::new(), Vec::new());
    for e in install
        .key()
        .entries()
        .iter()
        .filter(|e| e.id.ty == ResType::Lyt)
    {
        let (_, bytes) = install.read(e.id, None).unwrap().unwrap();
        let lyt = rakata_lyt::Lyt::parse(&bytes).unwrap();
        // Each room's transitions, by its index in the layout.
        let mut to: Vec<Option<BTreeSet<u32>>> = Vec::new();
        for r in &lyt.rooms {
            let id = ResId::new(
                ResRef::new(&r.model.to_ascii_lowercase()).unwrap(),
                ResType::Wok,
            );
            to.push(install.read(id, None).unwrap().map(|(_, bytes)| {
                let w = Walkmesh::parse(&bytes).unwrap();
                w.perimeter_edges
                    .iter()
                    .filter_map(|p| p.transition)
                    .collect()
            }));
        }
        missing += to.iter().filter(|t| t.is_none()).count();
        areas += 1;
        for (i, t) in to.iter().enumerate() {
            for &j in t.iter().flatten() {
                links += 1;
                let here = format!("{} {} -> {j}", e.id, lyt.rooms[i].model);
                match to.get(j as usize) {
                    None => past.push(here),
                    Some(None) => to_no_wok.push(here),
                    Some(Some(o)) if o.contains(&(i as u32)) => back += 1,
                    Some(Some(_)) => one_way.push(here),
                }
            }
        }
    }
    println!(
        "{areas} layouts, {missing} rooms without a WOK; {links} room-to-room transitions, \
         {back} answered by a transition back"
    );
    println!("  past the layout's rooms: {past:?}");
    println!("  into a room without a WOK: {to_no_wok:?}");
    println!("  not answered: {one_way:?}");
    // Every exception is a cutscene layout, which places rooms of another
    // area's layout whose WOKs number that area's rooms.
    assert!(
        past.iter()
            .chain(&to_no_wok)
            .all(|s| s.starts_with("stunt_")),
        "{past:?} {to_no_wok:?}"
    );
    assert!(one_way.is_empty(), "{one_way:?}");
}
