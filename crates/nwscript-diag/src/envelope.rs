//! The JSON document every command of every binary writes in `--format
//! json` mode: one object with the same envelope and a command-specific
//! `result`. `nws` and `rakata` share this type so a program reading either
//! finds the same keys; only the `game` field's shape is the binary's own.

use crate::json::JsonDiagnostic;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Envelope<G> {
    /// The binary that wrote the document: `nws` or `rakata`.
    pub tool: &'static str,
    /// The binary's version as `--version` prints it.
    pub version: &'static str,
    pub command: String,
    /// What `--game` named, in the binary's own shape: the profile for
    /// `nws`, the install directory for `rakata`.
    pub game: Option<G>,
    pub inputs: Vec<InputRecord>,
    pub outputs: Vec<OutputRecord>,
    pub diagnostics: Vec<JsonDiagnostic>,
    pub result: serde_json::Value,
}

impl<G> Envelope<G> {
    pub fn new(
        tool: &'static str,
        version: &'static str,
        command: impl Into<String>,
        game: Option<G>,
    ) -> Self {
        Envelope {
            tool,
            version,
            command: command.into(),
            game,
            inputs: Vec::new(),
            outputs: Vec::new(),
            diagnostics: Vec::new(),
            result: serde_json::Value::Object(Default::default()),
        }
    }
}

/// A file the command read, and what it was for.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct InputRecord {
    /// `source`, `include`, `nwscript`, `bytecode`, `world`, ...
    pub role: String,
    pub path: String,
    pub sha256: String,
}

/// A file the command wrote.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct OutputRecord {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}
