//! Textures, layouts, walkmeshes and models compared by what they hold.
//!
//! A texture by its header (size, pixel format, how its pixels divide into
//! images, mip levels), each level's pixels as changed or not, and its TXI
//! by instruction; a layout by room, track, obstacle and door hook, each by
//! model name; room visibility by room and by the rooms each sees; a
//! walkmesh by its counts, the faces of each surface material, the edges
//! leading into each room, and the arrays derived from its faces; a model
//! (its MDL and MDX read as one) by its header, its nodes by name, each
//! mesh's textures, flags and counts, and its animations by name.
//!
//! Every comparison here reads both sides with the format's crate, and
//! returns no sites exactly when the crate's own equality holds: a pair
//! that differs only in layout (another writer's arrangement of the same
//! data) is the same data. Where the data differs in a field none of the
//! sites name, a site names the array or stored structure that differs.

use std::collections::{BTreeMap, HashMap};

use rakata_lyt::{DoorHook, Lyt, Placement, Vis};
use rakata_mdl::{Animation, Mesh, Model, ModelFile, Node, NodeFile};
use rakata_res::ContentHash;
use rakata_texture::{Layout, Tga, Tpc, Txi, TxiEntry};
use rakata_walkmesh::Walkmesh;

use super::{Change, Site};

fn header<T: PartialEq + ToString>(sites: &mut Vec<Site>, field: &str, a: T, b: T) {
    if a != b {
        sites.push(Site::Header {
            field: field.to_owned(),
            before: Some(a.to_string()),
            after: Some(b.to_string()),
        });
    }
}

fn part(
    change: Change,
    part: &str,
    name: &str,
    field: Option<&str>,
    before: Option<String>,
    after: Option<String>,
) -> Site {
    Site::Part {
        change,
        part: part.to_owned(),
        name: name.to_owned(),
        field: field.map(str::to_owned),
        before,
        after,
    }
}

/// A changed field of a named part, when the two texts differ.
fn changed(sites: &mut Vec<Site>, kind: &str, name: &str, field: &str, a: String, b: String) {
    if a != b {
        sites.push(part(
            Change::Changed,
            kind,
            name,
            Some(field),
            Some(a),
            Some(b),
        ));
    }
}

fn short_hash(bytes: &[u8]) -> String {
    ContentHash::of(bytes).to_hex()[..12].to_owned()
}

fn bits3(v: [f32; 3]) -> [u32; 3] {
    v.map(f32::to_bits)
}

fn vec3(v: [f32; 3]) -> String {
    format!("({}, {}, {})", v[0], v[1], v[2])
}

fn float(sites: &mut Vec<Site>, field: &str, a: f32, b: f32) {
    if a.to_bits() != b.to_bits() {
        sites.push(Site::Header {
            field: field.to_owned(),
            before: Some(a.to_string()),
            after: Some(b.to_string()),
        });
    }
}

/// Two lists of names paired without regard to case, the n-th of a
/// repeated name with the n-th: each pair, then each name only one side
/// has, as indices with the name shown (the second side's spelling, with
/// its occurrence after the first, `name #2`).
fn pair_names(a: &[&str], b: &[&str]) -> Vec<(Option<usize>, Option<usize>, String)> {
    let mut theirs: HashMap<String, Vec<usize>> = HashMap::new();
    for (j, n) in b.iter().enumerate() {
        theirs.entry(n.to_ascii_lowercase()).or_default().push(j);
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut used = vec![false; b.len()];
    let shown = |n: &str, k: usize| {
        if k == 0 {
            n.to_owned()
        } else {
            format!("{n} #{}", k + 1)
        }
    };
    let mut out = Vec::new();
    for (i, n) in a.iter().enumerate() {
        let key = n.to_ascii_lowercase();
        let k = *seen.entry(key.clone()).and_modify(|k| *k += 1).or_insert(0);
        match theirs.get(&key).and_then(|v| v.get(k)) {
            Some(&j) => {
                used[j] = true;
                out.push((Some(i), Some(j), shown(b[j], k)));
            }
            None => out.push((Some(i), None, shown(n, k))),
        }
    }
    let mut seen_b: HashMap<String, usize> = HashMap::new();
    for (j, n) in b.iter().enumerate() {
        let k = *seen_b
            .entry(n.to_ascii_lowercase())
            .and_modify(|k| *k += 1)
            .or_insert(0);
        if !used[j] {
            out.push((None, Some(j), shown(n, k)));
        }
    }
    out
}

// Textures.

fn layout_text(l: Layout) -> String {
    match l {
        Layout::Single => "single".to_owned(),
        Layout::Cube => "cube".to_owned(),
        Layout::Flipbook { numx, numy } => format!("flipbook {numx}x{numy}"),
    }
}

fn level_name(l: Layout, image: usize, level: usize) -> String {
    match l {
        Layout::Single => format!("level {level}"),
        Layout::Cube => format!("face {image} level {level}"),
        Layout::Flipbook { .. } => format!("frame {image} level {level}"),
    }
}

/// Two TPCs: the header, each level's pixels when the two are shaped
/// alike (the pixel data whole when not), and the TXI after the pixels.
pub(super) fn tpc(a: &Tpc<'_>, b: &Tpc<'_>) -> Vec<Site> {
    let mut sites = Vec::new();
    header(&mut sites, "width", a.width, b.width);
    header(&mut sites, "height", a.height, b.height);
    header(
        &mut sites,
        "format",
        format!("{:?}", a.format),
        format!("{:?}", b.format),
    );
    header(
        &mut sites,
        "layout",
        layout_text(a.layout),
        layout_text(b.layout),
    );
    header(&mut sites, "images", a.images.len(), b.images.len());
    header(&mut sites, "levels", a.level_count(), b.level_count());
    header(&mut sites, "mip_count", a.mip_count, b.mip_count);
    header(&mut sites, "data_size", a.data_size, b.data_size);
    float(&mut sites, "alpha_mean", a.alpha_mean, b.alpha_mean);
    if a.reserved != b.reserved {
        let shown = |r: &[u8]| {
            let set = r.iter().filter(|&&x| x != 0).count();
            format!("{set} of {} bytes set", r.len())
        };
        header(&mut sites, "reserved", shown(a.reserved), shown(b.reserved));
    }
    let shaped_alike = a.format == b.format
        && a.layout == b.layout
        && a.images.len() == b.images.len()
        && a.image_size() == b.image_size();
    if shaped_alike {
        for (i, (ia, ib)) in a.images.iter().zip(&b.images).enumerate() {
            for l in 0..ia.levels.len().max(ib.levels.len()) {
                let shown = |x: &rakata_texture::Level<'_>| {
                    format!("{}x{} #{}", x.width, x.height, short_hash(x.data))
                };
                let name = level_name(b.layout, i, l);
                match (ia.levels.get(l), ib.levels.get(l)) {
                    (Some(x), Some(y)) if x.data != y.data || x.width != y.width => {
                        sites.push(part(
                            Change::Changed,
                            "pixels",
                            &name,
                            None,
                            Some(shown(x)),
                            Some(shown(y)),
                        ));
                    }
                    (Some(x), None) => sites.push(part(
                        Change::Removed,
                        "pixels",
                        &name,
                        None,
                        Some(shown(x)),
                        None,
                    )),
                    (None, Some(y)) => sites.push(part(
                        Change::Added,
                        "pixels",
                        &name,
                        None,
                        None,
                        Some(shown(y)),
                    )),
                    _ => {}
                }
            }
        }
    } else if a.pixels != b.pixels {
        header(
            &mut sites,
            "pixels",
            format!("#{}", short_hash(a.pixels)),
            format!("#{}", short_hash(b.pixels)),
        );
    }
    let empty = Txi::default();
    sites.extend(txi(
        a.txi.as_ref().unwrap_or(&empty),
        b.txi.as_ref().unwrap_or(&empty),
    ));
    sites
}

