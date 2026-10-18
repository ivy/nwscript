//! Walkmeshes as a format: an area room's `.wok`, a placeable's `.pwk` and
//! a door's `.dwk`, which share one layout, round-trip through
//! `rakata-walkmesh`'s reader and writer, the AABB tree, adjacency and
//! perimeter as stored and the bytes after the last array kept as read.

use rakata_walkmesh::Walkmesh as Bwm;

use crate::conformance::{Format, Problem};

/// The walkmesh, `.wok`, `.pwk` and `.dwk`.
pub struct Walkmesh;

fn problem(e: rakata_walkmesh::Error) -> Problem {
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

/// The header's nine array offsets, at 76 to 132, by name.
const OFFSETS: [(&str, usize); 9] = [
    ("vertices", 76),
    ("faces", 84),
    ("materials", 88),
    ("normals", 92),
    ("planes", 96),
    ("aabb", 104),
    ("adjacency", 116),
    ("edges", 124),
    ("perimeters", 132),
];

fn word(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

impl Format for Walkmesh {
    type Model<'a> = Bwm<'a>;

    fn name(&self) -> &'static str {
        "walkmesh"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["wok", "pwk", "dwk"]
    }

    fn read<'a>(&self, bytes: &'a [u8]) -> Result<Bwm<'a>, Problem> {
        Bwm::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &Bwm<'_>) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    /// The same header values, vertices, faces, tree, adjacency and
    /// perimeter, floats bit for bit, and the same trailing bytes.
    fn semantic_eq(&self, a: &Bwm<'_>, b: &Bwm<'_>) -> bool {
        a.semantic_eq(b)
    }

    /// The arrays whose offset departs from the one the writer gives them,
    /// and the bytes after the last array.
    fn layout(&self, bytes: &[u8], model: &Bwm<'_>) -> Option<String> {
        let written = model.to_bytes().ok()?;
        let departing: Vec<String> = OFFSETS
            .iter()
            .filter_map(|&(name, at)| {
                let (read, ours) = (word(bytes, at)?, word(&written, at)?);
                (read != ours).then(|| format!("{name} at {read} for {ours}"))
            })
            .collect();
        Some(format!(
            "walkmesh: type {}, offsets {}, {} trailing bytes",
            model.kind.to_raw(),
            if departing.is_empty() {
                "as written".to_string()
            } else {
                departing.join(", ")
            },
            model.trailer.len()
        ))
    }
}
