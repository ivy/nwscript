//! `rakata mdl show`: a binary model read with its MDX, its node tree with
//! each node's kind, each mesh's textures and counts, each light's,
//! emitter's and saber's header, its animations, and its supermodel chain
//! with the animations it inherits, over `rakata-mdl`.

use clap::{Args, Subcommand};
use serde_json::{Value as Json, json};

use super::Ctx;
use super::inspect::{self, LookupArgs, count, point, resource_forms};
use crate::Exit;
use rakata::Envelope;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::inspect::{Loaded, Lookup, diagnostic};
use rakata::mdl::{
    Animation, Chain, ChainError, Classification, Controller, Emitter, Light, Lineage, Model, Node,
    NodeKind, Saber, controller_types,
};

const SHOW: &str = concat!(
    "\
A model is its MDL and its MDX, the file of the same name with the other
extension, found the way the MDL was: through the install for a NAME.EXT,
beside the file for a PATH, in the same archive for a member. Either may be
named. Prints the model's name, classification, supermodel and counts, its
bounds, then the node tree, each node indented under its parent with its kind,
and for a mesh its texture, lightmap, vertex and face counts. A light, an
emitter and a saber blade each get a second line: a light's colour, radius,
multiplier, priority, flags and flare textures; an emitter's update (how its
particles are born), render, blend, texture or chunk model, frame grid,
birth rate, life and frames; a blade's extent and its two words. Then each
animation with its length, node count and events. A supermodel is looked up
by name through the install with --game, else beside the model; the chain
prints each model of it and where it was found, then the animations the
model inherits and from which.

--controllers prints every controller in full under its node, by its name
on the node's kind (`birth_rate` on an emitter, `radius` on a light; `type N`
where the meaning is not established), and every animated node's keys under
its animation.

--format json gives one document with the same fields: each node's number,
transform, every controller with its keys, and each light's, emitter's and
saber's header in full (a blade's vertices included); with --controllers,
each animation's nodes and their keys too.

",
    resource_forms!(),
    "

Examples:
  rakata mdl show m01aa_01a.mdl --game DIR
  rakata mdl show c_rancor.mdl --game DIR
  rakata mdl show v_muzflash_01.mdl --controllers --game DIR
  rakata --format json mdl show ./box.mdl"
);

#[derive(Args, Debug)]
#[command(after_help = "\
Example:
  rakata mdl show m01aa_01a.mdl --game DIR")]
pub struct MdlArgs {
    #[command(subcommand)]
    pub action: MdlAction,
}

#[derive(Subcommand, Debug)]
pub enum MdlAction {
    /// Print a model's node tree, meshes, animations and supermodel chain.
    #[command(after_help = SHOW)]
    Show {
        /// The model: NAME.mdl (or NAME.mdx), a PATH or an ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        /// Print every controller's keys, by name, under its node and
        /// under each animation.
        #[arg(long)]
        controllers: bool,
        #[command(flatten)]
        lookup: LookupArgs,
    },
}

/// One model of the chain past the first: its name and where its MDL lay.
struct Found {
    name: String,
    source: String,
}

