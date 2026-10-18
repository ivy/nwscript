//! The layout facts of a GFF file as written: what another writer may have
//! done differently from BioWare's tool, read from the bytes rather than
//! the tree, so a file that is equal after re-read can be told apart by
//! its layout and named.

use std::fmt;

use crate::error::Result;
use crate::model::FieldType;
use crate::read::{FIELD_LEN, HEADER_LEN, Header, STRUCT_LEN};

/// How a file is laid out, for the facts a tree does not carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Survey {
    pub file_type: String,
    pub version: String,
    /// Whether the six sections follow the header in the document's order,
    /// each directly after the last, and the file ends with the last.
    pub sections_adjacent: bool,
    pub struct_count: u32,
    pub field_count: u32,
    pub label_count: u32,
    /// The data block's size, and the bytes some field's value covers.
    pub data_bytes: u32,
    pub data_used: u32,
    /// The field indices array's size, and the bytes a live struct's list
    /// covers; BioWare's incremental writer leaves stale lists, so used is
    /// at most the size and often less.
    pub field_indices_bytes: u32,
    pub field_indices_used: u32,
    pub list_indices_bytes: u32,
    pub list_indices_used: u32,
    /// `CExoLocString`s whose total-size word disagrees with the substrings
    /// that follow it.
    pub locstring_totals_wrong: u32,
    /// `BYTE`, `CHAR`, `WORD` and `SHORT` fields whose data word carries
    /// bits above the type's width.
    pub narrow_fields_with_wide_values: u32,
}

