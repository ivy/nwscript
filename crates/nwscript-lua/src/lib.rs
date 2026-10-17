//! The Lua binding of the façade: the test and scripting language.
//!
//! [`install`] puts the `nws` module, busted-style `describe`/`it`, and an
//! `assert` table with engine-call matchers into a Lua state. The engine a
//! script runs against is a [`World`](nwscript_vm::World) whose routines a
//! test can replace one by one: `w.engine.GetReputation = function(a, b)
//! return 100 end` answers that routine for that world, and the call counts
//! as modelled. `testing.md` beside this crate is the reference, printed by `nws test --reference` ([`REFERENCE`]); `lua/nws.lua` declares
//! the API for editors.

/// The reference for tests and Lua scripts: worlds, objects, runs,
/// assertions and snapshots.
pub const REFERENCE: &str = include_str!("../testing.md");

mod convert;
mod runner;
mod world;

pub use runner::{TestOutcome, run_lua_file, run_test_file};

use mlua::{Lua, Table, Value as LuaValue};
use nwscript_sema::{ConstValue, EngineTable};
use nwscript_vm::Routine;
use std::rc::Rc;

/// Finds a script's bytecode by name, without extension, and says where it was.
pub type FindScript = Rc<dyn Fn(&str) -> Option<(String, Vec<u8>)>>;

/// What the binding needs from its host: the engine table and a way to find
/// a script's bytecode by name.
#[derive(Clone)]
pub struct Host {
    /// The game profile's name, `k1` or `k2`.
    pub game: String,
    pub table: Rc<EngineTable>,
    pub routines: Rc<Vec<Routine>>,
    /// The bytecode of a script by name, without extension, and where it was found.
    pub find: FindScript,
}

/// Install the `nws` module, `describe`, `it`, `before_each` and the
/// extended `assert` into `lua`.
pub fn install(lua: &Lua, host: Host) -> mlua::Result<()> {
    let nws = lua.create_table()?;
    let any = lua.create_table()?;
    lua.set_named_registry_value("nws.any", any.clone())?;
    nws.set("any", any)?;
    let consts = lua.create_table()?;
    for c in &host.table.constants {
        let v = match &c.value {
            ConstValue::Int(i) => LuaValue::Integer(i64::from(*i)),
            ConstValue::Float(f) => LuaValue::Number(f64::from(*f)),
            ConstValue::String(s) => LuaValue::String(lua.create_string(s)?),
            _ => continue,
        };
        consts.set(c.name.as_str(), v)?;
    }
    nws.set("const", consts)?;
    nws.set("game", host.game.clone())?;
    world::install(lua, &nws, host)?;
    lua.globals().set("nws", nws)?;
    runner::install_assert(lua)?;
    Ok(())
}

pub(crate) fn any_sentinel(lua: &Lua) -> mlua::Result<Table> {
    lua.named_registry_value("nws.any")
}
