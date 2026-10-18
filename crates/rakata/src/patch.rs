//! The installer's reader through the façade: `rakata-patch` as it is, and
//! the turn of its [`Issue`]s into diagnostics with spans into the files it
//! read, under the codes this crate registers.

pub use rakata_patch::*;

use nwscript_diag::{Diagnostic, FileId, SourceMap, Span};

/// Add the files `loaded` read to `map` and render its issues against them.
/// An issue whose line is 0 has no span.
pub fn diagnostics(loaded: &Loaded, map: &mut SourceMap) -> Vec<Diagnostic> {
    let ids: Vec<FileId> = loaded
        .sources
        .iter()
        .map(|s| map.add(s.path.display().to_string(), s.bytes.clone()))
        .collect();
    loaded
        .issues
        .iter()
        .map(|issue| {
            let code = crate::codes::lookup(issue.code)
                .unwrap_or_else(|| panic!("rakata-patch code {} is not registered", issue.code));
            let d = Diagnostic::new(code, issue.message.clone());
            match (ids.get(issue.file), loaded.sources.get(issue.file)) {
                (Some(&id), Some(src)) if issue.line > 0 => {
                    let (start, end) = line_span(&src.bytes, issue.line);
                    d.primary(Span::new(id, start, end), "")
                }
                _ => d,
            }
        })
        .collect()
}

/// Byte offsets of one-based `line` in `bytes`, without its line ending.
fn line_span(bytes: &[u8], line: u32) -> (usize, usize) {
    let mut start = 0usize;
    let mut n = 1u32;
    for (i, &b) in bytes.iter().enumerate() {
        if n == line {
            break;
        }
        if b == b'\n' {
            n += 1;
            start = i + 1;
        }
    }
    let mut end = bytes[start..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |p| start + p);
    if end > start && bytes[end - 1] == b'\r' {
        end -= 1;
    }
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_spans_skip_the_line_ending() {
        let b = b"[A]\r\nX=1\r\nY=2";
        assert_eq!(line_span(b, 1), (0, 3));
        assert_eq!(line_span(b, 2), (5, 8));
        assert_eq!(line_span(b, 3), (10, 13));
    }
}