/// Survey `bytes`. Fails as [`crate::Gff::parse`] would on a header that
/// is not a GFF's or a section beyond the end; a field whose data runs past
/// the block is counted as using nothing.
pub fn survey(bytes: &[u8]) -> Result<Survey> {
    let h = Header::parse(bytes)?;
    let w = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
    let end = |off: u32, len: u32| (off as u64 + len as u64) as usize;
    let sections_adjacent = h.struct_offset == HEADER_LEN as u32
        && h.field_offset as u64
            == h.struct_offset as u64 + STRUCT_LEN as u64 * h.struct_count as u64
        && h.label_offset as u64 == h.field_offset as u64 + FIELD_LEN as u64 * h.field_count as u64
        && h.field_data_offset as u64 == h.label_offset as u64 + 16 * h.label_count as u64
        && h.field_indices_offset as u64 == h.field_data_offset as u64 + h.field_data_count as u64
        && h.list_indices_offset as u64
            == h.field_indices_offset as u64 + h.field_indices_count as u64
        && end(h.list_indices_offset, h.list_indices_count) == bytes.len();
    let within = |off: u32, len: u32| end(off, len) <= bytes.len();
    if !(within(
        h.struct_offset,
        h.struct_count.saturating_mul(STRUCT_LEN as u32),
    ) && within(
        h.field_offset,
        h.field_count.saturating_mul(FIELD_LEN as u32),
    ) && within(h.field_data_offset, h.field_data_count)
        && within(h.field_indices_offset, h.field_indices_count)
        && within(h.list_indices_offset, h.list_indices_count))
    {
        return Err(crate::Error::Truncated {
            what: "a section".to_string(),
            offset: 0,
            len: 0,
            available: bytes.len(),
        });
    }

    // Bytes each array uses: a bitmap, since lists may overlap or repeat.
    let mut data_used = vec![false; h.field_data_count as usize];
    let mut list_used = vec![false; h.list_indices_count as usize];
    let mut fidx_used = vec![false; h.field_indices_count as usize];
    let mark = |map: &mut Vec<bool>, at: u32, len: usize| {
        let at = at as usize;
        if let Some(e) = at.checked_add(len)
            && e <= map.len()
        {
            map[at..e].fill(true);
        }
    };

    let data_at = h.field_data_offset as usize;
    let data_len = h.field_data_count as usize;
    let data_word = |d: u32, plus: usize| -> Option<u32> {
        let p = data_at.checked_add(d as usize)?.checked_add(plus)?;
        (p + 4 <= data_at + data_len).then(|| w(p))
    };
    let mut locstring_totals_wrong = 0;
    let mut narrow = 0;
    for i in 0..h.field_count as usize {
        let at = h.field_offset as usize + i * FIELD_LEN;
        let (ty, d) = (w(at), w(at + 8));
        let Some(ty) = FieldType::from_id(ty) else {
            continue;
        };
        match ty {
            FieldType::Byte | FieldType::Char if d > 0xFF => narrow += 1,
            FieldType::Word | FieldType::Short if d > 0xFFFF => narrow += 1,
            FieldType::Dword64 | FieldType::Int64 | FieldType::Double | FieldType::StrRef => {
                mark(&mut data_used, d, 8);
            }
            FieldType::Orientation => mark(&mut data_used, d, 16),
            FieldType::Vector => mark(&mut data_used, d, 12),
            FieldType::String | FieldType::Void => {
                if let Some(n) = data_word(d, 0) {
                    mark(&mut data_used, d, 4 + n as usize);
                }
            }
            FieldType::ResRef => {
                let p = data_at + d as usize;
                if p < data_at + data_len {
                    mark(&mut data_used, d, 1 + bytes[p] as usize);
                }
            }
            FieldType::LocString => {
                if let (Some(total), Some(count)) = (data_word(d, 0), data_word(d, 8)) {
                    mark(&mut data_used, d, 4 + total as usize);
                    let mut p = 12usize;
                    let mut ok = true;
                    for _ in 0..count {
                        match data_word(d, p + 4) {
                            Some(len) => p += 8 + len as usize,
                            None => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if !ok || p != 4 + total as usize {
                        locstring_totals_wrong += 1;
                    }
                }
            }
            FieldType::List => {
                let p = h.list_indices_offset as usize + d as usize;
                if p + 4 <= h.list_indices_offset as usize + h.list_indices_count as usize {
                    let n = w(p) as usize;
                    mark(&mut list_used, d, 4 + 4 * n);
                }
            }
            _ => {}
        }
    }
    for i in 0..h.struct_count as usize {
        let at = h.struct_offset as usize + i * STRUCT_LEN;
        let (d, n) = (w(at + 4), w(at + 8));
        if n >= 2 {
            mark(&mut fidx_used, d, 4 * n as usize);
        }
    }
    let count = |m: &[bool]| m.iter().filter(|&&b| b).count() as u32;
    Ok(Survey {
        file_type: String::from_utf8_lossy(&h.file_type).trim_end().to_string(),
        version: String::from_utf8_lossy(&h.version).into_owned(),
        sections_adjacent,
        struct_count: h.struct_count,
        field_count: h.field_count,
        label_count: h.label_count,
        data_bytes: h.field_data_count,
        data_used: count(&data_used),
        field_indices_bytes: h.field_indices_count,
        field_indices_used: count(&fidx_used),
        list_indices_bytes: h.list_indices_count,
        list_indices_used: count(&list_used),
        locstring_totals_wrong,
        narrow_fields_with_wide_values: narrow,
    })
}

impl fmt::Display for Survey {
    /// One line of clauses separated by `; `, each a fact a glob can pin:
    /// `gff: UTC V3.2; sections adjacent; data: all used; field indices:
    /// 256 stale bytes; list indices: all used; locstring totals: all
    /// right; narrow fields: clean`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gff: {} {}; sections ", self.file_type, self.version)?;
        f.write_str(if self.sections_adjacent {
            "adjacent"
        } else {
            "apart"
        })?;
        let unused = |f: &mut fmt::Formatter<'_>, name: &str, bytes: u32, used: u32| {
            write!(f, "; {name}: ")?;
            match bytes.checked_sub(used) {
                Some(0) => f.write_str("all used"),
                Some(n) => write!(f, "{n} unused bytes"),
                None => f.write_str("overlapping"),
            }
        };
        unused(f, "data", self.data_bytes, self.data_used)?;
        unused(
            f,
            "field indices",
            self.field_indices_bytes,
            self.field_indices_used,
        )?;
        unused(
            f,
            "list indices",
            self.list_indices_bytes,
            self.list_indices_used,
        )?;
        f.write_str("; locstring totals: ")?;
        match self.locstring_totals_wrong {
            0 => f.write_str("all right")?,
            n => write!(f, "{n} wrong")?,
        }
        f.write_str("; narrow fields: ")?;
        match self.narrow_fields_with_wide_values {
            0 => f.write_str("clean"),
            n => write!(f, "{n} with wide values"),
        }
    }
}
