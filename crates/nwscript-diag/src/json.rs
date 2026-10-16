//! The JSON shape of a diagnostic, as the PRD's "The JSON document" shows it.

use crate::{Diagnostic, Severity, SourceMap};
use serde::Serialize;

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct JsonDiagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub spans: Vec<JsonSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    pub explain: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct JsonSpan {
    pub path: String,
    pub start: JsonPos,
    pub end: JsonPos,
    pub primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct JsonPos {
    pub line: usize,
    pub col: usize,
    pub offset: usize,
}

pub fn to_json(d: &Diagnostic, map: &SourceMap) -> JsonDiagnostic {
    let pos = |file, offset| {
        let lc = map.get(file).line_col(offset);
        JsonPos {
            line: lc.line,
            col: lc.col,
            offset,
        }
    };
    JsonDiagnostic {
        code: d.code.id,
        severity: d.severity,
        message: d.message.clone(),
        spans: d
            .labels
            .iter()
            .map(|l| JsonSpan {
                path: map.get(l.span.file).name.clone(),
                start: pos(l.span.file, l.span.start),
                end: pos(l.span.file, l.span.end),
                primary: l.primary,
                label: l.message.clone(),
            })
            .collect(),
        help: d.help.clone(),
        notes: d.notes.clone(),
        explain: format!("nws explain {}", d.code.id),
    }
}
