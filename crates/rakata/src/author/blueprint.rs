//! Blueprints edited by field path: a creature, item or placeable cloned
//! from one the game ships, with the fields that make it the mod's changed.
//!
//! A field keeps its type: setting `Str`, a `BYTE`, to a `WORD` is refused
//! rather than written, since the engine reads each field by the type its
//! format document gives. A field the struct lacks is added with the value's
//! type, as BioWare's tool adds an optional field.

use rakata_gff::{Gff, Label, Path, Segment, Value};

use super::{Error, Result};

/// The value at `path` (`Str`, `ItemList[0].Dropable`).
pub fn get(gff: &Gff, path: &str) -> Result<Value> {
    Ok(gff.get(&Path::parse(path)?)?.clone())
}

/// Set the field at `path` to `value`, and return the value it held, if
/// any. The field is added when its struct exists and lacks it. Fails when
/// the path does not parse or names no struct (`RE0606`, `RE0607`), and
/// when the field holds another type (`RE1406`).
pub fn set(gff: &mut Gff, path: &str, value: Value) -> Result<Option<Value>> {
    let parsed = Path::parse(path)?;
    let (Some(parent), Some(Segment::Field(label))) =
        (parsed.parent(), parsed.segments().last().cloned())
    else {
        return Err(rakata_gff::Error::NoSuchPath {
            at: path.to_string(),
            reason: "names a list element or the root, not a field".to_string(),
        }
        .into());
    };
    let s = gff.struct_at_mut(&parent)?;
    set_in(s, label, value, path)
}

fn set_in(
    s: &mut rakata_gff::Struct,
    label: Label,
    value: Value,
    path: &str,
) -> Result<Option<Value>> {
    if let Some(old) = s.get(&label.to_string())
        && old.field_type() != value.field_type()
    {
        return Err(Error::FieldType {
            path: path.to_string(),
            holds: old.field_type().name(),
            given: value.field_type().name(),
        });
    }
    Ok(s.set(label, value))
}

/// Set `label` to `value` in every element of the list at `list`
/// (`ItemList`), adding it where an element lacks it, and return how many
/// elements there are. A path naming no list leaves the blueprint as it
/// was and returns 0: a creature with no inventory has no `ItemList`.
/// Fails when an element holds the field with another type (`RE1406`),
/// with no element changed.
pub fn set_each(gff: &mut Gff, list: &str, label: &str, value: Value) -> Result<usize> {
    let parsed = Path::parse(list)?;
    let label = Label::new(label)?;
    let Ok(Value::List(items)) = gff.get_mut(&parsed) else {
        return Ok(0);
    };
    for (i, item) in items.iter().enumerate() {
        if let Some(old) = item.get(&label.to_string())
            && old.field_type() != value.field_type()
        {
            return Err(Error::FieldType {
                path: format!("{list}[{i}].{label}"),
                holds: old.field_type().name(),
                given: value.field_type().name(),
            });
        }
    }
    for item in items.iter_mut() {
        item.set(label, value.clone());
    }
    Ok(items.len())
}

/// A copy of `source` as the blueprint `template`: its `TemplateResRef`
/// names the copy, which is the resref the game spawns it by. Fails when
/// the name is longer than a resref (`RE0608`) or the field holds another
/// type (`RE1406`).
pub fn clone(source: &Gff, template: &str) -> Result<Gff> {
    let mut gff = source.clone();
    set(&mut gff, "TemplateResRef", Value::resref(template)?)?;
    Ok(gff)
}
