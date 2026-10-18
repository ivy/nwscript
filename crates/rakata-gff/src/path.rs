//! Field paths: one field of a GFF named from the command line or a diff.
//!
//! A path is a sequence of labels separated by `.`, each optionally followed
//! by `[n]` to pick the n-th struct of a list field:
//! `EntryList[3].RepliesList[0].Index`, `Map.MapResX`, `Tag`. The empty
//! path names the root struct. A label is any text without `.`, `[` or
//! `]`, so `Creature List[0].TemplateResRef` works; a label holding those
//! characters cannot be addressed by a path.

use std::fmt;
use std::str::FromStr;

use crate::error::{Error, Result};
use crate::model::{Gff, Label, Struct, Value};

/// One step of a [`Path`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Segment {
    /// The field with this label in the current struct.
    Field(Label),
    /// The n-th struct of the current list.
    Index(usize),
}

/// A path to a field or a struct, from the root.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Path(Vec<Segment>);

impl Path {
    /// The root struct.
    pub const fn root() -> Self {
        Path(Vec::new())
    }

    /// Parse a path. The empty string is the root.
    pub fn parse(text: &str) -> Result<Self> {
        let bad = |reason: &'static str| Error::BadPath {
            text: text.to_owned(),
            reason,
        };
        let mut segments = Vec::new();
        if text.is_empty() {
            return Ok(Path(segments));
        }
        for part in text.split('.') {
            let (label, rest) = match part.find('[') {
                Some(i) => (&part[..i], &part[i..]),
                None => (part, ""),
            };
            if label.is_empty() {
                return Err(bad("a segment has no label"));
            }
            if label.contains(']') {
                return Err(bad("a `]` without its `[`"));
            }
            segments.push(Segment::Field(
                Label::new(label).map_err(|_| bad("a label longer than sixteen bytes"))?,
            ));
            let mut rest = rest;
            while let Some(r) = rest.strip_prefix('[') {
                let Some(close) = r.find(']') else {
                    return Err(bad("a `[` without its `]`"));
                };
                let n: usize = r[..close]
                    .parse()
                    .map_err(|_| bad("an index that is not a number"))?;
                segments.push(Segment::Index(n));
                rest = &r[close + 1..];
            }
            if !rest.is_empty() {
                return Err(bad("text after an index that is not `.` or `[`"));
            }
        }
        Ok(Path(segments))
    }

    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// This path with a field segment appended.
    pub fn field(&self, label: Label) -> Self {
        let mut p = self.clone();
        p.0.push(Segment::Field(label));
        p
    }

    /// This path with an index segment appended.
    pub fn index(&self, n: usize) -> Self {
        let mut p = self.clone();
        p.0.push(Segment::Index(n));
        p
    }

    pub fn push(&mut self, segment: Segment) {
        self.0.push(segment);
    }

    /// The path without its last segment, if any.
    pub fn parent(&self) -> Option<Self> {
        self.0.split_last().map(|(_, rest)| Path(rest.to_vec()))
    }

    /// The path up to and excluding segment `n`, as text.
    fn prefix(&self, n: usize) -> String {
        Path(self.0[..n].to_vec()).to_string()
    }
}

impl FromStr for Path {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Path::parse(s)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, s) in self.0.iter().enumerate() {
            match s {
                Segment::Field(l) => {
                    if i > 0 {
                        f.write_str(".")?;
                    }
                    write!(f, "{l}")?;
                }
                Segment::Index(n) => write!(f, "[{n}]")?,
            }
        }
        Ok(())
    }
}

/// Where a path lands.
enum Node<'a> {
    Struct(&'a Struct),
    Field(&'a Value),
}

enum NodeMut<'a> {
    Struct(&'a mut Struct),
    Field(&'a mut Value),
}

fn no_such(path: &Path, n: usize, reason: String) -> Error {
    Error::NoSuchPath {
        at: path.prefix(n),
        reason,
    }
}

fn resolve<'a>(root: &'a Struct, path: &Path) -> Result<Node<'a>> {
    let mut node = Node::Struct(root);
    for (i, seg) in path.0.iter().enumerate() {
        node = match (node, seg) {
            (Node::Struct(s) | Node::Field(Value::Struct(s)), Segment::Field(l)) => {
                let f = s
                    .fields
                    .iter()
                    .find(|f| f.label == *l)
                    .ok_or_else(|| no_such(path, i, format!("no field `{l}`")))?;
                Node::Field(&f.value)
            }
            (Node::Field(Value::List(items)), Segment::Index(n)) => {
                let s = items.get(*n).ok_or_else(|| {
                    no_such(
                        path,
                        i,
                        format!("index {n} is past the end of a list of {}", items.len()),
                    )
                })?;
                Node::Struct(s)
            }
            (Node::Field(v), Segment::Index(_)) => {
                return Err(no_such(
                    path,
                    i,
                    format!("a {} field is not a list", v.field_type()),
                ));
            }
            (Node::Field(v), Segment::Field(_)) => {
                return Err(no_such(
                    path,
                    i,
                    format!("a {} field is not a struct", v.field_type()),
                ));
            }
            (Node::Struct(_), Segment::Index(_)) => {
                return Err(no_such(path, i, "a struct is not a list".to_string()));
            }
        };
    }
    Ok(node)
}

