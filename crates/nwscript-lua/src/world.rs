//! `nws.world`, `nws.script`, and the userdata behind them.

use crate::Host;
use crate::convert::{from_lua, to_lua};
use mlua::{
    AnyUserData, Lua, MetaMethod, MultiValue, Table, UserData, UserDataFields, UserDataMethods,
    Value as LuaValue,
};
use nwscript_ncs::Program;
use nwscript_vm::world::Object;
use nwscript_vm::{
    Call, Closure, Engine, Limits, OBJECT_INVALID, Reply, Routine, Value, World, run,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A world and the routines a test has replaced in it.
pub struct LWorld {
    pub w: Rc<RefCell<World>>,
    pub engine: Table,
}

/// A world's global variables, by name.
pub struct LGlobals {
    w: Rc<RefCell<World>>,
}

/// An object in a world.
#[derive(Clone)]
pub struct LObject {
    pub w: Rc<RefCell<World>>,
    pub id: i32,
}

/// An engine structure value: its type number and handle.
#[derive(Clone, Copy)]
pub struct LEngine {
    pub kind: u8,
    pub handle: u32,
}

/// A loaded script.
pub struct LScript {
    name: String,
    program: Rc<Program>,
    routines: Rc<Vec<Routine>>,
}

/// The outcome of a run.
pub struct LRun {
    pub w: Rc<RefCell<World>>,
    pub caller: i32,
    pub result: Option<Value>,
    pub as_bool: bool,
    pub trace: Vec<Call>,
    pub unmodelled: Vec<String>,
    pub fault: Option<String>,
}

fn kind_of(v: &LuaValue) -> mlua::Result<i32> {
    Ok(match v {
        LuaValue::Integer(i) => *i as i32,
        LuaValue::String(s) => match s.to_str()?.as_ref() {
            "creature" => 1,
            "item" => 2,
            "trigger" => 4,
            "door" => 8,
            "waypoint" => 32,
            "placeable" => 64,
            "store" => 128,
            "area" => 0,
            other => {
                return Err(mlua::Error::runtime(format!(
                    "unknown object kind `{other}`: creature, item, trigger, door, waypoint, placeable, store or area"
                )));
            }
        },
        _ => {
            return Err(mlua::Error::runtime(
                "an object kind is a name such as \"creature\" or an OBJECT_TYPE_* number",
            ));
        }
    })
}

fn object_of(v: &LuaValue) -> mlua::Result<i32> {
    match v {
        LuaValue::Nil => Ok(OBJECT_INVALID),
        LuaValue::UserData(u) => Ok(u.borrow::<LObject>()?.id),
        LuaValue::Integer(i) => Ok(*i as i32),
        _ => Err(mlua::Error::runtime("expected an object")),
    }
}

impl LObject {
    fn with<R>(&self, f: impl FnOnce(&Object) -> R) -> Option<R> {
        self.w.borrow().objects.get(&self.id).map(f)
    }

    fn set_field(&self, key: &str, v: &LuaValue) -> mlua::Result<()> {
        let mut w = self.w.borrow_mut();
        let o = w
            .objects
            .get_mut(&self.id)
            .ok_or_else(|| mlua::Error::runtime("the object is not in the world"))?;
        let int = || -> mlua::Result<i32> {
            match v {
                LuaValue::Integer(i) => Ok(*i as i32),
                LuaValue::Boolean(b) => Ok(i32::from(*b)),
                _ => Err(mlua::Error::runtime(format!("`{key}` takes an integer"))),
            }
        };
        let truth =
            || matches!(v, LuaValue::Boolean(true)) || matches!(v, LuaValue::Integer(i) if *i != 0);
        match key {
            "tag" => o.tag = v.to_string()?,
            "name" => o.name = v.to_string()?,
            "resref" => o.resref = v.to_string()?,
            "hp" => o.hp = int()?,
            "max_hp" => o.max_hp = int()?,
            "dead" => o.dead = truth(),
            "race" => o.race = int()?,
            "gender" => o.gender = int()?,
            "faction" => o.faction = int()?,
            "ai_style" => o.ai_style = int()?,
            "plot" => o.plot = truth(),
            "in_combat" => o.in_combat = truth(),
            "is_pc" => o.is_pc = truth(),
            "area" => o.area = object_of(v)?,
            "base_item" => o.base_item = int()?,
            "facing" => o.facing = v.as_f64().unwrap_or(0.0) as f32,
            "position" => {
                let LuaValue::Table(t) = v else {
                    return Err(mlua::Error::runtime("`position` takes {x, y, z}"));
                };
                let c = |k: &str, i: i64| -> f32 {
                    t.get::<f64>(k).or_else(|_| t.get::<f64>(i)).unwrap_or(0.0) as f32
                };
                o.position = [c("x", 1), c("y", 2), c("z", 3)];
            }
            other => {
                return Err(mlua::Error::runtime(format!(
                    "objects have no field `{other}`; `nws test --reference` lists them"
                )));
            }
        }
        Ok(())
    }
}

impl UserData for LObject {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method(MetaMethod::Eq, |_, a, b: AnyUserData| {
            Ok(b.borrow::<LObject>().is_ok_and(|b| b.id == a.id))
        });
        m.add_meta_method(MetaMethod::ToString, |_, o, ()| {
            Ok(o.with(|x| format!("object#{} ({})", o.id, x.tag))
                .unwrap_or_else(|| format!("object#{}", o.id)))
        });
        m.add_meta_method(MetaMethod::Index, |lua, o, key: String| {
            if let Ok(methods) = lua.named_registry_value::<Table>("nws.object_methods")
                && let Ok(LuaValue::Function(f)) = methods.raw_get::<LuaValue>(key.as_str())
            {
                return Ok(LuaValue::Function(f));
            }
            let v = o.with(|x| -> mlua::Result<LuaValue> {
                Ok(match key.as_str() {
                    "id" => LuaValue::Integer(i64::from(o.id)),
                    "tag" => LuaValue::String(lua.create_string(&x.tag)?),
                    "name" => LuaValue::String(lua.create_string(&x.name)?),
                    "resref" => LuaValue::String(lua.create_string(&x.resref)?),
                    "hp" => LuaValue::Integer(i64::from(x.hp)),
                    "max_hp" => LuaValue::Integer(i64::from(x.max_hp)),
                    "dead" => LuaValue::Boolean(x.dead),
                    "race" => LuaValue::Integer(i64::from(x.race)),
                    "gender" => LuaValue::Integer(i64::from(x.gender)),
                    "faction" => LuaValue::Integer(i64::from(x.faction)),
                    "ai_style" => LuaValue::Integer(i64::from(x.ai_style)),
                    "plot" => LuaValue::Boolean(x.plot),
                    "in_combat" => LuaValue::Boolean(x.in_combat),
                    "is_pc" => LuaValue::Boolean(x.is_pc),
                    "destroyed" => LuaValue::Boolean(x.destroyed),
                    "base_item" => LuaValue::Integer(i64::from(x.base_item)),
                    "facing" => LuaValue::Number(f64::from(x.facing)),
                    "position" => {
                        let t = lua.create_table()?;
                        t.set("x", x.position[0])?;
                        t.set("y", x.position[1])?;
                        t.set("z", x.position[2])?;
                        LuaValue::Table(t)
                    }
                    "inventory" => {
                        let t = lua.create_table()?;
                        for (i, id) in x.inventory.iter().enumerate() {
                            t.set(
                                i + 1,
                                LObject {
                                    w: o.w.clone(),
                                    id: *id,
                                },
                            )?;
                        }
                        LuaValue::Table(t)
                    }
                    _ => LuaValue::Nil,
                })
            });
            match v {
                Some(r) => r,
                None if key == "id" => Ok(LuaValue::Integer(i64::from(o.id))),
                None => Ok(LuaValue::Nil),
            }
        });
        m.add_meta_method(
            MetaMethod::NewIndex,
            |_, o, (key, v): (String, LuaValue)| o.set_field(&key, &v),
        );
    }
}

