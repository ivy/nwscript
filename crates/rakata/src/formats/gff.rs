//! GFF as one format: every extension the engine stores in GFF V3.2, and
//! `.res`, the generic member of a save. The file-type field inside the
//! header says what a `.res` holds; the reader takes any type.

use crate::conformance::{Format, Problem};

/// GFF V3.2 under every extension the engine uses for it.
pub struct Gff;

fn problem(e: rakata_gff::Error) -> Problem {
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

impl Format for Gff {
    type Model<'a> = rakata_gff::Gff;

    fn name(&self) -> &'static str {
        "gff"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[
            "gff", "res", "are", "ifo", "git", "gic", "utc", "utd", "ute", "uti", "utp", "uts",
            "utm", "utt", "utw", "utg", "dlg", "jrl", "fac", "itp", "ptm", "ptt", "bic", "gui",
            "pth", "btc", "bti", "btd", "bte", "btp", "bts", "btt", "btm", "btg",
        ]
    }

    fn read(&self, bytes: &[u8]) -> Result<rakata_gff::Gff, Problem> {
        rakata_gff::Gff::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &rakata_gff::Gff) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    fn semantic_eq(&self, a: &rakata_gff::Gff, b: &rakata_gff::Gff) -> bool {
        a.semantic_eq(b)
    }

    /// The layout facts another writer may set differently from BioWare,
    /// as [`rakata_gff::survey`] reads them from the bytes: the file type,
    /// whether the sections are adjacent, unused bytes in the data block
    /// and the two index arrays, locstring totals that disagree with their
    /// substrings, and narrow fields carrying wide values.
    fn layout(&self, bytes: &[u8], _model: &rakata_gff::Gff) -> Option<String> {
        rakata_gff::survey(bytes).ok().map(|s| s.to_string())
    }
}
