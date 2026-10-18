use assert_cmd::Command;
use predicates::prelude::*;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

/// The keys of the envelope `nws --format json` writes, in the order its
/// `cli.rs` test checks them plus `tool`, `command` and `result`.
const ENVELOPE_KEYS: [&str; 8] = [
    "tool",
    "version",
    "command",
    "game",
    "inputs",
    "outputs",
    "diagnostics",
    "result",
];

#[test]
fn explain_prints_the_explanation() {
    rakata()
        .args(["explain", "RE0001"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "RE0001: an input file could not be read",
        ));
}

#[test]
fn a_diagnostic_points_at_rakata_explain_not_nws() {
    let missing = std::env::temp_dir().join(format!("rakata-cli-missing-{}", std::process::id()));
    let text = rakata()
        .args(["--game", missing.to_str().unwrap(), "which", "x.utc"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&text.stderr);
    assert!(stderr.contains("run `rakata explain RE"), "{stderr}");
    assert!(!stderr.contains("nws explain"), "{stderr}");
    let json = rakata()
        .args([
            "--format",
            "json",
            "--game",
            missing.to_str().unwrap(),
            "which",
            "x.utc",
        ])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let explain = doc["diagnostics"][0]["explain"].as_str().unwrap();
    assert!(explain.starts_with("rakata explain RE"), "{explain}");
}

#[test]
fn explain_unknown_code_is_a_usage_error() {
    for code in ["RE9999", "E0001"] {
        rakata()
            .args(["explain", code])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("no diagnostic code"));
    }
}

#[test]
fn json_mode_writes_exactly_one_document_with_the_nws_envelope() {
    let out = rakata()
        .args(["--format", "json", "explain", "re0001"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let mut docs =
        serde_json::Deserializer::from_slice(&out.stdout).into_iter::<serde_json::Value>();
    let doc = docs.next().unwrap().unwrap();
    assert!(
        docs.next().is_none(),
        "more than one JSON document on stdout"
    );
    assert_eq!(doc["tool"], "rakata");
    assert_eq!(doc["version"], rakata::VERSION);
    assert_eq!(doc["command"], "explain");
    assert_eq!(doc["game"], serde_json::Value::Null);
    assert_eq!(doc["result"]["code"], "RE0001");
    let mut keys: Vec<&str> = doc
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = ENVELOPE_KEYS.to_vec();
    expected.sort_unstable();
    assert_eq!(keys, expected, "the envelope's keys differ from nws's");
}

#[test]
fn the_game_is_recorded_as_named() {
    let out = rakata()
        .args([
            "--game",
            "games/swkotor",
            "--format",
            "json",
            "explain",
            "RE0001",
        ])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["game"], "games/swkotor");
}

#[test]
fn unknown_flag_is_a_usage_error() {
    rakata()
        .args(["--frobnicate", "explain", "RE0001"])
        .assert()
        .code(2);
}

#[test]
fn version_is_the_build_identity() {
    let out = rakata().arg("--version").output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(text.trim(), format!("rakata {}", rakata::VERSION));
    assert!(
        rakata::VERSION.starts_with(env!("CARGO_PKG_VERSION")),
        "{}",
        rakata::VERSION
    );
}

#[test]
fn exit_codes_are_the_documented_table() {
    use rakata::Exit;
    assert_eq!(Exit::Ok as u8, 0);
    assert_eq!(Exit::Errors as u8, 1);
    assert_eq!(Exit::Usage as u8, 2);
    assert_eq!(Exit::Different as u8, 3);
    assert_eq!(Exit::Internal as u8, 101);
}

#[test]
fn every_help_has_an_example_and_lists_only_built_commands() {
    let top = rakata().arg("--help").output().unwrap();
    let text = String::from_utf8_lossy(&top.stdout);
    assert!(text.contains("Examples:"));
    let mut commands: Vec<&str> = text
        .lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .filter(|c| *c != "help")
        .collect();
    commands.sort_unstable();
    assert_eq!(
        commands,
        [
            "2da",
            "cat",
            "conformance",
            "corpus",
            "decompile",
            "dialog",
            "diff",
            "explain",
            "flags",
            "gff",
            "index",
            "install",
            "lint",
            "lip",
            "lua",
            "quest",
            "refs",
            "save",
            "show",
            "tlk",
            "which"
        ],
        "--help lists commands not yet built"
    );
    for cmd in [
        "2da",
        "cat",
        "conformance",
        "corpus",
        "decompile",
        "dialog",
        "diff",
        "explain",
        "flags",
        "gff",
        "index",
        "install",
        "lint",
        "lip",
        "lua",
        "quest",
        "refs",
        "save",
        "show",
        "tlk",
        "which",
    ] {
        let out = rakata().args([cmd, "--help"]).output().unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("Example"),
            "{cmd} --help has no example"
        );
    }
}