pub fn run(ctx: &Ctx, args: &MdlArgs) -> Exit {
    let MdlAction::Show {
        name,
        controllers,
        lookup,
    } = &args.action;
    let lookup = lookup.lookup(ctx);
    let mut env = ctx.envelope("mdl show");
    let map = SourceMap::new();
    let fail = |env, diags| ctx.finish(env, "", diags, &map, Exit::Ok);
    let given = match inspect::load(&mut env, "resource", name, &lookup) {
        Ok(l) => l,
        Err(d) => return fail(env, vec![d]),
    };
    let ext = given.extension().unwrap_or_default();
    if ext != "mdl" && ext != "mdx" {
        let d = inspect::unread_type(&given, "mdl show", "a .mdl or its .mdx");
        return fail(env, vec![d]);
    }
    let mut diags = Vec::new();
    let other = if ext == "mdl" { "mdx" } else { "mdl" };
    let partner = match inspect::with_extension(name, other) {
        Some(n) => match inspect::load_if_found(&mut env, other, &n, &lookup) {
            Ok(p) => p,
            Err(d) => return fail(env, vec![d]),
        },
        None => None,
    };
    let (mdl, mdx) = match (ext.as_str(), partner) {
        ("mdl", p) => {
            if p.is_none() {
                diags.push(Diagnostic::new(
                    &rakata::codes::RW1302,
                    format!(
                        "`{}` has no MDX beside it: the model is read without vertex data",
                        given.source
                    ),
                ));
            }
            (given, p)
        }
        (_, Some(m)) => (m, Some(given)),
        (_, None) => {
            let d = Diagnostic::new(
                &rakata::codes::RE1307,
                format!("`{}` is an MDX with no MDL beside it", given.source),
            )
            .help("an MDX is read with its MDL: name the MDL, or put it beside the MDX");
            return fail(env, vec![d]);
        }
    };
    let mdx_bytes: &[u8] = mdx.as_ref().map_or(&[], |m| &m.bytes);
    let model = match Model::parse(&mdl.bytes, mdx_bytes) {
        Ok(m) => m,
        Err(e) => {
            diags.push(diagnostic(e.code(), &e).note(format!("while reading `{}`", mdl.source)));
            return fail(env, diags);
        }
    };
    let (chain, found) = match supermodels(&mut env, name, &mdl, mdx_bytes, &lookup) {
        Ok((c, f)) => (Some(c), f),
        Err(d) => {
            diags.push(d);
            (None, Vec::new())
        }
    };
    let missing = chain.as_ref().and_then(|c| c.missing.clone());
    if let Some(m) = &missing {
        let mut d = Diagnostic::new(
            &rakata::codes::RW1303,
            format!(
                "the supermodel `{m}` is not found: the chain ends before it, and what it would give is not shown"
            ),
        );
        if lookup.game.is_none() {
            d = d.help("a supermodel is found through the install: name it with --game DIR");
        }
        diags.push(d);
    }
    let mut supers = Vec::new();
    if let Some(chain) = &chain {
        match chain.parse() {
            Ok(models) => supers = models,
            Err(e) => diags.push(diagnostic(e.code(), &e).note("while reading a supermodel")),
        }
    }
    let mut lineage_models = vec![model.clone()];
    lineage_models.extend(supers.into_iter().skip(1));
    let inherited: Vec<(usize, String)> = match Lineage::new(&lineage_models) {
        Ok(l) => l
            .animations()
            .into_iter()
            .filter(|(i, _)| *i > 0)
            .map(|(i, a)| (i, a.name.to_owned()))
            .collect(),
        Err(e) => {
            diags.push(diagnostic(e.code(), &e));
            Vec::new()
        }
    };
    let (text, mut result) = show(
        &model,
        &lineage_models,
        &found,
        missing.as_deref(),
        &inherited,
        *controllers,
    );
    result["source"] = json!(mdl.source);
    result["mdx"] = json!(mdx.as_ref().map(|m| m.source.clone()));
    env.result = result;
    ctx.finish(env, &text, diags, &map, Exit::Ok)
}

/// The model's chain of supermodels, the model first, each looked up by
/// name through the install and else beside `named`, the name the model
/// was given by; with where each supermodel was found.
fn supermodels(
    env: &mut Envelope,
    named: &str,
    mdl: &Loaded,
    mdx: &[u8],
    lookup: &Lookup<'_>,
) -> Result<(Chain, Vec<Found>), Diagnostic> {
    let mut found = Vec::new();
    let mut first = true;
    let stem = mdl
        .name
        .rsplit_once('.')
        .map_or(mdl.name.as_str(), |(s, _)| s);
    let loaded: Result<Chain, ChainError<Diagnostic>> = Chain::load(stem, |name: &str| {
        if std::mem::take(&mut first) {
            return Ok(Some((mdl.bytes.clone(), mdx.to_vec())));
        }
        let file = format!("{name}.mdl");
        let mut candidates = Vec::new();
        if lookup.game.is_some() {
            candidates.push(file.clone());
        }
        let near = inspect::beside(named, &file);
        if !candidates.contains(&near) {
            candidates.push(near);
        }
        for c in candidates {
            if let Some(m) = inspect::load_if_found(env, "supermodel", &c, lookup)? {
                let x = match inspect::with_extension(&c, "mdx") {
                    Some(n) => inspect::load_if_found(env, "supermodel mdx", &n, lookup)?,
                    None => None,
                };
                found.push(Found {
                    name: name.to_owned(),
                    source: m.source,
                });
                return Ok(Some((m.bytes, x.map(|x| x.bytes).unwrap_or_default())));
            }
        }
        Ok(None)
    });
    match loaded {
        Ok(c) => Ok((c, found)),
        Err(ChainError::Load { error, name }) => {
            Err(error.note(format!("while reading the supermodel `{name}`")))
        }
        Err(ChainError::Model(e)) => Err(diagnostic(e.code(), &e)),
    }
}

