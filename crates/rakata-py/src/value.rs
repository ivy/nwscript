//! GFF values between Rust and Python. A value read becomes a plain Python
//! value; a value written takes its field type from the field it replaces,
//! or from the type name the caller gives, since Python has one `int` where
//! the format has eight integer types.

use pyo3::IntoPyObjectExt;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};
use rakata::gff::{FieldType, LocString, Struct, Value};
use rakata::tlk::codepage;

use crate::fail;

/// The field type a name gives, as the format documents spell it (`BYTE`,
/// `CResRef`, `CExoLocString`), in any case.
pub fn field_type(name: &str) -> PyResult<FieldType> {
    FieldType::ALL
        .into_iter()
        .find(|t| t.name().eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            let names: Vec<&str> = FieldType::ALL.iter().map(|t| t.name()).collect();
            PyValueError::new_err(format!(
                "{name:?} is not a GFF field type; the types are {}",
                names.join(", ")
            ))
        })
}

fn text(bytes: &[u8]) -> String {
    codepage::decode(bytes)
}

fn encode(s: &str) -> PyResult<Vec<u8>> {
    codepage::encode(s).map_err(|c| {
        PyValueError::new_err(format!(
            "{c:?} has no byte in the game's code page (Windows-1252)"
        ))
    })
}

fn struct_to_py<'py>(py: Python<'py>, s: &Struct) -> PyResult<Bound<'py, PyAny>> {
    let d = PyDict::new(py);
    for f in &s.fields {
        d.set_item(f.label.to_string(), to_py(py, &f.value)?)?;
    }
    Ok(d.into_any())
}

/// A value as Python holds it: an `int` for each integer type and a
/// strref, a `float`, `str` for a `CExoString` (decoded from the game's
/// code page) and a `CResRef`, `bytes` for `VOID`, a tuple of floats for
/// a vector or an orientation, `{"strref": int | None, "substrings": {id:
/// str}}` for a `CExoLocString`, a dict of fields for a struct and a list
/// of them for a list.
pub fn to_py<'py>(py: Python<'py>, v: &Value) -> PyResult<Bound<'py, PyAny>> {
    match v {
        Value::Byte(x) => x.into_bound_py_any(py),
        Value::Char(x) => x.into_bound_py_any(py),
        Value::Word(x) => x.into_bound_py_any(py),
        Value::Short(x) => x.into_bound_py_any(py),
        Value::Dword(x) => x.into_bound_py_any(py),
        Value::Int(x) => x.into_bound_py_any(py),
        Value::Dword64(x) => x.into_bound_py_any(py),
        Value::Int64(x) => x.into_bound_py_any(py),
        Value::Float(x) => x.into_bound_py_any(py),
        Value::Double(x) => x.into_bound_py_any(py),
        Value::StrRef(x) => x.into_bound_py_any(py),
        Value::String(b) => text(b).into_bound_py_any(py),
        Value::ResRef(r) => r.to_string().into_bound_py_any(py),
        Value::Void(b) => Ok(PyBytes::new(py, b).into_any()),
        Value::Orientation(q) => PyTuple::new(py, q)?.into_bound_py_any(py),
        Value::Vector(q) => PyTuple::new(py, q)?.into_bound_py_any(py),
        Value::LocString(l) => {
            let d = PyDict::new(py);
            d.set_item("strref", l.has_strref().then_some(l.strref))?;
            let subs = PyDict::new(py);
            for s in &l.substrings {
                subs.set_item(s.id, text(&s.text))?;
            }
            d.set_item("substrings", subs)?;
            Ok(d.into_any())
        }
        Value::Struct(s) => struct_to_py(py, s),
        Value::List(items) => {
            let l = PyList::empty(py);
            for s in items {
                l.append(struct_to_py(py, s)?)?;
            }
            Ok(l.into_any())
        }
    }
}

fn floats<const N: usize>(obj: &Bound<'_, PyAny>, what: &str) -> PyResult<[f32; N]> {
    let v: Vec<f32> = obj.extract()?;
    v.try_into()
        .map_err(|_| PyValueError::new_err(format!("{what} is {N} floats")))
}

/// A Python value as a GFF value of type `ty`. Structs and lists are not
/// built from Python; change their fields by path instead.
pub fn from_py(obj: &Bound<'_, PyAny>, ty: FieldType) -> PyResult<Value> {
    Ok(match ty {
        FieldType::Byte => Value::Byte(obj.extract()?),
        FieldType::Char => Value::Char(obj.extract()?),
        FieldType::Word => Value::Word(obj.extract()?),
        FieldType::Short => Value::Short(obj.extract()?),
        FieldType::Dword => Value::Dword(obj.extract()?),
        FieldType::Int => Value::Int(obj.extract()?),
        FieldType::Dword64 => Value::Dword64(obj.extract()?),
        FieldType::Int64 => Value::Int64(obj.extract()?),
        FieldType::Float => Value::Float(obj.extract()?),
        FieldType::Double => Value::Double(obj.extract()?),
        FieldType::StrRef => Value::StrRef(obj.extract()?),
        FieldType::String => Value::String(encode(&obj.extract::<String>()?)?),
        FieldType::ResRef => Value::resref(&obj.extract::<String>()?).map_err(fail)?,
        FieldType::Void => Value::Void(obj.extract::<Vec<u8>>()?),
        FieldType::Orientation => Value::Orientation(floats(obj, "an orientation")?),
        FieldType::Vector => Value::Vector(floats(obj, "a vector")?),
        FieldType::LocString => Value::LocString(locstring(obj)?),
        FieldType::Struct | FieldType::List => {
            return Err(PyTypeError::new_err(
                "a Struct or List field is not set whole; set its fields by path",
            ));
        }
    })
}

/// A `CExoLocString` from an `int` strref or the dict [`to_py`] gives.
fn locstring(obj: &Bound<'_, PyAny>) -> PyResult<LocString> {
    if let Ok(strref) = obj.extract::<u32>() {
        return Ok(LocString::strref(strref));
    }
    let d = obj.cast::<PyDict>().map_err(|_| {
        PyTypeError::new_err(
            "a CExoLocString is an int strref or {\"strref\": int | None, \"substrings\": {id: str}}",
        )
    })?;
    let mut l = LocString::none();
    if let Some(s) = d.get_item("strref")?
        && !s.is_none()
    {
        l.strref = s.extract()?;
    }
    if let Some(subs) = d.get_item("substrings")? {
        for (id, t) in subs.cast::<PyDict>()?.iter() {
            l.set(id.extract()?, encode(&t.extract::<String>()?)?);
        }
    }
    Ok(l)
}