fn resolve_mut<'a>(root: &'a mut Struct, path: &Path) -> Result<NodeMut<'a>> {
    let mut node = NodeMut::Struct(root);
    for (i, seg) in path.0.iter().enumerate() {
        node = match (node, seg) {
            (NodeMut::Struct(s) | NodeMut::Field(Value::Struct(s)), Segment::Field(l)) => {
                let f = s
                    .fields
                    .iter_mut()
                    .find(|f| f.label == *l)
                    .ok_or_else(|| no_such(path, i, format!("no field `{l}`")))?;
                NodeMut::Field(&mut f.value)
            }
            (NodeMut::Field(Value::List(items)), Segment::Index(n)) => {
                let len = items.len();
                let s = items.get_mut(*n).ok_or_else(|| {
                    no_such(
                        path,
                        i,
                        format!("index {n} is past the end of a list of {len}"),
                    )
                })?;
                NodeMut::Struct(s)
            }
            (NodeMut::Field(v), Segment::Index(_)) => {
                return Err(no_such(
                    path,
                    i,
                    format!("a {} field is not a list", v.field_type()),
                ));
            }
            (NodeMut::Field(v), Segment::Field(_)) => {
                return Err(no_such(
                    path,
                    i,
                    format!("a {} field is not a struct", v.field_type()),
                ));
            }
            (NodeMut::Struct(_), Segment::Index(_)) => {
                return Err(no_such(path, i, "a struct is not a list".to_string()));
            }
        };
    }
    Ok(node)
}

impl Gff {
    /// The value at `path`, which must end in a field.
    pub fn get(&self, path: &Path) -> Result<&Value> {
        match resolve(&self.root, path)? {
            Node::Field(v) => Ok(v),
            Node::Struct(_) => Err(Error::NoSuchPath {
                at: path.to_string(),
                reason: "names a struct, not a field".to_string(),
            }),
        }
    }

    /// The value at `path`, mutably.
    pub fn get_mut(&mut self, path: &Path) -> Result<&mut Value> {
        match resolve_mut(&mut self.root, path)? {
            NodeMut::Field(v) => Ok(v),
            NodeMut::Struct(_) => Err(Error::NoSuchPath {
                at: path.to_string(),
                reason: "names a struct, not a field".to_string(),
            }),
        }
    }

    /// Replace the value at `path`, which must name an existing field, and
    /// return the old value. The new value may be of another type.
    pub fn set(&mut self, path: &Path, value: Value) -> Result<Value> {
        let slot = self.get_mut(path)?;
        Ok(std::mem::replace(slot, value))
    }

    /// The struct at `path`: the root for the empty path, a list element
    /// for a path ending in an index, or the struct a `Struct` field holds.
    pub fn struct_at(&self, path: &Path) -> Result<&Struct> {
        match resolve(&self.root, path)? {
            Node::Struct(s) | Node::Field(Value::Struct(s)) => Ok(s),
            Node::Field(v) => Err(Error::NoSuchPath {
                at: path.to_string(),
                reason: format!("names a {} field, not a struct", v.field_type()),
            }),
        }
    }

    /// The struct at `path`, mutably.
    pub fn struct_at_mut(&mut self, path: &Path) -> Result<&mut Struct> {
        match resolve_mut(&mut self.root, path)? {
            NodeMut::Struct(s) | NodeMut::Field(Value::Struct(s)) => Ok(s),
            NodeMut::Field(v) => Err(Error::NoSuchPath {
                at: path.to_string(),
                reason: format!("names a {} field, not a struct", v.field_type()),
            }),
        }
    }
}