fn txi_text(e: &TxiEntry) -> String {
    if e.rows.is_empty() {
        e.value.clone()
    } else {
        format!("{} [{} rows: {}]", e.value, e.rows.len(), e.rows.join("; "))
    }
}

/// Two TXIs by instruction, keywords paired without regard to case.
pub(super) fn txi(a: &Txi, b: &Txi) -> Vec<Site> {
    let mut sites = Vec::new();
    if a.preamble != b.preamble {
        header(
            &mut sites,
            "txi preamble",
            a.preamble.join("; "),
            b.preamble.join("; "),
        );
    }
    let ka: Vec<&str> = a.entries.iter().map(|e| e.key.as_str()).collect();
    let kb: Vec<&str> = b.entries.iter().map(|e| e.key.as_str()).collect();
    for (i, j, name) in pair_names(&ka, &kb) {
        match (i.map(|i| &a.entries[i]), j.map(|j| &b.entries[j])) {
            (Some(x), Some(y)) => {
                if x.value != y.value || x.rows != y.rows {
                    sites.push(part(
                        Change::Changed,
                        "txi",
                        &name,
                        None,
                        Some(txi_text(x)),
                        Some(txi_text(y)),
                    ));
                }
            }
            (Some(x), None) => sites.push(part(
                Change::Removed,
                "txi",
                &name,
                None,
                Some(txi_text(x)),
                None,
            )),
            (None, Some(y)) => sites.push(part(
                Change::Added,
                "txi",
                &name,
                None,
                None,
                Some(txi_text(y)),
            )),
            (None, None) => {}
        }
    }
    sites
}

/// Two TGAs: the header, the image's pixels as changed or not, and the
/// fields beside the image (its ID, colour map, origin, attribute bits and
/// the bytes after it). How run-length packets fall is layout.
pub(super) fn tga(a: &Tga, b: &Tga) -> Vec<Site> {
    let mut sites = Vec::new();
    header(&mut sites, "width", a.width, b.width);
    header(&mut sites, "height", a.height, b.height);
    header(
        &mut sites,
        "image_type",
        a.image_type.byte(),
        b.image_type.byte(),
    );
    header(&mut sites, "bits", a.bits, b.bits);
    header(&mut sites, "alpha_bits", a.alpha_bits, b.alpha_bits);
    header(
        &mut sites,
        "origin",
        format!("{:?}", a.origin),
        format!("{:?}", b.origin),
    );
    if a.rgba != b.rgba {
        let shown = |t: &Tga| format!("{}x{} #{}", t.width, t.height, short_hash(&t.rgba));
        sites.push(part(
            Change::Changed,
            "pixels",
            "level 0",
            None,
            Some(shown(a)),
            Some(shown(b)),
        ));
    }
    let (la, lb) = (&a.layout, &b.layout);
    let bytes = |x: &[u8]| format!("{} bytes #{}", x.len(), short_hash(x));
    if la.image_id != lb.image_id {
        header(
            &mut sites,
            "image_id",
            bytes(&la.image_id),
            bytes(&lb.image_id),
        );
    }
    header(
        &mut sites,
        "color_map_type",
        la.color_map_type,
        lb.color_map_type,
    );
    header(
        &mut sites,
        "color_map_first",
        la.color_map_first,
        lb.color_map_first,
    );
    header(
        &mut sites,
        "color_map_len",
        la.color_map_len,
        lb.color_map_len,
    );
    header(
        &mut sites,
        "color_map_bits",
        la.color_map_bits,
        lb.color_map_bits,
    );
    if la.color_map != lb.color_map {
        header(
            &mut sites,
            "color_map",
            bytes(&la.color_map),
            bytes(&lb.color_map),
        );
    }
    header(&mut sites, "x_origin", la.x_origin, lb.x_origin);
    header(&mut sites, "y_origin", la.y_origin, lb.y_origin);
    header(
        &mut sites,
        "descriptor_high",
        la.descriptor_high,
        lb.descriptor_high,
    );
    if la.attribute_bits != lb.attribute_bits {
        let set = |v: &[bool]| format!("{} of {} set", v.iter().filter(|&&x| x).count(), v.len());
        header(
            &mut sites,
            "attribute_bits",
            set(&la.attribute_bits),
            set(&lb.attribute_bits),
        );
    }
    if la.tail != lb.tail {
        header(&mut sites, "tail", bytes(&la.tail), bytes(&lb.tail));
    }
    sites
}

