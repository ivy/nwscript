//! The archives as one format: a file's first eight bytes name its kind,
//! and each kind round-trips through `rakata-archive`'s reader and writer.
//! Members are handed to the suite under their own names, or by position
//! for a BIF, which holds none.

use rakata_archive::{Bif, Erf, Key, Rim};

use crate::conformance::{Format, Nested, Problem};

/// KEY, BIF, ERF (MOD, SAV, HAK) and RIM.
pub struct Archive;

/// An archive of any kind, as read.
pub enum Model<'a> {
    Key(Key),
    Bif(Bif<'a>),
    Erf(Erf<'a>),
    Rim(Rim<'a>),
}

fn problem(e: rakata_archive::Error) -> Problem {
    let code = e.code();
    let text = e.to_string();
    // The archive error renders as `CODE: message`; the code travels apart.
    let message = text
        .strip_prefix(code)
        .and_then(|t| t.strip_prefix(": "))
        .unwrap_or(&text)
        .to_string();
    Problem::new(code, message)
}

impl Format for Archive {
    type Model<'a> = Model<'a>;

    fn name(&self) -> &'static str {
        "archive"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["key", "bif", "erf", "mod", "sav", "hak", "rim"]
    }

    fn read<'a>(&self, bytes: &'a [u8]) -> Result<Model<'a>, Problem> {
        let kind = bytes.get(..4).unwrap_or(bytes);
        match kind {
            b"KEY " => Key::parse(bytes).map(Model::Key),
            b"BIFF" => Bif::parse(bytes).map(Model::Bif),
            b"RIM " => Rim::parse(bytes).map(Model::Rim),
            _ => Erf::parse(bytes).map(Model::Erf),
        }
        .map_err(problem)
    }

    fn write(&self, model: &Model<'_>) -> Result<Vec<u8>, Problem> {
        match model {
            Model::Key(k) => k.to_bytes(),
            Model::Bif(b) => b.to_bytes(),
            Model::Erf(e) => e.to_bytes(),
            Model::Rim(r) => r.to_bytes(),
        }
        .map_err(problem)
    }

    fn semantic_eq(&self, a: &Model<'_>, b: &Model<'_>) -> bool {
        match (a, b) {
            (Model::Key(a), Model::Key(b)) => a == b,
            (Model::Bif(a), Model::Bif(b)) => a == b,
            (Model::Erf(a), Model::Erf(b)) => a == b,
            (Model::Rim(a), Model::Rim(b)) => a == b,
            _ => false,
        }
    }

    /// The header words another writer may set differently from BioWare:
    /// for an ERF the three table offsets, the build date and the
    /// description strref; for a RIM the flag and the table offset; for a
    /// KEY and a BIF their table offsets. The layouts are those of
    /// `docs/formats/`.
    fn layout(&self, bytes: &[u8], model: &Model<'_>) -> Option<String> {
        let word = |at: usize| -> u32 {
            bytes
                .get(at..at + 4)
                .map_or(0, |b| u32::from_le_bytes(b.try_into().expect("four bytes")))
        };
        Some(match model {
            Model::Erf(e) => format!(
                "erf: strings at {}, keys at {}, resources at {}, build {}/{}, strref {:#x}",
                word(0x14),
                word(0x18),
                word(0x1C),
                e.build_year,
                e.build_day,
                e.description_strref
            ),
            Model::Rim(r) => format!("rim: flag {}, table at {}", r.flag, word(0x10)),
            Model::Key(k) => format!(
                "key: files at {}, keys at {}, build {}/{}",
                word(0x10),
                word(0x14),
                k.build_year,
                k.build_day
            ),
            Model::Bif(_) => format!("bif: table at {}", word(0x10)),
        })
    }

    fn members<'m>(&self, model: &'m Model<'_>) -> Vec<Nested<'m>> {
        match model {
            Model::Key(_) => Vec::new(),
            Model::Bif(b) => b
                .members()
                .iter()
                .enumerate()
                .map(|(i, m)| Nested {
                    name: format!("#{i}.{}", m.ty),
                    extension: m.ty.extension().map(str::to_string),
                    bytes: m.data(),
                })
                .collect(),
            Model::Erf(e) => e
                .members()
                .iter()
                .map(|m| Nested {
                    name: m.id.to_string(),
                    extension: m.id.ty.extension().map(str::to_string),
                    bytes: m.data(),
                })
                .collect(),
            Model::Rim(r) => r
                .members()
                .iter()
                .map(|m| Nested {
                    name: m.id.to_string(),
                    extension: m.id.ty.extension().map(str::to_string),
                    bytes: m.data(),
                })
                .collect(),
        }
    }
}
