//! Rendering for people, through miette. The adapter here is the only place
//! miette's types appear; the diagnostic itself stays plain data.

use crate::{Diagnostic, Severity, SourceMap};
use miette::{GraphicalReportHandler, GraphicalTheme, LabeledSpan, NamedSource, SourceSpan};
use std::fmt;

/// Whether rendered output may use colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Always,
    Never,
}

/// A diagnostic and its source, borrowed into the shape miette renders.
struct Adapter<'a> {
    d: &'a Diagnostic,
    source: Option<NamedSource<String>>,
    labels: Vec<LabeledSpan>,
    footer: Option<String>,
}

impl fmt::Debug for Adapter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.d.message)
    }
}

impl fmt::Display for Adapter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.d.message)
    }
}

impl std::error::Error for Adapter<'_> {}

impl miette::Diagnostic for Adapter<'_> {
    fn code<'b>(&'b self) -> Option<Box<dyn fmt::Display + 'b>> {
        Some(Box::new(self.d.code.id))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(match self.d.severity {
            Severity::Error => miette::Severity::Error,
            Severity::Warning => miette::Severity::Warning,
            Severity::Note | Severity::Help => miette::Severity::Advice,
        })
    }

    fn help<'b>(&'b self) -> Option<Box<dyn fmt::Display + 'b>> {
        self.footer
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.source.as_ref().map(|s| s as &dyn miette::SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        if self.labels.is_empty() {
            None
        } else {
            Some(Box::new(self.labels.iter().cloned()))
        }
    }
}

/// Render one diagnostic as text. miette draws one file per diagnostic, so
/// labels in other files than the primary one become notes with a location.
pub fn render(d: &Diagnostic, map: &SourceMap, color: Color) -> String {
    let primary_file = d
        .primary_span()
        .or_else(|| d.labels.first().map(|l| l.span))
        .map(|s| s.file);
    let mut notes = d.notes.clone();
    let mut labels = Vec::new();
    for l in &d.labels {
        let f = map.get(l.span.file);
        if Some(l.span.file) == primary_file {
            let start = f.display_offset(l.span.start);
            let end = f.display_offset(l.span.end);
            let span = SourceSpan::new(start.into(), end - start);
            labels.push(if l.primary {
                LabeledSpan::new_primary_with_span(l.message.clone(), span)
            } else {
                LabeledSpan::new_with_span(l.message.clone(), span)
            });
        } else {
            let lc = f.line_col(l.span.start);
            let what = l.message.as_deref().unwrap_or("related");
            notes.push(format!("{what}: {}:{}:{}", f.name, lc.line, lc.col));
        }
    }
    let mut footer: Vec<String> = Vec::new();
    if let Some(h) = &d.help {
        footer.push(h.clone());
    }
    footer.extend(notes.into_iter().map(|n| format!("note: {n}")));
    footer.push(format!(
        "note: run `nws explain {}` for the full explanation",
        d.code.id
    ));
    let adapter = Adapter {
        d,
        source: primary_file.map(|id| {
            let f = map.get(id);
            NamedSource::new(f.name.clone(), f.display_text())
        }),
        labels,
        footer: Some(footer.join("\n")),
    };
    let theme = match color {
        Color::Always => GraphicalTheme::unicode(),
        Color::Never => GraphicalTheme::unicode_nocolor(),
    };
    let mut out = String::new();
    GraphicalReportHandler::new_themed(theme)
        .with_width(100)
        .render_report(&mut out, &adapter)
        .expect("rendering to a String cannot fail");
    out
}
