//! A small model of the game world behind the engine routines scripts use
//! most: global and local variables, objects with tags and inventories, the
//! party, creatures' hit points, spells, feats and talents, positions, and
//! a seedable random source.
//!
//! Each routine the model covers is listed in [`MODELS`] with what it does
//! here; `docs/engine-model.md` is generated from that list. A routine the
//! model does not cover, or a case of one it declines (a criteria type
//! `GetNearestCreature` does not know), returns its type's default and is
//! marked unmodelled, exactly as under [`crate::Recording`].
//!
//! Routines are matched by name, so the same model serves any `nwscript.nss`
//! whose routines keep KotOR's names and parameter orders.

use crate::value::{Closure, OBJECT_INVALID, OBJECT_SELF, Value};
use crate::{Engine, Reply, Routine};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

/// How a modelled routine is modelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A function of its arguments alone, computed exactly.
    Pure,
    /// Reads or changes the world's state.
    State,
    /// Changes nothing a script can read back through the model, such as
    /// starting an animation or queueing an action; the call itself is the
    /// behaviour, and the trace records it.
    Recorded,
}

/// Every routine the world models, how, and what it does here.
pub const MODELS: &[(&str, Kind, &str)] = &[
    ("abs", Kind::Pure, "absolute value; `abs(i32::MIN)` wraps"),
    ("fabs", Kind::Pure, "absolute value"),
    ("IntToString", Kind::Pure, "decimal digits"),
    ("IntToFloat", Kind::Pure, "the nearest float"),
    (
        "FloatToInt",
        Kind::Pure,
        "truncated toward zero, saturating",
    ),
    (
        "FloatToString",
        Kind::Pure,
        "printf `%*.*f` with the width and decimals given (18 and 9 by default)",
    ),
    (
        "StringToInt",
        Kind::Pure,
        "C `atoi`: leading spaces, a sign, digits; 0 when there are none",
    ),
    ("GetStringLength", Kind::Pure, "length in bytes"),
    (
        "GetSubString",
        Kind::Pure,
        "the bytes from `nStart`, at most `nCount`; empty when out of range",
    ),
    ("GetStringLeft", Kind::Pure, "the first `nCount` bytes"),
    ("GetStringRight", Kind::Pure, "the last `nCount` bytes"),
    ("GetStringUpperCase", Kind::Pure, "ASCII upper case"),
    ("GetStringLowerCase", Kind::Pure, "ASCII lower case"),
    (
        "FindSubString",
        Kind::Pure,
        "the byte index of the first occurrence, or -1",
    ),
    (
        "Random",
        Kind::State,
        "a number from 0 to `nMaxInteger - 1` from the world's seeded generator; 0 when `nMaxInteger` is below 1",
    ),
    (
        "d2",
        Kind::State,
        "the sum of `nNumDice` rolls (at least one) of a two-sided die",
    ),
    ("d3", Kind::State, "as `d2`, three sides"),
    ("d4", Kind::State, "as `d2`, four sides"),
    ("d6", Kind::State, "as `d2`, six sides"),
    ("d8", Kind::State, "as `d2`, eight sides"),
    ("d10", Kind::State, "as `d2`, ten sides"),
    ("d12", Kind::State, "as `d2`, twelve sides"),
    ("d20", Kind::State, "as `d2`, twenty sides"),
    ("d100", Kind::State, "as `d2`, a hundred sides"),
    (
        "ShipBuild",
        Kind::State,
        "the world's `ship_build` flag, true by default as in the released game",
    ),
    (
        "GetGlobalNumber",
        Kind::State,
        "the world's global number, 0 when unset",
    ),
    ("SetGlobalNumber", Kind::State, "sets the global number"),
    (
        "GetGlobalBoolean",
        Kind::State,
        "the world's global boolean, false when unset",
    ),
    (
        "SetGlobalBoolean",
        Kind::State,
        "sets the global boolean (any non-zero value is true)",
    ),
    (
        "GetGlobalString",
        Kind::State,
        "the world's global string, empty when unset",
    ),
    ("SetGlobalString", Kind::State, "sets the global string"),
    (
        "GetLocalNumber",
        Kind::State,
        "the object's local number at the index, 0 when unset or the object is invalid",
    ),
    (
        "SetLocalNumber",
        Kind::State,
        "sets the object's local number",
    ),
    (
        "GetLocalBoolean",
        Kind::State,
        "the object's local boolean at the index",
    ),
    (
        "SetLocalBoolean",
        Kind::State,
        "sets the object's local boolean",
    ),
    (
        "GetIsObjectValid",
        Kind::State,
        "true for an object that exists and has not been destroyed",
    ),
    (
        "GetObjectByTag",
        Kind::State,
        "the `nNth` object (from 0) with the tag, ignoring case, in creation order",
    ),
    (
        "GetWaypointByTag",
        Kind::State,
        "the first waypoint with the tag",
    ),
    (
        "GetNearestObjectByTag",
        Kind::State,
        "the `nNth` nearest (from 1) other object in the target's area with the tag",
    ),
    (
        "GetNearestObject",
        Kind::State,
        "the `nNth` nearest other object in the target's area of the type mask",
    ),
    (
        "GetNearestCreature",
        Kind::State,
        "the `nNth` nearest other creature meeting the criteria; racial type, player character, alive, and has or lacks a spell effect are modelled, other criteria are not",
    ),
    (
        "GetTag",
        Kind::State,
        "the object's tag; empty for an invalid object",
    ),
    ("GetName", Kind::State, "the object's name"),
    (
        "GetObjectType",
        Kind::State,
        "the object's `OBJECT_TYPE_*`; `OBJECT_TYPE_INVALID` for an invalid one",
    ),
    ("GetFirstPC", Kind::State, "the party leader"),
    (
        "GetPCSpeaker",
        Kind::State,
        "the conversation's PC speaker, from the context",
    ),
    ("GetEnteringObject", Kind::State, "from the context"),
    ("GetExitingObject", Kind::State, "from the context"),
    ("GetLastOpenedBy", Kind::State, "from the context"),
    ("GetLastUsedBy", Kind::State, "from the context"),
    ("GetClickingObject", Kind::State, "from the context"),
    ("GetLastPerceived", Kind::State, "from the context"),
    (
        "GetLastAttacker",
        Kind::State,
        "the attackee's last attacker",
    ),
    ("GetUserDefinedEventNumber", Kind::State, "from the context"),
    ("GetModule", Kind::State, "the module object"),
    ("GetArea", Kind::State, "the object's area"),
    ("GetIsPC", Kind::State, "true for the player character"),
    (
        "GetPartyMemberByIndex",
        Kind::State,
        "the party member at the index, the leader at 0",
    ),
    ("GetPartyMemberCount", Kind::State, "the party's size"),
    (
        "IsObjectPartyMember",
        Kind::State,
        "true for an object in the party",
    ),
    (
        "IsNPCPartyMember",
        Kind::State,
        "true when the `NPC_*` number is in the party",
    ),
    ("GetSoloMode", Kind::State, "the world's solo-mode flag"),
    (
        "GetCurrentHitPoints",
        Kind::State,
        "the object's hit points",
    ),
    (
        "GetMaxHitPoints",
        Kind::State,
        "the object's maximum hit points",
    ),
    ("GetIsDead", Kind::State, "true when the creature is dead"),
    (
        "GetRacialType",
        Kind::State,
        "the creature's `RACIAL_TYPE_*`",
    ),
    ("GetGender", Kind::State, "the creature's `GENDER_*`"),
    ("GetHitDice", Kind::State, "the creature's total level"),
    (
        "GetLevelByClass",
        Kind::State,
        "the creature's levels in the class",
    ),
    (
        "GetStandardFaction",
        Kind::State,
        "the object's `STANDARD_FACTION_*`",
    ),
    (
        "GetIsEnemy",
        Kind::State,
        "true when the two objects' factions are listed as enemies in the world",
    ),
    (
        "GetIsFriend",
        Kind::State,
        "true when the two objects share a faction",
    ),
    (
        "GetIsNeutral",
        Kind::State,
        "true when neither enemies nor friends",
    ),
    (
        "GetObjectSeen",
        Kind::State,
        "true for a pair the world lists as seen, or for two objects in one area within the world's perception range (20 m by default)",
    ),
    (
        "GetIsInCombat",
        Kind::State,
        "the creature's in-combat flag",
    ),
    ("GetNPCAIStyle", Kind::State, "the creature's AI style"),
    ("SetNPCAIStyle", Kind::State, "sets the creature's AI style"),
    ("GetPlotFlag", Kind::State, "the object's plot flag"),
    ("SetPlotFlag", Kind::State, "sets the object's plot flag"),
    (
        "GetHasSpell",
        Kind::State,
        "1 when the creature knows the spell or Force power, else 0",
    ),
    (
        "GetHasFeat",
        Kind::State,
        "true when the creature has the feat",
    ),
    (
        "GetHasSpellEffect",
        Kind::State,
        "true when the object is under the spell's effect",
    ),
    ("TalentSpell", Kind::State, "a talent for the spell"),
    (
        "GetIsTalentValid",
        Kind::State,
        "true for a talent that is not null",
    ),
    (
        "GetIdFromTalent",
        Kind::State,
        "the talent's spell, feat or skill number",
    ),
    (
        "GetTypeFromTalent",
        Kind::State,
        "the talent's `TALENT_TYPE_*`",
    ),
    (
        "GetCreatureHasTalent",
        Kind::State,
        "true when the creature has the talent's spell, feat or skill",
    ),
    (
        "GetCreatureTalentRandom",
        Kind::State,
        "one of the creature's talents in the category, chosen by the world's generator; null when it has none",
    ),
    (
        "GetCreatureTalentBest",
        Kind::State,
        "the creature's first talent in the category, as listed; null when it has none",
    ),
    ("GetBaseItemType", Kind::State, "the item's `BASE_ITEM_*`"),
    (
        "GetItemInSlot",
        Kind::State,
        "the item the creature has in the slot",
    ),
    (
        "GetItemPossessedBy",
        Kind::State,
        "the first item in the creature's inventory or slots with the tag",
    ),
    (
        "GetFirstItemInInventory",
        Kind::State,
        "the first item in the object's inventory, starting its iteration",
    ),
    (
        "GetNextItemInInventory",
        Kind::State,
        "the next item of that iteration",
    ),
    (
        "CreateItemOnObject",
        Kind::State,
        "a new item, tagged with the template's name, in the target's inventory",
    ),
    (
        "DestroyObject",
        Kind::State,
        "destroys the object at once, whatever the delay",
    ),
    ("GetPosition", Kind::State, "the object's position"),
    ("GetFacing", Kind::State, "the object's facing in degrees"),
    (
        "GetLocation",
        Kind::State,
        "a location holding the object's area, position and facing",
    ),
    (
        "GetDistanceBetween",
        Kind::State,
        "the distance between two objects in one area; 0 otherwise",
    ),
    (
        "AssignCommand",
        Kind::Recorded,
        "the action runs after the script, as the given object",
    ),
    (
        "DelayCommand",
        Kind::Recorded,
        "the action runs after the script, as the caller; the delay is not modelled",
    ),
    (
        "ActionDoCommand",
        Kind::Recorded,
        "the action runs after the script, as the caller",
    ),
    ("ClearAllActions", Kind::Recorded, "recorded"),
    ("ActionPlayAnimation", Kind::Recorded, "recorded"),
    ("ActionWait", Kind::Recorded, "recorded"),
    ("ActionMoveToObject", Kind::Recorded, "recorded"),
    ("ActionForceMoveToObject", Kind::Recorded, "recorded"),
    ("ActionMoveToLocation", Kind::Recorded, "recorded"),
    ("ActionJumpToObject", Kind::Recorded, "recorded"),
    ("ActionJumpToLocation", Kind::Recorded, "recorded"),
    ("ActionAttack", Kind::Recorded, "recorded"),
    ("ActionUseTalentOnObject", Kind::Recorded, "recorded"),
    ("ActionStartConversation", Kind::Recorded, "recorded"),
    ("ActionPauseConversation", Kind::Recorded, "recorded"),
    ("ActionResumeConversation", Kind::Recorded, "recorded"),
    ("ActionSpeakString", Kind::Recorded, "recorded"),
    ("ActionEquipItem", Kind::Recorded, "recorded"),
    ("CancelCombat", Kind::Recorded, "recorded"),
    ("SetCommandable", Kind::Recorded, "recorded"),
    ("SetListenPattern", Kind::Recorded, "recorded"),
    ("SetFacing", Kind::Recorded, "recorded"),
    ("PlaySound", Kind::Recorded, "recorded"),
    ("SpeakString", Kind::Recorded, "recorded"),
    ("SendMessageToPC", Kind::Recorded, "recorded"),
    ("AurPostString", Kind::Recorded, "recorded"),
    ("PrintString", Kind::Recorded, "recorded"),
    ("PrintInteger", Kind::Recorded, "recorded"),
    ("PrintFloat", Kind::Recorded, "recorded"),
    ("PrintObject", Kind::Recorded, "recorded"),
    (
        "ApplyEffectToObject",
        Kind::Recorded,
        "recorded; the effect is not applied",
    ),
    (
        "GiveXPToCreature",
        Kind::Recorded,
        "recorded; the experience is not added",
    ),
];

