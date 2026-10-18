//! Saved games, through [`rakata_save`], the names of the creatures' local
//! variables read from the corpus of the user's own game, and the globals an
//! install declares.

pub use rakata_save::*;

use std::path::{Path, PathBuf};

use crate::diag::Diagnostic;
use crate::inspect::{self, Loaded, Lookup};

/// The names of local booleans and numbers that the include scripts of the
/// corpus at `corpus` define (`nss/k_inc_*.nss`). When there are none to
/// read, the `nss/` directory looked in is the error, for the
/// [`codes::NO_LOCAL_NAMES`] warning.
pub fn local_names(corpus: &Path) -> std::result::Result<LocalNames, PathBuf> {
    let dir = corpus.join("nss");
    match LocalNames::from_include_dir(&dir) {
        Ok(names) if !names.is_empty() => Ok(names),
        _ => Err(dir),
    }
}

/// The table that declares the campaign's globals.
pub const GLOBALCAT: &str = "globalcat.2da";

/// The globals the install declares: the rows of the `globalcat.2da` it
/// loads, by their `name` and `type` columns; a row whose type names no
/// kind is left out. The table is returned as read too, for the record of
/// inputs.
pub fn declarations(
    lookup: &Lookup<'_>,
) -> std::result::Result<(Declarations, Loaded), Diagnostic> {
    let loaded = inspect::load(GLOBALCAT, lookup)?;
    let table =
        crate::twoda::TwoDa::parse(&loaded.bytes).map_err(|e| inspect::diagnostic(e.code(), &e))?;
    let mut out = Declarations::new();
    for r in 0..table.rows.len() {
        let cell = |column| {
            table
                .get(r, column)
                .map_err(|e| inspect::diagnostic(e.code(), &e))
        };
        let (name, kind) = (cell("name")?, cell("type")?);
        if let Some(kind) = GlobalKind::parse(kind)
            && !name.is_empty()
        {
            out.add(name, kind);
        }
    }
    Ok((out, loaded))
}
