//! The tree as indented text, with byte ranges.

use crate::{Parse, SyntaxElement, text::latin1_bytes};
use rowan::WalkEvent;
use std::fmt::Write;

pub(crate) fn dump(p: &Parse) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for event in p.syntax().preorder_with_tokens() {
        match event {
            WalkEvent::Enter(element) => {
                let span = p.span_of(element.clone());
                let _ = write!(
                    out,
                    "{:indent$}{:?}@{}..{}",
                    "",
                    element.kind(),
                    span.start,
                    span.end,
                    indent = depth * 2
                );
                if let SyntaxElement::Token(t) = &element {
                    let _ = write!(out, " {}", quote(&latin1_bytes(t.text())));
                }
                out.push('\n');
                depth += 1;
            }
            WalkEvent::Leave(_) => depth -= 1,
        }
    }
    for d in p.diagnostics() {
        let (start, end) = d.primary_span().map_or((0, 0), |s| (s.start, s.end));
        let _ = writeln!(out, "{}@{start}..{end}: {}", d.code.id, d.message);
    }
    out
}

/// Bytes as a quoted string: printable ASCII as is, everything else escaped.
fn quote(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            b'\r' => s.push_str("\\r"),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => {
                let _ = write!(s, "\\x{b:02x}");
            }
        }
    }
    s.push('"');
    s
}
