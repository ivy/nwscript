//! Errors as a script sees them: a façade error carries its registered
//! code across the Rust boundary, and `pcall` and `xpcall` hand the script
//! a `rakata.Error` table, `{code = "RE1404", message = "..."}`, that
//! `error` raises again with its code intact.

use std::fmt;

use mlua::{Lua, Table, Value};

/// The binding's own codes: the `RE18xx` block of the `rakata` registry.
pub mod codes {
    /// A path a script reads or writes that is not one of its inputs.
    pub const NOT_AN_INPUT: &str = "RE1801";
    /// A module a script requires that is not in the script's directory.
    pub const NO_SUCH_MODULE: &str = "RE1802";
    /// The install asked for without `--game`.
    pub const NO_GAME: &str = "RE1803";
    /// A script that raised an error without a code, or did not load.
    pub const SCRIPT_FAILED: &str = "RE1804";
    /// A value of the wrong kind passed to a binding function.
    pub const BAD_ARGUMENT: &str = "RE1805";
    /// A file a script writes that could not be written.
    pub const WRITE_FAILED: &str = "RE1806";
    /// A file a script reads that could not be read: `rakata`'s own code.
    pub const READ_FAILED: &str = "RE0001";
}

/// An error with a registered code, as it travels through Lua.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coded {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for Coded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Coded {}

/// A Lua error carrying `code`.
pub fn raise(code: &'static str, message: impl Into<String>) -> mlua::Error {
    mlua::Error::external(Coded {
        code,
        message: message.into(),
    })
}

/// A bad argument (`RE1805`).
pub fn bad(message: impl Into<String>) -> mlua::Error {
    raise(codes::BAD_ARGUMENT, message)
}

/// The coded error inside `e`, through the callback and context wrappers.
pub fn find(e: &mlua::Error) -> Option<&Coded> {
    match e {
        mlua::Error::ExternalError(inner) => inner.downcast_ref::<Coded>(),
        mlua::Error::CallbackError { cause, .. } | mlua::Error::WithContext { cause, .. } => {
            find(cause)
        }
        _ => None,
    }
}

/// A façade result as a Lua result, the error's code kept.
pub trait Lift<T> {
    fn lift(self) -> mlua::Result<T>;
}

impl<T> Lift<T> for Result<T, rakata::author::Error> {
    fn lift(self) -> mlua::Result<T> {
        self.map_err(|e| raise(e.code(), e.message()))
    }
}

impl<T> Lift<T> for Result<T, rakata::dialog::Error> {
    fn lift(self) -> mlua::Result<T> {
        self.map_err(|e| raise(e.code(), e.message()))
    }
}

/// The format crates' errors, through the authoring API's, which carries
/// each one's code and message unchanged.
macro_rules! lift_via_author {
    ($($ty:ty),* $(,)?) => {
        $(impl<T> Lift<T> for Result<T, $ty> {
            fn lift(self) -> mlua::Result<T> {
                self.map_err(rakata::author::Error::from).lift()
            }
        })*
    };
}

lift_via_author!(
    rakata::res::Error,
    rakata::game::Error,
    rakata_dialog::Error,
    rakata::gff::Error,
    rakata::twoda::Error,
    rakata::lip::Error,
    rakata::tlk::Error,
);

const ERROR_META: &str = "rakata.Error";

/// The `rakata.Error` table for a coded error.
fn error_table(lua: &Lua, c: &Coded) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("code", c.code)?;
    t.set("message", c.message.as_str())?;
    let meta: Table = lua.named_registry_value(ERROR_META)?;
    t.set_metatable(Some(meta))?;
    Ok(t)
}

/// The coded error a `rakata.Error` table holds.
fn coded_of(lua: &Lua, v: &Value) -> mlua::Result<Option<Coded>> {
    let Value::Table(t) = v else {
        return Ok(None);
    };
    let meta: Table = lua.named_registry_value(ERROR_META)?;
    if t.metatable().as_ref() != Some(&meta) {
        return Ok(None);
    }
    let code: String = t.get("code")?;
    let message: String = t.get("message")?;
    let code = rakata::codes::lookup(&code)
        .map(|c| c.id)
        .unwrap_or(codes::SCRIPT_FAILED);
    Ok(Some(Coded { code, message }))
}

/// `rakata.Error`, and `pcall`, `xpcall` and `error` that keep codes: an
/// error raised by the binding reaches a handler as a `rakata.Error`, and
/// `error(e)` on one raises it again with its code.
pub fn install(lua: &Lua, rakata: &Table) -> mlua::Result<()> {
    let meta = lua.create_table()?;
    meta.set(
        "__tostring",
        lua.create_function(|_, t: Table| {
            let code: String = t.get("code")?;
            let message: String = t.get("message")?;
            Ok(format!("{code}: {message}"))
        })?,
    )?;
    meta.set("__name", ERROR_META)?;
    lua.set_named_registry_value(ERROR_META, meta.clone())?;
    let class = lua.create_table()?;
    class.set(
        "is",
        lua.create_function(|lua, v: Value| Ok(coded_of(lua, &v)?.is_some()))?,
    )?;
    rakata.set("Error", class)?;

    let lift = lua.create_function(|lua, v: Value| match &v {
        Value::Error(e) => match find(e) {
            Some(c) => Ok(Value::Table(error_table(lua, c)?)),
            None => Ok(v),
        },
        _ => Ok(v),
    })?;
    let reraise = lua.create_function(|lua, v: Value| match coded_of(lua, &v)? {
        Some(c) => Err::<(), _>(mlua::Error::external(c)),
        None => Ok(()),
    })?;
    lua.load(PRELUDE)
        .set_name("=rakata")
        .call::<()>((lift, reraise))?;
    Ok(())
}

const PRELUDE: &str = r#"
local lift, reraise = ...
local raw_pcall, raw_xpcall, raw_error = pcall, xpcall, error
local pack, unpack = table.pack, table.unpack
local function lifted(r)
  if not r[1] then r[2] = lift(r[2]) end
  return unpack(r, 1, r.n)
end
pcall = function(f, ...)
  return lifted(pack(raw_pcall(f, ...)))
end
xpcall = function(f, handler, ...)
  return raw_xpcall(f, function(e) return handler(lift(e)) end, ...)
end
error = function(e, level)
  reraise(e)
  if level == 0 then return raw_error(e, 0) end
  return raw_error(e, (level or 1) + 1)
end
"#;
