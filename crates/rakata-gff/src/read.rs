//! Parsing a GFF V3.2 file into the tree, keeping what the writer needs to
//! lay it out again: the label table, each struct's and field's array
//! position, and the index arrays whole.
//!
//! The layout is the one BioWare's GFF document describes: a 56-byte header
//! of fourteen little-endian dwords, then the struct array (12 bytes each),
//! the field array (12 bytes each), the label array (16 bytes each), the
//! field data block, the field indices array and the list indices array,
//! each located through the header's offset and count.

use rakata_res::ResRef;

use crate::error::{Error, Result};
use crate::model::{
    Field, FieldType, Gff, LABEL_LEN, Label, Layout, LocString, Struct, SubString, Value,
};

/// The header's length in bytes.
pub(crate) const HEADER_LEN: usize = 56;
/// A struct entry's length in bytes.
pub(crate) const STRUCT_LEN: usize = 12;
/// A field entry's length in bytes.
pub(crate) const FIELD_LEN: usize = 12;

/// The header as read.
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub file_type: [u8; 4],
    pub version: [u8; 4],
    pub struct_offset: u32,
    pub struct_count: u32,
    pub field_offset: u32,
    pub field_count: u32,
    pub label_offset: u32,
    pub label_count: u32,
    pub field_data_offset: u32,
    pub field_data_count: u32,
    pub field_indices_offset: u32,
    pub field_indices_count: u32,
    pub list_indices_offset: u32,
    pub list_indices_count: u32,
}

impl Header {
    /// The header of `bytes`, which must begin with any four-byte type and
    /// the version `V3.2`.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_LEN || &bytes[4..8] != b"V3.2" {
            return Err(Error::NotGff {
                found: String::from_utf8_lossy(&bytes[..bytes.len().min(8)]).into_owned(),
            });
        }
        let w = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
        Ok(Header {
            file_type: bytes[0..4].try_into().expect("four bytes"),
            version: bytes[4..8].try_into().expect("four bytes"),
            struct_offset: w(8),
            struct_count: w(12),
            field_offset: w(16),
            field_count: w(20),
            label_offset: w(24),
            label_count: w(28),
            field_data_offset: w(32),
            field_data_count: w(36),
            field_indices_offset: w(40),
            field_indices_count: w(44),
            list_indices_offset: w(48),
            list_indices_count: w(52),
        })
    }
}

/// The bytes of a section, or [`Error::Truncated`].
fn section<'a>(bytes: &'a [u8], what: &str, offset: u32, len: usize) -> Result<&'a [u8]> {
    let offset = offset as usize;
    match offset.checked_add(len) {
        Some(end) if end <= bytes.len() => Ok(&bytes[offset..end]),
        _ => Err(Error::Truncated {
            what: what.to_owned(),
            offset,
            len,
            available: bytes.len(),
        }),
    }
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("four bytes"))
}

/// One field entry as the file holds it.
#[derive(Clone, Copy)]
struct RawField {
    ty: u32,
    label: u32,
    data: u32,
}

/// One struct entry as the file holds it.
#[derive(Clone, Copy)]
struct RawStruct {
    id: u32,
    data: u32,
    count: u32,
}

struct Reader<'a> {
    structs: Vec<RawStruct>,
    fields: Vec<RawField>,
    labels: &'a [Label],
    data: &'a [u8],
    field_indices: &'a [u8],
    list_indices: &'a [u8],
    /// Which structs the tree has reached, so a struct reached twice (a
    /// cycle, or a shared child) is refused rather than followed forever.
    seen: Vec<bool>,
}

