//! The census over the game's own layouts and visibility files: every LYT
//! and VIS the KEY names, and any a module's capsules hold, is read and
//! written again and compared to its source, and the facts the format
//! leaves open are counted. The door hooks are matched against the doors
//! each area's GIT places, which is how the quaternion's component order is
//! established (`docs/formats/lyt.md`).
//!
//! Reads the user's own install from `$RAKATA_GAME_K1` (the folder holding
//! `chitin.key`) and skips, naming the variable, when it is unset. No game
//! bytes are committed.

use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::PI;
use std::path::PathBuf;

use rakata_game::{Install, Location, ModuleName, Profile};
use rakata_gff::{Gff, Value};
use rakata_lyt::{Lyt, Vis};
use rakata_res::{ResId, ResRef, ResType};

fn install() -> Option<Install> {
    match std::env::var("RAKATA_GAME_K1") {
        Ok(root) => Some(Install::open(PathBuf::from(root), Profile::K1).unwrap()),
        Err(_) => {
            eprintln!("skipped: $RAKATA_GAME_K1 is unset; set it to a KotOR 1 install");
            None
        }
    }
}

fn show(label: &str, v: &[String]) {
    println!("  {label}: {}", v.len());
    for s in v.iter().take(12) {
        println!("    {s}");
    }
    if v.len() > 12 {
        println!("    ... and {} more", v.len() - 12);
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

/// The count each room line of a VIS declares, if any, and the number of
/// indented lines that follow it, read from the text without the crate.
fn declared_counts(bytes: &[u8]) -> Vec<(String, Option<usize>, usize)> {
    let text = String::from_utf8_lossy(bytes);
    let mut out: Vec<(String, Option<usize>, usize)> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            out.last_mut().unwrap().2 += 1;
        } else {
            let mut t = line.split_ascii_whitespace();
            let room = t.next().unwrap().to_owned();
            out.push((room, t.next().map(|c| c.parse().unwrap()), 0));
        }
    }
    out
}

