//! Modes and lint levels as one value, for every front end: the CLI, the
//! nwnnsscomp shim and the bindings.

use nwscript_diag::{Diagnostic, Levels, SourceMap, codes};
use nwscript_sema::Analysis;

/// What a check accepts. Strict turns lints into errors; the compat modes
/// accept what another compiler accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Strict,
    Nwnnsscomp,
    Bioware,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "strict" => Some(Mode::Strict),
            "nwnnsscomp" => Some(Mode::Nwnnsscomp),
            "bioware" => Some(Mode::Bioware),
            _ => None,
        }
    }

    /// The levels this mode starts from; `--allow`/`--deny` are added on top.
    pub fn levels(self) -> Levels {
        let mut l = Levels::default();
        l.strict = self == Mode::Strict;
        if self != Mode::Strict {
            l.accepted = codes::NWNNSSCOMP_ACCEPTED.iter().map(|c| c.id).collect();
        }
        if self == Mode::Bioware {
            l.accepted
                .extend(codes::BIOWARE_ACCEPTED.iter().map(|c| c.id));
        }
        l
    }
}

/// A file's diagnostics with its lints added, comment directives applied,
/// and `levels` applied: what decides whether it compiles.
pub fn finish(
    analysis: Option<&Analysis>,
    mut diags: Vec<Diagnostic>,
    map: &SourceMap,
    levels: &Levels,
) -> Vec<Diagnostic> {
    if let Some(a) = analysis {
        diags.extend(nwscript_sema::lint::lint(&a.program));
    }
    nwscript_sema::lint::apply_directives(diags, map)
        .into_iter()
        .filter_map(|d| levels.apply(d))
        .collect()
}
