//! What the reader found wrong or odd, each under a registered code. The
//! codes are strings here, as in `rakata-res`: the `rakata` façade registers
//! them with their explanations and turns an [`Issue`] into a diagnostic
//! with a span into the file.

use serde::Serialize;

/// Diagnostic codes for the installer's inputs: the `RE02xx` and `RW02xx`
/// ranges of the `rakata` registry.
pub mod codes {
    /// A line that is neither a section header, `Key=Value`, a comment nor blank.
    pub const MALFORMED_LINE: &str = "RE0201";
    /// A `Key=Value` line before the first `[Section]` header.
    pub const ENTRY_BEFORE_SECTION: &str = "RE0202";
    /// A value in no form its key takes (a row index that is not a number, a field type that names no type).
    pub const BAD_VALUE: &str = "RE0203";
    /// A `namespaces.ini` whose `[Namespaces]` list is missing or empty.
    pub const NO_NAMESPACES: &str = "RE0204";
    /// A list entry names a section the file does not have; TSLPatcher skips the entry.
    pub const MISSING_SECTION: &str = "RW0201";
    /// A key the section's vocabulary does not have; kept on the model, not applied.
    pub const UNKNOWN_KEY: &str = "RW0202";
    /// A section or key written twice; the first is what TSLPatcher reads.
    pub const DUPLICATE: &str = "RW0203";
    /// A memory token (`2DAMEMORY#`, `StrRef#`) read by an operation before any operation sets it.
    pub const TOKEN_UNSET: &str = "RW0204";
}

/// Where an issue is: a file the reader was given, by index, and a line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// The registered code, `RE02xx` or `RW02xx`.
    pub code: &'static str,
    pub message: String,
    /// Index of the file among those the reader was given (`Loaded::sources`).
    pub file: usize,
    /// One-based line; 0 when the issue is about the file as a whole.
    pub line: u32,
}

impl Issue {
    pub fn new(code: &'static str, message: impl Into<String>, file: usize, line: u32) -> Self {
        Issue {
            code,
            message: message.into(),
            file,
            line,
        }
    }

    /// Warnings carry an `RW` code; everything else is an error.
    pub fn is_error(&self) -> bool {
        self.code.starts_with("RE")
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
