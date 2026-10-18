//! The GFF tree: a [`Gff`] holds a root [`Struct`], a struct holds
//! [`Field`]s, a field holds a [`Value`] of one of the types the engine
//! stores. Alongside the tree the model keeps what a byte-identical write
//! needs and the tree alone does not say: the label table in file order, each
//! struct's and field's position in the file's arrays, and the two index
//! arrays as read (see [`Layout`]).

use std::fmt;
use std::str::FromStr;

use rakata_res::ResRef;

use crate::error::{Error, Result};

/// The width of a label, and of a resref, in bytes.
pub const LABEL_LEN: usize = 16;

/// A field's label: at most sixteen bytes, NUL-padded in the file.
///
/// The padding bytes are kept as read, so the label table writes back as it
/// was; every label BioWare shipped pads with zeros. Equality covers the
/// padding too.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Label([u8; LABEL_LEN]);

impl Label {
    /// A label from text. Fails with [`Error::TooLong`] past sixteen bytes.
    pub fn new(text: &str) -> Result<Self> {
        if text.len() > LABEL_LEN {
            return Err(Error::TooLong {
                what: "label",
                text: text.to_owned(),
                len: text.len(),
            });
        }
        let mut raw = [0u8; LABEL_LEN];
        raw[..text.len()].copy_from_slice(text.as_bytes());
        Ok(Label(raw))
    }

    /// The sixteen bytes as the file holds them.
    pub const fn from_raw(raw: [u8; LABEL_LEN]) -> Self {
        Label(raw)
    }

    /// The sixteen bytes as the file holds them.
    pub const fn to_raw(self) -> [u8; LABEL_LEN] {
        self.0
    }

    /// The label's bytes up to the first NUL.
    pub fn as_bytes(&self) -> &[u8] {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(LABEL_LEN);
        &self.0[..end]
    }

    /// The label as text, when it is valid UTF-8. Every label BioWare
    /// shipped is ASCII.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.as_bytes()).ok()
    }

    /// Whether this label reads as `text`.
    pub fn is(&self, text: &str) -> bool {
        self.as_bytes() == text.as_bytes()
    }
}

impl FromStr for Label {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Label::new(s)
    }
}

impl TryFrom<&str> for Label {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        Label::new(s)
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.as_bytes()))
    }
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Label({:?})", String::from_utf8_lossy(self.as_bytes()))
    }
}

/// The type of a field, with the id the file stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FieldType {
    Byte = 0,
    Char = 1,
    Word = 2,
    Short = 3,
    Dword = 4,
    Int = 5,
    Dword64 = 6,
    Int64 = 7,
    Float = 8,
    Double = 9,
    /// `CExoString`.
    String = 10,
    /// `CResRef`.
    ResRef = 11,
    /// `CExoLocString`.
    LocString = 12,
    Void = 13,
    Struct = 14,
    List = 15,
    /// Four floats; KotOR only.
    Orientation = 16,
    /// Three floats; KotOR only.
    Vector = 17,
    /// A talk-table reference stored in the data block; KotOR only.
    StrRef = 18,
}

impl FieldType {
    /// Every type, in id order.
    pub const ALL: [FieldType; 19] = [
        FieldType::Byte,
        FieldType::Char,
        FieldType::Word,
        FieldType::Short,
        FieldType::Dword,
        FieldType::Int,
        FieldType::Dword64,
        FieldType::Int64,
        FieldType::Float,
        FieldType::Double,
        FieldType::String,
        FieldType::ResRef,
        FieldType::LocString,
        FieldType::Void,
        FieldType::Struct,
        FieldType::List,
        FieldType::Orientation,
        FieldType::Vector,
        FieldType::StrRef,
    ];

    /// The type with this id, if any.
    pub fn from_id(id: u32) -> Option<Self> {
        Self::ALL.get(id as usize).copied()
    }

    /// The id the file stores.
    pub const fn id(self) -> u32 {
        self as u32
    }