impl Reader<'_> {
    fn bad(what: &'static str, index: u32, count: usize) -> Error {
        Error::BadIndex {
            what,
            index,
            count: count as u32,
        }
    }

    /// `len` bytes of the data block at `at`, for the field `what`.
    fn data(&self, what: &str, at: usize, len: usize) -> Result<&[u8]> {
        match at.checked_add(len) {
            Some(end) if end <= self.data.len() => Ok(&self.data[at..end]),
            _ => Err(Error::Truncated {
                what: format!("the data of field {what}"),
                offset: at,
                len,
                available: self.data.len(),
            }),
        }
    }

    fn structure(&mut self, index: u32) -> Result<Struct> {
        let raw = *self
            .structs
            .get(index as usize)
            .ok_or_else(|| Self::bad("struct", index, self.structs.len()))?;
        if std::mem::replace(&mut self.seen[index as usize], true) {
            return Err(Error::BadIndex {
                what: "struct (reached twice)",
                index,
                count: self.structs.len() as u32,
            });
        }
        let field_indices: Vec<u32> = match raw.count {
            0 => Vec::new(),
            1 => vec![raw.data],
            n => {
                let at = raw.data as usize;
                let len = n as usize * 4;
                let end = at
                    .checked_add(len)
                    .filter(|&e| e <= self.field_indices.len());
                let Some(end) = end else {
                    return Err(Error::Truncated {
                        what: format!("the field indices of struct {index}"),
                        offset: at,
                        len,
                        available: self.field_indices.len(),
                    });
                };
                self.field_indices[at..end]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| u32::from_le_bytes(*c))
                    .collect()
            }
        };
        let mut fields = Vec::with_capacity(field_indices.len());
        for fi in field_indices {
            fields.push(self.field(fi)?);
        }
        Ok(Struct {
            id: raw.id,
            fields,
            index: Some(index),
            indices_at: (raw.count != 1).then_some((raw.data, raw.count)),
        })
    }

    fn field(&mut self, index: u32) -> Result<Field> {
        let raw = *self
            .fields
            .get(index as usize)
            .ok_or_else(|| Self::bad("field", index, self.fields.len()))?;
        let label = *self
            .labels
            .get(raw.label as usize)
            .ok_or_else(|| Self::bad("label", raw.label, self.labels.len()))?;
        let ty = FieldType::from_id(raw.ty).ok_or(Error::UnknownType {
            type_id: raw.ty,
            field: index,
        })?;
        let d = raw.data;
        let at = d as usize;
        let what = format!("{index} ({label})");
        let mut list_at = None;
        let value = match ty {
            FieldType::Byte => Value::Byte(d as u8),
            FieldType::Char => Value::Char(d as u8 as i8),
            FieldType::Word => Value::Word(d as u16),
            FieldType::Short => Value::Short(d as u16 as i16),
            FieldType::Dword => Value::Dword(d),
            FieldType::Int => Value::Int(d as i32),
            FieldType::Float => Value::Float(f32::from_bits(d)),
            FieldType::Dword64 => {
                let b = self.data(&what, at, 8)?;
                Value::Dword64(u64::from_le_bytes(b.try_into().expect("eight bytes")))
            }
            FieldType::Int64 => {
                let b = self.data(&what, at, 8)?;
                Value::Int64(i64::from_le_bytes(b.try_into().expect("eight bytes")))
            }
            FieldType::Double => {
                let b = self.data(&what, at, 8)?;
                Value::Double(f64::from_le_bytes(b.try_into().expect("eight bytes")))
            }
            FieldType::String => {
                let len = u32_at(self.data(&what, at, 4)?, 0) as usize;
                Value::String(self.data(&what, at + 4, len)?.to_vec())
            }
            FieldType::ResRef => {
                let len = self.data(&what, at, 1)?[0] as usize;
                let raw = self.data(&what, at + 1, len)?;
                let r = ResRef::from_bytes(raw).ok_or_else(|| Error::TooLong {
                    what: "resref",
                    text: String::from_utf8_lossy(raw).into_owned(),
                    len,
                })?;
                Value::ResRef(r)
            }
            FieldType::LocString => {
                let head = self.data(&what, at, 12)?;
                let total = u32_at(head, 0) as usize;
                let strref = u32_at(head, 4);
                let count = u32_at(head, 8);
                // The whole string lies within `total` bytes after the size
                // word; checking that first bounds every substring read.
                self.data(&what, at + 4, total)?;
                let mut p = at + 12;
                let mut substrings = Vec::with_capacity(count.min(64) as usize);
                for _ in 0..count {
                    let h = self.data(&what, p, 8)?;
                    let id = u32_at(h, 0);
                    let len = u32_at(h, 4) as usize;
                    let text = self.data(&what, p + 8, len)?.to_vec();
                    substrings.push(SubString { id, text });
                    p += 8 + len;
                }
                Value::LocString(LocString { strref, substrings })
            }
            FieldType::Void => {
                let len = u32_at(self.data(&what, at, 4)?, 0) as usize;
                Value::Void(self.data(&what, at + 4, len)?.to_vec())
            }
            FieldType::Orientation => {
                let b = self.data(&what, at, 16)?;
                let mut o = [0f32; 4];
                for (i, c) in b.as_chunks::<4>().0.iter().enumerate() {
                    o[i] = f32::from_le_bytes(*c);
                }
                Value::Orientation(o)
            }
            FieldType::Vector => {
                let b = self.data(&what, at, 12)?;
                let mut v = [0f32; 3];
                for (i, c) in b.as_chunks::<4>().0.iter().enumerate() {
                    v[i] = f32::from_le_bytes(*c);
                }
                Value::Vector(v)
            }
            FieldType::StrRef => {
                // A size word, 4 in every file measured, then the strref.
                let b = self.data(&what, at, 8)?;
                Value::StrRef(u32_at(b, 4))
            }
            FieldType::Struct => Value::Struct(self.structure(d)?),
            FieldType::List => {
                let head = self.list_slice(index, at, 4)?;
                let count = u32_at(head, 0) as usize;
                list_at = Some((d, count as u32));
                let body = self.list_slice(index, at + 4, count * 4)?;
                let indices: Vec<u32> = body
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| u32::from_le_bytes(*c))
                    .collect();
                let mut elements = Vec::with_capacity(indices.len());
                for si in indices {
                    elements.push(self.structure(si)?);
                }
                Value::List(elements)
            }
        };
        let width = match ty {
            FieldType::Byte | FieldType::Char => Some(0xFF),
            FieldType::Word | FieldType::Short => Some(0xFFFF),
            _ => None,
        };
        Ok(Field {
            label,
            value,
            index: Some(index),
            list_at,
            wide: width.and_then(|m: u32| (d & !m != 0).then_some(d)),
        })
    }

    fn list_slice(&self, field: u32, at: usize, len: usize) -> Result<&[u8]> {
        match at.checked_add(len) {
            Some(end) if end <= self.list_indices.len() => Ok(&self.list_indices[at..end]),
            _ => Err(Error::Truncated {
                what: format!("the list indices of field {field}"),
                offset: at,
                len,
                available: self.list_indices.len(),
            }),
        }
    }
}

