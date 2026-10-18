//! `rakata save show DIR`: a saved game by name, through [`rakata::save`]:
//! the party and the visited modules, the globals matching a pattern, a
//! module's creatures and pending events, one creature in full.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use serde_json::{Value, json};

use super::Ctx;
use crate::Exit;
use rakata::codes;
use rakata::diag::{Diagnostic, SourceMap, codes::Code};
use rakata::save::{
    Action, Creature, Effect, Event, GlobalValue, Globals, LocalNames, OBJECT_INVALID, ObjectRef,
    Param, Party, Place, Save, action_name,
};

/// What a number whose names live only in the engine is printed with.
const UNKNOWN: &str = "(table unknown)";

#[derive(Args, Debug)]
#[command(after_help = "\
A save is a directory under the game's Saves/ folder: NNNNNN - GameN/.
Names (modules, tags, globals) compare without regard to case.

Examples:
  rakata save show \"Saves/000023 - Game22\"
  rakata save show DIR --module danm14ac
  rakata save show DIR --module danm14ac --creature carth
  rakata save show DIR --globals OAJ_
  rakata --format json save show DIR --creature mission")]
pub struct SaveArgs {
    #[command(subcommand)]
    pub action: SaveAction,
}

#[derive(Subcommand, Debug)]
pub enum SaveAction {
    /// Print a save's party and visited modules; with --globals, the globals whose names contain PATTERN; with --module, that module's creatures and pending events; with --creature, every creature of that tag in full: tag as saved, Commandable, faction, animation, effects, local booleans and numbers, and the action queue.
    #[command(after_help = "\
--creature searches the module given by --module (its GIT's Creature List
and its IFO's player list), or every module the save holds, and the party
members' AVAILNPC<n>.utc. Local variables are named from the include scripts
of the corpus extracted from the user's own game (nss/k_inc_*.nss under
$NWSCRIPT_CORPUS, else ~/.cache/nwscript/corpus/k1); without it they are
shown by index, with warning RW1201. Effect types, animations, event ids and
most action ids are the engine's own numbers and are printed as numbers
marked (table unknown).

Examples:
  rakata save show \"Saves/000023 - Game22\" --module danm14ac --creature carth
  rakata save show DIR --creature MISSION
  rakata save show DIR --globals oaj_
  rakata --format json save show DIR --module danm14ac")]
    Show {
        /// The save's directory, the one holding SAVEGAME.sav.
        dir: PathBuf,
        /// The module whose state to show, by its stem (danm14ac).
        #[arg(long, short, value_name = "M")]
        module: Option<String>,
        /// Show every creature with this tag in full.
        #[arg(long, value_name = "TAG")]
        creature: Option<String>,
        /// Show the globals whose names contain PATTERN.
        #[arg(long, value_name = "PATTERN")]
        globals: Option<String>,
    },
}

/// The registered code for an error the save crate reports.
fn code_of(id: &str) -> &'static Code {
    codes::lookup(id).unwrap_or_else(|| panic!("diagnostic code {id} is not registered"))
}

fn error(e: &rakata::save::Error) -> Diagnostic {
    Diagnostic::new(code_of(e.code()), e.message())
}

pub fn run(ctx: &Ctx, args: &SaveArgs) -> Exit {
    match &args.action {
        SaveAction::Show {
            dir,
            module,
            creature,
            globals,
        } => show(
            ctx,
            dir,
            module.as_deref(),
            creature.as_deref(),
            globals.as_deref(),
        ),
    }
}

fn show(
    ctx: &Ctx,
    dir: &Path,
    module: Option<&str>,
    creature: Option<&str>,
    pattern: Option<&str>,
) -> Exit {
    let mut env = ctx.envelope("save show");
    let map = SourceMap::new();
    crate::working_on(dir.display().to_string());
    match report(dir, module, creature, pattern) {
        Ok((result, text, diags)) => {
            env.result = result;
            ctx.finish(env, &text, diags, &map, Exit::Ok)
        }
        Err(e) => ctx.finish(env, "", vec![error(&e)], &map, Exit::Ok),
    }
}

type Report = (Value, String, Vec<Diagnostic>);

