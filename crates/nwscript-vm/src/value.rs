//! Values on the stack and in engine calls.

use std::fmt;
use std::rc::Rc;

/// An NWScript type as engine routines declare it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Void,
    Int,
    Float,
    String,
    Object,
    Vector,
    /// An engine structure: `effect`, `event`, `location`, `talent` in KotOR 1,
    /// numbered by `nwscript.nss`'s `ENGINE_STRUCTURE_n` lines.
    Engine(u8),
    /// A deferred action: not on the stack, but the state the last `STORE_STATE` saved.
    Action,
}

impl Ty {
    /// Bytes the type occupies on the stack.
    pub fn size(self) -> i32 {
        match self {
            Ty::Void | Ty::Action => 0,
            Ty::Vector => 12,
            _ => 4,
        }
    }
}

/// A deferred action: the code to resume at and the stack it resumes with.
#[derive(Clone, Debug, PartialEq)]
pub struct Closure {
    /// The deferred action's number, in creation order.
    pub id: usize,
    /// The file offset the body starts at.
    pub resume: u32,
    /// The global variables below BP when the state was saved.
    pub globals: Vec<Value>,
    /// The top of the stack when the state was saved.
    pub locals: Vec<Value>,
}

/// A value: one stack element, a vector of three, or a deferred action.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Void,
    Int(i32),
    Float(f32),
    /// The string's bytes, Windows-1252.
    String(Rc<[u8]>),
    /// An object id as the bytecode holds it: 0 is `OBJECT_SELF`, 1 is `OBJECT_INVALID`.
    Object(i32),
    /// An engine structure of kind `n` and an engine-assigned handle; 0 is null.
    Engine(u8, u32),
    Vector([f32; 3]),
    Action(Rc<Closure>),
}

pub const OBJECT_SELF: i32 = 0;
pub const OBJECT_INVALID: i32 = 1;

impl Value {
    pub fn default_of(t: &Ty) -> Value {
        match t {
            Ty::Void | Ty::Action => Value::Void,
            Ty::Int => Value::Int(0),
            Ty::Float => Value::Float(0.0),
            Ty::String => Value::String(Rc::from(&b""[..])),
            Ty::Object => Value::Object(OBJECT_INVALID),
            Ty::Vector => Value::Vector([0.0; 3]),
            Ty::Engine(n) => Value::Engine(*n, 0),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Value::Void => "void",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Object(_) => "object",
            Value::Engine(..) => "engine structure",
            Value::Vector(_) => "vector",
            Value::Action(_) => "action",
        }
    }

    pub fn string(s: &str) -> Value {
        Value::String(Rc::from(s.as_bytes()))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Void => f.write_str("void"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x:?}"),
            Value::String(s) => write!(f, "\"{}\"", s.escape_ascii()),
            Value::Object(OBJECT_SELF) => f.write_str("OBJECT_SELF"),
            Value::Object(OBJECT_INVALID) => f.write_str("OBJECT_INVALID"),
            Value::Object(o) => write!(f, "object#{o}"),
            Value::Engine(k, 0) => write!(f, "engine{k}#null"),
            Value::Engine(k, h) => write!(f, "engine{k}#{h}"),
            Value::Vector([x, y, z]) => write!(f, "[{x:?}, {y:?}, {z:?}]"),
            Value::Action(c) => write!(f, "<deferred {}>", c.id),
        }
    }
}
