//! `rakata walkmesh show`: a WOK's, PWK's or DWK's counts, the faces of
//! each surface material (labelled through the install's
//! `surfacemat.2da`), the edges leading into each room, and its bounds,
//! over `rakata-walkmesh`.

use std::collections::BTreeMap;

use clap::{Args, Subcommand};
use serde_json::{Value as Json, json};

use super::Ctx;
use super::inspect::{self, LookupArgs, count, point, resource_forms};
use crate::Exit;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::inspect::diagnostic;
use rakata::walkmesh::{Kind, SurfaceMaterials, Walkmesh};

const SHOW: &str = concat!(
    "\
Prints the walkmesh's kind and counts (vertices, faces, the walkable faces,
perimeter edges and their loops, AABB nodes), the bounds of its vertices,
then one line per surface material with its face count, then one line per
room a perimeter edge leads into with its edge count. A DWK or PWK also
prints its two hook points and the position in its header. With --game the
materials are labelled, and the walkable faces counted, through the
install's surfacemat.2da; without it they are numbers. --format json gives
the same fields.

",
    resource_forms!(),
    "

Examples:
  rakata walkmesh show m01aa_01a.wok --game DIR
  rakata walkmesh show ./door.dwk
  rakata --format json walkmesh show m01aa_01a.wok --game DIR"
);

#[derive(Args, Debug)]
#[command(after_help = "\
Example:
  rakata walkmesh show m01aa_01a.wok --game DIR")]
pub struct WalkmeshArgs {
    #[command(subcommand)]
    pub action: WalkmeshAction,
}

#[derive(Subcommand, Debug)]
pub enum WalkmeshAction {
    /// Print a walkmesh's counts, materials, room transitions and bounds.
    #[command(after_help = SHOW)]
    Show {
        /// The walkmesh: NAME.wok, NAME.pwk or NAME.dwk, a PATH or an ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        #[command(flatten)]
        lookup: LookupArgs,
    },
}

pub fn run(ctx: &Ctx, args: &WalkmeshArgs) -> Exit {
    let WalkmeshAction::Show { name, lookup } = &args.action;
    let lookup = lookup.lookup(ctx);
    let mut env = ctx.envelope("walkmesh show");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let loaded = match inspect::load(&mut env, "resource", name, &lookup) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let ext = loaded.extension().unwrap_or_default();
    if !["wok", "pwk", "dwk"].contains(&ext.as_str()) {
        return fail(
            env,
            inspect::unread_type(&loaded, "walkmesh show", "a .wok, .pwk or .dwk"),
        );
    }
    let w = match Walkmesh::parse(&loaded.bytes) {
        Ok(w) => w,
        Err(e) => {
            let d = diagnostic(e.code(), &e).note(format!("while reading `{}`", loaded.source));
            return fail(env, d);
        }
    };
    let mut diags = Vec::new();
    let table = if lookup.game.is_some() {
        match surface_materials(&mut env, &lookup) {
            Ok(t) => Some(t),
            Err(d) => {
                diags.push(d);
                None
            }
        }
    } else {
        None
    };
    let (text, mut result) = show(&w, &ext, table.as_ref());
    result["source"] = json!(loaded.source);
    env.result = result;
    ctx.finish(env, &text, diags, &map, Exit::Ok)
}

/// The install's `surfacemat.2da`, read; a warning when it has none that
/// reads.
fn surface_materials(
    env: &mut rakata::Envelope,
    lookup: &rakata::inspect::Lookup<'_>,
) -> Result<SurfaceMaterials, Diagnostic> {
    let unlabelled = |why: String| {
        Diagnostic::new(&rakata::codes::RW1301, why)
            .note("the materials are shown by number, and walkable faces are not counted")
    };
    let loaded = match inspect::load_if_found(env, "surfacemat", "surfacemat.2da", lookup) {
        Ok(Some(l)) => l,
        Ok(None) => return Err(unlabelled("the install holds no surfacemat.2da".into())),
        Err(d) => return Err(unlabelled(format!("surfacemat.2da: {}", d.message))),
    };
    SurfaceMaterials::parse(&loaded.bytes)
        .map_err(|e| unlabelled(format!("`{}` does not read: {e}", loaded.source)))
}

