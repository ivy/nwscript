//! What a GFF contributes to an index: one [`FieldFact`] per field, at any
//! depth, and the [`Ref`]s its fields make to the game's symbols.
//!
//! Every `CResRef` field that is not empty is a reference to a resource;
//! every `CExoLocString` with a strref, and every `StrRef` field, is a
//! reference to the talk table; a `CExoString` under one of the labels in
//! [`TAG_LABELS`] is a reference to a tag. The site of each is the field's
//! [`Path`](crate::Path) as text. The functions are pure: they read the
//! tree and nothing else.

use rakata_res::Ref;

use crate::model::{FieldType, Gff, Struct, Value};

/// The labels whose `CExoString` value is an object's tag: an object's own
/// `Tag`, the destination of a transition (`LinkedTo`), the key a lock
/// asks for (`KeyName`), a dialogue's `Speaker`, `Listener` and animation
/// `Participant`, and the journal category a node updates (`Quest`).
pub const TAG_LABELS: &[&str] = &[
    "Tag",
    "LinkedTo",
    "KeyName",
    "Speaker",
    "Listener",
    "Participant",
    "Quest",
];

/// A field's value as an index stores it.
#[derive(Debug, Clone, PartialEq)]
pub enum FactValue {
    /// No scalar value: a `VOID`, or a `CExoLocString` that embeds no
    /// English text.
    Null,
    /// Every integer type, and a struct field's struct id.
    Int(i64),
    /// `FLOAT` and `DOUBLE`.
    Real(f64),
    /// A `CExoString`, or the English substring a `CExoLocString` embeds,
    /// as the file's code-page bytes.
    Bytes(Vec<u8>),
    /// A resref, a vector or orientation as space-separated numbers.
    Text(String),
}

/// One field of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldFact {
    /// The field's path from the root, as [`crate::Path`] writes it.
    pub path: String,
    /// The field's label.
    pub label: String,
    pub ty: FieldType,
    /// The value; a list's is its length, a struct's its struct id.
    pub value: FactValue,
    /// The strref of a `CExoLocString` that has one, or of a `StrRef`.
    pub strref: Option<u32>,
}

/// The facts of one GFF.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    /// Every field, depth first in file order.
    pub fields: Vec<FieldFact>,
    /// Every reference the fields make, in the same order.
    pub refs: Vec<Ref>,
}

/// Every field of `gff` and the references its fields make.
pub fn facts(gff: &Gff) -> Facts {
    let mut out = Facts::default();
    walk(&gff.root, "", &mut out, true);
    out
}

/// The references alone, for a caller that keeps the fields some other
/// way (a dialogue's nodes).
pub fn refs(gff: &Gff) -> Vec<Ref> {
    let mut out = Facts::default();
    walk(&gff.root, "", &mut out, false);
    out.refs
}

