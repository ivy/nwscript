//! Writing a [`Gff`] in BioWare's layout.
//!
//! The sections follow the header in the document's order, each directly
//! after the last. A file built from scratch is laid out as BioWare's tool
//! lays out a file it writes in one pass: the struct array and the field
//! array fill depth-first, a struct's fields in order with a child struct's
//! fields (or a list's elements, each in turn) following the field that
//! holds them; the field indices follow in struct order, the list indices
//! and the data block in field order, and the label table in order of first
//! use. A file that was read keeps the positions it was read with: every
//! struct and field returns to its array slot, each struct's index list and
//! each list's indices return to their offsets, and the bytes between them
//! (the stale lists BioWare's incremental writer left when a struct or a list
//! grew) are written back from the arrays as read. A struct or field added
//! since the read takes the next free slot, and an index list that no longer
//! fits where it was is appended, which is what the engine's own writer
//! does.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::model::{Field, Gff, Label, Layout, Struct, Value};
use crate::read::{FIELD_LEN, HEADER_LEN, STRUCT_LEN};

/// What a field points at below it, by walk position.
enum Children {
    None,
    Struct(usize),
    List(Vec<usize>),
}

struct SNode<'a> {
    s: &'a Struct,
    /// Walk positions of the struct's fields, in struct order.
    fields: Vec<usize>,
}

struct FNode<'a> {
    f: &'a Field,
    children: Children,
}

/// The tree flattened in depth-first order: a struct, then each of its
/// fields with whatever hangs below it before the next field.
struct Walk<'a> {
    structs: Vec<SNode<'a>>,
    fields: Vec<FNode<'a>>,
}

impl<'a> Walk<'a> {
    fn of(root: &'a Struct) -> Self {
        let mut w = Walk {
            structs: Vec::new(),
            fields: Vec::new(),
        };
        w.structure(root);
        w
    }

    fn structure(&mut self, s: &'a Struct) -> usize {
        let pos = self.structs.len();
        self.structs.push(SNode {
            s,
            fields: Vec::with_capacity(s.fields.len()),
        });
        for f in &s.fields {
            let fpos = self.fields.len();
            self.fields.push(FNode {
                f,
                children: Children::None,
            });
            self.structs[pos].fields.push(fpos);
            let children = match &f.value {
                Value::Struct(child) => Children::Struct(self.structure(child)),
                Value::List(items) => {
                    Children::List(items.iter().map(|c| self.structure(c)).collect())
                }
                _ => Children::None,
            };
            self.fields[fpos].children = children;
        }
        pos
    }
}

/// The array position of each item, from the positions they were read with:
/// items keep their relative order by hint, an item without a hint (added
/// since the read) follows every item with one, and ties fall back to walk
/// order. A tree read whole and unchanged gets exactly its file's order.
fn ranks(hints: &[Option<u32>]) -> Vec<u32> {
    let mut order: Vec<usize> = (0..hints.len()).collect();
    order.sort_by_key(|&i| (hints[i].unwrap_or(u32::MAX), i));
    let mut rank = vec![0u32; hints.len()];
    for (r, i) in order.into_iter().enumerate() {
        rank[i] = r as u32;
    }
    rank
}

/// An index array being assembled: the image as read (or empty), with the
/// ranges live lists claim, and a tail for lists that fit nowhere.
struct IndexArray {
    bytes: Vec<u8>,
    claimed: Vec<bool>,
}

impl IndexArray {
    fn new(image: Option<&[u8]>) -> Self {
        let bytes = image.map(<[u8]>::to_vec).unwrap_or_default();
        IndexArray {
            claimed: vec![false; bytes.len()],
            bytes,
        }
    }

    /// Claim `len` bytes at `at` when the range lies within the image and
    /// nothing else has claimed any of it.
    fn claim(&mut self, at: u32, len: usize) -> bool {
        let at = at as usize;
        let Some(end) = at.checked_add(len) else {
            return false;
        };
        if end > self.bytes.len() || self.claimed[at..end].iter().any(|&c| c) {
            return false;
        }
        self.claimed[at..end].fill(true);
        true
    }

    /// Append `len` bytes and claim them.
    fn append(&mut self, len: usize) -> u32 {
        let at = self.bytes.len();
        self.bytes.resize(at + len, 0);
        self.claimed.resize(at + len, true);
        at as u32
    }

    fn put(&mut self, at: u32, words: impl IntoIterator<Item = u32>) {
        let mut p = at as usize;
        for w in words {
            self.bytes[p..p + 4].copy_from_slice(&w.to_le_bytes());
            p += 4;
        }
    }
}

