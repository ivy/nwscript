//! A diff as text: one block per path, its sites indented beneath, and a
//! closing count.

use std::fmt::Write as _;

use super::{Change, Diff, Entry, FieldChange, Shown, Site};

fn mark(c: Change) -> char {
    match c {
        Change::Added => '+',
        Change::Removed => '-',
        Change::Changed => '~',
        Change::Renamed => 'R',
    }
}

fn word(c: Change) -> &'static str {
    match c {
        Change::Added => "added",
        Change::Removed => "removed",
        Change::Changed => "changed",
        Change::Renamed => "renamed",
    }
}

fn opt(v: Option<&str>) -> &str {
    v.unwrap_or("(absent)")
}

fn shown(v: Option<&Shown>) -> String {
    v.map_or_else(|| "(absent)".to_owned(), |s| s.value.clone())
}

/// At most `n` characters of `s`, with an ellipsis when cut.
fn clip(s: &str, n: usize) -> String {
    let one = s.replace(['\n', '\r'], " ");
    if one.chars().count() <= n {
        one
    } else {
        let mut t: String = one.chars().take(n).collect();
        t.push('…');
        t
    }
}

pub(super) fn text(d: &Diff) -> String {
    let mut out = String::new();
    for e in &d.entries {
        entry(&mut out, e);
    }
    let count = |c: Change| d.entries.iter().filter(|e| e.change == c).count();
    if d.entries.is_empty() {
        let _ = write!(out, "no differences");
    } else {
        let _ = write!(
            out,
            "{} paths differ: {} added, {} removed, {} changed, {} renamed",
            d.entries.len(),
            count(Change::Added),
            count(Change::Removed),
            count(Change::Changed),
            count(Change::Renamed),
        );
    }
    let _ = write!(out, " ({} pairs compared", d.compared);
    if d.same_data > 0 {
        let _ = write!(
            out,
            "; {} differ in layout only, their data equal",
            d.same_data
        );
    }
    out.push(')');
    out
}

fn entry(out: &mut String, e: &Entry) {
    let _ = write!(out, "{} ", mark(e.change));
    if let Some(from) = &e.renamed_from {
        let _ = write!(out, "{from} -> ");
    }
    out.push_str(&e.path);
    let mut facts = Vec::new();
    if let Some(f) = &e.format {
        facts.push(f.clone());
    }
    let side = match e.change {
        Change::Removed => e.a.as_ref(),
        _ => e.b.as_ref().or(e.a.as_ref()),
    };
    if let Some(f) = side
        && (e.sites.is_empty() || !e.compared)
    {
        facts.push(format!("{} bytes", f.size));
        for h in &f.header {
            facts.push(format!("{} {}", h.field, h.value));
        }
    }
    if !e.compared && e.a.is_some() && e.b.is_some() {
        facts.push("contents not compared".into());
        if let (Some(a), Some(b)) = (&e.a, &e.b) {
            facts.push(format!("sha256 {} -> {}", &a.sha256[..12], &b.sha256[..12]));
            for h in &a.header {
                if let Some(hb) = b.header.iter().find(|x| x.field == h.field)
                    && hb.value != h.value
                {
                    facts.push(format!("{} {} -> {}", h.field, h.value, hb.value));
                }
            }
        }
    }
    if !facts.is_empty() {
        let _ = write!(out, " ({})", facts.join(", "));
    }
    if !e.against.is_empty() {
        let verb = if e.change == Change::Added {
            "over"
        } else {
            "uncovering"
        };
        let _ = write!(out, ", {verb} {}", e.against.join(", "));
        if e.sites.is_empty() && e.compared {
            out.push_str(": the same data");
        }
    }
    out.push('\n');
    for s in &e.sites {
        site(out, s);
    }
}

fn field(out: &mut String, indent: &str, f: &FieldChange) {
    let _ = writeln!(
        out,
        "{indent}{}: {} -> {}",
        f.path,
        clip(&shown(f.before.as_ref()), 200),
        clip(&shown(f.after.as_ref()), 200)
    );
}

fn site(out: &mut String, s: &Site) {
    const IN: &str = "    ";
    match s {
        Site::Header {
            field,
            before,
            after,
        } => {
            let _ = writeln!(
                out,
                "{IN}{field}: {} -> {}",
                opt(before.as_deref()),
                opt(after.as_deref())
            );
        }
        Site::Field(f) => field(out, IN, f),
        Site::Column { change, column } => {
            let _ = writeln!(out, "{IN}column `{column}` {}", word(*change));
        }
        Site::Row { change, row, cells } => {
            let cells = cells
                .iter()
                .map(|c| format!("{}={}", c.column, c.value))
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(out, "{IN}row `{row}` {}: {cells}", word(*change));
        }
        Site::Cell {
            row,
            column,
            before,
            after,
        } => {
            let _ = writeln!(
                out,
                "{IN}row `{row}`, column `{column}`: {before:?} -> {after:?}"
            );
        }
        Site::Strref {
            change,
            strref,
            field,
            before,
            after,
        } => match change {
            Change::Changed => {
                let _ = writeln!(
                    out,
                    "{IN}strref {strref} {field}: {:?} -> {:?}",
                    opt(before.as_deref()),
                    opt(after.as_deref())
                );
            }
            c => {
                let t = after.as_deref().or(before.as_deref()).unwrap_or("");
                let _ = writeln!(out, "{IN}strref {strref} {}: {:?}", word(*c), clip(t, 120));
            }
        },
        Site::Node(n) => {
            let at = match (&n.a, &n.b) {
                (Some(a), Some(b)) if a != b => format!("{a} -> {b}"),
                (Some(a), _) => a.clone(),
                (None, Some(b)) => b.clone(),
                (None, None) => String::new(),
            };
            let _ = write!(out, "{IN}{} {} {} ({at})", mark(n.change), n.kind, n.key);
            if let Some(t) = n.text.as_ref().filter(|_| !n.key.starts_with("text:")) {
                let _ = write!(out, " {:?}", clip(t, 100));
            }
            if !n.links.is_empty() {
                let _ = write!(out, " -> {}", n.links.join(", "));
            }
            out.push('\n');
            for f in &n.fields {
                field(out, "        ", f);
            }
        }
        Site::Resource {
            change,
            resource,
            before,
            after,
        } => {
            let _ = writeln!(
                out,
                "{IN}{resource} {}: {} -> {}",
                word(*change),
                opt(before.as_deref()),
                opt(after.as_deref())
            );
        }
    }
}