/// Methods on objects. `__index` answers a method's name from this table
/// and any other name as a field.
fn object_methods(lua: &Lua) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set(
        "give",
        lua.create_function(|_, (o, tag): (AnyUserData, String)| {
            let o = o.borrow::<LObject>()?.clone();
            let id = o.w.borrow_mut().create(Object {
                kind: 2,
                tag: tag.clone(),
                resref: tag.clone(),
                name: tag,
                ..Object::default()
            });
            if let Some(x) = o.w.borrow_mut().objects.get_mut(&o.id) {
                x.inventory.push(id);
            }
            Ok(LObject { w: o.w.clone(), id })
        })?,
    )?;
    t.set(
        "equip",
        lua.create_function(|_, (o, slot, tag): (AnyUserData, i32, String)| {
            let o = o.borrow::<LObject>()?.clone();
            let id = o.w.borrow_mut().create(Object {
                kind: 2,
                tag: tag.clone(),
                resref: tag.clone(),
                name: tag,
                ..Object::default()
            });
            if let Some(x) = o.w.borrow_mut().objects.get_mut(&o.id) {
                x.slots.insert(slot, id);
            }
            Ok(LObject { w: o.w.clone(), id })
        })?,
    )?;
    macro_rules! set_add {
        ($name:literal, $field:ident) => {
            t.set(
                $name,
                lua.create_function(|_, (o, n): (AnyUserData, i32)| {
                    let o = o.borrow::<LObject>()?;
                    if let Some(x) = o.w.borrow_mut().objects.get_mut(&o.id) {
                        x.$field.insert(n);
                    }
                    Ok(())
                })?,
            )?;
        };
    }
    set_add!("learn", spells);
    set_add!("feat", feats);
    set_add!("affect", spell_effects);
    t.set(
        "level",
        lua.create_function(|_, (o, class, levels): (AnyUserData, i32, i32)| {
            let o = o.borrow::<LObject>()?;
            if let Some(x) = o.w.borrow_mut().objects.get_mut(&o.id) {
                x.classes.insert(class, levels);
            }
            Ok(())
        })?,
    )?;
    t.set(
        "talent",
        lua.create_function(|_, (o, category, kind, id): (AnyUserData, i32, i32, i32)| {
            let o = o.borrow::<LObject>()?;
            if let Some(x) = o.w.borrow_mut().objects.get_mut(&o.id) {
                x.talents.push((category, kind, id));
            }
            Ok(())
        })?,
    )?;
    t.set(
        "set_local",
        lua.create_function(|_, (o, index, v): (AnyUserData, i32, LuaValue)| {
            let o = o.borrow::<LObject>()?;
            let mut w = o.w.borrow_mut();
            let x = w
                .objects
                .get_mut(&o.id)
                .ok_or_else(|| mlua::Error::runtime("the object is not in the world"))?;
            match v {
                LuaValue::Boolean(b) => {
                    x.booleans.insert(index, b);
                }
                LuaValue::Integer(i) => {
                    x.numbers.insert(index, i as i32);
                }
                _ => return Err(mlua::Error::runtime("a local is a boolean or an integer")),
            }
            Ok(())
        })?,
    )?;
    t.set(
        "valid",
        lua.create_function(|_, o: AnyUserData| {
            let o = o.borrow::<LObject>()?;
            Ok(o.w.borrow().get(o.id).is_some())
        })?,
    )?;
    Ok(t)
}