const OBJECT_TYPE_CREATURE: i32 = 1;
const OBJECT_TYPE_ITEM: i32 = 2;
const OBJECT_TYPE_WAYPOINT: i32 = 32;
const OBJECT_TYPE_INVALID: i32 = 32767;

/// One object in the world.
#[derive(Clone, Debug, Default)]
pub struct Object {
    /// An `OBJECT_TYPE_*`: 1 creature, 2 item, 4 trigger, 8 door, 32 waypoint, 64 placeable.
    pub kind: i32,
    pub tag: String,
    /// The blueprint the object was made from.
    pub resref: String,
    pub name: String,
    pub area: i32,
    pub position: [f32; 3],
    pub facing: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub dead: bool,
    pub race: i32,
    pub gender: i32,
    /// Levels by `CLASS_TYPE_*`.
    pub classes: BTreeMap<i32, i32>,
    pub faction: i32,
    pub ai_style: i32,
    pub plot: bool,
    pub in_combat: bool,
    pub is_pc: bool,
    pub spells: BTreeSet<i32>,
    pub feats: BTreeSet<i32>,
    pub skills: BTreeMap<i32, i32>,
    pub spell_effects: BTreeSet<i32>,
    /// Talents in the order the creature lists them: (category, `TALENT_TYPE_*`, id).
    pub talents: Vec<(i32, i32, i32)>,
    pub inventory: Vec<i32>,
    /// Equipped items by `INVENTORY_SLOT_*`.
    pub slots: BTreeMap<i32, i32>,
    pub base_item: i32,
    pub last_attacker: i32,
    pub numbers: BTreeMap<i32, i32>,
    pub booleans: BTreeMap<i32, bool>,
    pub destroyed: bool,
}