// Layouts.

/// Placements of one kind paired by model name: added, removed and moved.
/// The order the common ones stand in matters (a walkmesh's transitions
/// name rooms by their index in the layout), so a change of it is a site.
fn placements(sites: &mut Vec<Site>, kind: &str, a: &[Placement], b: &[Placement]) {
    let na: Vec<&str> = a.iter().map(|p| p.model.as_str()).collect();
    let nb: Vec<&str> = b.iter().map(|p| p.model.as_str()).collect();
    let mut common = Vec::new();
    for (i, j, name) in pair_names(&na, &nb) {
        match (i, j) {
            (Some(i), Some(j)) => {
                common.push((i, j));
                if bits3(a[i].position) != bits3(b[j].position) {
                    sites.push(part(
                        Change::Changed,
                        kind,
                        &name,
                        Some("position"),
                        Some(vec3(a[i].position)),
                        Some(vec3(b[j].position)),
                    ));
                }
            }
            (Some(i), None) => sites.push(part(
                Change::Removed,
                kind,
                &name,
                None,
                Some(format!("at {}", vec3(a[i].position))),
                None,
            )),
            (None, Some(j)) => sites.push(part(
                Change::Added,
                kind,
                &name,
                None,
                None,
                Some(format!("at {}", vec3(b[j].position))),
            )),
            (None, None) => {}
        }
    }
    let mut by_b = common.clone();
    by_b.sort_by_key(|&(_, j)| j);
    if by_b != common {
        header(
            sites,
            &format!("{kind} order"),
            common
                .iter()
                .map(|&(i, _)| a[i].model.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            by_b.iter()
                .map(|&(_, j)| b[j].model.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
}

fn hook_text(d: &DoorHook) -> String {
    let q = d.orientation;
    format!(
        "at {}, orientation ({}, {}, {}, {})",
        vec3(d.position),
        q.w,
        q.x,
        q.y,
        q.z
    )
}

/// Two room layouts: the dependency line, then rooms, tracks, obstacles
/// and door hooks, each by name.
pub(super) fn lyt(a: &Lyt, b: &Lyt) -> Vec<Site> {
    let mut sites = Vec::new();
    header(
        &mut sites,
        "file_dependency",
        a.file_dependency.clone().unwrap_or_default(),
        b.file_dependency.clone().unwrap_or_default(),
    );
    placements(&mut sites, "room", &a.rooms, &b.rooms);
    placements(&mut sites, "track", &a.tracks, &b.tracks);
    placements(&mut sites, "obstacle", &a.obstacles, &b.obstacles);
    let key = |d: &DoorHook| format!("{}/{}", d.room, d.name);
    let ka: Vec<String> = a.door_hooks.iter().map(key).collect();
    let kb: Vec<String> = b.door_hooks.iter().map(key).collect();
    let ra: Vec<&str> = ka.iter().map(String::as_str).collect();
    let rb: Vec<&str> = kb.iter().map(String::as_str).collect();
    for (i, j, name) in pair_names(&ra, &rb) {
        match (i.map(|i| &a.door_hooks[i]), j.map(|j| &b.door_hooks[j])) {
            (Some(x), Some(y)) => {
                if !x.bits_eq(y) {
                    if x.unknown != y.unknown {
                        changed(
                            &mut sites,
                            "door hook",
                            &name,
                            "unknown",
                            x.unknown.to_string(),
                            y.unknown.to_string(),
                        );
                    }
                    let (hx, hy) = (hook_text(x), hook_text(y));
                    if hx != hy || x.room != y.room || x.name != y.name {
                        sites.push(part(
                            Change::Changed,
                            "door hook",
                            &name,
                            Some("placement"),
                            Some(hx),
                            Some(hy),
                        ));
                    }
                }
            }
            (Some(x), None) => sites.push(part(
                Change::Removed,
                "door hook",
                &name,
                None,
                Some(hook_text(x)),
                None,
            )),
            (None, Some(y)) => sites.push(part(
                Change::Added,
                "door hook",
                &name,
                None,
                None,
                Some(hook_text(y)),
            )),
            (None, None) => {}
        }
    }
    sites
}

/// Two visibility files: rooms by name, and for a room both list, each
/// room it sees added or removed. Names compare without regard to case,
/// as the engine compares them; the order a room's list is in is a site of
/// its own when the rooms seen are the same.
pub(super) fn vis(a: &Vis, b: &Vis) -> Vec<Site> {
    let mut sites = Vec::new();
    let na: Vec<&str> = a.rooms.iter().map(|r| r.room.as_str()).collect();
    let nb: Vec<&str> = b.rooms.iter().map(|r| r.room.as_str()).collect();
    for (i, j, name) in pair_names(&na, &nb) {
        match (i.map(|i| &a.rooms[i]), j.map(|j| &b.rooms[j])) {
            (Some(x), Some(y)) => {
                let lower = |v: &[String]| -> Vec<String> {
                    v.iter().map(|s| s.to_ascii_lowercase()).collect()
                };
                let (lx, ly) = (lower(&x.visible), lower(&y.visible));
                let mut gone = Vec::new();
                for (k, s) in lx.iter().enumerate() {
                    if !ly.contains(s) {
                        gone.push(k);
                    }
                }
                let mut new = Vec::new();
                for (k, s) in ly.iter().enumerate() {
                    if !lx.contains(s) {
                        new.push(k);
                    }
                }
                for k in &gone {
                    sites.push(part(
                        Change::Removed,
                        "sees",
                        &format!("{name} -> {}", x.visible[*k]),
                        None,
                        None,
                        None,
                    ));
                }
                for k in &new {
                    sites.push(part(
                        Change::Added,
                        "sees",
                        &format!("{name} -> {}", y.visible[*k]),
                        None,
                        None,
                        None,
                    ));
                }
                if gone.is_empty() && new.is_empty() && lx != ly {
                    let (mut sx, mut sy) = (lx.clone(), ly.clone());
                    sx.sort();
                    sy.sort();
                    // The same rooms in another order, or a room listed twice.
                    sites.push(part(
                        Change::Changed,
                        "room",
                        &name,
                        Some(if sx == sy { "order" } else { "repeats" }),
                        Some(x.visible.join(", ")),
                        Some(y.visible.join(", ")),
                    ));
                }
            }
            (Some(x), None) => sites.push(part(
                Change::Removed,
                "room",
                &name,
                None,
                Some(format!("sees {}", x.visible.join(", "))),
                None,
            )),
            (None, Some(y)) => sites.push(part(
                Change::Added,
                "room",
                &name,
                None,
                None,
                Some(format!("sees {}", y.visible.join(", "))),
            )),
            (None, None) => {}
        }
    }
    sites
}

// Walkmeshes.

/// Two walkmeshes: the header and counts, the vertices moved, the faces of
/// each surface material, the perimeter edges leading into each room, and
/// the arrays derived from the faces (planes, adjacency, perimeter, the
/// AABB tree) where they differ beyond what those sites say.
pub(super) fn walkmesh(a: &Walkmesh<'_>, b: &Walkmesh<'_>) -> Vec<Site> {
    let mut sites = Vec::new();
    header(
        &mut sites,
        "kind",
        format!("{:?}", a.kind),
        format!("{:?}", b.kind),
    );
    for (field, x, y) in [
        (
            "relative_points[0]",
            a.relative_points[0],
            b.relative_points[0],
        ),
        (
            "relative_points[1]",
            a.relative_points[1],
            b.relative_points[1],
        ),
        (
            "absolute_points[0]",
            a.absolute_points[0],
            b.absolute_points[0],
        ),
        (
            "absolute_points[1]",
            a.absolute_points[1],
            b.absolute_points[1],
        ),
        ("position", a.position, b.position),
    ] {
        if bits3(x) != bits3(y) {
            header(&mut sites, field, vec3(x), vec3(y));
        }
    }
    header(&mut sites, "unknown", a.unknown, b.unknown);
    header(&mut sites, "vertices", a.vertices.len(), b.vertices.len());
    header(&mut sites, "faces", a.faces.len(), b.faces.len());
    header(
        &mut sites,
        "walkable faces",
        a.adjacency.len(),
        b.adjacency.len(),
    );
    header(
        &mut sites,
        "perimeter loops",
        a.perimeters.len(),
        b.perimeters.len(),
    );
    header(
        &mut sites,
        "perimeter edges",
        a.perimeter_edges.len(),
        b.perimeter_edges.len(),
    );
    header(&mut sites, "aabb nodes", a.aabb.len(), b.aabb.len());
    if a.trailer != b.trailer {
        header(
            &mut sites,
            "trailer",
            format!("{} bytes", a.trailer.len()),
            format!("{} bytes", b.trailer.len()),
        );
    }
    if a.vertices.len() == b.vertices.len() && a.vertices.as_bytes() != b.vertices.as_bytes() {
        let moved = a
            .vertices
            .iter()
            .zip(b.vertices.iter())
            .filter(|(x, y)| bits3(*x) != bits3(*y))
            .count();
        sites.push(part(
            Change::Changed,
            "array",
            "vertices",
            None,
            None,
            Some(format!("{moved} of {} moved", a.vertices.len())),
        ));
    }
    if a.faces.len() == b.faces.len() {
        let rewired = a
            .faces
            .iter()
            .zip(&b.faces)
            .filter(|(x, y)| x.vertices != y.vertices)
            .count();
        if rewired > 0 {
            sites.push(part(
                Change::Changed,
                "array",
                "faces",
                None,
                None,
                Some(format!("{rewired} of {} on other vertices", a.faces.len())),
            ));
        }
        let rematerialled = a
            .faces
            .iter()
            .zip(&b.faces)
            .filter(|(x, y)| x.material != y.material)
            .count();
        if rematerialled > 0 {
            sites.push(part(
                Change::Changed,
                "array",
                "materials",
                None,
                None,
                Some(format!(
                    "{rematerialled} of {} faces of another material",
                    a.faces.len()
                )),
            ));
        }
    }
    let materials = |w: &Walkmesh<'_>| {
        let mut m: BTreeMap<u32, usize> = BTreeMap::new();
        for f in &w.faces {
            *m.entry(f.material).or_default() += 1;
        }
        m
    };
    counts(
        &mut sites,
        "material",
        &materials(a),
        &materials(b),
        "faces",
    );
    let transitions = |w: &Walkmesh<'_>| {
        let mut m: BTreeMap<u32, usize> = BTreeMap::new();
        for e in &w.perimeter_edges {
            if let Some(r) = e.transition {
                *m.entry(r).or_default() += 1;
            }
        }
        m
    };
    let (ta, tb) = (transitions(a), transitions(b));
    counts(&mut sites, "transition to room", &ta, &tb, "edges");
    let planes = |w: &Walkmesh<'_>| -> Vec<[u32; 4]> {
        w.faces
            .iter()
            .map(|f| {
                let n = bits3(f.normal);
                [n[0], n[1], n[2], f.distance.to_bits()]
            })
            .collect()
    };
    // Each node's floats by their bits, an absent index as -1.
    let aabb = |w: &Walkmesh<'_>| -> Vec<[u32; 11]> {
        let ix = |i: Option<u32>| i.unwrap_or(u32::MAX);
        w.aabb
            .iter()
            .map(|n| {
                let (lo, hi) = (bits3(n.min), bits3(n.max));
                [
                    lo[0],
                    lo[1],
                    lo[2],
                    hi[0],
                    hi[1],
                    hi[2],
                    ix(n.face),
                    n.unknown,
                    n.plane,
                    ix(n.left),
                    ix(n.right),
                ]
            })
            .collect()
    };
    // An array whose length changed differs as the counts above say.
    for (name, differs) in [
        (
            "planes",
            a.faces.len() == b.faces.len() && planes(a) != planes(b),
        ),
        (
            "adjacency",
            a.adjacency.len() == b.adjacency.len() && a.adjacency != b.adjacency,
        ),
        (
            "perimeter",
            a.perimeter_edges.len() == b.perimeter_edges.len()
                && (a.perimeter_edges != b.perimeter_edges || a.perimeters != b.perimeters)
                && ta == tb,
        ),
        ("aabb", a.aabb.len() == b.aabb.len() && aabb(a) != aabb(b)),
    ] {
        if differs {
            sites.push(part(Change::Changed, "array", name, None, None, None));
        }
    }
    sites
}

