//! Diagnostics as data: codes, spans, explanations, and their miette and JSON renderings.
//!
//! A [`Diagnostic`] is a value: a registered [`Code`], a [`Severity`], a
//! message, labelled [`Span`]s into files held by a [`SourceMap`], and
//! optional help. Rendering is separate: [`render`] draws it for a person
//! through miette, [`json`] shapes it for a program, and the `lsp` feature
//! converts it for a language server. Nothing in the toolchain builds a
//! diagnostic from rendered text.

pub mod codes;
pub mod json;
pub mod levels;
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod render;
mod source;

pub use codes::Code;
pub use levels::{Level, Levels};
pub use source::{FileId, LineCol, SourceFile, SourceMap, Span};

use serde::Serialize;

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Help,
    Note,
    Warning,
    Error,
}

/// A span with an optional message; the primary label is where the problem is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub message: Option<String>,
    pub primary: bool,
}

/// One finding: what, where, and what to do about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static Code,
    pub severity: Severity,
    pub message: String,
    pub labels: Vec<Label>,
    pub help: Option<String>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// A diagnostic at the code's default severity.
    pub fn new(code: &'static Code, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: code.severity,
            message: message.into(),
            labels: Vec::new(),
            help: None,
            notes: Vec::new(),
        }
    }

    /// Add the primary label: where the problem is.
    pub fn primary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: Some(message.into()).filter(|m: &String| !m.is_empty()),
            primary: true,
        });
        self
    }

    /// Add a secondary label: context that explains the primary one.
    pub fn secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: Some(message.into()).filter(|m: &String| !m.is_empty()),
            primary: false,
        });
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// The span of the primary label, if any.
    pub fn primary_span(&self) -> Option<Span> {
        self.labels.iter().find(|l| l.primary).map(|l| l.span)
    }
}

/// Diagnostics collected while doing one piece of work.
pub type Diagnostics = Vec<Diagnostic>;

/// True when any diagnostic is an error.
pub fn has_errors(ds: &[Diagnostic]) -> bool {
    ds.iter().any(Diagnostic::is_error)
}
