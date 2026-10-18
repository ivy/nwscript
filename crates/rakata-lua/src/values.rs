//! Conversions shared by the tables: byte strings, resource names, option
//! tables, and the façade's records as Lua tables.

use mlua::{FromLua, Lua, Table, Value};
use rakata::dialog::NodeKey;
use rakata::game::{Install, Location, Profile};
use rakata::res::ResId;
use rakata_dialog::Finding;

use crate::error::{Lift, bad};

/// Bytes as a Lua string, which holds any bytes.
pub fn bytes(lua: &Lua, b: &[u8]) -> mlua::Result<mlua::LuaString> {
    lua.create_string(b)
}

/// A resource name, `NAME.ext`.
pub fn res_id(name: &str) -> mlua::Result<ResId> {
    ResId::parse(name).lift()
}

/// `k1` or `k2`.
pub fn profile(name: &str) -> mlua::Result<Profile> {
    match name.to_ascii_lowercase().as_str() {
        "k1" => Ok(Profile::K1),
        "k2" => Ok(Profile::K2),
        _ => Err(bad(format!("`{name}` is not a game profile: k1 or k2"))),
    }
}

pub fn profile_name(p: Profile) -> &'static str {
    match p {
        Profile::K1 => "k1",
        Profile::K2 => "k2",
    }
}

/// An options table: every key must be one the function takes.
pub struct Opts {
    table: Option<Table>,
}

impl Opts {
    /// `value`, which is nil or a table whose keys are among `allowed`;
    /// `what` names the function in the message.
    pub fn new(value: Option<Table>, what: &str, allowed: &[&str]) -> mlua::Result<Opts> {
        if let Some(t) = &value {
            for pair in t.pairs::<Value, Value>() {
                let (k, _) = pair?;
                let ok = match &k {
                    Value::String(s) => allowed.iter().any(|a| s.as_bytes()[..] == *a.as_bytes()),
                    _ => false,
                };
                if !ok {
                    let shown = match &k {
                        Value::String(s) => s.to_string_lossy(),
                        other => other.type_name().to_string(),
                    };
                    return Err(bad(format!(
                        "{what} takes no option `{shown}`; its options are {}",
                        allowed.join(", ")
                    )));
                }
            }
        }
        Ok(Opts { table: value })
    }

    pub fn get<T: FromLua>(&self, key: &str) -> mlua::Result<Option<T>> {
        match &self.table {
            None => Ok(None),
            Some(t) => t.get::<Option<T>>(key),
        }
    }
}

/// Where a copy lies, relative to the install when it lies inside it:
/// `Override/x.2da`, `modules/danm14ac_s.rim:k_pdan_juhani31.ncs`,
/// `data/2da.bif[12]`.
pub fn location(install: &Install, l: &Location) -> String {
    let rel = |p: &std::path::Path| {
        p.strip_prefix(install.root())
            .unwrap_or(p)
            .display()
            .to_string()
    };
    match l {
        Location::File { path } => rel(path),
        Location::Capsule { path, member } => format!("{}:{member}", rel(path)),
        Location::Bif { path, index, .. } => format!("{}[{index}]", rel(path)),
    }
}

/// A dialogue finding: `{code, place, key, subject, via, message}`.
pub fn finding(lua: &Lua, dialog: &str, f: &Finding) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("code", f.code)?;
    t.set("place", f.place.to_string())?;
    t.set("key", key_text(&f.key))?;
    t.set("subject", f.subject.as_str())?;
    t.set("via", f.via.clone())?;
    t.set("message", f.message(dialog))?;
    Ok(t)
}

/// A key as `NodeKey.parse` reads it back: `voice:R`, `strref:N`,
/// `text:T`, `E3`.
pub fn key_text(k: &NodeKey) -> String {
    match k {
        NodeKey::Text(t) => format!("text:{t}"),
        NodeKey::Index(kind, index) => rakata_dialog::NodeRef {
            kind: *kind,
            index: *index,
        }
        .to_string(),
        other => other.to_string(),
    }
}

/// A JSON value as Lua: objects as tables, arrays as sequences, null as nil.
pub fn json(lua: &Lua, v: &serde_json::Value) -> mlua::Result<Value> {
    use serde_json::Value as J;
    Ok(match v {
        J::Null => Value::Nil,
        J::Bool(b) => Value::Boolean(*b),
        J::Number(n) => match n.as_i64() {
            Some(i) => Value::Integer(i),
            None => Value::Number(n.as_f64().unwrap_or(f64::NAN)),
        },
        J::String(s) => Value::String(lua.create_string(s)?),
        J::Array(items) => {
            let t = lua.create_table()?;
            for (i, item) in items.iter().enumerate() {
                t.raw_set(i + 1, json(lua, item)?)?;
            }
            Value::Table(t)
        }
        J::Object(map) => {
            let t = lua.create_table()?;
            for (k, item) in map {
                t.raw_set(k.as_str(), json(lua, item)?)?;
            }
            Value::Table(t)
        }
    })
}
