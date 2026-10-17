//! NWScript types and constant values.

use std::fmt;

/// A type as the language knows it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Int,
    Float,
    String,
    Object,
    Vector,
    Action,
    /// The engine structure numbered `n` by `nwscript.nss`'s `ENGINE_STRUCTURE_n`.
    Engine(u8),
    /// A script-declared `struct NAME`.
    Struct(String),
}

impl Type {
    /// Bytes on the stack; a struct's size depends on its fields and is
    /// not known here.
    pub fn size(&self) -> Option<u32> {
        Some(match self {
            Type::Void | Type::Action => 0,
            Type::Vector => 12,
            Type::Struct(_) => return None,
            _ => 4,
        })
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => f.write_str("void"),
            Type::Int => f.write_str("int"),
            Type::Float => f.write_str("float"),
            Type::String => f.write_str("string"),
            Type::Object => f.write_str("object"),
            Type::Vector => f.write_str("vector"),
            Type::Action => f.write_str("action"),
            Type::Engine(n) => write!(f, "engine structure {n}"),
            Type::Struct(n) => write!(f, "struct {n}"),
        }
    }
}

/// A value known before the program runs: a constant's, or a default's.
#[derive(Clone, Debug, PartialEq)]
pub enum ConstValue {
    Int(i32),
    Float(f32),
    /// Windows-1252 bytes.
    String(Vec<u8>),
    /// 0 is `OBJECT_SELF`, 1 is `OBJECT_INVALID`.
    Object(i32),
    Vector([f32; 3]),
}

impl ConstValue {
    pub fn ty(&self) -> Type {
        match self {
            ConstValue::Int(_) => Type::Int,
            ConstValue::Float(_) => Type::Float,
            ConstValue::String(_) => Type::String,
            ConstValue::Object(_) => Type::Object,
            ConstValue::Vector(_) => Type::Vector,
        }
    }
}

impl fmt::Display for ConstValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstValue::Int(i) => write!(f, "{i}"),
            ConstValue::Float(x) => write!(f, "{x:?}"),
            ConstValue::String(s) => write!(f, "\"{}\"", s.escape_ascii()),
            ConstValue::Object(0) => f.write_str("OBJECT_SELF"),
            ConstValue::Object(1) => f.write_str("OBJECT_INVALID"),
            ConstValue::Object(o) => write!(f, "object {o}"),
            ConstValue::Vector([x, y, z]) => write!(f, "[{x:?}, {y:?}, {z:?}]"),
        }
    }
}