    /// The type's name as the format documents spell it.
    pub const fn name(self) -> &'static str {
        match self {
            FieldType::Byte => "BYTE",
            FieldType::Char => "CHAR",
            FieldType::Word => "WORD",
            FieldType::Short => "SHORT",
            FieldType::Dword => "DWORD",
            FieldType::Int => "INT",
            FieldType::Dword64 => "DWORD64",
            FieldType::Int64 => "INT64",
            FieldType::Float => "FLOAT",
            FieldType::Double => "DOUBLE",
            FieldType::String => "CExoString",
            FieldType::ResRef => "CResRef",
            FieldType::LocString => "CExoLocString",
            FieldType::Void => "VOID",
            FieldType::Struct => "Struct",
            FieldType::List => "List",
            FieldType::Orientation => "Orientation",
            FieldType::Vector => "Vector",
            FieldType::StrRef => "StrRef",
        }
    }

    /// Whether the value lives in the field data block rather than in the
    /// field entry itself. Structs and lists are neither: they point into
    /// the struct array and the list indices.
    pub const fn in_data_block(self) -> bool {
        matches!(
            self,
            FieldType::Dword64
                | FieldType::Int64
                | FieldType::Double
                | FieldType::String
                | FieldType::ResRef
                | FieldType::LocString
                | FieldType::Void
                | FieldType::Orientation
                | FieldType::Vector
                | FieldType::StrRef
        )
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One substring of a [`LocString`]: text for one language and gender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubString {
    /// The file's string id: the language id times two, plus one for the
    /// feminine form.
    pub id: u32,
    /// The text, in the game's code page, as read.
    pub text: Vec<u8>,
}

impl SubString {
    /// The language id: the string id halved.
    pub const fn language(&self) -> u32 {
        self.id / 2
    }

    /// Whether this is the feminine form.
    pub const fn feminine(&self) -> bool {
        self.id % 2 == 1
    }
}

/// A `CExoLocString`: a talk-table reference and embedded substrings by
/// language and gender, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocString {
    /// The strref; [`LocString::NO_STRREF`] when the string carries none.
    pub strref: u32,
    pub substrings: Vec<SubString>,
}

impl LocString {
    /// The strref of a string that does not reference the talk table.
    pub const NO_STRREF: u32 = 0xFFFF_FFFF;

    /// No strref and no substrings.
    pub const fn none() -> Self {
        LocString {
            strref: Self::NO_STRREF,
            substrings: Vec::new(),
        }
    }

    /// A strref alone.
    pub const fn strref(strref: u32) -> Self {
        LocString {
            strref,
            substrings: Vec::new(),
        }
    }

    /// Whether the string references the talk table.
    pub const fn has_strref(&self) -> bool {
        self.strref != Self::NO_STRREF
    }

    /// The substring with this string id.
    pub fn get(&self, id: u32) -> Option<&[u8]> {
        self.substrings
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.text.as_slice())
    }

    /// Set the substring with this string id, replacing one present or
    /// appending.
    pub fn set(&mut self, id: u32, text: impl Into<Vec<u8>>) {
        let text = text.into();
        match self.substrings.iter_mut().find(|s| s.id == id) {
            Some(s) => s.text = text,
            None => self.substrings.push(SubString { id, text }),
        }
    }
}

/// A field's value.
///
/// Strings are bytes in the game's code page, as read; floats compare by
/// their bits, so a value read writes back exactly and a NaN is equal to
/// itself.
#[derive(Debug, Clone)]
pub enum Value {
    Byte(u8),
    Char(i8),
    Word(u16),
    Short(i16),
    Dword(u32),
    Int(i32),
    Dword64(u64),
    Int64(i64),
    Float(f32),
    Double(f64),
    /// `CExoString`.
    String(Vec<u8>),
    /// `CResRef`.
    ResRef(ResRef),
    /// `CExoLocString`.
    LocString(LocString),
    Void(Vec<u8>),
    Struct(Struct),
    List(Vec<Struct>),
    Orientation([f32; 4]),
    Vector([f32; 3]),
    StrRef(u32),
}

impl Value {
    /// The value's type.
    pub const fn field_type(&self) -> FieldType {
        match self {
            Value::Byte(_) => FieldType::Byte,
            Value::Char(_) => FieldType::Char,
            Value::Word(_) => FieldType::Word,
            Value::Short(_) => FieldType::Short,
            Value::Dword(_) => FieldType::Dword,
            Value::Int(_) => FieldType::Int,
            Value::Dword64(_) => FieldType::Dword64,
            Value::Int64(_) => FieldType::Int64,
            Value::Float(_) => FieldType::Float,
            Value::Double(_) => FieldType::Double,
            Value::String(_) => FieldType::String,
            Value::ResRef(_) => FieldType::ResRef,
            Value::LocString(_) => FieldType::LocString,
            Value::Void(_) => FieldType::Void,
            Value::Struct(_) => FieldType::Struct,
            Value::List(_) => FieldType::List,
            Value::Orientation(_) => FieldType::Orientation,
            Value::Vector(_) => FieldType::Vector,
            Value::StrRef(_) => FieldType::StrRef,
        }
    }