fn report(
    dir: &Path,
    module: Option<&str>,
    creature: Option<&str>,
    pattern: Option<&str>,
) -> rakata::save::Result<Report> {
    let save = Save::open(dir)?;
    let info = save.info();
    let party = save.party()?;
    let modules = save.modules()?;
    let mut diags = Vec::new();
    let mut text = String::new();

    let shown_name = dir
        .file_name()
        .map_or_else(|| dir.display().to_string(), |n| n.to_string_lossy().into());
    let _ = writeln!(text, "save     {shown_name}");
    if let Some(n) = &info.save_name {
        let _ = writeln!(text, "name     {n}");
    }
    let _ = writeln!(
        text,
        "area     {} ({})",
        info.area_name.as_deref().unwrap_or("?"),
        info.last_module.as_deref().unwrap_or("?")
    );
    let _ = writeln!(text, "party    {}", party_line(&party));
    let _ = writeln!(text, "modules  {}", modules.join(" "));

    let mut result = json!({
        "dir": dir,
        "area_name": info.area_name,
        "last_module": info.last_module,
        "save_name": info.save_name,
        "time_played": info.time_played,
        "party": party_json(&party),
        "modules": modules,
        "globals": Value::Null,
        "creatures": Value::Null,
        "events": Value::Null,
    });

    if let Some(p) = pattern {
        let globals = save.globals()?;
        let (t, j) = globals_section(&globals, p);
        text.push_str(&t);
        result["globals"] = j;
    }

    if module.is_some() || creature.is_some() {
        let names = local_names(&mut diags);
        let mut found = match creature {
            Some(tag) => save.find_creatures(tag, module)?,
            None => save.creatures(module)?,
        };
        // The module's own creatures first, as they are listed; with no
        // --creature, the party members are listed after them.
        found.sort_by_key(|c| matches!(c.place, Place::PartyMember { .. }));
        if let Some(tag) = creature
            && found.is_empty()
        {
            let r#where = module.map_or_else(|| "any module".to_owned(), str::to_owned);
            diags.push(Diagnostic::new(
                code_of(rakata::save::codes::NO_SUCH_CREATURE),
                format!("no creature tagged `{tag}` in {where} or the party members"),
            ));
        }
        let _ = writeln!(text);
        if creature.is_some() {
            for c in &found {
                text.push_str(&creature_detail(c, &names));
            }
        } else {
            let _ = writeln!(
                text,
                "creatures of {} and the party ({})",
                module.unwrap_or("every module"),
                found.len()
            );
            for c in &found {
                text.push_str(&creature_line(c, &names));
            }
        }
        result["creatures"] =
            Value::Array(found.iter().map(|c| creature_json(c, &names)).collect());
    }

    if let Some(m) = module {
        let events = save.events(m)?;
        let _ = writeln!(text, "\nevents of {m} ({})", events.len());
        for e in &events {
            text.push_str(&event_line(e));
        }
        result["events"] = Value::Array(events.iter().map(event_json).collect());
    }
    Ok((result, text, diags))
}

/// The local variables' names from the corpus, or none with the warning.
fn local_names(diags: &mut Vec<Diagnostic>) -> LocalNames {
    match rakata::save::local_names(&rakata::corpus::default_out()) {
        Ok(n) => n,
        Err(dir) => {
            diags.push(
                Diagnostic::new(
                    code_of(rakata::save::codes::NO_LOCAL_NAMES),
                    format!(
                        "local variables are shown by index: no include scripts under {}",
                        dir.display()
                    ),
                )
                .help("extract the corpus with `rakata corpus extract --game DIR`, or set $NWSCRIPT_CORPUS"),
            );
            LocalNames::new()
        }
    }
}

