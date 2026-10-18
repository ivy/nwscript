use nwscript_diag::{Diagnostic, Level, Levels, Severity, SourceMap, Span, codes, json, render};

fn sample() -> (Diagnostic, SourceMap) {
    let mut map = SourceMap::new();
    let src =
        b"void main()\r\n{\r\n    object o = GetFirstPCx();\r\n    int n = 1;\r\n}\r\n".to_vec();
    let f = map.add("k_act_com1.nss", src.clone());
    let at = |needle: &str| {
        let i = src
            .windows(needle.len())
            .position(|w| w == needle.as_bytes())
            .unwrap();
        Span::new(f, i, i + needle.len())
    };
    let d = Diagnostic::new(&codes::E0001, "no function named `GetFirstPCx`")
        .primary(
            at("GetFirstPCx"),
            "not declared in nwscript.nss or any include",
        )
        .secondary(at("object o"), "the value is assigned here")
        .help("did you mean `GetFirstPC`?");
    (d, map)
}

#[test]
fn renders_two_spans() {
    let (d, map) = sample();
    insta::assert_snapshot!(render::render(&d, &map, render::Color::Never));
}

#[test]
fn json_shape() {
    let (d, map) = sample();
    let j = serde_json::to_string_pretty(&json::to_json(&d, &map)).unwrap();
    insta::assert_snapshot!(j);
}

#[test]
fn every_code_has_an_explanation_and_every_explanation_a_code() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("explain");
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .trim_end_matches(".md")
                .to_string()
        })
        .collect();
    files.sort();
    let mut ids: Vec<String> = codes::ALL.iter().map(|c| c.id.to_string()).collect();
    ids.sort();
    assert_eq!(files, ids, "explain/*.md and the registry disagree");
    for c in codes::ALL {
        assert!(
            c.explanation.trim().len() > 40,
            "{} has a stub explanation",
            c.id
        );
        assert!(!c.summary.is_empty());
    }
}

#[test]
fn levels_promote_and_allow() {
    let (d, _) = sample();
    let mut l = Levels::default();
    // E0001 is an error, not a lint: it can be denied but not allowed away.
    l.set("E0001", Level::Allow);
    assert!(l.apply(d.clone()).is_some());
    let w = d.clone().with_severity(Severity::Warning);
    l.deny_warnings = true;
    assert_eq!(
        l.apply(w).unwrap().severity,
        Severity::Warning.max(Severity::Error)
    );
}

#[test]
fn lookup_is_case_insensitive() {
    assert_eq!(codes::lookup("e0001").map(|c| c.id), Some("E0001"));
    assert!(codes::lookup("E9999").is_none());
}

/// A second toolset's registry: its own prefix, its own explanation
/// directory, declared through the same macro.
mod other {
    nwscript_diag::codes! {
        dir "tests/explain";
        T0001 Warning "a code registered by another crate";
    }
}

#[test]
fn another_crate_registers_its_own_codes() {
    assert_eq!(other::ALL.len(), 1);
    let c = codes::lookup_in(other::ALL, "t0001").unwrap();
    assert_eq!(c.id, "T0001");
    assert_eq!(c.severity, Severity::Warning);
    assert!(c.explanation.contains("another crate"));
    // The two registries are disjoint: neither answers for the other.
    assert!(codes::lookup("T0001").is_none());
    assert!(codes::lookup_in(other::ALL, "E0001").is_none());
}
