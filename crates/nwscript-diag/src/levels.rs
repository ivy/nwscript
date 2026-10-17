//! Lint levels: the `--allow`, `--deny` and `--deny warnings` controls,
//! applied to diagnostics after they are produced. Code generation never
//! sees them; they decide only what is reported and what fails a run.

use crate::{Diagnostic, Severity};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Allow,
    Warn,
    Deny,
}

/// Per-code overrides, the mode, and the `--deny warnings` switch.
#[derive(Clone, Debug, Default)]
pub struct Levels {
    overrides: HashMap<&'static str, Level>,
    pub deny_warnings: bool,
    /// Strict mode: a lint is an error unless it is one of
    /// [`crate::codes::STRICT_WARNINGS`] or an override says otherwise.
    pub strict: bool,
    /// Error codes demoted to warnings because the mode accepts them
    /// (`--compat bioware` and [`crate::codes::BIOWARE_ACCEPTED`]).
    pub accepted: Vec<&'static str>,
}

impl Levels {
    pub fn set(&mut self, code: &'static str, level: Level) {
        self.overrides.insert(code, level);
    }

    pub fn get(&self, code: &str) -> Option<Level> {
        self.overrides.get(code).copied()
    }

    /// Apply the levels: `None` when the diagnostic is allowed away. Errors
    /// that are not lints cannot be allowed or demoted.
    pub fn apply(&self, mut d: Diagnostic) -> Option<Diagnostic> {
        let lint = d.code.severity == Severity::Warning && d.code.id.starts_with('W');
        match (lint, self.overrides.get(d.code.id)) {
            (true, Some(Level::Allow)) => return None,
            (true, Some(Level::Warn)) => d.severity = Severity::Warning,
            (_, Some(Level::Deny)) => d.severity = Severity::Error,
            (true, None)
                if self.strict
                    && !crate::codes::STRICT_WARNINGS
                        .iter()
                        .any(|c| c.id == d.code.id) =>
            {
                d.severity = Severity::Error
            }
            (false, None) if self.accepted.contains(&d.code.id) => d.severity = Severity::Warning,
            _ => {}
        }
        if self.deny_warnings && d.severity == Severity::Warning {
            d.severity = Severity::Error;
        }
        Some(d)
    }
}