/// Counts by key, as parts added, removed or changed.
fn counts(
    sites: &mut Vec<Site>,
    kind: &str,
    a: &BTreeMap<u32, usize>,
    b: &BTreeMap<u32, usize>,
    what: &str,
) {
    let mut keys: Vec<&u32> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        let name = k.to_string();
        let shown = |n: usize| format!("{n} {what}");
        match (a.get(k), b.get(k)) {
            (Some(&x), Some(&y)) if x != y => sites.push(part(
                Change::Changed,
                kind,
                &name,
                None,
                Some(shown(x)),
                Some(shown(y)),
            )),
            (Some(&x), None) => {
                sites.push(part(
                    Change::Removed,
                    kind,
                    &name,
                    None,
                    Some(shown(x)),
                    None,
                ));
            }
            (None, Some(&y)) => {
                sites.push(part(Change::Added, kind, &name, None, None, Some(shown(y))));
            }
            _ => {}
        }
    }
}

// Models.

/// One side of a model: the view for meaning and the stored form for
/// equality.
pub(super) struct ModelPair<'a> {
    pub view: Model<'a>,
    pub file: ModelFile,
}

impl<'a> ModelPair<'a> {
    pub fn parse(mdl: &'a [u8], mdx: &'a [u8]) -> Result<Self, String> {
        let view = Model::parse(mdl, mdx).map_err(|e| e.to_string())?;
        let file = ModelFile::parse(mdl, mdx).map_err(|e| e.to_string())?;
        Ok(ModelPair { view, file })
    }
}

