//! Reading typed values out of a GFF struct, with the member's name carried
//! for the error when a field the game writes is missing.

use rakata_gff::{Struct, Value};

use crate::{Error, Result};

/// A struct and where it is, for errors.
#[derive(Clone, Copy)]
pub(crate) struct At<'a> {
    pub member: &'a str,
    pub path: &'a str,
    pub s: &'a Struct,
}

impl<'a> At<'a> {
    pub fn new(member: &'a str, path: &'a str, s: &'a Struct) -> Self {
        At { member, path, s }
    }

    fn shape(&self, label: &str, expected: &'static str) -> Error {
        Error::BadShape {
            member: self.member.to_owned(),
            path: if self.path.is_empty() {
                label.to_owned()
            } else {
                format!("{}.{label}", self.path)
            },
            expected,
        }
    }

    /// The elements of a list field, or an error naming it.
    pub fn list(&self, label: &str) -> Result<&'a [Struct]> {
        self.s
            .get(label)
            .and_then(Value::as_list)
            .ok_or_else(|| self.shape(label, "a list"))
    }

    /// The bytes of a `VOID` field, or an error naming it.
    pub fn void(&self, label: &str) -> Result<&'a [u8]> {
        match self.s.get(label) {
            Some(Value::Void(b)) => Ok(b),
            _ => Err(self.shape(label, "a VOID field")),
        }
    }

    /// The text of a `CExoString` field, or an error naming it.
    pub fn string(&self, label: &str) -> Result<String> {
        string(self.s, label).ok_or_else(|| self.shape(label, "a string"))
    }
}

/// An integer field of any width, widened.
pub(crate) fn int(s: &Struct, label: &str) -> Option<i64> {
    value_int(s.get(label)?)
}

/// An integer value of any width, widened.
pub(crate) fn value_int(v: &Value) -> Option<i64> {
    Some(match v {
        Value::Byte(x) => i64::from(*x),
        Value::Char(x) => i64::from(*x),
        Value::Word(x) => i64::from(*x),
        Value::Short(x) => i64::from(*x),
        Value::Dword(x) => i64::from(*x),
        Value::Int(x) => i64::from(*x),
        Value::Dword64(x) => *x as i64,
        Value::Int64(x) => *x,
        _ => return None,
    })
}

/// An unsigned 32-bit field, such as an object id.
pub(crate) fn u32_field(s: &Struct, label: &str) -> Option<u32> {
    int(s, label).and_then(|v| u32::try_from(v).ok())
}

/// A `CExoString` or `CResRef` field as text, lossily.
pub(crate) fn string(s: &Struct, label: &str) -> Option<String> {
    match s.get(label)? {
        Value::String(b) => Some(String::from_utf8_lossy(b).into_owned()),
        Value::ResRef(r) => Some(String::from_utf8_lossy(r.as_bytes()).into_owned()),
        _ => None,
    }
}

/// A float field.
pub(crate) fn float(s: &Struct, label: &str) -> Option<f32> {
    match s.get(label)? {
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// The first embedded substring of a `CExoLocString` field, lossily.
pub(crate) fn loc_text(s: &Struct, label: &str) -> Option<String> {
    match s.get(label)? {
        Value::LocString(l) => l
            .substrings
            .first()
            .map(|t| String::from_utf8_lossy(&t.text).into_owned()),
        _ => None,
    }
}