impl Gff {
    /// Parse a GFF V3.2 file. Every section the header names must lie
    /// within `bytes`, every index within its array, and every field type
    /// be one of the nineteen the engine stores; the first struct is the
    /// root.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let h = Header::parse(bytes)?;
        let structs_raw = section(
            bytes,
            "the struct array",
            h.struct_offset,
            h.struct_count as usize * STRUCT_LEN,
        )?;
        let fields_raw = section(
            bytes,
            "the field array",
            h.field_offset,
            h.field_count as usize * FIELD_LEN,
        )?;
        let labels_raw = section(
            bytes,
            "the label array",
            h.label_offset,
            h.label_count as usize * LABEL_LEN,
        )?;
        let data = section(
            bytes,
            "the field data block",
            h.field_data_offset,
            h.field_data_count as usize,
        )?;
        let field_indices = section(
            bytes,
            "the field indices array",
            h.field_indices_offset,
            h.field_indices_count as usize,
        )?;
        let list_indices = section(
            bytes,
            "the list indices array",
            h.list_indices_offset,
            h.list_indices_count as usize,
        )?;
        if h.struct_count == 0 {
            return Err(Error::BadIndex {
                what: "struct",
                index: 0,
                count: 0,
            });
        }
        let structs: Vec<RawStruct> = structs_raw
            .as_chunks::<STRUCT_LEN>()
            .0
            .iter()
            .map(|c| RawStruct {
                id: u32_at(c, 0),
                data: u32_at(c, 4),
                count: u32_at(c, 8),
            })
            .collect();
        let fields: Vec<RawField> = fields_raw
            .as_chunks::<FIELD_LEN>()
            .0
            .iter()
            .map(|c| RawField {
                ty: u32_at(c, 0),
                label: u32_at(c, 4),
                data: u32_at(c, 8),
            })
            .collect();
        let labels: Vec<Label> = labels_raw
            .as_chunks::<LABEL_LEN>()
            .0
            .iter()
            .map(|c| Label::from_raw(*c))
            .collect();
        let seen = vec![false; structs.len()];
        let mut reader = Reader {
            structs,
            fields,
            labels: &labels,
            data,
            field_indices,
            list_indices,
            seen,
        };
        let root = reader.structure(0)?;
        Ok(Gff {
            file_type: h.file_type,
            version: h.version,
            root,
            labels,
            layout: Some(Layout {
                field_indices: field_indices.to_vec(),
                list_indices: list_indices.to_vec(),
            }),
        })
    }
}