    /// A `CExoString` from text.
    pub fn string(text: &str) -> Self {
        Value::String(text.as_bytes().to_vec())
    }

    /// A `CResRef` from text; fails past sixteen bytes.
    pub fn resref(name: &str) -> Result<Self> {
        ResRef::new(name)
            .map(Value::ResRef)
            .map_err(|_| Error::TooLong {
                what: "resref",
                text: name.to_owned(),
                len: name.len(),
            })
    }

    /// The `CExoString` as text, lossily; `None` for another type.
    pub fn as_str(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Value::String(b) => Some(String::from_utf8_lossy(b)),
            _ => None,
        }
    }

    /// The struct a `Struct` field holds; `None` for another type.
    pub const fn as_struct(&self) -> Option<&Struct> {
        match self {
            Value::Struct(s) => Some(s),
            _ => None,
        }
    }

    /// The elements a `List` field holds; `None` for another type.
    pub fn as_list(&self) -> Option<&[Struct]> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    /// Whether two values are the same, with floats compared by bits and
    /// structs compared by [`Struct::semantic_eq`].
    pub fn semantic_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Struct(a), Value::Struct(b)) => a.semantic_eq(b),
            (Value::List(a), Value::List(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.semantic_eq(y))
            }
            _ => self == other,
        }
    }
}

