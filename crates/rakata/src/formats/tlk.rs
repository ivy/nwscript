//! The talk table as a format: `dialog.tlk` round-trips through
//! `rakata-tlk`'s reader and writer.

use rakata_tlk::Tlk as Table;

use crate::conformance::{Format, Problem};

/// `TLK V3.0`.
pub struct Tlk;

fn problem(e: rakata_tlk::Error) -> Problem {
    let code = e.code();
    let text = e.to_string();
    // The error renders as `CODE: message`; the code travels apart.
    let message = text
        .strip_prefix(code)
        .and_then(|t| t.strip_prefix(": "))
        .unwrap_or(&text)
        .to_string();
    Problem::new(code, message)
}

impl Format for Tlk {
    type Model<'a> = Table;

    fn name(&self) -> &'static str {
        "tlk"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["tlk"]
    }

    fn read(&self, bytes: &[u8]) -> Result<Table, Problem> {
        Table::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &Table) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    fn semantic_eq(&self, a: &Table, b: &Table) -> bool {
        a == b
    }

    /// The header and table facts another writer may set differently from
    /// BioWare: where the string data begins, the distinct flag values, where
    /// an entry without text points, and whether the texts lie back to back
    /// in strref order. The layout is that of `docs/formats/tlk.md`.
    fn layout(&self, bytes: &[u8], model: &Table) -> Option<String> {
        let word = |at: usize| -> u32 {
            bytes
                .get(at..at + 4)
                .map_or(0, |b| u32::from_le_bytes(b.try_into().expect("four bytes")))
        };
        let strings_at = word(16);
        let mut flags: Vec<u32> = model.entries.iter().map(|e| e.flags).collect();
        flags.sort_unstable();
        flags.dedup();
        let flags = flags
            .iter()
            .map(|f| format!("{f:#x}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut running = 0u32;
        let mut contiguous = true;
        let mut textless_at_zero = true;
        for i in 0..model.entries.len() {
            let at = 20 + i * 40;
            let flags = word(at);
            let offset = word(at + 28);
            let size = word(at + 32);
            if flags & rakata_tlk::Entry::TEXT_PRESENT == 0 {
                textless_at_zero &= offset == 0 && size == 0;
            } else {
                contiguous &= offset == running;
                running = running.wrapping_add(size);
            }
        }
        Some(format!(
            "tlk: strings at {strings_at}, flags {{{flags}}}, text-less entries {}, texts {}",
            if textless_at_zero {
                "at offset 0"
            } else {
                "at the running offset"
            },
            if contiguous {
                "contiguous in strref order"
            } else {
                "not contiguous"
            },
        ))
    }
}
