//! Lip-sync files as a format: a `.lip` round-trips through `rakata-lip`'s
//! reader and writer, floats bit for bit.

use crate::conformance::{Format, Problem};

/// LIP V1.0.
pub struct Lip;

fn problem(e: rakata_lip::Error) -> Problem {
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

impl Format for Lip {
    type Model<'a> = rakata_lip::Lip;

    fn name(&self) -> &'static str {
        "lip"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["lip"]
    }

    fn read(&self, bytes: &[u8]) -> Result<rakata_lip::Lip, Problem> {
        rakata_lip::Lip::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &rakata_lip::Lip) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    /// The same duration and keyframes bit for bit: a float that changed
    /// is data lost, not a layout chosen.
    fn semantic_eq(&self, a: &rakata_lip::Lip, b: &rakata_lip::Lip) -> bool {
        a.bits_eq(b)
    }

    /// The facts another writer could choose differently: the duration
    /// against the last keyframe's time, the key count, and whether the
    /// keyframes are in time order. The layout has no offsets to vary.
    fn layout(&self, _bytes: &[u8], model: &rakata_lip::Lip) -> Option<String> {
        let last = model.keys.last().map(|k| k.time);
        let end = match last {
            None => "no keys",
            Some(t) if t == model.length => "last key at the duration",
            Some(t) if t < model.length => "last key before the duration",
            Some(_) => "last key after the duration",
        };
        Some(format!(
            "lip: {} keys, {end}, {}",
            model.keys.len(),
            if model.is_sorted() {
                "in time order"
            } else {
                "out of time order"
            }
        ))
    }
}
