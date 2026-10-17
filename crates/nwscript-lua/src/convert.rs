//! Values between the machine and Lua.

use crate::world::{LEngine, LObject};
use mlua::{Lua, Value as LuaValue};
use nwscript_vm::{OBJECT_SELF, Ty, Value, World};
use std::cell::RefCell;
use std::rc::Rc;

/// A machine value as Lua sees it: integers, numbers, strings, objects and
/// engine structures as userdata, vectors as `{x, y, z}`, deferred actions
/// as `"<deferred n>"`. `OBJECT_SELF` is shown as the object it stood for.
pub fn to_lua(lua: &Lua, w: &Rc<RefCell<World>>, v: &Value, caller: i32) -> mlua::Result<LuaValue> {
    Ok(match v {
        Value::Void => LuaValue::Nil,
        Value::Int(i) => LuaValue::Integer(i64::from(*i)),
        Value::Float(f) => LuaValue::Number(f64::from(*f)),
        Value::String(s) => LuaValue::String(lua.create_string(&s[..])?),
        Value::Object(o) => {
            let id = if *o == OBJECT_SELF { caller } else { *o };
            LuaValue::UserData(lua.create_userdata(LObject { w: w.clone(), id })?)
        }
        Value::Engine(k, h) => LuaValue::UserData(lua.create_userdata(LEngine {
            kind: *k,
            handle: *h,
        })?),
        Value::Vector([x, y, z]) => {
            let t = lua.create_table()?;
            t.set("x", *x)?;
            t.set("y", *y)?;
            t.set("z", *z)?;
            LuaValue::Table(t)
        }
        Value::Action(c) => LuaValue::String(lua.create_string(format!("<deferred {}>", c.id))?),
    })
}

/// A Lua value as the machine value of type `ty`, for an overridden
/// routine's return.
pub fn from_lua(v: &LuaValue, ty: &Ty) -> Result<Value, String> {
    let bad = |want: &str| Err(format!("expected {want}, got {}", v.type_name()));
    Ok(match ty {
        Ty::Void | Ty::Action => Value::Void,
        Ty::Int => match v {
            LuaValue::Integer(i) => Value::Int(*i as i32),
            LuaValue::Number(n) => Value::Int(*n as i32),
            LuaValue::Boolean(b) => Value::Int(i32::from(*b)),
            LuaValue::Nil => Value::Int(0),
            _ => return bad("an integer or boolean"),
        },
        Ty::Float => match v {
            LuaValue::Integer(i) => Value::Float(*i as f32),
            LuaValue::Number(n) => Value::Float(*n as f32),
            LuaValue::Nil => Value::Float(0.0),
            _ => return bad("a number"),
        },
        Ty::String => match v {
            LuaValue::String(s) => Value::String(Rc::from(&s.as_bytes()[..])),
            LuaValue::Nil => Value::string(""),
            _ => return bad("a string"),
        },
        Ty::Object => match v {
            LuaValue::UserData(u) => match u.borrow::<LObject>() {
                Ok(o) => Value::Object(o.id),
                Err(_) => return bad("an object"),
            },
            LuaValue::Integer(i) => Value::Object(*i as i32),
            LuaValue::Nil => Value::Object(nwscript_vm::OBJECT_INVALID),
            _ => return bad("an object"),
        },
        Ty::Engine(k) => match v {
            LuaValue::UserData(u) => match u.borrow::<LEngine>() {
                Ok(e) => Value::Engine(e.kind, e.handle),
                Err(_) => return bad("an engine structure"),
            },
            LuaValue::Nil => Value::Engine(*k, 0),
            _ => return bad("an engine structure"),
        },
        Ty::Vector => match v {
            LuaValue::Table(t) => {
                let c = |k: &str, i: i64| -> f32 {
                    t.get::<f64>(k).or_else(|_| t.get::<f64>(i)).unwrap_or(0.0) as f32
                };
                Value::Vector([c("x", 1), c("y", 2), c("z", 3)])
            }
            LuaValue::Nil => Value::Vector([0.0; 3]),
            _ => return bad("a vector table {x, y, z}"),
        },
    })
}

/// Whether a recorded argument matches a test's expectation: `nws.any`
/// matches anything; numbers compare by value; objects by id.
pub fn matches(lua: &Lua, actual: &Value, want: &LuaValue, caller: i32) -> bool {
    if let LuaValue::Table(t) = want
        && let Ok(any) = crate::any_sentinel(lua)
        && t.equals(&any).unwrap_or(false)
    {
        return true;
    }
    match (actual, want) {
        (Value::Int(a), LuaValue::Integer(b)) => i64::from(*a) == *b,
        (Value::Int(a), LuaValue::Boolean(b)) => (*a != 0) == *b,
        (Value::Int(a), LuaValue::Number(b)) => f64::from(*a) == *b,
        (Value::Float(a), LuaValue::Number(b)) => f64::from(*a) == *b,
        (Value::Float(a), LuaValue::Integer(b)) => f64::from(*a) == *b as f64,
        (Value::String(a), LuaValue::String(b)) => a[..] == b.as_bytes()[..],
        (Value::Object(a), LuaValue::UserData(u)) => u
            .borrow::<LObject>()
            .is_ok_and(|o| o.id == if *a == OBJECT_SELF { caller } else { *a }),
        (Value::Object(a), LuaValue::Integer(b)) => i64::from(*a) == *b,
        (Value::Engine(k, h), LuaValue::UserData(u)) => u
            .borrow::<LEngine>()
            .is_ok_and(|e| e.kind == *k && e.handle == *h),
        (Value::Vector(v), LuaValue::Table(t)) => {
            let c = |k: &str| t.get::<f64>(k).ok();
            [c("x"), c("y"), c("z")]
                == [
                    Some(f64::from(v[0])),
                    Some(f64::from(v[1])),
                    Some(f64::from(v[2])),
                ]
        }
        _ => false,
    }
}
