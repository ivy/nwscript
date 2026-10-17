//! `describe`/`it`, the extended `assert`, and running test and script files.

use crate::convert::matches;
use crate::world::LRun;
use crate::{Host, install};
use mlua::{AnyUserData, Function, Lua, MultiValue, Table, Value as LuaValue};
use std::path::Path;

/// One test's outcome.
#[derive(Clone, Debug)]
pub struct TestOutcome {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
    /// The trace of the last script the test ran, for a failure.
    pub trace: String,
}

/// Install the matcher table as `assert`, keeping `assert(v, msg)` callable.
pub fn install_assert(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    let original: Function = g.get("assert")?;
    let a = lua.create_table()?;
    let mt = lua.create_table()?;
    mt.set(
        "__call",
        lua.create_function(move |_, args: MultiValue| {
            original.call::<MultiValue>(args.into_iter().skip(1).collect::<MultiValue>())
        })?,
    )?;
    a.set_metatable(Some(mt))?;
    let fail = |m: String| Err::<(), _>(mlua::Error::runtime(m));
    a.set(
        "is_true",
        lua.create_function(move |_, (v, msg): (LuaValue, Option<String>)| {
            if matches!(v, LuaValue::Boolean(true)) {
                Ok(())
            } else {
                fail(msg.unwrap_or(format!("expected true, got {}", show(&v))))
            }
        })?,
    )?;
    a.set(
        "is_false",
        lua.create_function(move |_, (v, msg): (LuaValue, Option<String>)| {
            if matches!(v, LuaValue::Boolean(false)) {
                Ok(())
            } else {
                fail(msg.unwrap_or(format!("expected false, got {}", show(&v))))
            }
        })?,
    )?;
    a.set(
        "is_nil",
        lua.create_function(move |_, v: LuaValue| {
            if v.is_nil() {
                Ok(())
            } else {
                fail(format!("expected nil, got {}", show(&v)))
            }
        })?,
    )?;
    a.set(
        "equal",
        lua.create_function(
            move |_, (want, got, msg): (LuaValue, LuaValue, Option<String>)| {
                if want.equals(&got)? {
                    Ok(())
                } else {
                    fail(msg.unwrap_or(format!("expected {}, got {}", show(&want), show(&got))))
                }
            },
        )?,
    )?;
    a.set(
        "called",
        lua.create_function(|lua, args: MultiValue| {
            let (run, name, want) = run_name_args(args)?;
            let r = run.borrow::<LRun>()?;
            let hit = r.trace.iter().any(|c| {
                c.name == name
                    && want
                        .iter()
                        .enumerate()
                        .all(|(i, w)| c.args.get(i).is_some_and(|a| matches(lua, a, w, r.caller)))
            });
            if hit {
                Ok(())
            } else {
                Err(mlua::Error::runtime(format!(
                    "expected a call to {name}{} in the trace:\n{}",
                    describe_args(&want),
                    r.trace_text()
                )))
            }
        })?,
    )?;
    a.set(
        "not_called",
        lua.create_function(|lua, args: MultiValue| {
            let (run, name, want) = run_name_args(args)?;
            let r = run.borrow::<LRun>()?;
            let hit = r.trace.iter().any(|c| {
                c.name == name
                    && want
                        .iter()
                        .enumerate()
                        .all(|(i, w)| c.args.get(i).is_some_and(|a| matches(lua, a, w, r.caller)))
            });
            if hit {
                Err(mlua::Error::runtime(format!(
                    "expected no call to {name}{}, but the trace has one:\n{}",
                    describe_args(&want),
                    r.trace_text()
                )))
            } else {
                Ok(())
            }
        })?,
    )?;
    a.set(
        "called_before",
        lua.create_function(|_, (run, first, second): (AnyUserData, String, String)| {
            let r = run.borrow::<LRun>()?;
            let a = r.trace.iter().position(|c| c.name == first);
            let b = r.trace.iter().position(|c| c.name == second);
            match (a, b) {
                (Some(a), Some(b)) if a < b => Ok(()),
                _ => Err(mlua::Error::runtime(format!(
                    "expected {first} before {second}:\n{}",
                    r.trace_text()
                ))),
            }
        })?,
    )?;
    a.set(
        "modelled",
        lua.create_function(|_, run: AnyUserData| {
            let r = run.borrow::<LRun>()?;
            if r.unmodelled.is_empty() {
                Ok(())
            } else {
                Err(mlua::Error::runtime(format!(
                    "the run depends on unmodelled routines: {}",
                    r.unmodelled.join(", ")
                )))
            }
        })?,
    )?;
    g.set("assert", a)?;
    Ok(())
}

fn run_name_args(args: MultiValue) -> mlua::Result<(AnyUserData, String, Vec<LuaValue>)> {
    let mut it = args.into_iter();
    let run = match it.next() {
        Some(LuaValue::UserData(u)) => u,
        _ => {
            return Err(mlua::Error::runtime(
                "the first argument is a run, as `script:run(w)` returns",
            ));
        }
    };
    let name = match it.next() {
        Some(LuaValue::String(s)) => s.to_str()?.to_string(),
        _ => {
            return Err(mlua::Error::runtime(
                "the second argument is the routine's name",
            ));
        }
    };
    Ok((run, name, it.collect()))
}

