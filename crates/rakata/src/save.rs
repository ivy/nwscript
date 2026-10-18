//! Saved games, through [`rakata_save`], and the names of the creatures'
//! local variables read from the corpus of the user's own game.

pub use rakata_save::*;

use std::path::{Path, PathBuf};

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