fn show(w: &Walkmesh<'_>, ext: &str, table: Option<&SurfaceMaterials>) -> (String, Json) {
    let kind = match w.kind {
        Kind::Area => "area room",
        Kind::Object => "object",
    };
    let walkable = table.map(|t| w.walkable_faces(t).count());
    let loops = w.perimeters.len();
    let mut text = format!(
        "walkmesh {ext} ({kind}), {}, {}{}, {} in {}, {}",
        count(w.vertices.len(), "vertex"),
        count(w.faces.len(), "face"),
        walkable
            .map(|n| format!(" ({n} walkable)"))
            .unwrap_or_default(),
        count(w.perimeter_edges.len(), "perimeter edge"),
        count(loops, "loop"),
        count(w.aabb.len(), "AABB node"),
    );
    let bounds = w
        .vertices
        .iter()
        .fold(None, |b: Option<([f32; 3], [f32; 3])>, v| {
            let (mut lo, mut hi) = b.unwrap_or((v, v));
            for i in 0..3 {
                lo[i] = lo[i].min(v[i]);
                hi[i] = hi[i].max(v[i]);
            }
            Some((lo, hi))
        });
    if let Some((lo, hi)) = bounds {
        text.push_str(&format!("\nbounds {} to {}", point(lo), point(hi)));
    }
    if w.kind == Kind::Object {
        text.push_str(&format!(
            "\nhooks {} and {}, absolute {} and {}, position {}",
            point(w.relative_points[0]),
            point(w.relative_points[1]),
            point(w.absolute_points[0]),
            point(w.absolute_points[1]),
            point(w.position)
        ));
    }
    let mut materials: BTreeMap<u32, usize> = BTreeMap::new();
    for f in &w.faces {
        *materials.entry(f.material).or_default() += 1;
    }
    if !materials.is_empty() {
        text.push_str("\nmaterials");
    }
    let mut materials_json = Vec::new();
    for (&m, &n) in &materials {
        let row = table.and_then(|t| t.get(m));
        text.push_str(&format!("\n  {m}"));
        if let Some(r) = row {
            text.push_str(&format!(
                " {}{}",
                r.label,
                if r.walkable { " walkable" } else { "" }
            ));
        }
        text.push_str(&format!("  {}", count(n, "face")));
        materials_json.push(json!({
            "material": m,
            "label": row.map(|r| r.label.clone()),
            "walkable": row.map(|r| r.walkable),
            "faces": n,
        }));
    }
    let mut transitions: BTreeMap<u32, usize> = BTreeMap::new();
    for e in &w.perimeter_edges {
        if let Some(room) = e.transition {
            *transitions.entry(room).or_default() += 1;
        }
    }
    if !transitions.is_empty() {
        text.push_str("\ntransitions");
    }
    for (room, n) in &transitions {
        text.push_str(&format!("\n  room {room}  {}", count(*n, "edge")));
    }
    let json = json!({
        "format": ext,
        "kind": match w.kind { Kind::Area => "area", Kind::Object => "object" },
        "vertices": w.vertices.len(),
        "faces": w.faces.len(),
        "walkable_faces": walkable,
        "adjacency": w.adjacency.len(),
        "perimeter_edges": w.perimeter_edges.len(),
        "perimeter_loops": loops,
        "aabb_nodes": w.aabb.len(),
        "bounds": bounds.map(|(lo, hi)| json!({"min": lo, "max": hi})),
        "relative_points": w.relative_points,
        "absolute_points": w.absolute_points,
        "position": w.position,
        "unknown": w.unknown,
        "materials": materials_json,
        "transitions": transitions.iter().map(|(room, n)| json!({
            "room": room,
            "edges": n,
        })).collect::<Vec<_>>(),
    });
    (text, json)
}