fn opt_name(n: Option<&str>) -> String {
    n.unwrap_or("(none)").to_owned()
}

fn mesh_text(m: &Mesh<'_>) -> String {
    format!(
        "texture {}, {} vertices, {} faces",
        opt_name(m.texture),
        m.vertex_count,
        m.faces.len()
    )
}

fn node_text(model: &Model<'_>, n: &Node<'_>) -> String {
    let mut s = format!("{:?}", n.kind);
    if let Some(p) = n.parent {
        s.push_str(&format!(" under {}", model.nodes[p].name));
    }
    if let Some(m) = &n.mesh {
        s.push_str(&format!(", {}", mesh_text(m)));
    }
    s
}

fn quat(q: rakata_mdl::Quaternion) -> String {
    format!("({}, {}, {}, {})", q.w, q.x, q.y, q.z)
}

fn quat_bits(q: rakata_mdl::Quaternion) -> [u32; 4] {
    [q.w, q.x, q.y, q.z].map(f32::to_bits)
}

fn attr_bits<const N: usize>(a: &rakata_mdl::Attribute<'_, N>) -> Vec<[u32; N]> {
    a.iter().map(|v| v.map(f32::to_bits)).collect()
}

/// A controller as a comparable value: type, field, columns, and the
/// stored times and values.
type ControllerKey<'n> = (u32, u16, u8, &'n [u8], &'n [u8]);

