//! The index of a game tree ([`rakata_index`]) over an install: the files
//! the round-trip suite's [`walk`](crate::conformance::walk) names, and the
//! engine's routines from the tree's own `nwscript.nss`, so every script's
//! calls are read from its bytecode with the names that tree declares.

use std::path::{Path, PathBuf};

use nwscript::diag::{Diagnostic, SourceMap};
use nwscript::sema::Type;
use rakata_game::{Install, Profile};

pub use rakata_index::{Answer, Direction, Hit, Location, RefKind, RefsQuery, refs};
pub use rakata_index::{
    ArgType, BuildReport, COUNTED, CoverageRow, Error, Param, Routine, Rows, SCHEMA_VERSION,
    Status, Timings, TreeCoverage, Warning, codes, coverage, digest, open, parse_changed, query,
    query_script,
};

use crate::codes as registry;

/// What [`build`] is asked for.
#[derive(Debug, Clone)]
pub struct Build<'a> {
    /// The tree: the folder holding `chitin.key`.
    pub game: &'a Path,
    pub db: &'a Path,
    /// The tree's label.
    pub label: &'a str,
    pub parent: Option<&'a str>,
    /// A file of changed paths, as `changed.txt` writes them.
    pub changed: Option<&'a Path>,
    /// Threads; 0 for one per CPU.
    pub threads: usize,
}

/// The database `rakata index` uses when no `--db` is given:
/// `$XDG_CACHE_HOME/rakata/index.sqlite`, or under `~/.cache`.
pub fn default_db() -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(cache.join("rakata").join("index.sqlite"))
}

/// A diagnostic under a registered code.
pub fn diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    let code = registry::lookup(code)
        .unwrap_or_else(|| panic!("diagnostic code {code} is not registered"));
    Diagnostic::new(code, message)
}

/// An index error as a diagnostic.
pub fn error(e: &Error) -> Diagnostic {
    diagnostic(e.code(), e.message())
}

fn arg_type(t: &Type) -> ArgType {
    match t {
        Type::Int => ArgType::Int,
        Type::Float => ArgType::Float,
        Type::String => ArgType::String,
        Type::Object => ArgType::Object,
        Type::Vector => ArgType::Vector,
        Type::Action => ArgType::Action,
        Type::Void => ArgType::Void,
        _ => ArgType::Other,
    }
}

/// The engine's routines as the install's `nwscript.nss` declares them;
/// empty when it holds none.
pub fn routines(game: &Path) -> Vec<Routine> {
    let Ok(mut install) = Install::open(game, Profile::K1) else {
        return Vec::new();
    };
    let mut map = SourceMap::new();
    match nwscript::engine::load_table(&mut install, &mut map) {
        Ok(Some((table, _))) => routines_of(&table),
        _ => Vec::new(),
    }
}

/// The engine's routines as `table` declares them, by number.
pub fn routines_of(table: &nwscript::sema::EngineTable) -> Vec<Routine> {
    table
        .functions
        .iter()
        .map(|f| Routine {
            name: f.name.clone(),
            params: f
                .params
                .iter()
                .map(|p| Param {
                    name: p.name.clone(),
                    ty: arg_type(&p.ty),
                })
                .collect(),
            ret: arg_type(&f.ret),
        })
        .collect()
}

/// Build the index of the tree at `o.game` into `o.db`.
pub fn build(o: &Build) -> Result<BuildReport, Diagnostic> {
    let files = crate::conformance::walk(o.game)?;
    let changed = match o.changed {
        Some(p) => {
            let text = std::fs::read_to_string(p).map_err(|source| {
                error(&Error::Changed {
                    path: p.to_path_buf(),
                    source,
                })
            })?;
            Some(parse_changed(&text))
        }
        None => None,
    };
    let routines = routines(o.game);
    let mut report = rakata_index::build(&rakata_index::BuildOptions {
        root: o.game,
        files: &files,
        db: o.db,
        label: o.label,
        parent: o.parent,
        changed: changed.as_deref(),
        routines: &routines,
        threads: o.threads,
    })
    .map_err(|e| error(&e))?;
    if routines.is_empty() {
        report.warnings.push(Warning {
            code: codes::NO_ENGINE_TABLE,
            message: format!(
                "`{}` holds no nwscript.nss: calls name their routines by number",
                o.game.display()
            ),
        });
    }
    Ok(report)
}