impl UserData for LEngine {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("kind", |_, e| Ok(e.kind));
        f.add_field_method_get("handle", |_, e| Ok(e.handle));
    }
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method(MetaMethod::Eq, |_, a, b: AnyUserData| {
            Ok(b.borrow::<LEngine>()
                .is_ok_and(|b| b.kind == a.kind && b.handle == a.handle))
        });
        m.add_meta_method(MetaMethod::ToString, |_, e, ()| {
            Ok(format!("engine{}#{}", e.kind, e.handle))
        });
    }
}

impl UserData for LGlobals {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method(MetaMethod::Index, |lua, g, name: String| {
            let w = g.w.borrow();
            Ok(if let Some(n) = w.globals_number.get(&name) {
                LuaValue::Integer(i64::from(*n))
            } else if let Some(b) = w.globals_boolean.get(&name) {
                LuaValue::Boolean(*b)
            } else if let Some(s) = w.globals_string.get(&name) {
                LuaValue::String(lua.create_string(s)?)
            } else {
                LuaValue::Nil
            })
        });
        m.add_meta_method(
            MetaMethod::NewIndex,
            |_, g, (name, v): (String, LuaValue)| {
                let mut w = g.w.borrow_mut();
                match v {
                    LuaValue::Integer(i) => {
                        w.globals_number.insert(name, i as i32);
                    }
                    LuaValue::Boolean(b) => {
                        w.globals_boolean.insert(name, b);
                    }
                    LuaValue::String(s) => {
                        w.globals_string.insert(name, s.as_bytes().to_vec());
                    }
                    LuaValue::Nil => {
                        w.globals_number.remove(&name);
                        w.globals_boolean.remove(&name);
                        w.globals_string.remove(&name);
                    }
                    other => {
                        return Err(mlua::Error::runtime(format!(
                            "a global is an integer, boolean or string, not {}",
                            other.type_name()
                        )));
                    }
                }
                Ok(())
            },
        );
    }
}