/// What the engine would know about why the script runs.
#[derive(Clone, Debug)]
pub struct Context {
    /// `OBJECT_SELF`.
    pub caller: i32,
    pub pc_speaker: i32,
    pub entering: i32,
    pub exiting: i32,
    pub last_opened_by: i32,
    pub last_used_by: i32,
    pub clicking: i32,
    pub last_perceived: i32,
    pub user_defined_event: i32,
}

impl Default for Context {
    fn default() -> Self {
        Context {
            caller: OBJECT_INVALID,
            pc_speaker: OBJECT_INVALID,
            entering: OBJECT_INVALID,
            exiting: OBJECT_INVALID,
            last_opened_by: OBJECT_INVALID,
            last_used_by: OBJECT_INVALID,
            clicking: OBJECT_INVALID,
            last_perceived: OBJECT_INVALID,
            user_defined_event: 0,
        }
    }
}

/// The world: objects, variables, the party, and a seeded generator.
#[derive(Clone, Debug)]
pub struct World {
    pub objects: BTreeMap<i32, Object>,
    pub globals_number: BTreeMap<String, i32>,
    pub globals_boolean: BTreeMap<String, bool>,
    pub globals_string: BTreeMap<String, Vec<u8>>,
    /// Party members, the leader first.
    pub party: Vec<i32>,
    /// `NPC_*` numbers in the party.
    pub npc_party: BTreeSet<i32>,
    /// Pairs of `STANDARD_FACTION_*` that are hostile to each other.
    pub enemies: BTreeSet<(i32, i32)>,
    /// Pairs (target, source) that see each other regardless of distance.
    pub seen: BTreeSet<(i32, i32)>,
    pub perception_range: f32,
    pub module: i32,
    pub ship_build: bool,
    pub solo_mode: bool,
    pub context: Context,
    rng: u64,
    next_id: i32,
    locations: Vec<(i32, [f32; 3], f32)>,
    talent_handles: Vec<(i32, i32)>,
    inventory_cursor: HashMap<i32, usize>,
    owners: HashMap<usize, i32>,
}

