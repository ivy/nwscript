//! Recorded runs: a script, the world it started in, and everything it did,
//! kept as TOML and replayed as a test with no logic of its own.

use nwscript_vm::world::Object;
use nwscript_vm::{Call, Engine, Limits, Recording, Routine, Run, World, run};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A world to start a run in, written by name rather than id: objects are
/// referred to by tag.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldSpec {
    #[serde(default)]
    pub seed: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub globals_number: BTreeMap<String, i32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub globals_boolean: BTreeMap<String, bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub globals_string: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<ObjectSpec>,
    /// Party members by tag, the leader first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub party: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub npc_party: Vec<i32>,
    /// Context objects by tag: `self`, `speaker`, `entering`, `exiting`, `opened_by`, `used_by`, `clicking`, `perceived`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub user_event: i32,
}

fn is_zero(i: &i32) -> bool {
    *i == 0
}

/// One object of a [`WorldSpec`].
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectSpec {
    /// An `OBJECT_TYPE_*` number: 1 creature, 2 item, 4 trigger, 8 door, 32 waypoint, 64 placeable.
    pub kind: i32,
    pub tag: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_pc: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub dead: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f32; 3]>,
    /// Tags of items in the inventory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<String>,
}

impl WorldSpec {
    /// Build the world. Objects get ids in the order listed, so a spec always
    /// gives the same world.
    pub fn build(&self) -> Result<World, String> {
        let mut w = World::new(self.seed);
        let module = w.module;
        let mut by_tag: BTreeMap<String, i32> = BTreeMap::new();
        for o in &self.objects {
            let id = w.create(Object {
                kind: o.kind,
                tag: o.tag.clone(),
                name: o.name.clone(),
                hp: o.hp.unwrap_or(0),
                max_hp: o.max_hp.unwrap_or(0),
                is_pc: o.is_pc,
                dead: o.dead,
                faction: o.faction.unwrap_or(0),
                position: o.position.unwrap_or([0.0; 3]),
                area: module,
                ..Object::default()
            });
            for tag in &o.inventory {
                let item = w.create(Object {
                    kind: 2,
                    tag: tag.clone(),
                    resref: tag.clone(),
                    name: tag.clone(),
                    ..Object::default()
                });
                if let Some(x) = w.objects.get_mut(&id) {
                    x.inventory.push(item);
                }
            }
            by_tag.entry(o.tag.to_ascii_lowercase()).or_insert(id);
        }
        let find = |tag: &str| {
            by_tag
                .get(&tag.to_ascii_lowercase())
                .copied()
                .ok_or_else(|| format!("the world names `{tag}`, which no object has as its tag"))
        };
        w.globals_number = self.globals_number.clone();
        w.globals_boolean = self.globals_boolean.clone();
        w.globals_string = self
            .globals_string
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().into_bytes()))
            .collect();
        w.party = self
            .party
            .iter()
            .map(|t| find(t))
            .collect::<Result<_, _>>()?;
        w.npc_party = self.npc_party.iter().copied().collect();
        w.context.user_defined_event = self.user_event;
        for (role, tag) in &self.context {
            let id = find(tag)?;
            let slot = match role.as_str() {
                "self" => &mut w.context.caller,
                "speaker" => &mut w.context.pc_speaker,
                "entering" => &mut w.context.entering,
                "exiting" => &mut w.context.exiting,
                "opened_by" => &mut w.context.last_opened_by,
                "used_by" => &mut w.context.last_used_by,
                "clicking" => &mut w.context.clicking,
                "perceived" => &mut w.context.last_perceived,
                other => return Err(format!("unknown context `{other}`")),
            };
            *slot = id;
        }
        Ok(w)
    }
}

/// One recorded engine call, as text so a snapshot reads and diffs plainly.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CallRecord {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ret: String,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub modelled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deferred: Option<usize>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

impl From<&Call> for CallRecord {
    fn from(c: &Call) -> Self {
        CallRecord {
            name: c.name.clone(),
            args: c.args.iter().map(|a| a.to_string()).collect(),
            ret: if c.ret == nwscript_vm::Value::Void {
                String::new()
            } else {
                c.ret.to_string()
            },
            modelled: c.modelled,
            deferred: c.deferred,
        }
    }
}

impl std::fmt::Display for CallRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(d) = self.deferred {
            write!(f, "[deferred {d}] ")?;
        }
        write!(f, "{}({})", self.name, self.args.join(", "))?;
        if !self.ret.is_empty() {
            write!(f, " -> {}", self.ret)?;
        }
        if !self.modelled {
            f.write_str(" (unmodelled)")?;
        }
        Ok(())
    }
}

/// Which engine a snapshot ran against.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EngineKind {
    Recording,
    World,
}

/// A recorded run.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// The script's name, found as NAME.ncs the way tests find scripts.
    pub script: String,
    pub game: String,
    /// The sha256 of the bytecode recorded, for the reader; replay compares behaviour, not bytes.
    pub script_sha256: String,
    pub engine: EngineKind,
    #[serde(default)]
    pub world: WorldSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<CallRecord>,
}

/// Run a program as a snapshot describes and return what happened.
pub fn execute(
    program: &nwscript_ncs::Program,
    routines: &[Routine],
    engine: EngineKind,
    world: &WorldSpec,
    limits: Limits,
) -> Result<Run, String> {
    let mut w;
    let mut rec = Recording;
    let e: &mut dyn Engine = match engine {
        EngineKind::Recording => &mut rec,
        EngineKind::World => {
            w = world.build()?;
            &mut w
        }
    };
    Ok(run(program, routines, e, limits))
}

/// Record a run as a snapshot.
pub fn record(
    script: &str,
    game: &str,
    bytes: &[u8],
    engine: EngineKind,
    world: WorldSpec,
    r: &Run,
) -> Snapshot {
    Snapshot {
        script: script.into(),
        game: game.into(),
        script_sha256: crate::sha256_hex(bytes),
        engine,
        world,
        result: r.result.as_ref().map(|v| v.to_string()),
        fault: r.fault.as_ref().map(|f| f.to_string()),
        calls: r.trace.iter().map(CallRecord::from).collect(),
    }
}

/// Compare a replay with its snapshot: `None` when they agree, otherwise
/// where they first differ.
pub fn compare(s: &Snapshot, r: &Run) -> Option<String> {
    let got: Vec<CallRecord> = r.trace.iter().map(CallRecord::from).collect();
    for (i, (want, have)) in s.calls.iter().zip(&got).enumerate() {
        if want != have {
            return Some(format!(
                "call {} differs:\n  recorded: {want}\n  now:      {have}",
                i + 1
            ));
        }
    }
    if s.calls.len() != got.len() {
        let (more, which) = if got.len() > s.calls.len() {
            ("now makes", &got[s.calls.len()])
        } else {
            ("recorded", &s.calls[got.len()])
        };
        return Some(format!(
            "the run {more} more calls; the first extra is call {}: {which}",
            s.calls.len().min(got.len()) + 1
        ));
    }
    let result = r.result.as_ref().map(|v| v.to_string());
    if result != s.result {
        return Some(format!(
            "the result differs: recorded {:?}, now {:?}",
            s.result, result
        ));
    }
    let fault = r.fault.as_ref().map(|f| f.to_string());
    if fault != s.fault {
        return Some(format!(
            "the fault differs: recorded {:?}, now {:?}",
            s.fault, fault
        ));
    }
    None
}
