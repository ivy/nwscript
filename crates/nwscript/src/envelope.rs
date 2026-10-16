//! The JSON document every command writes in `--format json` mode: one
//! object with the same envelope and a command-specific `result`.

use crate::Game;
use nwscript_diag::json::JsonDiagnostic;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Envelope {
    pub tool: &'static str,
    pub version: &'static str,
    pub command: String,
    pub game: Option<Game>,
    pub inputs: Vec<InputRecord>,
    pub outputs: Vec<OutputRecord>,
    pub diagnostics: Vec<JsonDiagnostic>,
    pub result: serde_json::Value,
}

impl Envelope {
    pub fn new(command: impl Into<String>, game: Option<Game>) -> Self {
        Envelope {
            tool: "nws",
            version: env!("CARGO_PKG_VERSION"),
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