macro_rules! context_field {
    ($f:ident, $name:literal, $field:ident) => {
        $f.add_field_method_get($name, |_, w| {
            Ok(LObject {
                w: w.w.clone(),
                id: w.w.borrow().context.$field,
            })
        });
        $f.add_field_method_set($name, |_, w, v: LuaValue| {
            w.w.borrow_mut().context.$field = object_of(&v)?;
            Ok(())
        });
    };
}

impl UserData for LWorld {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("globals", |_, w| Ok(LGlobals { w: w.w.clone() }));
        f.add_field_method_get("engine", |_, w| Ok(w.engine.clone()));
        f.add_field_method_get("module", |_, w| {
            Ok(LObject {
                w: w.w.clone(),
                id: w.w.borrow().module,
            })
        });
        context_field!(f, "self", caller);
        context_field!(f, "speaker", pc_speaker);
        context_field!(f, "entering", entering);
        context_field!(f, "exiting", exiting);
        context_field!(f, "opened_by", last_opened_by);
        context_field!(f, "used_by", last_used_by);
        context_field!(f, "clicking", clicking);
        context_field!(f, "perceived", last_perceived);
        f.add_field_method_get("user_event", |_, w| {
            Ok(w.w.borrow().context.user_defined_event)
        });
        f.add_field_method_set("user_event", |_, w, v: i32| {
            w.w.borrow_mut().context.user_defined_event = v;
            Ok(())
        });
        f.add_field_method_get("party", |lua, w| {
            let t = lua.create_table()?;
            for (i, id) in w.w.borrow().party.iter().enumerate() {
                t.set(
                    i + 1,
                    LObject {
                        w: w.w.clone(),
                        id: *id,
                    },
                )?;
            }
            Ok(t)
        });
        f.add_field_method_set("party", |_, w, t: Table| {
            let ids: Vec<i32> = t
                .sequence_values::<LuaValue>()
                .map(|v| object_of(&v?))
                .collect::<mlua::Result<_>>()?;
            w.w.borrow_mut().party = ids;
            Ok(())
        });
        f.add_field_method_get("npc_party", |lua, w| {
            lua.create_sequence_from(w.w.borrow().npc_party.iter().copied())
        });
        f.add_field_method_set("npc_party", |_, w, t: Table| {
            w.w.borrow_mut().npc_party = t.sequence_values::<i32>().collect::<mlua::Result<_>>()?;
            Ok(())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method(
            "create",
            |_, w, (kind, tag, fields): (LuaValue, String, Option<Table>)| {
                let id = w.w.borrow_mut().create(Object {
                    kind: kind_of(&kind)?,
                    tag: tag.clone(),
                    name: tag,
                    ..Object::default()
                });
                let o = LObject { w: w.w.clone(), id };
                if let Some(t) = fields {
                    for pair in t.pairs::<String, LuaValue>() {
                        let (k, v) = pair?;
                        o.set_field(&k, &v)?;
                    }
                }
                Ok(o)
            },
        );
        m.add_method("object", |_, w, tag: String| {
            let found =
                w.w.borrow()
                    .objects
                    .iter()
                    .find(|(_, o)| !o.destroyed && o.tag.eq_ignore_ascii_case(&tag))
                    .map(|(id, _)| *id);
            Ok(found.map(|id| LObject { w: w.w.clone(), id }))
        });
        m.add_method("enemies", |_, w, (a, b): (i32, i32)| {
            w.w.borrow_mut().enemies.insert((a, b));
            Ok(())
        });
    }
}

/// The engine a run sees: the test's replacements first, then the world.
struct LuaEngine<'a> {
    lua: &'a Lua,
    world: Rc<RefCell<World>>,
    overrides: Table,
    caller: i32,
    error: Option<mlua::Error>,
}