fn walk(s: &Struct, prefix: &str, out: &mut Facts, fields: bool) {
    for f in &s.fields {
        let label = f.label.to_string();
        let path = if prefix.is_empty() {
            label.clone()
        } else {
            format!("{prefix}.{label}")
        };
        let mut strref = None;
        let value = match &f.value {
            Value::Byte(v) => FactValue::Int(i64::from(*v)),
            Value::Char(v) => FactValue::Int(i64::from(*v)),
            Value::Word(v) => FactValue::Int(i64::from(*v)),
            Value::Short(v) => FactValue::Int(i64::from(*v)),
            Value::Dword(v) => FactValue::Int(i64::from(*v)),
            Value::Int(v) => FactValue::Int(i64::from(*v)),
            Value::Dword64(v) => FactValue::Int(*v as i64),
            Value::Int64(v) => FactValue::Int(*v),
            Value::Float(v) => FactValue::Real(f64::from(*v)),
            Value::Double(v) => FactValue::Real(*v),
            Value::String(b) => {
                if !b.is_empty() && TAG_LABELS.iter().any(|t| f.label.is(t)) {
                    out.refs.push(Ref::tag(
                        String::from_utf8_lossy(b).into_owned(),
                        path.clone(),
                    ));
                }
                FactValue::Bytes(b.clone())
            }
            Value::ResRef(r) => {
                let name = String::from_utf8_lossy(r.as_bytes()).into_owned();
                if !name.is_empty() {
                    out.refs.push(Ref::resref(&name, path.clone()));
                }
                FactValue::Text(name)
            }
            Value::LocString(l) => {
                if l.has_strref() {
                    strref = Some(l.strref);
                    out.refs.push(Ref::strref(l.strref, path.clone()));
                }
                match l.get(0) {
                    Some(text) => FactValue::Bytes(text.to_vec()),
                    None => FactValue::Null,
                }
            }
            Value::Void(_) => FactValue::Null,
            Value::Struct(inner) => {
                if fields {
                    out.fields.push(FieldFact {
                        path: path.clone(),
                        label,
                        ty: FieldType::Struct,
                        value: FactValue::Int(i64::from(inner.id)),
                        strref: None,
                    });
                }
                walk(inner, &path, out, fields);
                continue;
            }
            Value::List(items) => {
                if fields {
                    out.fields.push(FieldFact {
                        path: path.clone(),
                        label,
                        ty: FieldType::List,
                        value: FactValue::Int(items.len() as i64),
                        strref: None,
                    });
                }
                for (i, item) in items.iter().enumerate() {
                    walk(item, &format!("{path}[{i}]"), out, fields);
                }
                continue;
            }
            Value::Orientation(v) => FactValue::Text(numbers(v)),
            Value::Vector(v) => FactValue::Text(numbers(v)),
            Value::StrRef(v) => {
                if *v != u32::MAX {
                    strref = Some(*v);
                    out.refs.push(Ref::strref(*v, path.clone()));
                }
                FactValue::Int(i64::from(*v))
            }
        };
        if fields {
            out.fields.push(FieldFact {
                path,
                label,
                ty: f.value.field_type(),
                value,
                strref,
            });
        }
    }
}

fn numbers(v: &[f32]) -> String {
    v.iter().map(f32::to_string).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Label, LocString};
    use rakata_res::{RefKind, ResRef};

    fn label(s: &str) -> Label {
        Label::new(s).unwrap()
    }

    fn sample() -> Gff {
        let mut gff = Gff::new(*b"GIT ");
        let mut creature = Struct::new(4);
        creature
            .push(label("Tag"), Value::string("Juhani"))
            .push(
                label("TemplateResRef"),
                Value::ResRef(ResRef::new("Dan14_Juhani").unwrap()),
            )
            .push(label("XPosition"), Value::Float(1.5));
        let mut name = LocString::strref(42724);
        name.set(0, b"Juhani".to_vec());
        gff.root
            .push(label("Creature List"), Value::List(vec![creature]))
            .push(label("Name"), Value::LocString(name))
            .push(label("Comment"), Value::string("Tag"))
            .push(label("OnEnter"), Value::ResRef(ResRef::new("").unwrap()));
        gff
    }

    #[test]
    fn every_field_has_its_path_and_value() {
        let f = facts(&sample());
        let paths: Vec<&str> = f.fields.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "Creature List",
                "Creature List[0].Tag",
                "Creature List[0].TemplateResRef",
                "Creature List[0].XPosition",
                "Name",
                "Comment",
                "OnEnter"
            ]
        );
        assert_eq!(f.fields[0].value, FactValue::Int(1));
        assert_eq!(f.fields[0].ty, FieldType::List);
        assert_eq!(f.fields[2].value, FactValue::Text("Dan14_Juhani".into()));
        assert_eq!(f.fields[3].value, FactValue::Real(1.5));
        assert_eq!(f.fields[4].strref, Some(42724));
        assert_eq!(f.fields[4].value, FactValue::Bytes(b"Juhani".to_vec()));
    }

    #[test]
    fn references_are_resrefs_strrefs_and_tags_by_label() {
        let r = refs(&sample());
        let got: Vec<(RefKind, &str, &str)> = r
            .iter()
            .map(|x| (x.kind, x.value.as_str(), x.site.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (RefKind::Tag, "Juhani", "Creature List[0].Tag"),
                (
                    RefKind::ResRef,
                    "dan14_juhani",
                    "Creature List[0].TemplateResRef"
                ),
                (RefKind::StrRef, "42724", "Name"),
            ]
        );
        // `refs` alone collects no fields and the same references.
        assert_eq!(facts(&sample()).refs, r);
    }
}
