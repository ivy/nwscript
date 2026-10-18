//! `[GFFList]` changes to one file: field edits by path and field additions,
//! in the order the section lists them, with tokens filled and the
//! `2DAMEMORY#` stores of additions made.
//!
//! A path is labels and list indices joined by `\`, as `changes.ini` writes
//! it (`Door List\0\TransitionDestin`); a number is an index when the field
//! before it is a list. Labels compare as written.

use super::{Failure, Tokens};
use crate::changes::{self, AddField, AddStoreWhat, FieldPath, FieldTarget, GffEdit};
use crate::issue::codes;
use crate::value::Value as IniValue;
use rakata_gff::{Gff, Label, LocString, Path, Struct, Value};

fn cannot(msg: String) -> Failure {
    Failure::new(codes::CANNOT_APPLY, msg)
}

/// One change of the section, in file order.
enum Change<'a> {
    Edit(&'a GffEdit),
    Add(&'a AddField),
}

/// Apply the edits and additions in the order of their lines. Every change
/// is tried; the failures are returned, and the file holds what applied.
pub fn apply(
    gff: &mut Gff,
    edits: &[GffEdit],
    adds: &[AddField],
    tokens: &mut Tokens,
) -> Vec<Failure> {
    let mut changes: Vec<(u32, Change)> = edits
        .iter()
        .map(|e| (e.line, Change::Edit(e)))
        .chain(adds.iter().map(|a| (a.line, Change::Add(a))))
        .collect();
    changes.sort_by_key(|(line, _)| *line);
    let mut failures = Vec::new();
    for (line, change) in changes {
        let r = match change {
            Change::Edit(e) => edit(gff, e, tokens),
            Change::Add(a) => {
                let parent = a.path.clone().unwrap_or_default();
                add(gff, a, &parent, tokens)
            }
        };
        if let Err(mut f) = r {
            f.message = format!("line {line}: {}", f.message);
            failures.push(f);
        }
    }
    failures
}

/// The path `text` names in this file, labels and indices told apart by
/// the type of the field each number follows.
pub fn path(gff: &Gff, text: &str) -> Result<Path, Failure> {
    let mut p = Path::root();
    let mut in_list = false;
    for seg in text.split('\\').filter(|s| !s.is_empty()) {
        if in_list {
            let n: usize = seg.parse().map_err(|_| {
                cannot(format!(
                    "`{text}`: `{seg}` follows a list and is not an index"
                ))
            })?;
            p = p.index(n);
            in_list = false;
            gff.struct_at(&p)
                .map_err(|e| cannot(format!("`{text}`: {e}")))?;
            continue;
        }
        let label = Label::new(seg)
            .map_err(|_| cannot(format!("`{text}`: `{seg}` is longer than sixteen bytes")))?;
        p = p.field(label);
        match gff.get(&p) {
            Ok(Value::List(_)) => in_list = true,
            Ok(_) => {}
            Err(e) => return Err(cannot(format!("`{text}`: {e}"))),
        }
    }
    Ok(p)
}

fn edit(gff: &mut Gff, e: &GffEdit, tokens: &Tokens) -> Result<(), Failure> {
    let text_path = match &e.path {
        FieldPath::Literal(p) => p.clone(),
        FieldPath::Memory(slot) => tokens.resolve(&IniValue::Memory(*slot))?,
    };
    let p = path(gff, &text_path)?;
    let value = tokens.resolve(&e.value)?;
    let old = gff
        .get(&p)
        .map_err(|err| cannot(format!("`{text_path}`: {err}")))?
        .clone();
    let new = match (e.target.clone(), old) {
        (FieldTarget::Value, Value::LocString(mut l)) => {
            l.strref = strref(&value)?;
            Value::LocString(l)
        }
        (FieldTarget::Value, old) => {
            parse_like(&old, &value).map_err(|m| cannot(format!("`{text_path}`: {m}")))?
        }
        (FieldTarget::StrRef, Value::LocString(mut l)) => {
            l.strref = strref(&value)?;
            Value::LocString(l)
        }
        (FieldTarget::Substring(id), Value::LocString(mut l)) => {
            l.set(id, encode(&value)?);
            Value::LocString(l)
        }
        (_, old) => {
            return Err(cannot(format!(
                "`{text_path}` is a {} field, which has no strref or substrings",
                old.field_type()
            )));
        }
    };
    gff.set(&p, new)
        .map_err(|err| cannot(format!("`{text_path}`: {err}")))?;
    Ok(())
}

/// What the parent of an addition is.
enum Parent {
    Struct,
    List(usize),
}

fn parent_kind(gff: &Gff, p: &Path, shown: &str) -> Result<Parent, Failure> {
    if p.is_root() {
        return Ok(Parent::Struct);
    }
    if let Ok(Value::List(l)) = gff.get(p) {
        return Ok(Parent::List(l.len()));
    }
    gff.struct_at(p).map(|_| Parent::Struct).map_err(|e| {
        cannot(format!(
            "`{shown}` is not a struct or a list to add to: {e}"
        ))
    })
}

fn add(gff: &mut Gff, a: &AddField, parent_text: &str, tokens: &mut Tokens) -> Result<(), Failure> {
    let parent = path(gff, parent_text)?;
    let shown_parent = if parent_text.is_empty() {
        "the root"
    } else {
        parent_text
    };
    let new_value = new_value(a, tokens)?;
    let (added_text, list_index) = match parent_kind(gff, &parent, shown_parent)? {
        Parent::List(len) => {
            let Value::Struct(s) = new_value else {
                return Err(cannot(format!(
                    "[{}] adds a {:?} to the list `{shown_parent}`, which holds only structs",
                    a.section, a.field_type
                )));
            };
            match gff.get_mut(&parent) {
                Ok(Value::List(l)) => l.push(s),
                _ => unreachable!("parent_kind found a list"),
            }
            (join(parent_text, &len.to_string()), Some(len))
        }
        Parent::Struct => {
            if a.label.is_empty() {
                return Err(cannot(format!(
                    "[{}] adds a field to the struct `{shown_parent}` without a Label",
                    a.section
                )));
            }
            let label = Label::new(&a.label).map_err(|_| {
                cannot(format!(
                    "[{}]: the label `{}` is longer than sixteen bytes",
                    a.section, a.label
                ))
            })?;
            let s: &mut Struct = gff
                .struct_at_mut(&parent)
                .map_err(|e| cannot(format!("`{shown_parent}`: {e}")))?;
            match s.get_mut(&a.label) {
                // A struct or list already there is the one the children go
                // into; a scalar takes the new value.
                Some(existing)
                    if existing.field_type() == new_value.field_type()
                        && matches!(existing, Value::Struct(_) | Value::List(_)) => {}
                Some(existing) => *existing = new_value,
                None => {
                    s.push(label, new_value);
                }
            }
            if !gff.labels.contains(&label) {
                gff.labels.push(label);
            }
            (join(parent_text, &a.label), None)
        }
    };
    for st in &a.stores {
        let v = match st.what {
            AddStoreWhat::ListIndex => match list_index {
                Some(i) => i.to_string(),
                None => {
                    return Err(cannot(format!(
                        "[{}] stores ListIndex, but the field was not added to a list",
                        a.section
                    )));
                }
            },
            AddStoreWhat::FieldPath => added_text.clone(),
        };
        tokens.memory.insert(st.slot, v);
    }
    for child in &a.children {
        add(gff, child, &added_text, tokens)?;
    }
    Ok(())
}

fn join(parent: &str, seg: &str) -> String {
    if parent.is_empty() {
        seg.to_string()
    } else {
        format!("{parent}\\{seg}")
    }
}

fn new_value(a: &AddField, tokens: &Tokens) -> Result<Value, Failure> {
    use changes::FieldType as F;
    let text = match &a.value {
        Some(v) => tokens.resolve(v)?,
        None => String::new(),
    };
    let bad = |m: String| cannot(format!("[{}]: {m}", a.section));
    fn num(t: &str) -> &str {
        if t.trim().is_empty() { "0" } else { t }
    }
    Ok(match a.field_type {
        F::Struct => Value::Struct(Struct::new(a.type_id.unwrap_or(0))),
        F::List => Value::List(Vec::new()),
        F::ExoLocString => {
            let mut l = LocString::none();
            if let Some(s) = &a.strref {
                l.strref = strref(&tokens.resolve(s)?)?;
            }
            for (id, t) in &a.substrings {
                l.set(*id, encode(t)?);
            }
            Value::LocString(l)
        }
        F::Byte => Value::Byte(int(num(&text)).map_err(bad)?),
        F::Char => Value::Char(int(num(&text)).map_err(bad)?),
        F::Word => Value::Word(int(num(&text)).map_err(bad)?),
        F::Short => Value::Short(int(num(&text)).map_err(bad)?),
        F::Dword => Value::Dword(int(num(&text)).map_err(bad)?),
        F::Int => Value::Int(int(num(&text)).map_err(bad)?),
        F::Dword64 => Value::Dword64(int(num(&text)).map_err(bad)?),
        F::Int64 => Value::Int64(int(num(&text)).map_err(bad)?),
        F::Float => Value::Float(float(num(&text)).map_err(bad)? as f32),
        F::Double => Value::Double(float(num(&text)).map_err(bad)?),
        F::ExoString => Value::String(encode(&text)?),
        F::ResRef => Value::resref(&text).map_err(|e| bad(e.to_string()))?,
        F::Void => Value::Void(hex(&text).map_err(bad)?),
        F::Orientation => Value::Orientation(floats::<4>(&text).map_err(bad)?),
        F::Position => Value::Vector(floats::<3>(&text).map_err(bad)?),
    })
}

/// `text` as a value of `old`'s type.
fn parse_like(old: &Value, text: &str) -> Result<Value, String> {
    Ok(match old {
        Value::Byte(_) => Value::Byte(int(text)?),
        Value::Char(_) => Value::Char(int(text)?),
        Value::Word(_) => Value::Word(int(text)?),
        Value::Short(_) => Value::Short(int(text)?),
        Value::Dword(_) => Value::Dword(int(text)?),
        Value::Int(_) => Value::Int(int(text)?),
        Value::Dword64(_) => Value::Dword64(int(text)?),
        Value::Int64(_) => Value::Int64(int(text)?),
        Value::Float(_) => Value::Float(float(text)? as f32),
        Value::Double(_) => Value::Double(float(text)?),
        Value::String(_) => Value::String(
            rakata_tlk::codepage::encode(text)
                .map_err(|c| format!("`{c}` is not in the game's code page"))?,
        ),
        Value::ResRef(_) => Value::resref(text).map_err(|e| e.to_string())?,
        Value::Void(_) => Value::Void(hex(text)?),
        Value::Orientation(_) => Value::Orientation(floats::<4>(text)?),
        Value::Vector(_) => Value::Vector(floats::<3>(text)?),
        Value::StrRef(_) => Value::StrRef(strref(text).map_err(|f| f.message)?),
        Value::LocString(_) | Value::Struct(_) | Value::List(_) => {
            return Err(format!("a {} field takes no value", old.field_type()));
        }
    })
}

/// An integer of the field's width. A negative number in an unsigned field
/// is its two's complement (`-1` in a DWORD is `0xFFFFFFFF`); a number
/// written with a fraction must be whole.
fn int<T: TryFrom<i128>>(text: &str) -> Result<T, String> {
    let t = text.trim();
    let n: i128 = match t.parse::<i128>() {
        Ok(n) => n,
        Err(_) => {
            let f = float(t)?;
            if f.fract() != 0.0 {
                return Err(format!("`{t}` is not a whole number"));
            }
            f as i128
        }
    };
    let width = std::mem::size_of::<T>() as u32 * 8;
    let n = if n < 0 && T::try_from(n).is_err() && n >= -(1i128 << (width - 1)) {
        n + (1i128 << width)
    } else {
        n
    };
    T::try_from(n).map_err(|_| format!("`{t}` does not fit a {width}-bit field"))
}

/// A decimal number; a comma is taken as the decimal separator, as the
/// German or French Windows some authors used writes it.
fn float(text: &str) -> Result<f64, String> {
    let t = text.trim().replace(',', ".");
    t.parse::<f64>()
        .map_err(|_| format!("`{}` is not a number", text.trim()))
}

fn floats<const N: usize>(text: &str) -> Result<[f32; N], String> {
    let parts: Vec<&str> = text.split('|').collect();
    if parts.len() != N {
        return Err(format!("`{text}` is not {N} numbers joined by `|`"));
    }
    let mut out = [0f32; N];
    for (o, p) in out.iter_mut().zip(parts) {
        *o = float(p)? as f32;
    }
    Ok(out)
}

fn hex(text: &str) -> Result<Vec<u8>, String> {
    let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if !t.len().is_multiple_of(2) {
        return Err(format!("`{text}` is not hex bytes"));
    }
    (0..t.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&t[i..i + 2], 16).map_err(|_| format!("`{text}` is not hex bytes"))
        })
        .collect()
}

/// A strref as written: a number, `-1` for none.
fn strref(text: &str) -> Result<u32, Failure> {
    int::<u32>(text).map_err(|m| cannot(format!("a strref: {m}")))
}

fn encode(text: &str) -> Result<Vec<u8>, Failure> {
    rakata_tlk::codepage::encode(text)
        .map_err(|c| cannot(format!("`{c}` is not in the game's code page")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_wrap_negative_into_unsigned_fields() {
        assert_eq!(int::<u32>("-1").unwrap(), u32::MAX);
        assert_eq!(int::<u8>("255").unwrap(), 255);
        assert!(int::<u8>("256").is_err());
        assert_eq!(int::<i16>("-3").unwrap(), -3);
        assert_eq!(int::<u16>("7.0").unwrap(), 7);
        assert!(int::<u16>("7.5").is_err());
    }

    #[test]
    fn a_decimal_comma_is_a_decimal_point() {
        assert_eq!(floats::<3>("92,5|62,25|-1").unwrap(), [92.5, 62.25, -1.0]);
        assert_eq!(float("1.5").unwrap(), 1.5);
    }
}