fn party_line(p: &Party) -> String {
    if p.members.is_empty() {
        return "(the player alone)".to_owned();
    }
    p.members
        .iter()
        .map(|m| {
            let tag = m.tag.as_deref().unwrap_or("?");
            let lead = if m.is_leader { ", leader" } else { "" };
            format!("{tag} (AVAILNPC{}{lead})", m.npc)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn party_json(p: &Party) -> Value {
    json!({
        "members": p.members.iter().map(|m| json!({
            "npc": m.npc, "tag": m.tag, "is_leader": m.is_leader,
        })).collect::<Vec<_>>(),
        "available": p.available.iter().map(|a| json!({
            "npc": a.npc, "available": a.available, "selectable": a.selectable,
        })).collect::<Vec<_>>(),
        "gold": p.gold,
        "xp_pool": p.xp_pool,
        "controlled": p.controlled,
    })
}

fn global_text(v: &GlobalValue) -> String {
    match v {
        GlobalValue::Boolean(b) => b.to_string(),
        GlobalValue::Number(n) => n.to_string(),
        GlobalValue::String(s) => format!("{s:?}"),
        GlobalValue::Location(l) => format!(
            "({}, {}, {}) facing ({}, {}, {})",
            l.position[0],
            l.position[1],
            l.position[2],
            l.orientation[0],
            l.orientation[1],
            l.orientation[2]
        ),
    }
}

fn global_json(v: &GlobalValue) -> Value {
    match v {
        GlobalValue::Boolean(b) => json!(b),
        GlobalValue::Number(n) => json!(n),
        GlobalValue::String(s) => json!(s),
        GlobalValue::Location(l) => json!({
            "position": l.position, "orientation": l.orientation,
        }),
    }
}

fn globals_section(g: &Globals, pattern: &str) -> (String, Value) {
    let found: Vec<_> = g.matching(pattern).collect();
    let mut t = format!("\nglobals matching `{pattern}` ({})\n", found.len());
    let width = found.iter().map(|v| v.name.len()).max().unwrap_or(0);
    for v in &found {
        let _ = writeln!(
            t,
            "  {:<8} {:<width$}  {}",
            v.value.kind(),
            v.name,
            global_text(&v.value)
        );
    }
    let j = found
        .iter()
        .map(|v| json!({"name": v.name, "type": v.value.kind(), "value": global_json(&v.value)}))
        .collect();
    (t, Value::Array(j))
}

fn object_text(o: &ObjectRef) -> String {
    if o.id == OBJECT_INVALID {
        return "none".to_owned();
    }
    match &o.tag {
        Some(t) => format!("{} {t}", o.id),
        None => format!("{} (not in this state)", o.id),
    }
}

fn object_json(o: &ObjectRef) -> Value {
    json!({ "id": o.id, "tag": o.tag, "invalid": o.id == OBJECT_INVALID })
}

fn place_json(p: &Place) -> Value {
    match p {
        Place::Module {
            module,
            member,
            path,
        } => json!({"kind": "module", "module": module, "member": member, "path": path}),
        Place::PartyMember { npc } => {
            json!({"kind": "party", "npc": npc, "member": format!("AVAILNPC{npc}.utc")})
        }
    }
}

fn boolean_names(c: &Creature, names: &LocalNames) -> Vec<String> {
    c.booleans
        .iter()
        .map(|&i| match names.boolean(i) {
            Some(n) => format!("{i} {n}"),
            None => i.to_string(),
        })
        .collect()
}

fn number_names(c: &Creature, names: &LocalNames) -> Vec<String> {
    c.numbers
        .iter()
        .enumerate()
        .filter(|(_, v)| **v != 0)
        .map(|(i, v)| match names.number(i as u32) {
            Some(n) => format!("{i} {n} = {v}"),
            None => format!("{i} = {v}"),
        })
        .collect()
}

fn opt<T: ToString>(v: Option<T>) -> String {
    v.map_or_else(|| "?".to_owned(), |v| v.to_string())
}

fn faction_text(c: &Creature) -> String {
    match (&c.faction, &c.faction_name) {
        (Some(f), Some(n)) => format!("{f} {n}"),
        (Some(f), None) => f.to_string(),
        _ => "?".to_owned(),
    }
}

fn creature_line(c: &Creature, names: &LocalNames) -> String {
    let flags = boolean_names(c, names);
    let shown = if c.tag.is_empty() {
        format!("(untagged {})", c.name.as_deref().unwrap_or("?"))
    } else {
        c.tag.clone()
    };
    format!(
        "  {:<20} {}  commandable {}  faction {}{}\n",
        shown,
        c.place.describe(),
        opt(c.commandable),
        faction_text(c),
        if flags.is_empty() {
            String::new()
        } else {
            format!("  booleans {}", flags.join(", "))
        }
    )
}

fn effect_text(e: &Effect) -> String {
    let mut s = format!(
        "type {} {UNKNOWN}  subtype {}  creator {}",
        e.effect_type,
        e.subtype,
        object_text(&e.creator)
    );
    if let Some(d) = e.duration {
        let _ = write!(s, "  duration {d}");
    }
    if let Some(sp) = e.spell {
        let _ = write!(s, "  spell {sp}");
    }
    if e.ints.iter().any(|&i| i != 0) {
        let last = e.ints.iter().rposition(|&i| i != 0).unwrap_or(0);
        let ints: Vec<String> = e.ints[..=last].iter().map(i32::to_string).collect();
        let _ = write!(s, "  ints {}", ints.join(" "));
    }
    s
}

fn param_text(p: &Param) -> String {
    match p {
        Param::Int(i) => format!("int {i}"),
        Param::Float(f) => format!("float {f}"),
        Param::Object(o) => format!("object {}", object_text(o)),
        Param::String(s) => format!("string {s:?}"),
        Param::Script { name, code_size } => {
            format!("script {name:?} ({} bytes)", opt(*code_size))
        }
        Param::Other { type_id, value } => format!("type {type_id} ({value})"),
    }
}

fn action_text(a: &Action) -> String {
    let name = action_name(a.id).map_or_else(|| UNKNOWN.to_owned(), str::to_owned);
    let mut s = format!("{} {name}", a.id);
    if let Some(g) = a.group {
        let _ = write!(s, "  group {g}");
    }
    if !a.params.is_empty() {
        let ps: Vec<String> = a.params.iter().map(param_text).collect();
        let _ = write!(s, "  ({})", ps.join(", "));
    }
    s
}

fn creature_detail(c: &Creature, names: &LocalNames) -> String {
    let mut t = format!("{}  {}\n", c.tag, c.place.describe());
    let mut row = |k: &str, v: String| {
        let _ = writeln!(t, "  {k:<12}{v}");
    };
    row("tag", format!("{:?} (as saved)", c.tag));
    row("object", opt(c.object_id));
    row("name", c.name.clone().unwrap_or_else(|| "?".to_owned()));
    row("commandable", opt(c.commandable));
    row("faction", faction_text(c));
    row(
        "animation",
        c.animation
            .map_or_else(|| "?".to_owned(), |a| format!("{a} {UNKNOWN}")),
    );
    let b = boolean_names(c, names);
    row(
        "booleans",
        if b.is_empty() {
            "none set".to_owned()
        } else {
            b.join(", ")
        },
    );
    let n = number_names(c, names);
    row(
        "numbers",
        if n.is_empty() {
            "all 0".to_owned()
        } else {
            n.join(", ")
        },
    );
    row("effects", c.effects.len().to_string());
    for e in &c.effects {
        let _ = writeln!(t, "    {}", effect_text(e));
    }
    let _ = writeln!(t, "  {:<12}{}", "actions", c.actions.len());
    for (i, a) in c.actions.iter().enumerate() {
        let _ = writeln!(t, "    [{i}] {}", action_text(a));
    }
    t
}

fn param_json(p: &Param) -> Value {
    match p {
        Param::Int(i) => json!({"type": "int", "value": i}),
        Param::Float(f) => json!({"type": "float", "value": f}),
        Param::Object(o) => json!({"type": "object", "value": object_json(o)}),
        Param::String(s) => json!({"type": "string", "value": s}),
        Param::Script { name, code_size } => {
            json!({"type": "script", "value": {"name": name, "code_size": code_size}})
        }
        Param::Other { type_id, value } => {
            json!({"type": "other", "type_id": type_id, "value": value})
        }
    }
}

fn creature_json(c: &Creature, names: &LocalNames) -> Value {
    json!({
        "place": place_json(&c.place),
        "object_id": c.object_id,
        "tag": c.tag,
        "name": c.name,
        "commandable": c.commandable,
        "faction": {"id": c.faction, "name": c.faction_name},
        "animation": {"value": c.animation, "name": Value::Null, "note": "table unknown"},
        "effects": c.effects.iter().map(|e| json!({
            "id": e.id,
            "type": e.effect_type,
            "type_name": Value::Null,
            "note": "table unknown",
            "subtype": e.subtype,
            "creator": object_json(&e.creator),
            "duration": e.duration,
            "spell": e.spell,
            "ints": e.ints,
        })).collect::<Vec<_>>(),
        "booleans": c.booleans.iter().map(|&i| json!({
            "index": i, "name": names.boolean(i),
        })).collect::<Vec<_>>(),
        "numbers": c.numbers.iter().enumerate().map(|(i, v)| json!({
            "index": i, "name": names.number(i as u32), "value": v,
        })).collect::<Vec<_>>(),
        "actions": c.actions.iter().map(|a| json!({
            "id": a.id,
            "name": action_name(a.id),
            "group": a.group,
            "params": a.params.iter().map(param_json).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn event_line(e: &Event) -> String {
    format!(
        "  day {} time {}  event {} {UNKNOWN}  object {}  caller {}{}\n",
        opt(e.day),
        opt(e.time),
        opt(e.event_id),
        object_text(&e.object),
        object_text(&e.caller),
        e.script
            .as_deref()
            .map_or_else(String::new, |s| format!("  script {s}"))
    )
}

fn event_json(e: &Event) -> Value {
    json!({
        "day": e.day,
        "time": e.time,
        "event_id": e.event_id,
        "note": "table unknown",
        "object": object_json(&e.object),
        "caller": object_json(&e.caller),
        "script": e.script,
    })
}