/// A tree with a valid empty `chitin.key`, in a fresh temporary directory.
fn install(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rakata-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Override")).unwrap();
    std::fs::write(
        dir.join("chitin.key"),
        rakata_archive::Key::new().to_bytes().unwrap(),
    )
    .unwrap();
    dir
}

#[test]
fn corpus_extract_and_decompile_without_a_game_are_usage_errors_naming_the_flag() {
    for action in ["extract", "decompile"] {
        rakata()
            .args(["corpus", action])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("--game"));
    }
}

#[test]
fn corpus_verify_without_a_manifest_reports_re1003() {
    let dir = install("corpus-verify");
    rakata()
        .args([
            "corpus",
            "verify",
            "--out",
            dir.to_str().unwrap(),
            "--manifest",
            dir.join("absent.toml").to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1003"));
}

#[test]
fn corpus_decompile_over_an_install_without_nwscript_reports_re1002() {
    let dir = install("corpus-nonwscript");
    rakata()
        .args([
            "--game",
            dir.to_str().unwrap(),
            "corpus",
            "decompile",
            "--out",
            dir.join("corpus").to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1002"));
}

#[test]
fn conformance_without_a_game_is_a_usage_error_naming_the_flag() {
    rakata()
        .arg("conformance")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"));
}

#[test]
fn conformance_over_a_directory_that_is_not_an_install_reports_re0501() {
    let dir = std::env::temp_dir().join(format!("rakata-cli-noinstall-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    rakata()
        .args(["--game", dir.to_str().unwrap(), "conformance"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE0501"));
}

#[test]
fn conformance_with_an_unknown_format_is_a_usage_error() {
    let dir = install("only");
    rakata()
        .args([
            "--game",
            dir.to_str().unwrap(),
            "conformance",
            "--only",
            "nope",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no format `nope`"));
}

#[test]
fn conformance_writes_one_envelope_and_the_report_files() {
    let dir = install("json");
    let report = dir.join("report.json");
    let md = dir.join("report.md");
    let out = rakata()
        .args([
            "--game",
            dir.to_str().unwrap(),
            "--format",
            "json",
            "conformance",
            "--only",
            "archive",
            "--report",
            report.to_str().unwrap(),
            "--markdown",
            md.to_str().unwrap(),
            "--known",
            dir.join("absent.toml").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut docs =
        serde_json::Deserializer::from_slice(&out.stdout).into_iter::<serde_json::Value>();
    let doc = docs.next().unwrap().unwrap();
    assert!(
        docs.next().is_none(),
        "more than one JSON document on stdout"
    );
    assert_eq!(doc["command"], "conformance");
    assert_eq!(doc["game"], dir.to_str().unwrap());
    assert_eq!(doc["result"]["formats"][0]["name"], "archive");
    assert_eq!(doc["result"]["formats"][0]["counts"]["identical"], 1);
    assert_eq!(doc["result"]["files"], 1);
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(written["files"], 1);
    let text = std::fs::read_to_string(&md).unwrap();
    assert!(text.starts_with("# Round-trip conformance"), "{text}");
    assert!(text.contains("| `archive` | 1 | 1 | 0 | 0 | |"), "{text}");
}

#[test]
fn conformance_text_mode_prints_the_summary() {
    let dir = install("text");
    rakata()
        .args(["--game", dir.to_str().unwrap(), "conformance"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("| Format | Files | Identical |"));
}

/// A GFF V3.2 of type `ty` with an empty root struct.
fn empty_gff(ty: &[u8; 4]) -> Vec<u8> {
    let mut b = ty.to_vec();
    b.extend(b"V3.2");
    // Structs at 56, one; every other section at 68, empty.
    for (offset, count) in [(56u32, 1u32), (68, 0), (68, 0), (68, 0), (68, 0), (68, 0)] {
        b.extend(offset.to_le_bytes());
        b.extend(count.to_le_bytes());
    }
    b.extend(u32::MAX.to_le_bytes());
    b.extend(0u32.to_le_bytes());
    b.extend(0u32.to_le_bytes());
    b
}

#[test]
fn dialog_show_without_a_game_is_a_usage_error_naming_the_flag() {
    rakata()
        .args(["dialog", "show", "banter.dlg"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"));
}

#[test]
fn dialog_show_of_a_dialogue_the_install_lacks_reports_re0404() {
    let dir = install("dialog-missing");
    rakata()
        .args([
            "--game",
            dir.to_str().unwrap(),
            "dialog",
            "show",
            "banter.dlg",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE0404"));
}

#[test]
fn dialog_show_of_a_gff_that_is_not_a_dialogue_reports_re1101() {
    let dir = install("dialog-utc");
    let file = dir.join("p_bastila.utc");
    std::fs::write(&file, empty_gff(b"UTC ")).unwrap();
    rakata()
        .args(["--game", dir.to_str().unwrap(), "dialog", "show"])
        .arg(&file)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1101"));
}

#[test]
fn dialog_show_of_an_override_dialogue_finds_the_module_holding_its_name() {
    let dir = install("dialog-override");
    std::fs::create_dir_all(dir.join("modules")).unwrap();
    std::fs::write(dir.join("Override/x.dlg"), empty_gff(b"DLG ")).unwrap();
    let rim = |path: &str| {
        let mut r = rakata_archive::Rim::new();
        r.add(
            rakata_res::ResId::parse("x.dlg").unwrap(),
            empty_gff(b"DLG "),
        )
        .unwrap();
        std::fs::write(dir.join(path), r.to_bytes().unwrap()).unwrap();
    };
    rim("modules/danm13.rim");
    let show = || {
        rakata()
            .args(["--game", dir.to_str().unwrap(), "dialog", "show", "x.dlg"])
            .assert()
    };
    show().code(0).stdout(predicate::str::starts_with(
        "Override/x.dlg (module danm13)\n",
    ));
    rim("modules/danm14aa.rim");
    show().code(1).stderr(predicate::str::contains(
        "x.dlg is in Override/ and in 2 modules: danm13, danm14aa",
    ));
}

#[test]
fn dialog_show_writes_the_transcript_as_one_json_document() {
    let dir = install("dialog-json");
    let file = dir.join("empty.dlg");
    std::fs::write(&file, empty_gff(b"DLG ")).unwrap();
    let out = rakata()
        .args([
            "--format",
            "json",
            "--game",
            dir.to_str().unwrap(),
            "dialog",
            "show",
        ])
        .arg(&file)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["command"], "dialog show");
    let t = &doc["result"]["transcript"];
    assert_eq!(t["dialog"], "empty.dlg");
    assert_eq!(t["entries"], 0);
    assert!(t["nodes"].as_array().unwrap().is_empty());
}

/// E0 voiced `cli_vo`, linking to R0 under `k_con_x` and to R1; R0 and R1
/// share strref 5, so only their indices name them; E1 is unlinked text.
fn keyed_dialog() -> Vec<u8> {
    use rakata_gff::{Gff, Label, LocString, Struct, Value};
    let label = |s: &str| Label::new(s).unwrap();
    let link = |index: u32, active: &str| {
        let mut s = Struct::new(0);
        s.push(label("Index"), Value::Dword(index))
            .push(label("Active"), Value::resref(active).unwrap())
            .push(label("IsChild"), Value::Byte(0));
        s
    };
    let node = |list: &str, said: LocString, vo: &str, links: Vec<Struct>| {
        let mut s = Struct::new(0);
        s.push(label("Text"), Value::LocString(said))
            .push(label("VO_ResRef"), Value::resref(vo).unwrap())
            .push(label(list), Value::List(links));
        s
    };
    let mut hi = LocString::none();
    hi.set(0, b"Hi.");
    let mut alone = LocString::none();
    alone.set(0, b"Alone.");
    let entries = vec![
        node(
            "RepliesList",
            hi,
            "cli_vo",
            vec![link(0, "k_con_x"), link(1, "")],
        ),
        node("RepliesList", alone, "", vec![]),
    ];
    let replies = vec![
        node("EntriesList", LocString::strref(5), "", vec![]),
        node("EntriesList", LocString::strref(5), "", vec![]),
    ];
    let mut start = Struct::new(0);
    start
        .push(label("Index"), Value::Dword(0))
        .push(label("Active"), Value::resref("").unwrap());
    let mut g = Gff::new(*b"DLG ");
    g.root
        .push(label("EntryList"), Value::List(entries))
        .push(label("ReplyList"), Value::List(replies))
        .push(label("StartingList"), Value::List(vec![start]));
    g.to_bytes().unwrap()
}

#[test]
fn dialog_show_prints_each_nodes_key_and_node_shows_one_with_its_links() {
    let dir = install("dialog-keys");
    let file = dir.join("keys.dlg");
    std::fs::write(&file, keyed_dialog()).unwrap();
    let show = |args: &[&str]| {
        rakata()
            .args(["--game", dir.to_str().unwrap(), "dialog", "show"])
            .arg(&file)
            .args(args)
            .output()
            .unwrap()
    };
    let out = show(&[]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("E0 [voice:cli_vo] (owner): \"Hi.\""),
        "{text}"
    );
    assert!(text.contains("R1 [index:R1 (fragile)] (player)"), "{text}");
    assert!(text.contains("E1 [text:\"Alone.\"] (owner)"), "{text}");

    let out = show(&["--node", "voice:CLI_VO", "--node", "R1"]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains(
            "E0 [voice:cli_vo] (owner): \"Hi.\"\n     voice cli_vo, no file under streamwaves/\n  if k_con_x [missing]\n  -> R0 [index:R0 (fragile)]\n  -> R1 [index:R1 (fragile)]\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("\nR1 [index:R1 (fragile)] (player)"),
        "{text}"
    );
    assert!(!text.contains("E1"), "{text}");

    let out = rakata()
        .args(["--format", "json", "--game", dir.to_str().unwrap()])
        .args(["dialog", "show"])
        .arg(&file)
        .args(["--node", "E0"])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let nodes = doc["result"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["key"], "voice:cli_vo");
    assert_eq!(nodes[0]["fragile"], false);
    assert_eq!(doc["result"]["scripts"][0]["name"], "k_con_x");

    for (key, code) in [
        ("strref:9", "RE1102"),
        ("strref:5", "RE1103"),
        ("E7", "RE1102"),
        ("Q1", "RE1104"),
    ] {
        let out = show(&["--node", key]);
        assert_eq!(out.status.code(), Some(1), "{key}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains(code), "{key}: {err}");
    }
}

#[test]
fn dialog_lint_reports_a_missing_end_script_at_end_with_no_node_key() {
    use rakata_gff::{Gff, Label, Value};
    let dir = install("dialog-end");
    let mut g = Gff::new(*b"DLG ");
    for (field, script) in [("EndConversation", "k_end_gone"), ("EndConverAbort", "")] {
        g.root
            .push(Label::new(field).unwrap(), Value::resref(script).unwrap());
    }
    std::fs::write(dir.join("Override/ending.dlg"), g.to_bytes().unwrap()).unwrap();
    let game = dir.to_str().unwrap();
    let known = dir.join("known.toml");
    let lint = |extra: &[&str]| {
        rakata()
            .args([
                "--format",
                "json",
                "--game",
                game,
                "dialog",
                "lint",
                "ending.dlg",
            ])
            .args(extra)
            .output()
            .unwrap()
    };
    let out = lint(&["--write-known", known.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = &doc["result"]["results"][0]["findings"];
    assert_eq!(findings.as_array().unwrap().len(), 1, "{findings}");
    let f = &findings[0];
    assert_eq!(f["code"], "RW1102");
    assert_eq!(f["at"], "end");
    assert!(f["key"].is_null(), "{f}");
    assert_eq!(f["subject"], "k_end_gone");
    assert_eq!(
        f["message"],
        "ending.dlg EndConversation names the script k_end_gone, which no layer of the install holds"
    );
    let written = std::fs::read_to_string(&known).unwrap();
    assert!(written.contains("at = \"end\""), "{written}");
    let out = lint(&["--known", known.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}
