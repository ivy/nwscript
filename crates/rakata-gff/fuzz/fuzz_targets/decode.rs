//! `Gff::parse` and `survey` on arbitrary bytes: each returns a file or an
//! error, never panics, and a file read answers `facts`, `diff` and a path
//! to each of its fields. `Path::parse` reads the same bytes as text.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_gff::{Gff, Path, Struct, Value, diff, facts, survey};

/// Every field's path under `at`, and the value there.
fn walk(gff: &Gff, s: &Struct, at: &Path, depth: usize) {
    for (i, f) in s.fields.iter().enumerate() {
        // A label given twice in a struct is reached at its first field.
        if s.fields.iter().position(|g| g.label == f.label) != Some(i) {
            continue;
        }
        let p = at.field(f.label);
        let v = gff.get(&p).expect("a field read is reached by its path");
        assert!(v.semantic_eq(&f.value));
        let _ = (f.value.as_str(), Path::parse(&p.to_string()));
        if depth < 64 {
            match &f.value {
                Value::Struct(c) => walk(gff, c, &p, depth + 1),
                Value::List(l) => {
                    for (i, c) in l.iter().enumerate() {
                        walk(gff, c, &p.index(i), depth + 1);
                    }
                }
                _ => {}
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let _ = survey(data);
    if let Ok(g) = Gff::parse(data) {
        let _ = (g.file_type_str(), facts::facts(&g), facts::refs(&g));
        assert!(diff(&g, &g).is_empty());
        walk(&g, &g.root, &Path::root(), 0);
    }
    if let Ok(text) = std::str::from_utf8(data)
        && let Ok(p) = Path::parse(text)
    {
        assert_eq!(
            Path::parse(&p.to_string()).expect("a printed path reads"),
            p
        );
        let _ = p.parent();
    }
});
