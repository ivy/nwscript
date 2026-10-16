//! Conversion to the language server protocol's diagnostic.

use crate::{Diagnostic, Severity, SourceMap};
use lsp_types as lsp;

/// The LSP diagnostic for `d`, positioned by its primary label. LSP counts
/// columns in UTF-16 units; NWScript sources are single-byte, so the byte
/// column is the same number.
pub fn to_lsp(d: &Diagnostic, map: &SourceMap) -> lsp::Diagnostic {
    let range = d
        .primary_span()
        .map(|s| {
            let f = map.get(s.file);
            let a = f.line_col(s.start);
            let b = f.line_col(s.end);
            let p = |lc: crate::LineCol| lsp::Position::new(lc.line as u32 - 1, lc.col as u32 - 1);
            lsp::Range::new(p(a), p(b))
        })
        .unwrap_or_default();
    lsp::Diagnostic {
        range,
        severity: Some(match d.severity {
            Severity::Error => lsp::DiagnosticSeverity::ERROR,
            Severity::Warning => lsp::DiagnosticSeverity::WARNING,
            Severity::Note => lsp::DiagnosticSeverity::INFORMATION,
            Severity::Help => lsp::DiagnosticSeverity::HINT,
        }),
        code: Some(lsp::NumberOrString::String(d.code.id.into())),
        source: Some("nws".into()),
        message: match &d.help {
            Some(h) => format!("{}\nhelp: {h}", d.message),
            None => d.message.clone(),
        },
        ..Default::default()
    }
}
