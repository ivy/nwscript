//! The differences between two GFF trees, as paths.

use crate::model::{Gff, Struct, Value};
use crate::path::Path;

/// One difference: the path, and the value on each side. `before` is `None`
/// for a field or list element only the second file has, `after` for one
/// only the first has. A path that names a struct (the root, a list
/// element, a `Struct` field) carries the struct ids as `Value::Dword`s
/// when they differ; a list element one side lacks carries it whole as
/// `Value::Struct`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub path: Path,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// Every difference between the two trees, in the first file's order.
/// Fields match by label within a struct, in any order; list elements match
/// by position. The label table and the layout are not compared, nor are
/// the file type and version ([`Gff::semantic_eq`] covers those).
pub fn diff(a: &Gff, b: &Gff) -> Vec<Change> {
    let mut out = Vec::new();
    diff_struct(&a.root, &b.root, &Path::root(), &mut out);
    out
}

fn diff_struct(a: &Struct, b: &Struct, path: &Path, out: &mut Vec<Change>) {
    if a.id != b.id {
        out.push(Change {
            path: path.clone(),
            before: Some(Value::Dword(a.id)),
            after: Some(Value::Dword(b.id)),
        });
    }
    // A struct may repeat a label (the format forbids it; files do not, but
    // a tool might), so the n-th field of a label matches the n-th.
    let mut used = vec![false; b.fields.len()];
    for fa in &a.fields {
        let here = path.field(fa.label);
        let partner = b
            .fields
            .iter()
            .enumerate()
            .find(|(i, fb)| !used[*i] && fb.label == fa.label);
        match partner {
            None => out.push(Change {
                path: here,
                before: Some(fa.value.clone()),
                after: None,
            }),
            Some((i, fb)) => {
                used[i] = true;
                diff_value(&fa.value, &fb.value, &here, out);
            }
        }
    }
    for (i, fb) in b.fields.iter().enumerate() {
        if !used[i] {
            out.push(Change {
                path: path.field(fb.label),
                before: None,
                after: Some(fb.value.clone()),
            });
        }
    }
}

fn diff_value(a: &Value, b: &Value, path: &Path, out: &mut Vec<Change>) {
    match (a, b) {
        (Value::Struct(sa), Value::Struct(sb)) => diff_struct(sa, sb, path, out),
        (Value::List(la), Value::List(lb)) => {
            for (i, (sa, sb)) in la.iter().zip(lb).enumerate() {
                diff_struct(sa, sb, &path.index(i), out);
            }
            for (i, sa) in la.iter().enumerate().skip(lb.len()) {
                out.push(Change {
                    path: path.index(i),
                    before: Some(Value::Struct(sa.clone())),
                    after: None,
                });
            }
            for (i, sb) in lb.iter().enumerate().skip(la.len()) {
                out.push(Change {
                    path: path.index(i),
                    before: None,
                    after: Some(Value::Struct(sb.clone())),
                });
            }
        }
        _ if a == b => {}
        _ => out.push(Change {
            path: path.clone(),
            before: Some(a.clone()),
            after: Some(b.clone()),
        }),
    }
}