fn classification(c: Classification) -> String {
    match c {
        Classification::Other => "other".into(),
        Classification::Effect => "effect".into(),
        Classification::Tile => "tile".into(),
        Classification::Character => "character".into(),
        Classification::Door => "door".into(),
        Classification::Lightsaber => "lightsaber".into(),
        Classification::Placeable => "placeable".into(),
        Classification::Unknown(b) => format!("unknown ({b})"),
    }
}

fn kind(k: NodeKind) -> String {
    match k {
        NodeKind::Dummy => "dummy".into(),
        NodeKind::Light => "light".into(),
        NodeKind::Emitter => "emitter".into(),
        NodeKind::Reference => "reference".into(),
        NodeKind::Trimesh => "trimesh".into(),
        NodeKind::Skin => "skin".into(),
        NodeKind::AnimMesh => "animmesh".into(),
        NodeKind::Dangly => "dangly".into(),
        NodeKind::Aabb => "aabb".into(),
        NodeKind::Saber => "saber".into(),
        NodeKind::Other(f) => format!("other ({f:#05x})"),
    }
}

/// One node's line: its kind, and its mesh's textures and counts.
fn node_line(n: &Node<'_>) -> String {
    let mut line = format!("{}  {}", n.name, kind(n.kind));
    if let Some(m) = &n.mesh {
        if let Some(t) = m.texture {
            line.push_str(&format!("  texture {t}"));
        }
        if let Some(l) = m.lightmap {
            line.push_str(&format!("  lightmap {l}"));
        }
        line.push_str(&format!(
            "  {}  {}",
            count(m.vertex_count, "vertex"),
            count(m.faces.len(), "face")
        ));
        if !m.render {
            line.push_str("  not rendered");
        }
    }
    if let Some(s) = &n.skin {
        line.push_str(&format!("  {}", count(s.bones.len(), "bone")));
    }
    line
}

/// A float as the text prints it: the shortest form that reads back.
fn num(x: f32) -> String {
    format!("{x}")
}

fn nums(v: &[f32]) -> String {
    v.iter().map(|&x| num(x)).collect::<Vec<_>>().join(" ")
}

/// A controller's name on a node of kind `kind`, or `type N`.
fn controller_name(kind: NodeKind, c: &Controller<'_>) -> String {
    controller_types::name(kind, c.kind).map_or_else(|| format!("type {}", c.kind), str::to_owned)
}

/// Key `i`'s value: an orientation's rotation `w, x, y, z`, a compressed
/// one decoded; else its row of floats as stored.
fn key_value(c: &Controller<'_>, i: usize) -> Vec<f32> {
    match c.orientation(i) {
        Some(k) => vec![k.value.w, k.value.x, k.value.y, k.value.z],
        None => c.floats(i).map(Iterator::collect).unwrap_or_default(),
    }
}

/// The first key's value of the node's controller of type `ty`.
fn rest(n: &Node<'_>, ty: u32) -> Option<Vec<f32>> {
    n.controller(ty)
        .filter(|c| !c.is_empty())
        .map(|c| key_value(c, 0))
}

