//! A creature as a save keeps it: the fields a question about its behaviour
//! starts from, read out of its GFF struct. The layout is in
//! `docs/formats/save.md`, "Creatures".

use std::collections::HashMap;

use rakata_gff::{Struct, Value};

use crate::fields::{float, int, loc_text, string, u32_field, value_int};

/// The object id the game stores where there is no object: an empty hand's
/// `LeftEquip`, an unused effect object slot.
pub const OBJECT_INVALID: u32 = 0x7F00_0000;

/// Where in the save a creature's record is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// In a module's state: `member` is the GFF (`m14ac.git`, `Module.ifo`),
    /// `path` the struct within it (`Creature List[5]`).
    Module {
        module: String,
        member: String,
        path: String,
    },
    /// A party member's record, `AVAILNPC<npc>.utc` in `SAVEGAME.sav`.
    PartyMember { npc: u32 },
}

impl Place {
    /// The place as one line: `danm14ac/m14ac.git Creature List[5]`,
    /// `SAVEGAME.sav/AVAILNPC6.utc`.
    pub fn describe(&self) -> String {
        match self {
            Place::Module {
                module,
                member,
                path,
            } => format!("{module}/{member} {path}"),
            Place::PartyMember { npc } => format!("SAVEGAME.sav/AVAILNPC{npc}.utc"),
        }
    }
}

/// An object id with the tag of the object it names, when that object is in
/// the same state as the reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRef {
    pub id: u32,
    /// The tag as saved, or the first name of an object whose tag is empty
    /// (the player character's is); `None` when the save holds no object
    /// with this id where it was looked for.
    pub tag: Option<String>,
}

/// One effect on a creature.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    /// The effect's serial number (`Id`).
    pub id: Option<u64>,
    /// The engine's effect type. Its names live in the engine.
    pub effect_type: u32,
    pub subtype: u32,
    pub duration: Option<f32>,
    pub creator: ObjectRef,
    /// The spell that applied it; `None` for `0xFFFFFFFF`, none.
    pub spell: Option<u32>,
    /// The effect's integer parameters (`IntList`), as many as
    /// `NumIntegers` says.
    pub ints: Vec<i32>,
}

/// One parameter of a queued action.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Int(i32),
    Float(f32),
    Object(ObjectRef),
    String(String),
    /// A stored script situation (a `DelayCommand` or `ActionDoCommand`
    /// body): its script's name and code size.
    Script {
        name: String,
        code_size: Option<i64>,
    },
    /// A parameter type not listed here: its `Type` and the value's GFF
    /// type name.
    Other {
        type_id: u32,
        value: String,
    },
}

/// One entry of a creature's action queue, in queue order.
#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    /// The engine's action id. [`crate::action_name`] names those a save has
    /// shown the meaning of.
    pub id: u32,
    /// `GroupActionId`: the actions one script call queued share it.
    pub group: Option<u32>,
    pub params: Vec<Param>,
}

/// A creature, read from a save.
#[derive(Debug, Clone, PartialEq)]
pub struct Creature {
    pub place: Place,
    pub object_id: Option<u32>,
    /// The tag as the save has it, which may differ in case from the
    /// blueprint's.
    pub tag: String,
    /// The first name's embedded text, if any.
    pub name: Option<String>,
    pub commandable: Option<i64>,
    pub faction: Option<i64>,
    /// The faction's name from the save's `REPUTE.fac`.
    pub faction_name: Option<String>,
    /// The engine's animation number. Its names live in the engine.
    pub animation: Option<i64>,
    pub effects: Vec<Effect>,
    /// The local booleans that are set (`SWVarTable.BitArray`), by index.
    pub booleans: Vec<u32>,
    /// Every local number (`SWVarTable.ByteArray`), by index.
    pub numbers: Vec<u8>,
    pub actions: Vec<Action>,
}

/// Object ids to tags, over the objects of one state.
#[derive(Debug, Clone, Default)]
pub(crate) struct Objects {
    tags: HashMap<u32, String>,
}

impl Objects {
    /// Every struct under `s` that carries an `ObjectId` and a `Tag`.
    pub fn add(&mut self, s: &Struct) {
        if let (Some(id), Some(tag)) = (u32_field(s, "ObjectId"), string(s, "Tag")) {
            let shown = if tag.is_empty() {
                loc_text(s, "FirstName").unwrap_or_default()
            } else {
                tag
            };
            self.tags.entry(id).or_insert(shown);
        }
        for f in &s.fields {
            match &f.value {
                Value::Struct(c) => self.add(c),
                Value::List(l) => l.iter().for_each(|c| self.add(c)),
                _ => {}
            }
        }
    }

    pub fn object(&self, id: u32) -> ObjectRef {
        ObjectRef {
            id,
            tag: self.tags.get(&id).cloned(),
        }
    }
}