fn controllers<'n>(n: &'n Node<'_>) -> Vec<ControllerKey<'n>> {
    n.controllers
        .iter()
        .map(|c| (c.kind, c.field, c.columns, c.time_bytes(), c.value_bytes()))
        .collect()
}

fn mesh_sites(sites: &mut Vec<Site>, name: &str, x: &Mesh<'_>, y: &Mesh<'_>) {
    let text = |t: Option<&str>| opt_name(t).to_ascii_lowercase();
    if text(x.texture) != text(y.texture) {
        changed(
            sites,
            "node",
            name,
            "texture",
            opt_name(x.texture),
            opt_name(y.texture),
        );
    }
    if text(x.lightmap) != text(y.lightmap) {
        changed(
            sites,
            "node",
            name,
            "lightmap",
            opt_name(x.lightmap),
            opt_name(y.lightmap),
        );
    }
    let fields: [(&str, String, String); 9] = [
        ("render", x.render.to_string(), y.render.to_string()),
        ("shadow", x.shadow.to_string(), y.shadow.to_string()),
        (
            "lightmapped",
            x.lightmapped.to_string(),
            y.lightmapped.to_string(),
        ),
        (
            "texture_count",
            x.texture_count.to_string(),
            y.texture_count.to_string(),
        ),
        (
            "transparency_hint",
            x.transparency_hint.to_string(),
            y.transparency_hint.to_string(),
        ),
        (
            "vertices",
            x.vertex_count.to_string(),
            y.vertex_count.to_string(),
        ),
        (
            "faces",
            x.faces.len().to_string(),
            y.faces.len().to_string(),
        ),
        (
            "mdx_flags",
            format!("{:#x}", x.mdx_flags),
            format!("{:#x}", y.mdx_flags),
        ),
        (
            "indices",
            x.indices.map_or(0, |i| i.len()).to_string(),
            y.indices.map_or(0, |i| i.len()).to_string(),
        ),
    ];
    for (field, a, b) in fields {
        changed(sites, "node", name, field, a, b);
    }
    for (field, a, b) in [
        ("diffuse", x.diffuse, y.diffuse),
        ("ambient", x.ambient, y.ambient),
    ] {
        if bits3(a) != bits3(b) {
            changed(sites, "node", name, field, vec3(a), vec3(b));
        }
    }
    if x.vertex_count == y.vertex_count {
        let mut which = Vec::new();
        if attr_bits(&x.positions) != attr_bits(&y.positions) {
            which.push("positions");
        }
        if x.normals.as_ref().map(attr_bits) != y.normals.as_ref().map(attr_bits) {
            which.push("normals");
        }
        for (what, p, q) in [("uv1", &x.uv1, &y.uv1), ("uv2", &x.uv2, &y.uv2)] {
            if p.as_ref().map(attr_bits) != q.as_ref().map(attr_bits) {
                which.push(what);
            }
        }
        if which.is_empty() && x.vertex_buffer != y.vertex_buffer {
            which.push("other attributes");
        }
        if !which.is_empty() {
            sites.push(part(
                Change::Changed,
                "node",
                name,
                Some("vertex data"),
                None,
                Some(format!("{} changed", which.join(", "))),
            ));
        }
    }
    if x.faces.len() == y.faces.len() && x.faces.bytes() != y.faces.bytes() {
        let (mut corners, mut planes, mut surfaces, mut adjacency) = (0, 0, 0, 0);
        for (f, g) in x.faces.iter().zip(y.faces.iter()) {
            corners += usize::from(f.vertices != g.vertices);
            planes += usize::from(
                bits3(f.normal) != bits3(g.normal) || f.distance.to_bits() != g.distance.to_bits(),
            );
            surfaces += usize::from(f.surface != g.surface);
            adjacency += usize::from(f.adjacent != g.adjacent);
        }
        let which: Vec<String> = [
            (corners, "on other vertices"),
            (planes, "with another plane"),
            (surfaces, "of another surface"),
            (adjacency, "with other neighbours"),
        ]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, what)| format!("{n} {what}"))
        .collect();
        sites.push(part(
            Change::Changed,
            "node",
            name,
            Some("face data"),
            None,
            Some(format!("of {}, {}", x.faces.len(), which.join(", "))),
        ));
    }
}

