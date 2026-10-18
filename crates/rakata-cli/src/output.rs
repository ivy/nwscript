//! Writing a command's outcome: rendered text, or one JSON envelope. The
//! same shape as `nws`'s, over the shared envelope.

use crate::cli::{ColorArg, Format};
use rakata::Envelope;
use rakata::diag::{Diagnostic, SourceMap, json::to_json, render};
use std::io::{IsTerminal, Write};

pub struct Output {
    pub format: Format,
    /// Text for an agent: tables unpadded, `index query` as TSV. Set when
    /// no `--format` was given and [`crate::agent::detected`].
    pub compact: bool,
    pub color: render::Color,
    pub quiet: bool,
}

impl Output {
    /// `format` as `--format` gave it; without it, text, compact when an
    /// agent runs this process.
    pub fn new(format: Option<Format>, color: ColorArg, quiet: bool) -> Self {
        let compact = format.is_none() && crate::agent::detected();
        let format = format.unwrap_or(Format::Text);
        let color = match color {
            ColorArg::Always => render::Color::Always,
            ColorArg::Never => render::Color::Never,
            ColorArg::Auto => {
                if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
                    || !std::io::stderr().is_terminal()
                {
                    render::Color::Never
                } else {
                    render::Color::Always
                }
            }
        };
        Output {
            format,
            compact,
            color,
            quiet,
        }
    }

    /// Emit a finished command: text to stdout and diagnostics to stderr, or
    /// the envelope (diagnostics inside it) as one JSON document on stdout.
    pub fn emit(&self, mut env: Envelope, text: &str, diags: &[Diagnostic], map: &SourceMap) {
        match self.format {
            Format::Json => {
                env.diagnostics = diags.iter().map(|d| to_json(d, map)).collect();
                let mut out = std::io::stdout().lock();
                let doc = serde_json::to_string_pretty(&env).expect("the envelope serializes");
                ignore_closed_pipe(writeln!(out, "{doc}"));
            }
            Format::Text => {
                let mut err = std::io::stderr().lock();
                for d in diags {
                    if self.quiet && !d.is_error() {
                        continue;
                    }
                    ignore_closed_pipe(write!(err, "{}", render::render(d, map, self.color)));
                }
                if !text.is_empty() {
                    let mut out = std::io::stdout().lock();
                    let nl = if text.ends_with('\n') { "" } else { "\n" };
                    ignore_closed_pipe(write!(out, "{text}{nl}"));
                }
            }
        }
    }
}

/// A reader that stops reading (`rakata ... | head`) is not an error; any
/// other write failure is.
fn ignore_closed_pipe(r: std::io::Result<()>) {
    if let Err(e) = r
        && e.kind() != std::io::ErrorKind::BrokenPipe
    {
        panic!("writing output: {e}");
    }
}