impl Default for World {
    fn default() -> Self {
        Self::new(0)
    }
}

impl World {
    /// An empty world: a module object, no area, nobody in the party.
    pub fn new(seed: u64) -> Self {
        let mut w = World {
            objects: BTreeMap::new(),
            globals_number: BTreeMap::new(),
            globals_boolean: BTreeMap::new(),
            globals_string: BTreeMap::new(),
            party: Vec::new(),
            npc_party: BTreeSet::new(),
            enemies: BTreeSet::new(),
            seen: BTreeSet::new(),
            perception_range: 20.0,
            module: OBJECT_INVALID,
            ship_build: true,
            solo_mode: false,
            context: Context::default(),
            rng: seed ^ 0x9E37_79B9_7F4A_7C15,
            next_id: 0x100,
            locations: Vec::new(),
            talent_handles: Vec::new(),
            inventory_cursor: HashMap::new(),
            owners: HashMap::new(),
        };
        w.module = w.create(Object {
            kind: 0,
            tag: "module".into(),
            ..Object::default()
        });
        w
    }

    /// Add an object and return its id.
    pub fn create(&mut self, o: Object) -> i32 {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.insert(id, o);
        id
    }

    pub fn get(&self, id: i32) -> Option<&Object> {
        self.objects.get(&self.resolve(id)).filter(|o| !o.destroyed)
    }

    pub fn get_mut(&mut self, id: i32) -> Option<&mut Object> {
        let id = self.resolve(id);
        self.objects.get_mut(&id).filter(|o| !o.destroyed)
    }