impl PartialEq for Value {
    /// Floats compare by bits; structs and lists compare in field order.
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Byte(a), Value::Byte(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Word(a), Value::Word(b)) => a == b,
            (Value::Short(a), Value::Short(b)) => a == b,
            (Value::Dword(a), Value::Dword(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Dword64(a), Value::Dword64(b)) => a == b,
            (Value::Int64(a), Value::Int64(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Double(a), Value::Double(b)) => a.to_bits() == b.to_bits(),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::ResRef(a), Value::ResRef(b)) => a.as_bytes() == b.as_bytes(),
            (Value::LocString(a), Value::LocString(b)) => a == b,
            (Value::Void(a), Value::Void(b)) => a == b,
            (Value::Struct(a), Value::Struct(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Orientation(a), Value::Orientation(b)) => {
                a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
            }
            (Value::Vector(a), Value::Vector(b)) => {
                a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
            }
            (Value::StrRef(a), Value::StrRef(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

/// A labelled value inside a struct.
#[derive(Debug, Clone)]
pub struct Field {
    pub label: Label,
    pub value: Value,
    /// The field's position in the file's field array, when it was read
    /// from one. Decides the order of the field array, and so of the data
    /// block, on write.
    pub(crate) index: Option<u32>,
    /// For a list, the offset of its indices in the list indices array and
    /// the element count there, as read; the slot is reused only while the
    /// count is unchanged.
    pub(crate) list_at: Option<(u32, u32)>,
}

impl Field {
    pub fn new(label: Label, value: Value) -> Self {
        Field {
            label,
            value,
            index: None,
            list_at: None,
        }
    }

    /// The field's position in the field array it was read from, if any.
    pub fn file_index(&self) -> Option<u32> {
        self.index
    }
}

impl PartialEq for Field {
    /// Label and value; where the field lay in a file does not count.
    fn eq(&self, other: &Field) -> bool {
        self.label == other.label && self.value == other.value
    }
}

impl Eq for Field {}

/// A struct: a programmer-defined id and fields in file order.
#[derive(Debug, Clone)]
pub struct Struct {
    /// The struct id; [`Struct::ROOT_ID`] for the top-level struct.
    pub id: u32,
    pub fields: Vec<Field>,
    /// The struct's position in the file's struct array, when it was read
    /// from one.
    pub(crate) index: Option<u32>,
    /// The `DataOrDataOffset` word as read with the field count beside it:
    /// the offset of the struct's field indices, reused only while the count
    /// is unchanged, or whatever a struct with no fields carried there.
    pub(crate) indices_at: Option<(u32, u32)>,
}

impl Struct {
    /// The id of every file's top-level struct.
    pub const ROOT_ID: u32 = 0xFFFF_FFFF;

    /// A struct with this id and no fields.
    pub const fn new(id: u32) -> Self {
        Struct {
            id,
            fields: Vec::new(),
            index: None,
            indices_at: None,
        }
    }

    /// The struct's position in the struct array it was read from, if any.
    pub fn file_index(&self) -> Option<u32> {
        self.index
    }

    /// The field with this label, the first when a file repeats one.
    pub fn field(&self, label: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.label.is(label))
    }

    pub fn field_mut(&mut self, label: &str) -> Option<&mut Field> {
        self.fields.iter_mut().find(|f| f.label.is(label))
    }

    /// The value of the field with this label.
    pub fn get(&self, label: &str) -> Option<&Value> {
        self.field(label).map(|f| &f.value)
    }

    pub fn get_mut(&mut self, label: &str) -> Option<&mut Value> {
        self.field_mut(label).map(|f| &mut f.value)
    }

    /// Set the field with this label, replacing its value or appending a
    /// new field. Returns the value replaced.
    pub fn set(&mut self, label: Label, value: Value) -> Option<Value> {
        match self.fields.iter_mut().find(|f| f.label == label) {
            Some(f) => Some(std::mem::replace(&mut f.value, value)),
            None => {
                self.fields.push(Field::new(label, value));
                None
            }
        }
    }

    /// Append a field without looking for one of the same label.
    pub fn push(&mut self, label: Label, value: Value) -> &mut Self {
        self.fields.push(Field::new(label, value));
        self
    }

    /// Remove the field with this label, returning it.
    pub fn remove(&mut self, label: &str) -> Option<Field> {
        let i = self.fields.iter().position(|f| f.label.is(label))?;
        Some(self.fields.remove(i))
    }

    /// Whether two structs hold the same id and the same fields, matched by
    /// label in any order, as the engine reads them.
    pub fn semantic_eq(&self, other: &Struct) -> bool {
        if self.id != other.id || self.fields.len() != other.fields.len() {
            return false;
        }
        let mut used = vec![false; other.fields.len()];
        'outer: for f in &self.fields {
            for (i, g) in other.fields.iter().enumerate() {
                if !used[i] && f.label == g.label && f.value.semantic_eq(&g.value) {
                    used[i] = true;
                    continue 'outer;
                }
            }
            return false;
        }
        true
    }
}

impl PartialEq for Struct {
    /// Id and fields in order; where the struct lay in a file does not
    /// count.
    fn eq(&self, other: &Struct) -> bool {
        self.id == other.id && self.fields == other.fields
    }
}

impl Eq for Struct {}

/// The two index arrays of the file a [`Gff`] was read from, kept whole so
/// the bytes no live struct or list uses (the stale copies BioWare's
/// incremental writer leaves behind) write back in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layout {
    pub field_indices: Vec<u8>,
    pub list_indices: Vec<u8>,
}

/// A GFF file: its four-byte type, its version, its root struct, and its
/// label table in file order.
#[derive(Debug, Clone)]
pub struct Gff {
    /// The file type, four bytes: `UTC `, `DLG `, `GIT `.
    pub file_type: [u8; 4],
    /// The version, four bytes: `V3.2`.
    pub version: [u8; 4],
    pub root: Struct,
    /// The label table as read, which the writer keeps and extends; a label
    /// a new field uses is appended.
    pub labels: Vec<Label>,
    pub(crate) layout: Option<Layout>,
}

impl Gff {
    /// The version this crate reads and writes.
    pub const VERSION: [u8; 4] = *b"V3.2";

    /// An empty file of this type (`"UTC "`, with its trailing space), with
    /// a root struct and no fields.
    pub fn new(file_type: [u8; 4]) -> Self {
        Gff {
            file_type,
            version: Self::VERSION,
            root: Struct::new(Struct::ROOT_ID),
            labels: Vec::new(),
            layout: None,
        }
    }

    /// The file type as text, lossily, without its trailing space.
    pub fn file_type_str(&self) -> String {
        String::from_utf8_lossy(&self.file_type)
            .trim_end()
            .to_string()
    }

    /// Whether two files hold the same type, version and data, layout
    /// aside: fields are matched by label in any order.
    pub fn semantic_eq(&self, other: &Gff) -> bool {
        self.file_type == other.file_type
            && self.version == other.version
            && self.root.semantic_eq(&other.root)
    }
}

impl PartialEq for Gff {
    /// Type, version and the tree in field order; the label table and the
    /// layout do not count.
    fn eq(&self, other: &Gff) -> bool {
        self.file_type == other.file_type
            && self.version == other.version
            && self.root == other.root
    }
}

impl Eq for Gff {}