/// Two nodes' controllers by type: the types added and removed, and those
/// whose keys differ. A geometry node's position and orientation
/// controllers repeat its header's, so they are left to those sites when
/// `moved` and `turned` say the header's differ.
fn controller_sites(
    sites: &mut Vec<Site>,
    name: &str,
    x: &Node<'_>,
    y: &Node<'_>,
    moved: bool,
    turned: bool,
) {
    fn by_kind<'n>(n: &'n Node<'_>) -> BTreeMap<u32, Vec<ControllerKey<'n>>> {
        let mut m: BTreeMap<u32, Vec<ControllerKey<'n>>> = BTreeMap::new();
        for k in controllers(n) {
            m.entry(k.0).or_default().push(k);
        }
        m
    }
    let (mx, my) = (by_kind(x), by_kind(y));
    let mut kinds: Vec<&u32> = mx.keys().chain(my.keys()).collect();
    kinds.sort();
    kinds.dedup();
    let (mut added, mut removed, mut keyed) = (Vec::new(), Vec::new(), Vec::new());
    for k in kinds {
        match (mx.get(k), my.get(k)) {
            (Some(_), None) => removed.push(k.to_string()),
            (None, Some(_)) => added.push(k.to_string()),
            (Some(a), Some(b)) if a != b => {
                let echoed = (*k == rakata_mdl::controller_types::POSITION && moved)
                    || (*k == rakata_mdl::controller_types::ORIENTATION && turned);
                if !echoed {
                    keyed.push(k.to_string());
                }
            }
            _ => {}
        }
    }
    let mut said = Vec::new();
    for (what, list) in [
        ("added", &added),
        ("removed", &removed),
        ("keys changed", &keyed),
    ] {
        if !list.is_empty() {
            said.push(format!("{what} {}", list.join(" ")));
        }
    }
    if !said.is_empty() {
        sites.push(part(
            Change::Changed,
            "node",
            name,
            Some("controllers"),
            None,
            Some(format!("types {}", said.join("; "))),
        ));
    }
}

fn node_sites(
    sites: &mut Vec<Site>,
    ma: &Model<'_>,
    mb: &Model<'_>,
    x: &Node<'_>,
    y: &Node<'_>,
    name: &str,
) {
    let parent = |m: &Model<'_>, n: &Node<'_>| n.parent.map(|p| m.nodes[p].name.to_owned());
    let (px, py) = (parent(ma, x), parent(mb, y));
    if px.as_deref().map(str::to_ascii_lowercase) != py.as_deref().map(str::to_ascii_lowercase) {
        changed(
            sites,
            "node",
            name,
            "parent",
            px.unwrap_or_else(|| "(root)".into()),
            py.unwrap_or_else(|| "(root)".into()),
        );
    }
    changed(
        sites,
        "node",
        name,
        "kind",
        format!("{:?}", x.kind),
        format!("{:?}", y.kind),
    );
    if bits3(x.position) != bits3(y.position) {
        changed(
            sites,
            "node",
            name,
            "position",
            vec3(x.position),
            vec3(y.position),
        );
    }
    if quat_bits(x.orientation) != quat_bits(y.orientation) {
        changed(
            sites,
            "node",
            name,
            "orientation",
            quat(x.orientation),
            quat(y.orientation),
        );
    }
    match (&x.mesh, &y.mesh) {
        (Some(mx), Some(my)) => mesh_sites(sites, name, mx, my),
        (Some(mx), None) => changed(sites, "node", name, "mesh", mesh_text(mx), "(none)".into()),
        (None, Some(my)) => changed(sites, "node", name, "mesh", "(none)".into(), mesh_text(my)),
        (None, None) => {}
    }
    match (&x.skin, &y.skin) {
        (Some(sx), Some(sy)) => {
            if sx.bones != sy.bones
                || sx.bone_map_bytes() != sy.bone_map_bytes()
                || attr_bits(&sx.weights) != attr_bits(&sy.weights)
                || attr_bits(&sx.bone_indices) != attr_bits(&sy.bone_indices)
            {
                sites.push(part(
                    Change::Changed,
                    "node",
                    name,
                    Some("skin"),
                    None,
                    None,
                ));
            }
        }
        (None, None) => {}
        _ => changed(
            sites,
            "node",
            name,
            "skin",
            x.skin.is_some().to_string(),
            y.skin.is_some().to_string(),
        ),
    }
    if controllers(x) != controllers(y) {
        controller_sites(
            sites,
            name,
            x,
            y,
            bits3(x.position) != bits3(y.position),
            quat_bits(x.orientation) != quat_bits(y.orientation),
        );
    }
}

fn anim_text(a: &Animation<'_>) -> String {
    format!(
        "{} s, {} nodes, {} events",
        a.length,
        a.nodes.len(),
        a.events.len()
    )
}

fn anim_sites(sites: &mut Vec<Site>, x: &Animation<'_>, y: &Animation<'_>, name: &str) {
    if x.length.to_bits() != y.length.to_bits() {
        changed(
            sites,
            "animation",
            name,
            "length",
            x.length.to_string(),
            y.length.to_string(),
        );
    }
    if x.transition.to_bits() != y.transition.to_bits() {
        changed(
            sites,
            "animation",
            name,
            "transition",
            x.transition.to_string(),
            y.transition.to_string(),
        );
    }
    changed(
        sites,
        "animation",
        name,
        "root",
        x.root.to_owned(),
        y.root.to_owned(),
    );
    let events = |a: &Animation<'_>| {
        a.events
            .iter()
            .map(|e| format!("{} {}", e.time, e.name))
            .collect::<Vec<_>>()
            .join("; ")
    };
    changed(sites, "animation", name, "events", events(x), events(y));
    let nx: Vec<&str> = x.nodes.iter().map(|n| n.name).collect();
    let ny: Vec<&str> = y.nodes.iter().map(|n| n.name).collect();
    let (mut added, mut removed, mut keyed) = (Vec::new(), Vec::new(), 0usize);
    for (i, j, node) in pair_names(&nx, &ny) {
        match (i, j) {
            (Some(i), Some(j)) => {
                if controllers(&x.nodes[i]) != controllers(&y.nodes[j]) {
                    keyed += 1;
                }
            }
            (Some(_), None) => removed.push(node),
            (None, Some(_)) => added.push(node),
            (None, None) => {}
        }
    }
    if !added.is_empty() || !removed.is_empty() {
        sites.push(part(
            Change::Changed,
            "animation",
            name,
            Some("nodes"),
            (!removed.is_empty()).then(|| format!("{} removed", removed.join(", "))),
            (!added.is_empty()).then(|| format!("{} added", added.join(", "))),
        ));
    }
    if keyed > 0 {
        sites.push(part(
            Change::Changed,
            "animation",
            name,
            Some("keys"),
            None,
            Some(format!("changed on {keyed} nodes")),
        ));
    }
}

