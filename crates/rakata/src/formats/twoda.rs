//! The tables as a format: `.2da` round-trips through `rakata-2da`'s reader
//! and writer. The layout line carries the header's size field against the
//! strings' real length, whether strings are shared, and their order, which
//! is how the known variants tell one writer's tables from another's.

use rakata_2da::TwoDa as Table;

use crate::conformance::{Format, Problem};

/// 2DA V2.b.
pub struct TwoDa;

fn problem(e: rakata_2da::Error) -> Problem {
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

impl Format for TwoDa {
    type Model<'a> = Table;

    fn name(&self) -> &'static str {
        "2da"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["2da"]
    }

    fn read(&self, bytes: &[u8]) -> Result<Table, Problem> {
        Table::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &Table) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    fn semantic_eq(&self, a: &Table, b: &Table) -> bool {
        a.semantic_eq(b)
    }

    fn layout(&self, bytes: &[u8], _model: &Table) -> Option<String> {
        rakata_2da::inspect(bytes).ok().map(|l| l.to_string())
    }
}