fn show(v: &LuaValue) -> String {
    match v {
        LuaValue::String(s) => format!("{:?}", s.to_string_lossy()),
        LuaValue::Nil => "nil".into(),
        LuaValue::Boolean(b) => b.to_string(),
        LuaValue::Integer(i) => i.to_string(),
        LuaValue::Number(n) => n.to_string(),
        other => other
            .to_string()
            .unwrap_or_else(|_| other.type_name().to_string()),
    }
}

fn describe_args(want: &[LuaValue]) -> String {
    if want.is_empty() {
        return String::new();
    }
    format!("({})", want.iter().map(show).collect::<Vec<_>>().join(", "))
}

/// Install `describe`, `it` and `before_each`, collecting tests into the registry.
fn install_collectors(lua: &Lua) -> mlua::Result<()> {
    let tests = lua.create_table()?;
    lua.set_named_registry_value("nws.tests", tests)?;
    lua.set_named_registry_value("nws.prefix", lua.create_table()?)?;
    lua.set_named_registry_value("nws.before", lua.create_table()?)?;
    lua.globals().set(
        "describe",
        lua.create_function(|lua, (name, body): (String, Function)| {
            let prefix: Table = lua.named_registry_value("nws.prefix")?;
            let before: Table = lua.named_registry_value("nws.before")?;
            let depth = before.raw_len();
            prefix.push(name)?;
            let r = body.call::<()>(());
            prefix.raw_remove(prefix.raw_len())?;
            while before.raw_len() > depth {
                before.raw_remove(before.raw_len())?;
            }
            r
        })?,
    )?;
    lua.globals().set(
        "before_each",
        lua.create_function(|lua, f: Function| {
            let before: Table = lua.named_registry_value("nws.before")?;
            before.push(f)
        })?,
    )?;
    lua.globals().set(
        "it",
        lua.create_function(|lua, (name, body): (String, Function)| {
            let prefix: Table = lua.named_registry_value("nws.prefix")?;
            let mut full: Vec<String> = prefix
                .sequence_values::<String>()
                .collect::<mlua::Result<_>>()?;
            full.push(name);
            let before: Table = lua.named_registry_value("nws.before")?;
            let hooks = lua.create_sequence_from(
                before
                    .sequence_values::<Function>()
                    .collect::<mlua::Result<Vec<_>>>()?,
            )?;
            let t = lua.create_table()?;
            t.set("name", full.join(" > "))?;
            t.set("body", body)?;
            t.set("before", hooks)?;
            let tests: Table = lua.named_registry_value("nws.tests")?;
            tests.push(t)
        })?,
    )?;
    Ok(())
}

fn new_state(host: &Host) -> mlua::Result<Lua> {
    let lua = Lua::new();
    install(&lua, host.clone())?;
    install_collectors(&lua)?;
    Ok(lua)
}

/// Load a test file and run each test it declares, matching `filter` when given.
pub fn run_test_file(
    host: &Host,
    path: &Path,
    filter: Option<&str>,
) -> Result<Vec<TestOutcome>, String> {
    let lua = new_state(host).map_err(|e| e.to_string())?;
    let code = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    lua.load(&code)
        .set_name(format!("@{}", path.display()))
        .exec()
        .map_err(|e| e.to_string())?;
    let tests: Table = lua
        .named_registry_value("nws.tests")
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for t in tests.sequence_values::<Table>() {
        let t = t.map_err(|e| e.to_string())?;
        let name: String = t.get("name").map_err(|e| e.to_string())?;
        if filter.is_some_and(|f| !name.contains(f)) {
            continue;
        }
        let _ = lua.set_named_registry_value("nws.last_trace", "");
        let body: Function = t.get("body").map_err(|e| e.to_string())?;
        let before: Table = t.get("before").map_err(|e| e.to_string())?;
        let r = before
            .sequence_values::<Function>()
            .try_for_each(|f| f?.call::<()>(()))
            .and_then(|()| body.call::<()>(()));
        let trace: String = lua
            .named_registry_value("nws.last_trace")
            .unwrap_or_default();
        out.push(match r {
            Ok(()) => TestOutcome {
                name,
                passed: true,
                message: None,
                trace,
            },
            Err(e) => TestOutcome {
                name,
                passed: false,
                message: Some(clean_error(&e)),
                trace,
            },
        });
    }
    Ok(out)
}

/// Run a Lua file with the `nws` API and `nws.args`.
pub fn run_lua_file(host: &Host, path: &Path, args: &[String]) -> Result<(), String> {
    let lua = new_state(host).map_err(|e| e.to_string())?;
    let nws: Table = lua.globals().get("nws").map_err(|e| e.to_string())?;
    nws.set(
        "args",
        lua.create_sequence_from(args.iter().cloned())
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let code = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    lua.load(&code)
        .set_name(format!("@{}", path.display()))
        .exec()
        .map_err(|e| clean_error(&e))
}

/// A Lua error's message without the Rust callback wrapping.
fn clean_error(e: &mlua::Error) -> String {
    match e {
        mlua::Error::CallbackError { traceback, cause } => {
            format!("{}\n{traceback}", clean_error(cause))
        }
        mlua::Error::RuntimeError(m) => m.clone(),
        other => other.to_string(),
    }
}