fn without_padding(n: &NodeFile) -> NodeFile {
    let mut n = n.clone();
    if let rakata_mdl::Body::Mesh(m) = &mut n.body
        && let Some(b) = &mut m.mdx
    {
        b.padding.clear();
    }
    n.children.clear();
    n
}

/// Two models, each its MDL and MDX read as one: the header, the nodes by
/// name (added, removed, moved to another parent, and each field that
/// differs, a mesh's textures and counts among them), and the animations
/// by name. A difference in a stored field the view does not read is named
/// by the node or animation that holds it.
pub(super) fn model(a: &ModelPair<'_>, b: &ModelPair<'_>) -> Vec<Site> {
    if a.file.semantic_eq(&b.file) {
        return Vec::new();
    }
    let (ma, mb) = (&a.view, &b.view);
    let mut sites = Vec::new();
    header(&mut sites, "name", ma.name, mb.name);
    header(
        &mut sites,
        "classification",
        format!("{:?}", ma.classification),
        format!("{:?}", mb.classification),
    );
    header(
        &mut sites,
        "subclassification",
        ma.subclassification,
        mb.subclassification,
    );
    header(&mut sites, "fog", ma.fog, mb.fog);
    header(
        &mut sites,
        "supermodel",
        opt_name(ma.supermodel),
        opt_name(mb.supermodel),
    );
    float(
        &mut sites,
        "animation_scale",
        ma.animation_scale,
        mb.animation_scale,
    );
    float(&mut sites, "radius", ma.radius, mb.radius);
    let bb = |m: &Model<'_>| (bits3(m.bounding_box.min), bits3(m.bounding_box.max));
    if bb(ma) != bb(mb) {
        let shown = |m: &Model<'_>| {
            format!(
                "{} to {}",
                vec3(m.bounding_box.min),
                vec3(m.bounding_box.max)
            )
        };
        header(&mut sites, "bounding_box", shown(ma), shown(mb));
    }
    header(&mut sites, "nodes", ma.nodes.len(), mb.nodes.len());
    header(
        &mut sites,
        "meshes",
        ma.meshes().count(),
        mb.meshes().count(),
    );
    header(
        &mut sites,
        "animations",
        ma.animations.len(),
        mb.animations.len(),
    );
    let na: Vec<&str> = ma.nodes.iter().map(|n| n.name).collect();
    let nb: Vec<&str> = mb.nodes.iter().map(|n| n.name).collect();
    let mut paired = Vec::new();
    for (i, j, name) in pair_names(&na, &nb) {
        match (i, j) {
            (Some(i), Some(j)) => {
                paired.push((i, j, name.clone()));
                node_sites(&mut sites, ma, mb, &ma.nodes[i], &mb.nodes[j], &name);
            }
            (Some(i), None) => sites.push(part(
                Change::Removed,
                "node",
                &name,
                None,
                Some(node_text(ma, &ma.nodes[i])),
                None,
            )),
            (None, Some(j)) => sites.push(part(
                Change::Added,
                "node",
                &name,
                None,
                None,
                Some(node_text(mb, &mb.nodes[j])),
            )),
            (None, None) => {}
        }
    }
    let aa: Vec<&str> = ma.animations.iter().map(|x| x.name).collect();
    let ab: Vec<&str> = mb.animations.iter().map(|x| x.name).collect();
    let mut anims = Vec::new();
    for (i, j, name) in pair_names(&aa, &ab) {
        match (i, j) {
            (Some(i), Some(j)) => {
                anims.push((i, j, name.clone()));
                anim_sites(&mut sites, &ma.animations[i], &mb.animations[j], &name);
            }
            (Some(i), None) => sites.push(part(
                Change::Removed,
                "animation",
                &name,
                None,
                Some(anim_text(&ma.animations[i])),
                None,
            )),
            (None, Some(j)) => sites.push(part(
                Change::Added,
                "animation",
                &name,
                None,
                None,
                Some(anim_text(&mb.animations[j])),
            )),
            (None, None) => {}
        }
    }
    if sites.is_empty() {
        // The data differs in a field the view does not read: name where.
        let (fa, fb) = (a.file.nodes(), b.file.nodes());
        for (i, j, name) in &paired {
            if let (Some(x), Some(y)) = (fa.get(*i), fb.get(*j))
                && without_padding(x) != without_padding(y)
            {
                sites.push(part(
                    Change::Changed,
                    "node",
                    name,
                    Some("stored fields"),
                    None,
                    None,
                ));
            }
        }
        for (i, j, name) in &anims {
            let (x, y) = (&a.file.animations[*i], &b.file.animations[*j]);
            if x != y {
                sites.push(part(
                    Change::Changed,
                    "animation",
                    name,
                    Some("stored fields"),
                    None,
                    None,
                ));
            }
        }
        if a.file.header != b.file.header
            || a.file.head_node != b.file.head_node
            || a.file.names != b.file.names
            || sites.is_empty()
        {
            sites.push(part(
                Change::Changed,
                "array",
                "model header",
                Some("stored fields"),
                None,
                None,
            ));
        }
    }
    sites
}