/// The creature in struct `s`, its references resolved through `objects`
/// and its faction named through `factions`.
pub(crate) fn read(s: &Struct, place: Place, objects: &Objects, factions: &[String]) -> Creature {
    let faction = int(s, "FactionID");
    let sw = s.get("SWVarTable").and_then(Value::as_struct);
    let mut booleans = Vec::new();
    let mut numbers = Vec::new();
    if let Some(sw) = sw {
        for (w, e) in list(sw, "BitArray").iter().enumerate() {
            let word = u32_field(e, "Variable").unwrap_or(0);
            for bit in 0..32 {
                if word & (1 << bit) != 0 {
                    booleans.push(w as u32 * 32 + bit);
                }
            }
        }
        numbers = list(sw, "ByteArray")
            .iter()
            .map(|e| int(e, "Variable").unwrap_or(0) as u8)
            .collect();
    }
    Creature {
        place,
        object_id: u32_field(s, "ObjectId"),
        tag: string(s, "Tag").unwrap_or_default(),
        name: loc_text(s, "FirstName"),
        commandable: int(s, "Commandable"),
        faction,
        faction_name: faction
            .and_then(|f| usize::try_from(f).ok())
            .and_then(|f| factions.get(f).cloned()),
        animation: int(s, "Animation"),
        effects: list(s, "EffectList")
            .iter()
            .map(|e| effect(e, objects))
            .collect(),
        booleans,
        numbers,
        actions: list(s, "ActionList")
            .iter()
            .map(|a| action(a, objects))
            .collect(),
    }
}

fn list<'a>(s: &'a Struct, label: &str) -> &'a [Struct] {
    s.get(label).and_then(Value::as_list).unwrap_or(&[])
}

fn effect(e: &Struct, objects: &Objects) -> Effect {
    let n = int(e, "NumIntegers").unwrap_or(0).max(0) as usize;
    Effect {
        id: int(e, "Id").map(|v| v as u64),
        effect_type: u32_field(e, "Type").unwrap_or(0),
        subtype: u32_field(e, "SubType").unwrap_or(0),
        duration: float(e, "Duration"),
        creator: objects.object(u32_field(e, "CreatorId").unwrap_or(OBJECT_INVALID)),
        spell: u32_field(e, "SpellId").filter(|&s| s != u32::MAX),
        ints: list(e, "IntList")
            .iter()
            .take(n)
            .map(|v| int(v, "Value").unwrap_or(0) as i32)
            .collect(),
    }
}

fn action(a: &Struct, objects: &Objects) -> Action {
    Action {
        id: u32_field(a, "ActionId").unwrap_or(0),
        group: u32_field(a, "GroupActionId"),
        // The game spells the label `Paramaters`.
        params: list(a, "Paramaters")
            .iter()
            .map(|p| param(p, objects))
            .collect(),
    }
}

/// A parameter by its `Type`: 1 an int, 2 a float, 3 an object, 4 a
/// string, 5 a script situation (`docs/formats/save.md`).
fn param(p: &Struct, objects: &Objects) -> Param {
    let type_id = u32_field(p, "Type").unwrap_or(0);
    let value = p.get("Value");
    match (type_id, value) {
        (1, Some(v)) if value_int(v).is_some() => Param::Int(value_int(v).unwrap_or(0) as i32),
        (2, Some(Value::Float(f))) => Param::Float(*f),
        (3, Some(v)) if value_int(v).is_some() => {
            Param::Object(objects.object(value_int(v).unwrap_or(0) as u32))
        }
        (4, Some(Value::String(b))) => Param::String(String::from_utf8_lossy(b).into_owned()),
        (5, Some(Value::Struct(s))) => Param::Script {
            name: string(s, "Name").unwrap_or_default(),
            code_size: int(s, "CodeSize"),
        },
        (t, v) => Param::Other {
            type_id: t,
            value: v.map_or("absent", |v| v.field_type().name()).to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rakata_gff::Label;

    fn l(t: &str) -> Label {
        Label::new(t).unwrap()
    }

    fn word(v: u32) -> Struct {
        let mut s = Struct::new(0);
        s.push(l("Variable"), Value::Dword(v));
        s
    }

    #[test]
    fn local_booleans_count_from_the_low_bit_of_each_word() {
        let mut sw = Struct::new(0);
        sw.push(
            l("BitArray"),
            Value::List(vec![word(1), word(0x4000_0000), word(0)]),
        );
        let mut c = Struct::new(4);
        c.push(l("Tag"), Value::string("carth"));
        c.push(l("SWVarTable"), Value::Struct(sw));
        let got = read(&c, Place::PartyMember { npc: 2 }, &Objects::default(), &[]);
        assert_eq!(got.booleans, vec![0, 62]);
        assert_eq!(got.tag, "carth");
    }

    #[test]
    fn action_parameters_read_by_type_and_objects_by_tag() {
        let mut target = Struct::new(4);
        target.push(l("ObjectId"), Value::Dword(569));
        target.push(l("Tag"), Value::string("mission"));
        let mut objects = Objects::default();
        objects.add(&target);

        let p = |t: u32, v: Value| {
            let mut s = Struct::new(1);
            s.push(l("Type"), Value::Dword(t));
            s.push(l("Value"), v);
            s
        };
        let mut a = Struct::new(0);
        a.push(l("ActionId"), Value::Dword(24));
        a.push(l("GroupActionId"), Value::Word(2));
        a.push(
            l("Paramaters"),
            Value::List(vec![
                p(3, Value::Dword(569)),
                p(4, Value::string("oaj_banter")),
                p(1, Value::Int(1)),
                p(2, Value::Float(0.5)),
                p(9, Value::Byte(0)),
            ]),
        );
        let got = action(&a, &objects);
        assert_eq!(got.id, 24);
        assert_eq!(got.group, Some(2));
        assert_eq!(
            got.params,
            vec![
                Param::Object(ObjectRef {
                    id: 569,
                    tag: Some("mission".into())
                }),
                Param::String("oaj_banter".into()),
                Param::Int(1),
                Param::Float(0.5),
                Param::Other {
                    type_id: 9,
                    value: "BYTE".into()
                },
            ]
        );
    }
}
