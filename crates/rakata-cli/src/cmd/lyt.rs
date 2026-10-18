//! `rakata lyt show`: an area's room layout (rooms, swoop tracks and
//! obstacles, door hooks) or its room visibility, over `rakata-lyt`.

use clap::{Args, Subcommand};
use serde_json::{Value as Json, json};

use super::Ctx;
use super::inspect::{self, LookupArgs, count, resource_forms};
use crate::Exit;
use rakata::diag::SourceMap;
use rakata::inspect::diagnostic;
use rakata::lyt::{Lyt, Placement, Vis, format_float};

const SHOW: &str = concat!(
    "\
The extension says the format. A LYT prints its counts, then each room,
track piece and obstacle as `MODEL  X Y Z`, and each door hook as
`ROOM NAME  X Y Z  orientation W X Y Z`, the numbers spelled as the file
spells them. A VIS prints one `ROOM  sees N: ROOM ...` line per room, in file
order. --format json gives the same fields.

",
    resource_forms!(),
    "

Examples:
  rakata lyt show m01aa.lyt --game DIR
  rakata lyt show m01aa.vis --game DIR
  rakata --format json lyt show ./m01aa.lyt"
);

#[derive(Args, Debug)]
#[command(after_help = "\
Example:
  rakata lyt show m01aa.lyt --game DIR")]
pub struct LytArgs {
    #[command(subcommand)]
    pub action: LytAction,
}

#[derive(Subcommand, Debug)]
pub enum LytAction {
    /// Print an area's room layout (.lyt) or room visibility (.vis).
    #[command(after_help = SHOW)]
    Show {
        /// The layout or visibility file: NAME.lyt or NAME.vis, a PATH or an ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        #[command(flatten)]
        lookup: LookupArgs,
    },
}

pub fn run(ctx: &Ctx, args: &LytArgs) -> Exit {
    let LytAction::Show { name, lookup } = &args.action;
    let mut env = ctx.envelope("lyt show");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let loaded = match inspect::load(&mut env, "resource", name, &lookup.lookup(ctx)) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let read = match loaded.extension().as_deref() {
        Some("lyt") => Lyt::parse(&loaded.bytes).map(|l| lyt(&l)),
        Some("vis") => Vis::parse(&loaded.bytes).map(|v| vis(&v)),
        _ => {
            return fail(
                env,
                inspect::unread_type(&loaded, "lyt show", "a .lyt or .vis"),
            );
        }
    };
    let (text, mut result) = match read {
        Ok(r) => r,
        Err(e) => {
            let d = diagnostic(e.code(), &e).note(format!("while reading `{}`", loaded.source));
            return fail(env, d);
        }
    };
    result["source"] = json!(loaded.source);
    env.result = result;
    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
}

/// Numbers as the file spells them, space-separated.
fn spelled(v: &[f32]) -> String {
    v.iter()
        .map(|x| format_float(*x).unwrap_or_else(|| x.to_string()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn placements(text: &mut String, heading: &str, list: &[Placement]) -> Json {
    if !list.is_empty() {
        text.push_str(&format!("\n{heading}"));
    }
    for p in list {
        text.push_str(&format!("\n  {}  {}", p.model, spelled(&p.position)));
    }
    json!(
        list.iter()
            .map(|p| json!({"model": p.model, "position": p.position}))
            .collect::<Vec<_>>()
    )
}

fn lyt(l: &Lyt) -> (String, Json) {
    let mut text = format!(
        "layout{}: {}, {}, {}, {}",
        l.file_dependency
            .as_ref()
            .map(|d| format!(" {d}"))
            .unwrap_or_default(),
        count(l.rooms.len(), "room"),
        count(l.tracks.len(), "track"),
        count(l.obstacles.len(), "obstacle"),
        count(l.door_hooks.len(), "door hook")
    );
    let rooms = placements(&mut text, "rooms", &l.rooms);
    let tracks = placements(&mut text, "tracks", &l.tracks);
    let obstacles = placements(&mut text, "obstacles", &l.obstacles);
    if !l.door_hooks.is_empty() {
        text.push_str("\ndoor hooks");
    }
    for h in &l.door_hooks {
        text.push_str(&format!(
            "\n  {} {}  {}  orientation {}",
            h.room,
            h.name,
            spelled(&h.position),
            spelled(&h.orientation.to_array())
        ));
    }
    let json = json!({
        "format": "lyt",
        "file_dependency": l.file_dependency,
        "rooms": rooms,
        "tracks": tracks,
        "obstacles": obstacles,
        "door_hooks": l.door_hooks.iter().map(|h| json!({
            "room": h.room,
            "name": h.name,
            "unknown": h.unknown,
            "position": h.position,
            "orientation": h.orientation.to_array(),
        })).collect::<Vec<_>>(),
    });
    (text, json)
}

fn vis(v: &Vis) -> (String, Json) {
    let mut text = format!("visibility: {}", count(v.rooms.len(), "room"));
    for r in &v.rooms {
        text.push_str(&format!("\n{}  sees {}", r.room, r.visible.len()));
        if !r.visible.is_empty() {
            text.push_str(&format!(": {}", r.visible.join(" ")));
        }
    }
    let json = json!({
        "format": "vis",
        "rooms": v.rooms.iter().map(|r| json!({
            "room": r.room,
            "visible": r.visible,
        })).collect::<Vec<_>>(),
    });
    (text, json)
}