    /// `OBJECT_SELF` is the caller.
    fn resolve(&self, id: i32) -> i32 {
        if id == OBJECT_SELF {
            self.context.caller
        } else {
            id
        }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64*: small, seedable and the same on every platform.
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        (self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    fn below(&mut self, n: i32) -> i32 {
        if n < 1 {
            0
        } else {
            (self.next_u32() % n as u32) as i32
        }
    }

    fn distance(&self, a: i32, b: i32) -> Option<f32> {
        let (a, b) = (self.get(a)?, self.get(b)?);
        if a.area != b.area {
            return None;
        }
        let d: f32 = (0..3)
            .map(|i| (a.position[i] - b.position[i]).powi(2))
            .sum();
        Some(d.sqrt())
    }

    /// Other live objects in `target`'s area matching `keep`, nearest first.
    fn nearest(&self, target: i32, keep: impl Fn(i32, &Object) -> bool) -> Vec<i32> {
        let t = self.resolve(target);
        let mut v: Vec<(f32, i32)> = self
            .objects
            .iter()
            .filter(|(id, o)| **id != t && !o.destroyed && keep(**id, o))
            .filter_map(|(id, _)| self.distance(t, *id).map(|d| (d, *id)))
            .collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        v.into_iter().map(|(_, id)| id).collect()
    }

    fn talent(&mut self, kind: i32, id: i32) -> Value {
        self.talent_handles.push((kind, id));
        Value::Engine(3, self.talent_handles.len() as u32)
    }

    fn talent_of(&self, v: &Value) -> Option<(i32, i32)> {
        match v {
            Value::Engine(_, h) if *h > 0 => self.talent_handles.get(*h as usize - 1).copied(),
            _ => None,
        }
    }

    fn are_enemies(&self, a: i32, b: i32) -> Option<bool> {
        let (fa, fb) = (self.get(a)?.faction, self.get(b)?.faction);
        Some(self.enemies.contains(&(fa, fb)) || self.enemies.contains(&(fb, fa)))
    }

    fn has_talent(&self, o: &Object, (kind, id): (i32, i32)) -> bool {
        match kind {
            0 => o.spells.contains(&id),
            1 => o.feats.contains(&id),
            2 => o.skills.get(&id).is_some_and(|r| *r > 0),
            _ => false,
        }
    }

    /// The model's value for a call, or `None` when it does not cover it.
    fn model(&mut self, name: &str, a: &[Value]) -> Option<Value> {
        let int = |i: usize| match a.get(i) {
            Some(Value::Int(v)) => *v,
            _ => 0,
        };
        let float = |i: usize| match a.get(i) {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        };
        let bytes = |i: usize| -> Vec<u8> {
            match a.get(i) {
                Some(Value::String(s)) => s.to_vec(),
                _ => Vec::new(),
            }
        };
        let obj = |i: usize| match a.get(i) {
            Some(Value::Object(o)) => *o,
            _ => OBJECT_INVALID,
        };
        let text = |i: usize| String::from_utf8_lossy(&bytes(i)).into_owned();
        let s = |b: Vec<u8>| Value::String(Rc::from(b));
        let b = |x: bool| Value::Int(i32::from(x));
        let o = |id: Option<i32>| Value::Object(id.unwrap_or(OBJECT_INVALID));
        let roll = |w: &mut World, sides: i32| -> Value {
            Value::Int((0..int(0).max(1)).map(|_| w.below(sides) + 1).sum())
        };
        Some(match name {
            "abs" => Value::Int(int(0).wrapping_abs()),
            "fabs" => Value::Float(float(0).abs()),
            "IntToString" => s(int(0).to_string().into_bytes()),
            "IntToFloat" => Value::Float(int(0) as f32),
            "FloatToInt" => Value::Int(float(0) as i32),
            "FloatToString" => {
                let (w, d) = (
                    a.get(1).map_or(18, |_| int(1)),
                    a.get(2).map_or(9, |_| int(2)),
                );
                s(format!(
                    "{:>w$.d$}",
                    float(0),
                    w = w.max(0) as usize,
                    d = d.max(0) as usize
                )
                .into_bytes())
            }
            "StringToInt" => {
                let t = text(0);
                let t = t.trim_start();
                let (neg, rest) = match t.as_bytes().first() {
                    Some(b'-') => (true, &t[1..]),
                    Some(b'+') => (false, &t[1..]),
                    _ => (false, t),
                };
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                let v = digits.bytes().fold(0i32, |acc, d| {
                    acc.wrapping_mul(10).wrapping_add(i32::from(d - b'0'))
                });
                Value::Int(if neg { v.wrapping_neg() } else { v })
            }
            "GetStringLength" => Value::Int(bytes(0).len() as i32),
            "GetSubString" => {
                let v = bytes(0);
                let start = int(1).max(0) as usize;
                let count = int(2).max(0) as usize;
                s(v.get(start..)
                    .map(|r| r[..count.min(r.len())].to_vec())
                    .unwrap_or_default())
            }
            "GetStringLeft" => {
                let v = bytes(0);
                s(v[..(int(1).max(0) as usize).min(v.len())].to_vec())
            }
            "GetStringRight" => {
                let v = bytes(0);
                s(v[v.len() - (int(1).max(0) as usize).min(v.len())..].to_vec())
            }
            "GetStringUpperCase" => s(bytes(0).to_ascii_uppercase()),
            "GetStringLowerCase" => s(bytes(0).to_ascii_lowercase()),
            "FindSubString" => {
                let (h, n) = (bytes(0), bytes(1));
                Value::Int(if n.is_empty() {
                    0
                } else {
                    h.windows(n.len())
                        .position(|w| w == n)
                        .map_or(-1, |p| p as i32)
                })
            }
            "Random" => Value::Int(self.below(int(0))),
            "d2" => roll(self, 2),
            "d3" => roll(self, 3),
            "d4" => roll(self, 4),
            "d6" => roll(self, 6),
            "d8" => roll(self, 8),
            "d10" => roll(self, 10),
            "d12" => roll(self, 12),
            "d20" => roll(self, 20),
            "d100" => roll(self, 100),
            "ShipBuild" => b(self.ship_build),
            "GetSoloMode" => b(self.solo_mode),
            "GetGlobalNumber" => {
                Value::Int(self.globals_number.get(&text(0)).copied().unwrap_or(0))
            }
            "SetGlobalNumber" => {
                self.globals_number.insert(text(0), int(1));
                Value::Void
            }
            "GetGlobalBoolean" => b(self.globals_boolean.get(&text(0)).copied().unwrap_or(false)),
            "SetGlobalBoolean" => {
                self.globals_boolean.insert(text(0), int(1) != 0);
                Value::Void
            }
            "GetGlobalString" => s(self
                .globals_string
                .get(&text(0))
                .cloned()
                .unwrap_or_default()),
            "SetGlobalString" => {
                self.globals_string.insert(text(0), bytes(1));
                Value::Void
            }
            "GetLocalNumber" => Value::Int(
                self.get(obj(0))
                    .and_then(|x| x.numbers.get(&int(1)).copied())
                    .unwrap_or(0),
            ),
            "SetLocalNumber" => {
                if let Some(x) = self.get_mut(obj(0)) {
                    x.numbers.insert(int(1), int(2));
                }
                Value::Void
            }
            "GetLocalBoolean" => b(self
                .get(obj(0))
                .and_then(|x| x.booleans.get(&int(1)).copied())
                .unwrap_or(false)),
            "SetLocalBoolean" => {
                if let Some(x) = self.get_mut(obj(0)) {
                    x.booleans.insert(int(1), int(2) != 0);
                }
                Value::Void
            }
            "GetIsObjectValid" => b(self.get(obj(0)).is_some()),
            "GetObjectByTag" => {
                let t = text(0);
                o(self
                    .objects
                    .iter()
                    .filter(|(_, x)| !x.destroyed && x.tag.eq_ignore_ascii_case(&t))
                    .map(|(id, _)| *id)
                    .nth(int(1).max(0) as usize))
            }
            "GetWaypointByTag" => {
                let t = text(0);
                o(self
                    .objects
                    .iter()
                    .find(|(_, x)| {
                        !x.destroyed
                            && x.kind == OBJECT_TYPE_WAYPOINT
                            && x.tag.eq_ignore_ascii_case(&t)
                    })
                    .map(|(id, _)| *id))
            }
            "GetNearestObjectByTag" => {
                let t = text(0);
                o(self
                    .nearest(obj(1), |_, x| x.tag.eq_ignore_ascii_case(&t))
                    .get((int(2).max(1) - 1) as usize)
                    .copied())
            }
            "GetNearestObject" => {
                let mask = a.first().map_or(-1, |_| int(0));
                o(self
                    .nearest(obj(1), |_, x| x.kind & mask != 0)
                    .get((int(2).max(1) - 1) as usize)
                    .copied())
            }
            "GetNearestCreature" => {
                let (ctype, cval) = (int(0), int(1));
                if a.get(4).is_some_and(|v| *v != Value::Int(-1)) {
                    return None;
                }
                let keep: Box<dyn Fn(&Object) -> bool> = match ctype {
                    0 => Box::new(move |x: &Object| x.race == cval),
                    1 => Box::new(move |x: &Object| x.is_pc == (cval != 0)),
                    4 => Box::new(move |x: &Object| x.dead != (cval != 0)),
                    5 => Box::new(move |x: &Object| x.spell_effects.contains(&cval)),
                    6 => Box::new(move |x: &Object| !x.spell_effects.contains(&cval)),
                    _ => return None,
                };
                o(self
                    .nearest(obj(2), |_, x| x.kind == OBJECT_TYPE_CREATURE && keep(x))
                    .get((int(3).max(1) - 1) as usize)
                    .copied())
            }
            "GetTag" => s(self
                .get(obj(0))
                .map(|x| x.tag.clone().into_bytes())
                .unwrap_or_default()),
            "GetName" => s(self
                .get(obj(0))
                .map(|x| x.name.clone().into_bytes())
                .unwrap_or_default()),
            "GetObjectType" => Value::Int(self.get(obj(0)).map_or(OBJECT_TYPE_INVALID, |x| x.kind)),
            "GetFirstPC" => o(self.party.first().copied()),
            "GetPCSpeaker" => Value::Object(self.context.pc_speaker),
            "GetEnteringObject" => Value::Object(self.context.entering),
            "GetExitingObject" => Value::Object(self.context.exiting),
            "GetLastOpenedBy" => Value::Object(self.context.last_opened_by),
            "GetLastUsedBy" => Value::Object(self.context.last_used_by),
            "GetClickingObject" => Value::Object(self.context.clicking),
            "GetLastPerceived" => Value::Object(self.context.last_perceived),
            "GetLastAttacker" => o(self.get(obj(0)).map(|x| x.last_attacker)),
            "GetUserDefinedEventNumber" => Value::Int(self.context.user_defined_event),
            "GetModule" => Value::Object(self.module),
            "GetArea" => o(self.get(obj(0)).map(|x| x.area)),
            "GetIsPC" => b(self.get(obj(0)).is_some_and(|x| x.is_pc)),
            "GetPartyMemberByIndex" => o(usize::try_from(int(0))
                .ok()
                .and_then(|i| self.party.get(i).copied())),
            "GetPartyMemberCount" => Value::Int(self.party.len() as i32),
            "IsObjectPartyMember" => b(self.party.contains(&self.resolve(obj(0)))),
            "IsNPCPartyMember" => b(self.npc_party.contains(&int(0))),
            "GetCurrentHitPoints" => Value::Int(self.get(obj(0)).map_or(0, |x| x.hp)),
            "GetMaxHitPoints" => Value::Int(self.get(obj(0)).map_or(0, |x| x.max_hp)),
            "GetIsDead" => b(self.get(obj(0)).is_some_and(|x| x.dead)),
            "GetRacialType" => Value::Int(self.get(obj(0)).map_or(0, |x| x.race)),
            "GetGender" => Value::Int(self.get(obj(0)).map_or(0, |x| x.gender)),
            "GetHitDice" => Value::Int(self.get(obj(0)).map_or(0, |x| x.classes.values().sum())),
            "GetLevelByClass" => Value::Int(
                self.get(obj(1))
                    .and_then(|x| x.classes.get(&int(0)).copied())
                    .unwrap_or(0),
            ),
            "GetStandardFaction" => Value::Int(self.get(obj(0)).map_or(0, |x| x.faction)),
            "GetIsEnemy" => b(self.are_enemies(obj(0), obj(1)).unwrap_or(false)),
            "GetIsFriend" => b(match (self.get(obj(0)), self.get(obj(1))) {
                (Some(x), Some(y)) => x.faction == y.faction,
                _ => false,
            }),
            "GetIsNeutral" => {
                let friends = matches!((self.get(obj(0)), self.get(obj(1))), (Some(x), Some(y)) if x.faction == y.faction);
                b(self.get(obj(0)).is_some()
                    && self.get(obj(1)).is_some()
                    && !friends
                    && !self.are_enemies(obj(0), obj(1)).unwrap_or(false))
            }
            "GetObjectSeen" => {
                let (t, src) = (self.resolve(obj(0)), self.resolve(obj(1)));
                b(self.seen.contains(&(t, src))
                    || self
                        .distance(t, src)
                        .is_some_and(|d| d <= self.perception_range))
            }
            "GetIsInCombat" => b(self.get(obj(0)).is_some_and(|x| x.in_combat)),
            "GetNPCAIStyle" => Value::Int(self.get(obj(0)).map_or(0, |x| x.ai_style)),
            "SetNPCAIStyle" => {
                if let Some(x) = self.get_mut(obj(0)) {
                    x.ai_style = int(1);
                }
                Value::Void
            }
            "GetPlotFlag" => b(self.get(obj(0)).is_some_and(|x| x.plot)),
            "SetPlotFlag" => {
                if let Some(x) = self.get_mut(obj(0)) {
                    x.plot = int(1) != 0;
                }
                Value::Void
            }
            "GetHasSpell" => b(self.get(obj(1)).is_some_and(|x| x.spells.contains(&int(0)))),
            "GetHasFeat" => b(self.get(obj(1)).is_some_and(|x| x.feats.contains(&int(0)))),
            "GetHasSpellEffect" => b(self
                .get(obj(1))
                .is_some_and(|x| x.spell_effects.contains(&int(0)))),
            "TalentSpell" => self.talent(0, int(0)),
            "GetIsTalentValid" => b(a.first().and_then(|t| self.talent_of(t)).is_some()),
            "GetIdFromTalent" => {
                Value::Int(a.first().and_then(|t| self.talent_of(t)).map_or(0, |t| t.1))
            }
            "GetTypeFromTalent" => {
                Value::Int(a.first().and_then(|t| self.talent_of(t)).map_or(0, |t| t.0))
            }
            "GetCreatureHasTalent" => {
                let t = a.first().and_then(|t| self.talent_of(t));
                b(match (t, self.get(obj(1))) {
                    (Some(t), Some(x)) => self.has_talent(x, t),
                    _ => false,
                })
            }
            "GetCreatureTalentRandom" | "GetCreatureTalentBest" => {
                let cat = int(0);
                let who = if name == "GetCreatureTalentRandom" {
                    obj(1)
                } else {
                    obj(2)
                };
                let list: Vec<(i32, i32)> = self
                    .get(who)
                    .map(|x| {
                        x.talents
                            .iter()
                            .filter(|t| t.0 == cat)
                            .map(|t| (t.1, t.2))
                            .collect()
                    })
                    .unwrap_or_default();
                if list.is_empty() {
                    Value::Engine(3, 0)
                } else {
                    let pick = if name == "GetCreatureTalentRandom" {
                        self.below(list.len() as i32) as usize
                    } else {
                        0
                    };
                    self.talent(list[pick].0, list[pick].1)
                }
            }
            "GetBaseItemType" => Value::Int(self.get(obj(0)).map_or(0, |x| x.base_item)),
            "GetItemInSlot" => o(self
                .get(obj(1))
                .and_then(|x| x.slots.get(&int(0)).copied())
                .filter(|i| self.get(*i).is_some())),
            "GetItemPossessedBy" => {
                let t = text(1);
                let found = self.get(obj(0)).and_then(|x| {
                    x.inventory
                        .iter()
                        .chain(x.slots.values())
                        .copied()
                        .find(|i| {
                            self.get(*i)
                                .is_some_and(|it| it.tag.eq_ignore_ascii_case(&t))
                        })
                });
                o(found)
            }
            "GetFirstItemInInventory" | "GetNextItemInInventory" => {
                let who = self.resolve(obj(0));
                let at = if name == "GetFirstItemInInventory" {
                    0
                } else {
                    self.inventory_cursor
                        .get(&who)
                        .copied()
                        .unwrap_or(usize::MAX)
                };
                let items: Vec<i32> = self
                    .get(who)
                    .map(|x| {
                        x.inventory
                            .iter()
                            .copied()
                            .filter(|i| self.get(*i).is_some())
                            .collect()
                    })
                    .unwrap_or_default();
                self.inventory_cursor.insert(who, at.saturating_add(1));
                o(items.get(at).copied())
            }
            "CreateItemOnObject" => {
                let target = self.resolve(obj(1));
                if self.get(target).is_none() {
                    return Some(Value::Object(OBJECT_INVALID));
                }
                let item = self.create(Object {
                    kind: OBJECT_TYPE_ITEM,
                    tag: text(0),
                    name: text(0),
                    ..Object::default()
                });
                if let Some(x) = self.get_mut(target) {
                    x.inventory.push(item);
                }
                Value::Object(item)
            }
            "DestroyObject" => {
                if let Some(x) = self.get_mut(obj(0)) {
                    x.destroyed = true;
                }
                Value::Void
            }
            "GetPosition" => Value::Vector(self.get(obj(0)).map_or([0.0; 3], |x| x.position)),
            "GetFacing" => Value::Float(self.get(obj(0)).map_or(0.0, |x| x.facing)),
            "GetLocation" => match self.get(obj(0)) {
                Some(x) => {
                    let l = (x.area, x.position, x.facing);
                    self.locations.push(l);
                    Value::Engine(2, self.locations.len() as u32)
                }
                None => Value::Engine(2, 0),
            },
            "GetDistanceBetween" => Value::Float(self.distance(obj(0), obj(1)).unwrap_or(0.0)),
            "AssignCommand" => {
                if let Some(Value::Action(c)) = a.get(1) {
                    let owner = self.resolve(obj(0));
                    self.owners.insert(c.id, owner);
                }
                Value::Void
            }
            "DelayCommand" | "ActionDoCommand" => {
                let i = if name == "DelayCommand" { 1 } else { 0 };
                if let Some(Value::Action(c)) = a.get(i) {
                    self.owners.insert(c.id, self.context.caller);
                }
                Value::Void
            }
            n if MODELS
                .iter()
                .any(|(m, k, _)| *m == n && *k == Kind::Recorded) =>
            {
                Value::Void
            }
            _ => return None,
        })
    }
}

impl Engine for World {
    fn call(&mut self, _routine: u16, sig: &Routine, args: &[Value]) -> Reply {
        match self.model(&sig.name, args) {
            Some(value) => Reply {
                value,
                modelled: true,
            },
            None => Reply {
                value: Value::default_of(&sig.ret),
                modelled: false,
            },
        }
    }

    fn deferred_begins(&mut self, closure: &Closure) {
        if let Some(owner) = self.owners.get(&closure.id) {
            self.context.caller = *owner;
        }
    }
}

/// `docs/engine-model.md`, generated from [`MODELS`].
pub fn model_doc() -> String {
    let mut s = String::from(
        "# The engine model\n\nThe engine routines `nwscript_vm::World` models, and what each does there. A routine not listed returns its type's default, and the call is marked unmodelled in the trace. This file is generated from `MODELS` in `crates/nwscript-vm/src/world.rs`; a test fails when the two differ.\n",
    );
    for (kind, title, intro) in [
        (
            Kind::Pure,
            "Computed exactly",
            "Functions of their arguments alone.",
        ),
        (
            Kind::State,
            "Modelled with the world's state",
            "They read or change the world: its objects, variables, party, context and generator.",
        ),
        (
            Kind::Recorded,
            "Recorded",
            "Their effects lie outside anything a script can read back through the model, so the call in the trace is the whole of their behaviour here.",
        ),
    ] {
        s.push_str(&format!(
            "\n## {title}\n\n{intro}\n\n| Routine | Here |\n| --- | --- |\n"
        ));
        for (name, k, what) in MODELS {
            if *k == kind {
                s.push_str(&format!("| `{name}` | {what} |\n"));
            }
        }
    }
    s
}