fn light_line(n: &Node<'_>, l: &Light<'_>) -> String {
    let mut parts = Vec::new();
    for (ty, label) in [
        (controller_types::light::COLOR, "color"),
        (controller_types::light::RADIUS, "radius"),
        (controller_types::light::MULTIPLIER, "multiplier"),
    ] {
        if let Some(v) = rest(n, ty) {
            parts.push(format!("{label} {}", nums(&v)));
        }
    }
    parts.push(format!("priority {}", l.priority));
    for (on, label) in [
        (l.ambient_only, "ambient only"),
        (l.shadow, "shadow"),
        (l.fading, "fading"),
    ] {
        if on {
            parts.push(label.to_owned());
        }
    }
    if !l.flares.is_empty() {
        let names: Vec<&str> = l.flares.iter().map(|f| f.texture).collect();
        parts.push(format!(
            "flares {} (radius {})",
            names.join(", "),
            num(l.flare_radius)
        ));
    }
    parts.join("  ")
}

fn emitter_line(n: &Node<'_>, e: &Emitter<'_>) -> String {
    use controller_types::emitter as t;
    let mut parts = vec![
        format!("update {}", e.update),
        format!("render {}", e.render),
        format!("blend {}", e.blend),
    ];
    if let Some(x) = e.texture {
        parts.push(format!("texture {x}"));
    }
    if let Some(x) = e.chunk {
        parts.push(format!("chunk {x}"));
    }
    parts.push(format!("grid {}x{}", e.x_grid, e.y_grid));
    if let Some(v) = rest(n, t::BIRTH_RATE) {
        parts.push(format!("birth rate {}", nums(&v)));
    }
    if let Some(v) = rest(n, t::LIFE_EXPECTANCY) {
        parts.push(format!("life {}", nums(&v)));
    }
    if let (Some(a), Some(b)) = (rest(n, t::FRAME_START), rest(n, t::FRAME_END)) {
        parts.push(format!("frames {} to {}", nums(&a), nums(&b)));
    }
    if e.two_sided {
        parts.push("two-sided".to_owned());
    }
    parts.join("  ")
}

fn saber_line(s: &Saber<'_>) -> String {
    let zs: Vec<f32> = s.positions.iter().map(|p| p[2]).collect();
    let lo = zs.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = zs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut line = format!("blade {}", count(s.positions.len(), "vertex"));
    if !zs.is_empty() {
        line.push_str(&format!(" along z {} to {}", num(lo), num(hi)));
    }
    line.push_str(&format!("  words {} {}", s.words[0], s.words[1]));
    line
}

/// The line under a light, an emitter or a saber node.
fn detail_line(n: &Node<'_>) -> Option<String> {
    if let Some(l) = &n.light {
        Some(light_line(n, l))
    } else if let Some(e) = &n.emitter {
        Some(emitter_line(n, e))
    } else {
        n.saber.as_ref().map(saber_line)
    }
}

/// Every controller of `n`, one line each, indented by `pad`.
fn controller_lines(text: &mut String, kind: NodeKind, n: &Node<'_>, pad: &str) {
    for c in &n.controllers {
        let keys: Vec<String> = (0..c.len())
            .map(|i| {
                let v = nums(&key_value(c, i));
                if c.len() == 1 && c.time(0) == Some(0.0) {
                    v
                } else {
                    format!("{}: {v}", num(c.time(i).unwrap_or_default()))
                }
            })
            .collect();
        let bezier = if c.is_bezier() { " (bezier)" } else { "" };
        text.push_str(&format!(
            "\n{pad}{}{bezier}  {}",
            controller_name(kind, c),
            keys.join(";  ")
        ));
    }
}

