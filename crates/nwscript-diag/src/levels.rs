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

/// Per-code overrides and the `--deny warnings` switch.
#[derive(Clone, Debug, Default)]
pub struct Levels {
    overrides: HashMap<&'static str, Level>,
    pub deny_warnings: bool,
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
        let lint = d.code.severity == Severity::Warning;
        match (lint, self.overrides.get(d.code.id)) {
            (true, Some(Level::Allow)) => return None,
            (true, Some(Level::Warn)) => d.severity = Severity::Warning,
            (_, Some(Level::Deny)) => d.severity = Severity::Error,
            _ => {}
        }
        if self.deny_warnings && d.severity == Severity::Warning {
            d.severity = Severity::Error;
        }
        Some(d)
    }
}