impl Engine for LuaEngine<'_> {
    fn call(&mut self, routine: u16, sig: &Routine, args: &[Value]) -> Reply {
        if let Ok(LuaValue::Function(f)) = self.overrides.raw_get::<LuaValue>(sig.name.as_str()) {
            let lua_args: mlua::Result<Vec<LuaValue>> = args
                .iter()
                .map(|a| to_lua(self.lua, &self.world, a, self.caller))
                .collect();
            let reply = lua_args
                .and_then(|a| f.call::<LuaValue>(MultiValue::from_iter(a)))
                .and_then(|v| {
                    from_lua(&v, &sig.ret)
                        .map_err(|e| mlua::Error::runtime(format!("w.engine.{}: {e}", sig.name)))
                });
            return match reply {
                Ok(value) => Reply {
                    value,
                    modelled: true,
                },
                Err(e) => {
                    self.error.get_or_insert(e);
                    Reply {
                        value: Value::default_of(&sig.ret),
                        modelled: true,
                    }
                }
            };
        }
        self.world.borrow_mut().call(routine, sig, args)
    }

    fn deferred_begins(&mut self, closure: &Closure) {
        self.world.borrow_mut().deferred_begins(closure);
    }
}

impl LScript {
    fn execute(&self, lua: &Lua, world: Option<AnyUserData>, as_bool: bool) -> mlua::Result<LRun> {
        let (w, overrides) = match world {
            Some(u) => {
                let lw = u.borrow::<LWorld>()?;
                (lw.w.clone(), lw.engine.clone())
            }
            None => (Rc::new(RefCell::new(World::new(0))), lua.create_table()?),
        };
        let caller = w.borrow().context.caller;
        let mut engine = LuaEngine {
            lua,
            world: w.clone(),
            overrides,
            caller,
            error: None,
        };
        let r = run(
            &self.program,
            &self.routines,
            &mut engine,
            Limits::default(),
        );
        if let Some(e) = engine.error {
            return Err(e);
        }
        let run = LRun {
            w,
            caller,
            result: r.result,
            as_bool,
            trace: r.trace,
            unmodelled: r.unmodelled,
            fault: r.fault.map(|f| format!("{}: {f}", self.name)),
        };
        lua.set_named_registry_value("nws.last_trace", run.trace_text())?;
        Ok(run)
    }
}

impl UserData for LScript {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("name", |_, s| Ok(s.name.clone()));
    }
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("run", |lua, s, w: Option<AnyUserData>| {
            s.execute(lua, w, false)
        });
        m.add_method("conditional", |lua, s, w: Option<AnyUserData>| {
            s.execute(lua, w, true)
        });
    }
}

