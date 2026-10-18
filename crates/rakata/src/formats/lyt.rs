//! Room layouts and room visibility as formats: a `.lyt` and a `.vis`
//! round-trip through `rakata-lyt`'s readers and writers, every line that
//! departs from the written form kept as read.

use crate::conformance::{Format, Problem};

/// The room layout, `.lyt`.
pub struct Lyt;

/// Room visibility, `.vis`.
pub struct Vis;

fn problem(e: rakata_lyt::Error) -> Problem {
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

impl Format for Lyt {
    type Model<'a> = rakata_lyt::Lyt;

    fn name(&self) -> &'static str {
        "lyt"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["lyt"]
    }

    fn read(&self, bytes: &[u8]) -> Result<rakata_lyt::Lyt, Problem> {
        rakata_lyt::Lyt::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &rakata_lyt::Lyt) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    /// The same rooms, tracks, obstacles and door hooks, floats bit for
    /// bit.
    fn semantic_eq(&self, a: &rakata_lyt::Lyt, b: &rakata_lyt::Lyt) -> bool {
        a.semantic_eq(b)
    }

    fn layout(&self, _bytes: &[u8], model: &rakata_lyt::Lyt) -> Option<String> {
        Some(format!(
            "lyt: {} lines departing, {} trailing bytes",
            model.departures(),
            model.trailer().len()
        ))
    }
}

impl Format for Vis {
    type Model<'a> = rakata_lyt::Vis;

    fn name(&self) -> &'static str {
        "vis"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["vis"]
    }

    fn read(&self, bytes: &[u8]) -> Result<rakata_lyt::Vis, Problem> {
        rakata_lyt::Vis::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &rakata_lyt::Vis) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    /// The same rooms seeing the same rooms, in order.
    fn semantic_eq(&self, a: &rakata_lyt::Vis, b: &rakata_lyt::Vis) -> bool {
        a.semantic_eq(b)
    }

    fn layout(&self, _bytes: &[u8], model: &rakata_lyt::Vis) -> Option<String> {
        Some(format!("vis: {} lines departing", model.departures()))
    }
}