/// Place each item's index list: a list with a hint that still fits is
/// placed there, in item order; the rest are appended, in item order.
fn place(array: &mut IndexArray, wants: &[(Option<u32>, usize)]) -> Vec<u32> {
    let mut at = vec![0u32; wants.len()];
    let mut pending = Vec::new();
    for (i, &(hint, len)) in wants.iter().enumerate() {
        match hint {
            Some(h) if array.claim(h, len) => at[i] = h,
            _ => pending.push(i),
        }
    }
    for i in pending {
        at[i] = array.append(wants[i].1);
    }
    at
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Encode a value that lives in the data block.
fn encode_data(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Dword64(x) => out.extend_from_slice(&x.to_le_bytes()),
        Value::Int64(x) => out.extend_from_slice(&x.to_le_bytes()),
        Value::Double(x) => out.extend_from_slice(&x.to_le_bytes()),
        Value::String(s) | Value::Void(s) => {
            push_u32(out, s.len() as u32);
            out.extend_from_slice(s);
        }
        Value::ResRef(r) => {
            out.push(r.len() as u8);
            out.extend_from_slice(r.as_bytes());
        }
        Value::LocString(l) => {
            let total: usize = 8 + l.substrings.iter().map(|s| 8 + s.text.len()).sum::<usize>();
            push_u32(out, total as u32);
            push_u32(out, l.strref);
            push_u32(out, l.substrings.len() as u32);
            for s in &l.substrings {
                push_u32(out, s.id);
                push_u32(out, s.text.len() as u32);
                out.extend_from_slice(&s.text);
            }
        }
        Value::Orientation(o) => {
            for x in o {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        Value::Vector(v) => {
            for x in v {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        Value::StrRef(s) => {
            push_u32(out, 4);
            push_u32(out, *s);
        }
        Value::Byte(_)
        | Value::Char(_)
        | Value::Word(_)
        | Value::Short(_)
        | Value::Dword(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Struct(_)
        | Value::List(_) => unreachable!("not a data-block type"),
    }
}

/// The word a simple value stores in its field entry.
fn simple_word(v: &Value) -> u32 {
    match v {
        Value::Byte(x) => u32::from(*x),
        Value::Char(x) => u32::from(*x as u8),
        Value::Word(x) => u32::from(*x),
        Value::Short(x) => u32::from(*x as u16),
        Value::Dword(x) => *x,
        Value::Int(x) => *x as u32,
        Value::Float(x) => x.to_bits(),
        _ => unreachable!("not a simple type"),
    }
}

impl Gff {
    /// The file's bytes, in the layout the module documentation describes.
    /// Fails only when the file would exceed the format's 32-bit offsets.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let walk = Walk::of(&self.root);
        let struct_rank = ranks(&walk.structs.iter().map(|n| n.s.index).collect::<Vec<_>>());
        let field_rank = ranks(&walk.fields.iter().map(|n| n.f.index).collect::<Vec<_>>());
        // Items by rank: `structs_by_rank[r]` is the walk position of the
        // struct written at array position `r`.
        let mut structs_by_rank = vec![0usize; walk.structs.len()];
        for (pos, &r) in struct_rank.iter().enumerate() {
            structs_by_rank[r as usize] = pos;
        }
        let mut fields_by_rank = vec![0usize; walk.fields.len()];
        for (pos, &r) in field_rank.iter().enumerate() {
            fields_by_rank[r as usize] = pos;
        }

        // Labels: the table as read, extended by first use in field order.
        let mut labels: Vec<Label> = self.labels.clone();
        let mut label_index: HashMap<Label, u32> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| (*l, i as u32))
            .collect();
        let mut field_label = vec![0u32; walk.fields.len()];
        for &pos in &fields_by_rank {
            let l = walk.fields[pos].f.label;
            let idx = *label_index.entry(l).or_insert_with(|| {
                labels.push(l);
                (labels.len() - 1) as u32
            });
            field_label[pos] = idx;
        }

        // Field indices: one list per struct with two or more fields, placed
        // in struct order.
        let layout: Option<&Layout> = self.layout.as_ref();
        let mut fidx = IndexArray::new(layout.map(|l| l.field_indices.as_slice()));
        let wants: Vec<(Option<u32>, usize)> = structs_by_rank
            .iter()
            .map(|&pos| {
                let n = walk.structs[pos].fields.len();
                let hint = walk.structs[pos]
                    .s
                    .indices_at
                    .filter(|&(_, count)| count as usize == n)
                    .map(|(at, _)| at);
                (hint, if n >= 2 { 4 * n } else { 0 })
            })
            .collect();
        let fidx_at = place_nonempty(&mut fidx, &wants);
        for (r, &pos) in structs_by_rank.iter().enumerate() {
            let node = &walk.structs[pos];
            if node.fields.len() >= 2 {
                fidx.put(fidx_at[r], node.fields.iter().map(|&fp| field_rank[fp]));
            }
        }

        // List indices: one block per list field, placed in field order.
        let mut lidx = IndexArray::new(layout.map(|l| l.list_indices.as_slice()));
        let wants: Vec<(Option<u32>, usize)> = fields_by_rank
            .iter()
            .map(|&pos| {
                let node = &walk.fields[pos];
                match &node.children {
                    Children::List(items) => (
                        node.f
                            .list_at
                            .filter(|&(_, count)| count as usize == items.len())
                            .map(|(at, _)| at),
                        4 + 4 * items.len(),
                    ),
                    _ => (None, 0),
                }
            })
            .collect();
        let lidx_at = place_nonempty(&mut lidx, &wants);
        for (r, &pos) in fields_by_rank.iter().enumerate() {
            if let Children::List(items) = &walk.fields[pos].children {
                lidx.put(
                    lidx_at[r],
                    std::iter::once(items.len() as u32)
                        .chain(items.iter().map(|&sp| struct_rank[sp])),
                );
            }
        }

        // The field array and the data block, in field order.
        let mut field_bytes = Vec::with_capacity(walk.fields.len() * FIELD_LEN);
        let mut data = Vec::new();
        for (r, &pos) in fields_by_rank.iter().enumerate() {
            let node = &walk.fields[pos];
            let ty = node.f.value.field_type();
            let d = match (&node.children, ty) {
                (Children::Struct(sp), _) => struct_rank[*sp],
                (Children::List(_), _) => lidx_at[r],
                (_, t) if t.in_data_block() => {
                    let at = data.len() as u32;
                    encode_data(&node.f.value, &mut data);
                    at
                }
                _ => simple_word(&node.f.value),
            };
            push_u32(&mut field_bytes, ty.id());
            push_u32(&mut field_bytes, field_label[pos]);
            push_u32(&mut field_bytes, d);
        }

        // The struct array, in struct order.
        let mut struct_bytes = Vec::with_capacity(walk.structs.len() * STRUCT_LEN);
        for (r, &pos) in structs_by_rank.iter().enumerate() {
            let node = &walk.structs[pos];
            let n = node.fields.len();
            let d = match n {
                0 => node.s.indices_at.map_or(0, |(at, _)| at),
                1 => field_rank[node.fields[0]],
                _ => fidx_at[r],
            };
            push_u32(&mut struct_bytes, node.s.id);
            push_u32(&mut struct_bytes, d);
            push_u32(&mut struct_bytes, n as u32);
        }

        let label_bytes: Vec<u8> = labels.iter().flat_map(|l| l.to_raw()).collect();
        let total = HEADER_LEN as u64
            + struct_bytes.len() as u64
            + field_bytes.len() as u64
            + label_bytes.len() as u64
            + data.len() as u64
            + fidx.bytes.len() as u64
            + lidx.bytes.len() as u64;
        if total > u64::from(u32::MAX) {
            return Err(Error::TooLarge { size: total });
        }
        let mut out = Vec::with_capacity(total as usize);
        out.extend_from_slice(&self.file_type);
        out.extend_from_slice(&self.version);
        let mut offset = HEADER_LEN as u32;
        for (bytes, count) in [
            (&struct_bytes, walk.structs.len() as u32),
            (&field_bytes, walk.fields.len() as u32),
            (&label_bytes, labels.len() as u32),
            (&data, data.len() as u32),
            (&fidx.bytes, fidx.bytes.len() as u32),
            (&lidx.bytes, lidx.bytes.len() as u32),
        ] {
            push_u32(&mut out, offset);
            push_u32(&mut out, count);
            offset += bytes.len() as u32;
        }
        out.extend_from_slice(&struct_bytes);
        out.extend_from_slice(&field_bytes);
        out.extend_from_slice(&label_bytes);
        out.extend_from_slice(&data);
        out.extend_from_slice(&fidx.bytes);
        out.extend_from_slice(&lidx.bytes);
        debug_assert_eq!(out.len() as u64, total);
        Ok(out)
    }
}

/// [`place`] over the items that want bytes; an item wanting none gets 0.
fn place_nonempty(array: &mut IndexArray, wants: &[(Option<u32>, usize)]) -> Vec<u32> {
    let nonempty: Vec<(usize, (Option<u32>, usize))> = wants
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, (_, len))| *len > 0)
        .collect();
    let placed = place(array, &nonempty.iter().map(|(_, w)| *w).collect::<Vec<_>>());
    let mut at = vec![0u32; wants.len()];
    for ((i, _), p) in nonempty.iter().zip(placed) {
        at[*i] = p;
    }
    at
}