fn tree(text: &mut String, nodes: &[Node<'_>], at: usize, depth: usize, controllers: bool) {
    let n = &nodes[at];
    let pad = "  ".repeat(depth + 1);
    text.push_str(&format!("\n{pad}{}", node_line(n)));
    if let Some(d) = detail_line(n) {
        text.push_str(&format!("\n{pad}    {d}"));
    }
    if controllers {
        controller_lines(text, n.kind, n, &format!("{pad}    "));
    }
    for &c in &n.children {
        tree(text, nodes, c, depth + 1, controllers);
    }
}

fn controller_json(kind: NodeKind, c: &Controller<'_>) -> Json {
    json!({
        "type": c.kind,
        "name": controller_types::name(kind, c.kind),
        "columns": c.columns,
        "bezier": c.is_bezier(),
        "compressed": c.is_compressed(),
        "keys": (0..c.len()).map(|i| json!({
            "time": c.time(i),
            "value": key_value(c, i),
        })).collect::<Vec<_>>(),
    })
}

fn light_json(l: &Light<'_>) -> Json {
    json!({
        "flare_radius": l.flare_radius,
        "flares": l.flares.iter().map(|f| json!({
            "texture": f.texture,
            "size": f.size,
            "position": f.position,
            "color_shift": f.color_shift,
        })).collect::<Vec<_>>(),
        "priority": l.priority,
        "ambient_only": l.ambient_only,
        "dynamic_type": l.dynamic_type,
        "affect_dynamic": l.affect_dynamic,
        "shadow": l.shadow,
        "generate_flare": l.generate_flare,
        "fading": l.fading,
    })
}

fn emitter_json(e: &Emitter<'_>) -> Json {
    json!({
        "dead_space": e.dead_space,
        "blast_radius": e.blast_radius,
        "blast_length": e.blast_length,
        "lightning_words": e.lightning_words,
        "x_grid": e.x_grid,
        "y_grid": e.y_grid,
        "space_type": e.space_type,
        "update": e.update,
        "render": e.render,
        "blend": e.blend,
        "texture": e.texture,
        "chunk": e.chunk,
        "two_sided": e.two_sided,
        "looping": e.looping,
        "render_order": e.render_order,
        "name_187": e.name_187,
        "flags": e.flags,
    })
}

fn saber_json(s: &Saber<'_>) -> Json {
    json!({
        "positions": s.positions.iter().collect::<Vec<_>>(),
        "uv": s.uv.iter().collect::<Vec<_>>(),
        "normals": s.normals.iter().collect::<Vec<_>>(),
        "words": s.words,
    })
}

/// The kind of the model's node an animation node animates: the geometry
/// node of the same number, else the animation node's own (a dummy).
fn animated_kind(m: &Model<'_>, n: &Node<'_>) -> NodeKind {
    m.nodes
        .iter()
        .find(|g| g.number == n.number)
        .map_or(n.kind, |g| g.kind)
}

/// An animation's nodes that hold keys, with their controllers.
fn animation_nodes_json(m: &Model<'_>, a: &Animation<'_>) -> Json {
    a.nodes
        .iter()
        .filter(|n| !n.controllers.is_empty())
        .map(|n| {
            let kind = animated_kind(m, n);
            json!({
                "name": n.name,
                "number": n.number,
                "controllers": n.controllers.iter().map(|c| controller_json(kind, c)).collect::<Vec<_>>(),
            })
        })
        .collect()
}

fn node_json(n: &Node<'_>) -> Json {
    json!({
        "name": n.name,
        "kind": kind(n.kind),
        "flags": n.flags,
        "number": n.number,
        "parent": n.parent,
        "children": n.children,
        "position": n.position,
        "orientation": [n.orientation.w, n.orientation.x, n.orientation.y, n.orientation.z],
        "controllers": n.controllers.iter().map(|c| controller_json(n.kind, c)).collect::<Vec<_>>(),
        "mesh": n.mesh.as_ref().map(|m| json!({
            "texture": m.texture,
            "lightmap": m.lightmap,
            "texture_count": m.texture_count,
            "lightmapped": m.lightmapped,
            "render": m.render,
            "shadow": m.shadow,
            "transparency_hint": m.transparency_hint,
            "vertices": m.vertex_count,
            "faces": m.faces.len(),
            "indices": m.indices.as_ref().map(|i| i.len()),
            "mdx_flags": m.mdx_flags,
            "stride": m.stride,
            "normals": m.normals.is_some(),
            "uv1": m.uv1.is_some(),
            "uv2": m.uv2.is_some(),
        })),
        "skin": n.skin.as_ref().map(|s| json!({"bones": s.bones.len()})),
        "light": n.light.as_ref().map(light_json),
        "emitter": n.emitter.as_ref().map(emitter_json),
        "saber": n.saber.as_ref().map(saber_json),
    })
}

fn show(
    m: &Model<'_>,
    chain: &[Model<'_>],
    found: &[Found],
    missing: Option<&str>,
    inherited: &[(usize, String)],
    controllers: bool,
) -> (String, Json) {
    let meshes = m.meshes().count();
    let mut text = format!(
        "model {}: {}, supermodel {}, {} ({}), {}",
        m.name,
        classification(m.classification),
        m.supermodel.unwrap_or("none"),
        count(m.nodes.len(), "node"),
        count(meshes, "mesh"),
        count(m.animations.len(), "animation")
    );
    text.push_str(&format!(
        "\nheader box {} to {}, radius {}, animation scale {}",
        point(m.bounding_box.min),
        point(m.bounding_box.max),
        m.radius,
        m.animation_scale
    ));
    let span = m.bounds();
    if let Some(b) = span {
        text.push_str(&format!(
            "\nrendered meshes span {} to {}",
            point(b.min),
            point(b.max)
        ));
    }
    if !m.nodes.is_empty() {
        text.push_str("\nnodes");
        tree(&mut text, &m.nodes, 0, 0, controllers);
    }
    if !m.animations.is_empty() {
        text.push_str("\nanimations");
    }
    for a in &m.animations {
        text.push_str(&format!(
            "\n  {}  {} s  {}",
            a.name,
            a.length,
            count(a.nodes.len(), "node")
        ));
        if !a.events.is_empty() {
            let events: Vec<String> = a
                .events
                .iter()
                .map(|e| format!("{} {}", e.name, e.time))
                .collect();
            text.push_str(&format!("  events {}", events.join(", ")));
        }
        if controllers {
            for n in a.nodes.iter().filter(|n| !n.controllers.is_empty()) {
                text.push_str(&format!("\n    {}", n.name));
                controller_lines(&mut text, animated_kind(m, n), n, "      ");
            }
        }
    }
    if m.supermodel.is_some() {
        text.push_str("\nsupermodels");
        for f in found {
            text.push_str(&format!("\n  {}  {}", f.name, f.source));
        }
        if let Some(n) = missing {
            text.push_str(&format!("\n  {n}  not found"));
        }
    }
    if !inherited.is_empty() {
        text.push_str("\ninherited animations");
        for (i, a) in inherited {
            text.push_str(&format!("\n  {a}  from {}", chain[*i].name));
        }
    }
    let json = json!({
        "format": "mdl",
        "name": m.name,
        "classification": classification(m.classification),
        "subclassification": m.subclassification,
        "fog": m.fog,
        "supermodel": m.supermodel,
        "bounding_box": {"min": m.bounding_box.min, "max": m.bounding_box.max},
        "bounds": span.map(|b| json!({"min": b.min, "max": b.max})),
        "radius": m.radius,
        "animation_scale": m.animation_scale,
        "nodes": m.nodes.iter().map(node_json).collect::<Vec<_>>(),
        "animations": m.animations.iter().map(|a| json!({
            "name": a.name,
            "length": a.length,
            "transition": a.transition,
            "root": a.root,
            "nodes": a.nodes.len(),
            "events": a.events.iter().map(|e| json!({"time": e.time, "name": e.name})).collect::<Vec<_>>(),
            "keyed_nodes": controllers.then(|| animation_nodes_json(m, a)),
        })).collect::<Vec<_>>(),
        "supermodels": found.iter().map(|f| json!({"name": f.name, "source": f.source})).collect::<Vec<_>>(),
        "missing_supermodel": missing,
        "inherited_animations": inherited.iter().map(|(i, a)| json!({
            "name": a,
            "from": chain[*i].name,
        })).collect::<Vec<_>>(),
    });
    (text, json)
}