#[test]
fn every_vanilla_layout_and_visibility_file_reads_and_writes_back() {
    let Some(install) = install() else {
        return;
    };

    // Layouts.
    let lyts = every(&install, ResType::Lyt);
    let (mut rooms, mut tracks, mut obstacles, mut hooks) = (0, 0, 0, 0);
    let mut identical = 0;
    let mut failed = Vec::new();
    let mut different = Vec::new();
    let mut departing = Vec::new();
    let mut trailers = Vec::new();
    let mut minigames = Vec::new();
    let mut hook_not_a_room: BTreeMap<String, usize> = BTreeMap::new();
    let mut unknown_values: BTreeMap<i32, usize> = BTreeMap::new();
    let mut forms: BTreeMap<&str, usize> = BTreeMap::new();
    let mut form_example: BTreeMap<&str, String> = BTreeMap::new();
    let mut not_unit = Vec::new();
    let mut lyt_names = BTreeSet::new();
    for (name, bytes) in &lyts {
        let lyt = match Lyt::parse(bytes) {
            Ok(l) => l,
            Err(e) => {
                failed.push(format!("{name}: {e}"));
                continue;
            }
        };
        lyt_names.insert(
            name.to_ascii_lowercase()
                .trim_end_matches(".lyt")
                .to_owned(),
        );
        match lyt.to_bytes() {
            Ok(w) if &w == bytes => identical += 1,
            Ok(w) => different.push(format!("{name}: {} bytes for {}", w.len(), bytes.len())),
            Err(e) => different.push(format!("{name}: {e}")),
        }
        if lyt.departures() > 0 {
            departing.push(format!("{name}: {} lines", lyt.departures()));
        }
        if !lyt.trailer().is_empty() {
            trailers.push(format!("{name}: {:?}", lyt.trailer()));
        }
        if !lyt.tracks.is_empty() || !lyt.obstacles.is_empty() {
            minigames.push(format!(
                "{name}: {} rooms, {} tracks, {} obstacles",
                lyt.rooms.len(),
                lyt.tracks.len(),
                lyt.obstacles.len()
            ));
        }
        rooms += lyt.rooms.len();
        tracks += lyt.tracks.len();
        obstacles += lyt.obstacles.len();
        hooks += lyt.door_hooks.len();
        for h in &lyt.door_hooks {
            if lyt.room(&h.room).is_none() {
                *hook_not_a_room.entry(name.clone()).or_default() += 1;
            }
            *unknown_values.entry(h.unknown).or_default() += 1;
            let q = h.orientation;
            let norm = (q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z).sqrt();
            if (norm - 1.0).abs() > 1e-4 {
                not_unit.push(format!("{name}: {} {:?} norm {norm}", h.name, q.to_array()));
            }
            // Which components are zero, or within 1e-4 of it: in file
            // order, the second and third zero is a rotation about Z if the
            // scalar is first (w x y z), about X if it is last (x y z w).
            let zero = |v: f32| v.abs() < 1e-4;
            let exact = q.x == 0.0 && q.y == 0.0;
            let form = match (zero(q.x) && zero(q.y), zero(q.w), zero(q.z), exact) {
                (true, false, false, true) => "2nd and 3rd zero, 1st and 4th not",
                (true, false, false, false) => "2nd and 3rd below 1e-4, 1st and 4th not",
                (true, false, true, _) => "only the 1st non-zero",
                (true, true, false, _) => "only the 4th non-zero",
                _ => "another form",
            };
            *forms.entry(form).or_default() += 1;
            form_example
                .entry(form)
                .or_insert_with(|| format!("{name} {} {:?}", h.name, q.to_array()));
        }
    }
    println!(
        "{} LYT: {rooms} rooms, {tracks} track pieces, {obstacles} obstacles, {hooks} door hooks",
        lyts.len()
    );
    println!("  identical: {identical}");
    show("failed", &failed);
    show("different", &different);
    show("with lines departing from the written form", &departing);
    show("with bytes after donelayout", &trailers);
    show("with tracks or obstacles", &minigames);
    println!(
        "  door hooks naming no room of their layout: {} in {} files: {hook_not_a_room:?}",
        hook_not_a_room.values().sum::<usize>(),
        hook_not_a_room.len()
    );
    println!("  door hooks' third field: {unknown_values:?}");
    show("door hooks not of unit length", &not_unit);
    println!("  door hook orientations, by which components are zero:");
    for (f, n) in &forms {
        println!("    {n:>4} {f}; e.g. {}", form_example[f]);
    }

    // Visibility.
    let viss = every(&install, ResType::Vis);
    let (mut vis_rooms, mut vis_edges) = (0, 0);
    let mut vis_identical = 0;
    let mut vis_failed = Vec::new();
    let mut vis_different = Vec::new();
    let mut vis_departing = Vec::new();
    let mut self_listed = Vec::new();
    let mut count_off = Vec::new();
    let mut no_count = Vec::new();
    let mut vis_names = BTreeSet::new();
    for (name, bytes) in &viss {
        let vis = match Vis::parse(bytes) {
            Ok(v) => v,
            Err(e) => {
                vis_failed.push(format!("{name}: {e}"));
                continue;
            }
        };
        vis_names.insert(
            name.to_ascii_lowercase()
                .trim_end_matches(".vis")
                .to_owned(),
        );
        match vis.to_bytes() {
            Ok(w) if &w == bytes => vis_identical += 1,
            Ok(w) => vis_different.push(format!("{name}: {} bytes for {}", w.len(), bytes.len())),
            Err(e) => vis_different.push(format!("{name}: {e}")),
        }
        if vis.departures() > 0 {
            vis_departing.push(format!("{name}: {} lines", vis.departures()));
        }
        vis_rooms += vis.rooms.len();
        for r in &vis.rooms {
            vis_edges += r.visible.len();
            if r.visible.iter().any(|v| v.eq_ignore_ascii_case(&r.room)) {
                self_listed.push(format!("{name}: {}", r.room));
            }
        }
        for (room, declared, found) in declared_counts(bytes) {
            match declared {
                None => no_count.push(format!("{name}: {room}, {found} rooms follow")),
                Some(d) if d != found => {
                    count_off.push(format!("{name}: {room} declares {d}, {found} follow"))
                }
                Some(_) => {}
            }
        }
    }
    println!(
        "{} VIS: {vis_rooms} room lines, {vis_edges} rooms seen",
        viss.len()
    );
    println!("  identical: {vis_identical}");
    show("failed", &vis_failed);
    show("different", &vis_different);
    show("with lines departing from the written form", &vis_departing);
    show("rooms listing themselves", &self_listed);
    show("room lines whose count disagrees", &count_off);
    show("room lines with no count", &no_count);
    let lyt_only: Vec<String> = lyt_names.difference(&vis_names).cloned().collect();
    let vis_only: Vec<String> = vis_names.difference(&lyt_names).cloned().collect();
    show("layouts with no visibility file", &lyt_only);
    show("visibility files with no layout", &vis_only);

    assert!(!lyts.is_empty() && !viss.is_empty(), "no LYT or VIS found");
    assert_eq!(
        forms.get("another form"),
        None,
        "a hook that is not a rotation about Z read w x y z"
    );
    assert!(failed.is_empty() && vis_failed.is_empty(), "files refused");
    assert!(
        different.is_empty() && vis_different.is_empty(),
        "files not reproduced"
    );
}

/// The smallest difference between two angles, in (-π, π].
fn angle_diff(a: f32, b: f32) -> f32 {
    let mut d = (a - b) % (2.0 * PI);
    if d <= -PI {
        d += 2.0 * PI;
    } else if d > PI {
        d -= 2.0 * PI;
    }
    d
}

fn float(s: &rakata_gff::Struct, label: &str) -> f32 {
    match s.get(label) {
        Some(Value::Float(v)) => *v,
        other => panic!("{label} is {other:?}"),
    }
}

