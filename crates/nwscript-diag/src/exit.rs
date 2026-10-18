//! The exit codes every binary of the workspace documents, one table for
//! `nws` and `rakata` alike.

use std::process::ExitCode;

/// The documented exit codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// The command did what was asked.
    Ok = 0,
    /// Diagnostics at error level were reported.
    Errors = 1,
    /// A bad invocation: unknown flag or code, a missing `--game`.
    Usage = 2,
    /// A comparison or verification found a difference.
    Different = 3,
    /// An internal error; the message asks for a bug report.
    Internal = 101,
}

impl From<Exit> for ExitCode {
    fn from(e: Exit) -> ExitCode {
        ExitCode::from(e as u8)
    }
}