impl LRun {
    pub fn trace_text(&self) -> String {
        let mut s = String::new();
        for c in &self.trace {
            s.push_str(&format!("    {c}\n"));
        }
        if let Some(f) = &self.fault {
            s.push_str(&format!("    fault: {f}\n"));
        }
        s
    }
}

impl UserData for LRun {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("result", |lua, r| match &r.result {
            None => Ok(LuaValue::Nil),
            Some(Value::Int(i)) if r.as_bool => Ok(LuaValue::Boolean(*i != 0)),
            Some(v) => to_lua(lua, &r.w, v, r.caller),
        });
        f.add_field_method_get("fault", |_, r| Ok(r.fault.clone()));
        f.add_field_method_get("unmodelled", |lua, r| {
            lua.create_sequence_from(r.unmodelled.iter().cloned())
        });
        f.add_field_method_get("trace", |lua, r| {
            let t = lua.create_table()?;
            for (i, c) in r.trace.iter().enumerate() {
                t.set(i + 1, call_table(lua, r, c)?)?;
            }
            Ok(t)
        });
    }
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("calls", |lua, r, name: String| {
            let t = lua.create_table()?;
            for c in r.trace.iter().filter(|c| c.name == name) {
                t.push(call_table(lua, r, c)?)?;
            }
            Ok(t)
        });
        m.add_method("text", |_, r, ()| Ok(r.trace_text()));
        m.add_meta_method(MetaMethod::ToString, |_, r, ()| Ok(r.trace_text()));
    }
}

fn call_table(lua: &Lua, r: &LRun, c: &Call) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("name", c.name.clone())?;
    t.set("routine", c.routine)?;
    let args = lua.create_table()?;
    for (i, a) in c.args.iter().enumerate() {
        args.set(i + 1, to_lua(lua, &r.w, a, r.caller)?)?;
    }
    t.set("args", args)?;
    t.set("ret", to_lua(lua, &r.w, &c.ret, r.caller)?)?;
    t.set("modelled", c.modelled)?;
    t.set("deferred", c.deferred)?;
    Ok(t)
}

/// Install `nws.world`, `nws.script` and `nws.object_methods`.
pub fn install(lua: &Lua, nws: &Table, host: Host) -> mlua::Result<()> {
    let methods = object_methods(lua)?;
    lua.set_named_registry_value("nws.object_methods", methods.clone())?;
    nws.set("object_methods", methods)?;
    nws.set(
        "world",
        lua.create_function(|lua, opts: Option<Table>| {
            let seed = opts
                .as_ref()
                .and_then(|t| t.get::<u64>("seed").ok())
                .unwrap_or(0);
            Ok(LWorld {
                w: Rc::new(RefCell::new(World::new(seed))),
                engine: lua.create_table()?,
            })
        })?,
    )?;
    let h = host.clone();
    nws.set(
        "script",
        lua.create_function(move |_, (name, opts): (String, Option<Table>)| {
            if let Some(g) = opts.as_ref().and_then(|t| t.get::<String>("game").ok())
                && g != h.game
            {
                return Err(mlua::Error::runtime(format!(
                    "the script asks for game `{g}` but nws runs `{}`; pass --game {g}",
                    h.game
                )));
            }
            let (path, bytes) = (h.find)(&name).ok_or_else(|| {
                mlua::Error::runtime(format!(
                    "no compiled script `{name}.ncs` in the test's directory or the -I directories"
                ))
            })?;
            let mut map = nwscript_diag::SourceMap::new();
            let program = nwscript_ncs::decode_named(path.clone(), &bytes, &mut map)
                .map_err(|d| mlua::Error::runtime(format!("{path}: {}", d.message)))?;
            Ok(LScript {
                name,
                program: Rc::new(program),
                routines: h.routines.clone(),
            })
        })?,
    )?;
    Ok(())
}
