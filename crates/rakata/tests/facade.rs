use rakata::codes;

#[test]
fn every_code_has_an_explanation_and_only_those() {
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
fn codes_carry_the_rakata_prefix_and_no_nwscript_code_answers() {
    for c in codes::ALL {
        assert!(
            c.id.starts_with("RE") || c.id.starts_with("RW"),
            "{} is not a rakata code",
            c.id
        );
        assert!(
            rakata::diag::codes::lookup(c.id).is_none(),
            "{} is also an nwscript code",
            c.id
        );
    }
    assert_eq!(codes::lookup("re0001").map(|c| c.id), Some("RE0001"));
    assert!(codes::lookup("E0001").is_none());
}

#[test]
fn the_version_starts_with_the_workspace_version() {
    assert!(
        rakata::VERSION.starts_with(env!("CARGO_PKG_VERSION")),
        "{}",
        rakata::VERSION
    );
}

#[test]
fn the_envelope_names_the_tool_and_the_version() {
    let env = rakata::envelope("explain", None);
    let doc = serde_json::to_value(&env).unwrap();
    assert_eq!(doc["tool"], "rakata");
    assert_eq!(doc["version"], rakata::VERSION);
    assert_eq!(doc["game"], serde_json::Value::Null);
    let env = rakata::envelope("explain", Some("games/swkotor".into()));
    let doc = serde_json::to_value(&env).unwrap();
    assert_eq!(doc["game"], "games/swkotor");
}

#[test]
fn every_code_a_data_crate_reports_is_registered() {
    for id in [
        rakata_res::codes::RESREF_TOO_LONG,
        rakata_res::codes::RESID_NO_EXTENSION,
        rakata_res::codes::UNKNOWN_EXTENSION,
        rakata::patch::codes::MALFORMED_LINE,
        rakata::patch::codes::ENTRY_BEFORE_SECTION,
        rakata::patch::codes::BAD_VALUE,
        rakata::patch::codes::NO_NAMESPACES,
        rakata::patch::codes::MISSING_SECTION,
        rakata::patch::codes::UNKNOWN_KEY,
        rakata::patch::codes::DUPLICATE,
        rakata::patch::codes::TOKEN_UNSET,
        rakata_lip::codes::IO,
        rakata_lip::codes::NOT_A_LIP,
        rakata_lip::codes::TRUNCATED,
        rakata_lip::codes::BAD_SHAPE,
        rakata_lip::codes::TRAILING_BYTES,
        rakata_lip::codes::TOO_MANY_KEYS,
        rakata_lyt::codes::NOT_A_LYT,
        rakata_lyt::codes::UNEXPECTED_LINE,
        rakata_lyt::codes::BAD_NUMBER,
        rakata_lyt::codes::COUNT_MISMATCH,
        rakata_lyt::codes::UNTERMINATED,
        rakata_lyt::codes::NOT_ASCII,
        rakata_lyt::codes::UNWRITABLE,
        rakata_lyt::codes::ORPHAN_VISIBLE,
        rakata_lyt::codes::BAD_VIS_LINE,
        rakata::save::codes::NOT_A_SAVE,
        rakata::save::codes::NO_SUCH_MODULE,
        rakata::save::codes::BAD_SHAPE,
        rakata::save::codes::NO_SUCH_CREATURE,
        rakata::save::codes::NO_LOCAL_NAMES,
        rakata::texture::codes::TPC_TRUNCATED,
        rakata::texture::codes::TPC_UNKNOWN_FORMAT,
        rakata::texture::codes::TPC_BAD_LAYOUT,
        rakata::texture::codes::TGA_TRUNCATED,
        rakata::texture::codes::TGA_UNSUPPORTED,
        rakata::texture::codes::TGA_RLE_OVERRUN,
        rakata::walkmesh::codes::NOT_A_WALKMESH,
        rakata::walkmesh::codes::TRUNCATED,
        rakata::walkmesh::codes::UNKNOWN_KIND,
        rakata::walkmesh::codes::BAD_INDEX,
        rakata::walkmesh::codes::BAD_SURFACE_TABLE,
        rakata::mdl::codes::NOT_BINARY,
        rakata::mdl::codes::TRUNCATED,
        rakata::mdl::codes::MDX_OUT_OF_RANGE,
        rakata::mdl::codes::BAD_TREE,
        rakata::mdl::codes::BAD_NAME,
        rakata::mdl::codes::BAD_VERTEX_LAYOUT,
        rakata::mdl::codes::BAD_INDEX,
    ] {
        assert!(codes::lookup(id).is_some(), "{id} is not registered");
    }
}
