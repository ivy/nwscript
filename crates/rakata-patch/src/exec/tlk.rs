//! `[TLKList]`: entries of the installer's TLK appended to `dialog.tlk`, the
//! strref each receives stored under its `StrRef#` token, and entries of a
//! replacement TLK written over existing ones.

use super::{Failure, Tokens};
use crate::changes::{TlkAppend, TlkReplaceEntry};
use crate::issue::codes;
use rakata_res::StrRef;
use rakata_tlk::Tlk;

/// Append the entries the tokens name, in the order listed. As TSLPatcher
/// does (`docs/research/2026-10-17-tslpatcher-semantics.md`), an entry equal
/// in every field to one dialog.tlk already holds is not appended and its
/// token takes the first such index, and two tokens naming the same entry
/// share one index.
pub fn append(
    dialog: &mut Tlk,
    source: &Tlk,
    source_name: &str,
    appends: &[TlkAppend],
    tokens: &mut Tokens,
) -> Result<String, Failure> {
    let mut added = Vec::with_capacity(appends.len());
    for a in appends {
        let entry = source
            .entries
            .get(a.index as usize)
            .ok_or_else(|| {
                Failure::new(
                    codes::CANNOT_APPLY,
                    format!(
                        "StrRef{} names entry {} of {source_name}, which has {} entries",
                        a.token,
                        a.index,
                        source.entries.len()
                    ),
                )
            })?
            .clone();
        added.push((a.token, entry));
    }
    let first = dialog.entries.len();
    let mut reused = 0;
    for (token, entry) in added {
        let s = match dialog.entries.iter().position(|e| same(e, &entry)) {
            Some(i) => {
                reused += 1;
                i as u32
            }
            None => dialog.append(entry).raw(),
        };
        tokens.strref.insert(token, s);
    }
    let appended = dialog.entries.len() - first;
    Ok(if reused == 0 {
        format!(
            "appended {appended} entries at {first}..{}",
            dialog.entries.len()
        )
    } else {
        format!(
            "appended {appended} entries at {first}..{}; {reused} tokens took an equal entry already there",
            dialog.entries.len()
        )
    })
}

/// Equal in every field, the sound length by its bits.
fn same(a: &rakata_tlk::Entry, b: &rakata_tlk::Entry) -> bool {
    a.text == b.text
        && a.sound == b.sound
        && a.sound_length.to_bits() == b.sound_length.to_bits()
        && a.flags == b.flags
        && a.volume_variance == b.volume_variance
        && a.pitch_variance == b.pitch_variance
}

/// Write the replacement file's entries over the dialog.tlk entries named.
pub fn replace(
    dialog: &mut Tlk,
    source: &Tlk,
    source_name: &str,
    entries: &[TlkReplaceEntry],
) -> Result<String, Failure> {
    let mut writes = Vec::with_capacity(entries.len());
    for e in entries {
        let entry = source.entries.get(e.index as usize).ok_or_else(|| {
            Failure::new(
                codes::CANNOT_APPLY,
                format!(
                    "line {}: entry {} of {source_name} does not exist; it has {}",
                    e.line,
                    e.index,
                    source.entries.len()
                ),
            )
        })?;
        let slot = StrRef::new(e.strref).index().unwrap_or(usize::MAX);
        dialog.entries.get(slot).ok_or_else(|| {
            Failure::new(
                codes::CANNOT_APPLY,
                format!(
                    "line {}: dialog.tlk has no entry {} to replace",
                    e.line, e.strref
                ),
            )
        })?;
        writes.push((slot, entry.clone()));
    }
    for (slot, entry) in writes {
        dialog.entries[slot] = entry;
    }
    Ok(format!("replaced {} entries", entries.len()))
}