#[test]
fn door_hooks_agree_with_the_doors_each_area_places() {
    let Some(install) = install() else {
        return;
    };
    let (mut doors, mut matched, mut far) = (0, 0, Vec::new());
    let mut wxyz: BTreeMap<&str, usize> = BTreeMap::new();
    let mut xyzw: BTreeMap<&str, usize> = BTreeMap::new();
    let mut examples: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut distances: BTreeMap<&str, usize> = BTreeMap::new();
    let classify = |d: f32| {
        if d.abs() < 0.01 {
            "agrees"
        } else if (d.abs() - PI).abs() < 0.01 {
            "a half turn off"
        } else {
            "otherwise"
        }
    };
    for m in install.modules() {
        for id in install.module_ids(m).unwrap() {
            if id.ty != ResType::Git {
                continue;
            }
            let (_, git) = install.read(id, Some(m)).unwrap().unwrap();
            let lyt_id = ResId::new(id.name, ResType::Lyt);
            let Some((_, lyt)) = install.read(lyt_id, Some(m)).unwrap() else {
                continue;
            };
            let lyt = Lyt::parse(&lyt).unwrap();
            let git = Gff::parse(&git).unwrap();
            let Some(list) = git.root.get("Door List").and_then(Value::as_list) else {
                continue;
            };
            for door in list {
                doors += 1;
                let (x, y, bearing) = (float(door, "X"), float(door, "Y"), float(door, "Bearing"));
                let Some((dist, hook)) = lyt
                    .door_hooks
                    .iter()
                    .map(|h| ((h.position[0] - x).hypot(h.position[1] - y), h))
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                else {
                    far.push(format!("{m}/{id}: a door and no hooks"));
                    continue;
                };
                let band = match dist {
                    d if d <= 0.001 => "<= 1 mm",
                    d if d <= 0.1 => "<= 10 cm",
                    d if d <= 1.0 => "<= 1 m",
                    _ => "> 1 m",
                };
                *distances.entry(band).or_default() += 1;
                if dist > 0.1 {
                    far.push(format!(
                        "{m}/{id}: door at ({x}, {y}) is {dist:.3} m from {}",
                        hook.name
                    ));
                    continue;
                }
                matched += 1;
                let [a, b, c, d] = hook.orientation.to_array();
                let as_wxyz = classify(angle_diff(2.0 * d.atan2(a), bearing));
                let as_xyzw = classify(angle_diff(2.0 * c.atan2(d), bearing));
                *wxyz.entry(as_wxyz).or_default() += 1;
                *xyzw.entry(as_xyzw).or_default() += 1;
                let e = examples.entry(as_wxyz).or_default();
                if e.len() < 3 {
                    e.push(format!(
                        "{m}: {} {} at ({}, {}), file [{a} {b} {c} {d}]; GIT bearing {bearing}",
                        hook.room, hook.name, hook.position[0], hook.position[1]
                    ));
                }
            }
        }
    }
    println!("{doors} GIT doors; nearest hook by distance: {distances:?}");
    println!("{matched} within 10 cm of a hook");
    show("doors with no hook within 10 cm", &far);
    println!("  read w x y z, the hook's yaw against the door's bearing: {wxyz:?}");
    println!("  read x y z w, the same: {xyzw:?}");
    for (k, v) in &examples {
        println!("  {k}:");
        for s in v {
            println!("    {s}");
        }
    }
    assert!(matched > 0, "no door matched a hook");
    assert_eq!(
        wxyz.get("otherwise"),
        None,
        "a hook read w x y z disagrees with its door by other than a half turn"
    );
}

#[test]
fn the_endar_spire_bridge_area_has_fifteen_rooms_with_their_hooks_and_visibility() {
    let Some(install) = install() else {
        return;
    };
    let m = ModuleName::new("end_m01aa");
    let read = |ty| {
        let id = ResId::new(ResRef::new("m01aa").unwrap(), ty);
        install.read(id, Some(&m)).unwrap().unwrap().1
    };
    let lyt = Lyt::parse(&read(ResType::Lyt)).unwrap();
    let vis = Vis::parse(&read(ResType::Vis)).unwrap();
    println!("m01aa.lyt: depends on {:?}", lyt.file_dependency);
    for r in &lyt.rooms {
        println!(
            "  room {} at {:?}, sees {:?}",
            r.model,
            r.position,
            vis.visible(&r.model)
        );
    }
    for h in &lyt.door_hooks {
        println!(
            "  hook {} {} at {:?}, w x y z {:?}",
            h.room,
            h.name,
            h.position,
            h.orientation.to_array()
        );
    }
    assert_eq!(lyt.rooms.len(), 15);
    assert_eq!(lyt.door_hooks.len(), 15);
    assert!(lyt.tracks.is_empty() && lyt.obstacles.is_empty());
    assert_eq!(vis.rooms.len(), 15);
    for r in &vis.rooms {
        assert!(lyt.room(&r.room).is_some(), "{} is not a room", r.room);
    }
}
