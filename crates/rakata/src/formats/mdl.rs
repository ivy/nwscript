//! Binary models as a pair format: a model's MDL and its MDX are read
//! together into `rakata-mdl`'s [`ModelFile`], written back together, and
//! each scored on its own bytes.

use rakata_mdl::ModelFile;

use crate::conformance::{PairFormat, Problem};

/// The binary model: `.mdl` with its `.mdx`.
pub struct Mdl;

fn problem(e: rakata_mdl::Error) -> Problem {
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

impl PairFormat for Mdl {
    type Model<'a> = ModelFile;

    fn name(&self) -> &'static str {
        "mdl"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mdl", "mdx"]
    }

    fn read(&self, mdl: &[u8], mdx: &[u8]) -> Result<ModelFile, Problem> {
        ModelFile::parse(mdl, mdx).map_err(problem)
    }

    fn write(&self, model: &ModelFile) -> Result<[Vec<u8>; 2], Problem> {
        let (mdl, mdx) = model.to_bytes().map_err(problem)?;
        Ok([mdl, mdx])
    }

    /// Every structure and field the same, the MDX's padding aside.
    fn semantic_eq(&self, a: &ModelFile, b: &ModelFile) -> bool {
        a.semantic_eq(b)
    }

    fn layout(&self, halves: [&[u8]; 2], _model: &ModelFile) -> [Option<String>; 2] {
        match rakata_mdl::layout(halves[0], halves[1]) {
            Ok(l) => [Some(l.mdl_line()), Some(l.mdx_line())],
            Err(_) => [None, None],
        }
    }
}
