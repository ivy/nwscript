//! What the reader found wrong or odd, each under a registered code. The
//! codes are strings here, as in `rakata-res`: the `rakata` façade registers
//! them with their explanations and turns an [`Issue`] into a diagnostic
//! with a span into the file.

use serde::Serialize;

/// Diagnostic codes for the installer, its inputs and its run: the `RE03xx`
/// and `RW03xx` ranges of the `rakata` registry.
pub mod codes {
    /// A line that is neither a section header, `Key=Value`, a comment nor blank.
    pub const MALFORMED_LINE: &str = "RE0301";
    /// A `Key=Value` line before the first `[Section]` header.
    pub const ENTRY_BEFORE_SECTION: &str = "RE0302";
    /// A value in no form its key takes (a row index that is not a number, a field type that names no type).
    pub const BAD_VALUE: &str = "RE0303";
    /// A `namespaces.ini` whose `[Namespaces]` list is missing or empty.
    pub const NO_NAMESPACES: &str = "RE0304";
    /// A list entry names a section the file does not have; TSLPatcher skips the entry.
    pub const MISSING_SECTION: &str = "RW0301";
    /// A key the section's vocabulary does not have; kept on the model, not applied.
    pub const UNKNOWN_KEY: &str = "RW0302";
    /// A section or key written twice; the first is what TSLPatcher reads.
    pub const DUPLICATE: &str = "RW0303";
    /// A memory token (`2DAMEMORY#`, `StrRef#`) read by an operation before any operation sets it.
    pub const TOKEN_UNSET: &str = "RW0304";
    /// A write that did not land: the file or member read back after the
    /// operation holds other bytes than the installer wrote.
    pub const READBACK_MISMATCH: &str = "RE0305";
    /// A file or capsule the installer could not write, or could not read
    /// back after writing.
    pub const WRITE_FAILED: &str = "RE0306";
    /// A file an operation needs, found in neither tslpatchdata, the
    /// destination nor the install.
    pub const SOURCE_MISSING: &str = "RE0307";
    /// A file an operation edits that does not parse as its format.
    pub const TARGET_UNREADABLE: &str = "RE0308";
    /// An operation that cannot apply as written: a row, column, field,
    /// entry, offset or sound the file does not have, or a value of a form
    /// the target cannot hold.
    pub const CANNOT_APPLY: &str = "RE0309";
    /// A script of `[CompileList]` that does not compile.
    pub const COMPILE_FAILED: &str = "RE0310";
    /// A memory token read while installing that no earlier operation set.
    pub const TOKEN_NO_VALUE: &str = "RE0311";
    /// A file `[Settings]` requires in `override` is not there; nothing is installed.
    pub const REQUIRED_MISSING: &str = "RE0312";
    /// A `File#` entry skipped because its destination already holds the file.
    pub const SKIPPED_EXISTS: &str = "RW0305";
}

/// Where an issue is: a file the reader was given, by index, and a line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// The registered code, `RE03xx` or `RW03xx`.
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
