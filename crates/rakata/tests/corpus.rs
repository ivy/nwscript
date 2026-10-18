//! `rakata corpus decompile` over the user's own game: the acceptance of
//! the decompiled corpus, measured on a real install. `$RAKATA_GAME_K1`
//! names a vanilla KotOR 1 install and `$RAKATA_BUILD` a modded build in
//! the same layout; each test skips, naming its variable, when it is unset.
//!
//! ```bash
//! RAKATA_GAME_K1=… RAKATA_BUILD=… cargo test -p rakata --release --test corpus -- --ignored --nocapture
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use rakata::corpus::{DecompileOptions, decompile};
use rakata::game::Profile;

fn tree(var: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("skipping: set {var} to a game install (the folder holding chitin.key)");
            None
        }
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rakata-corpus-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn header(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .take_while(|l| l.starts_with("// "))
        .map(|l| l[3..].to_string())
        .collect()
}

/// Every file under `dir`, recursively, sorted.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
#[ignore = "reads the game named by RAKATA_GAME_K1"]
fn the_vanilla_modules_decompile_with_headers_folds_and_the_call_index() {
    let Some(game) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let out = scratch("vanilla");
    let o = DecompileOptions {
        game,
        out: out.clone(),
        profile: Profile::K1,
        jobs: 0,
    };
    let (report, warnings) = decompile(&o).unwrap();
    eprintln!("{}", report.text());
    for w in &warnings {
        eprintln!("{}: {}", w.code.id, w.message);
    }
    assert_eq!(report.total.failed, 0);
    // The headline figure of docs/decompiler.md: no regression.
    let fraction = report.total.verified as f64 / report.total.functions as f64;
    assert!(
        fraction >= 0.999,
        "{} of {} verified",
        report.total.verified,
        report.total.functions
    );
    assert!(report.total.folded_functions > report.total.functions / 2);
    assert_eq!(report.total.unfolded, 0);

    let decompiled = out.join("decompiled");
    // Every file starts with the header.
    let mut hits = Vec::new();
    for p in files(&decompiled) {
        if p.extension().is_some_and(|x| x == "nss") {
            let h = header(&p);
            assert!(
                h.first().is_some_and(|l| l.contains(".ncs from ")),
                "{}: no header",
                p.display()
            );
            assert!(
                h.iter().any(|l| l.starts_with("loaded: ")),
                "{}",
                p.display()
            );
            assert!(
                h.iter().any(|l| l.starts_with("functions: ")),
                "{}",
                p.display()
            );
        }
        if fs::read(&p)
            .unwrap()
            .windows(12)
            .any(|w| w == b"k_pman_aioff")
        {
            hits.push(p.strip_prefix(&decompiled).unwrap().to_path_buf());
        }
    }
    // The cause of the Juhani mod's bug is found by grep: the script that
    // runs k_pman_aioff on the party, with the call index beside it, and
    // nothing but scripts otherwise.
    assert!(
        hits.contains(&PathBuf::from("danm14ac_s/k_pdan_juhani27.nss")),
        "{hits:?}"
    );
    assert!(hits.contains(&PathBuf::from("calls.jsonl")), "{hits:?}");
    assert!(
        hits.iter()
            .all(|h| h.extension().is_some_and(|x| x == "nss") || h == Path::new("calls.jsonl")),
        "{hits:?}"
    );
    let juhani = header(&decompiled.join("danm14ac_s/k_pdan_juhani27.nss"));
    assert_eq!(
        juhani[0],
        "k_pdan_juhani27.ncs from modules/danm14ac_s.rim (module danm14ac)"
    );
    assert!(
        juhani.iter().any(|l| l.contains("#include \"k_inc_man\"")),
        "{juhani:?}"
    );

    // One row per engine call: k_pdan_juhani27 calls ExecuteScript with
    // k_pman_aioff first.
    let calls = fs::read_to_string(decompiled.join("calls.jsonl")).unwrap();
    let row = calls
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .find(|r| r["script"] == "k_pdan_juhani27" && r["routine"] == "ExecuteScript")
        .expect("a row for the call");
    assert_eq!(row["capsule"], "danm14ac_s");
    assert_eq!(row["args"][0], "k_pman_aioff");
    assert!(row["offset"].is_u64());

    // Idempotent: a second run writes the same tree.
    let before: Vec<(PathBuf, Vec<u8>)> = files(&decompiled)
        .into_iter()
        .map(|p| (p.clone(), fs::read(&p).unwrap()))
        .collect();
    decompile(&o).unwrap();
    let after: Vec<(PathBuf, Vec<u8>)> = files(&decompiled)
        .into_iter()
        .map(|p| (p.clone(), fs::read(&p).unwrap()))
        .collect();
    assert!(before == after, "the second run wrote a different tree");
    fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "reads the build named by RAKATA_BUILD"]
fn a_script_in_a_module_capsule_and_override_names_override_as_the_loaded_copy() {
    let Some(game) = tree("RAKATA_BUILD") else {
        return;
    };
    let out = scratch("build");
    let (report, _) = decompile(&DecompileOptions {
        game,
        out: out.clone(),
        profile: Profile::K1,
        jobs: 0,
    })
    .unwrap();
    eprintln!("{}", report.text());
    assert!(
        report.sources.contains_key("override"),
        "{:?}",
        report.sources.keys()
    );
    let decompiled = out.join("decompiled");
    let mut checked = 0;
    for p in files(&decompiled.join("override")) {
        let name = p.file_stem().unwrap().to_string_lossy().into_owned();
        let h = header(&p);
        assert!(
            h.iter()
                .any(|l| l.starts_with("loaded: this copy, Override/")),
            "{h:?}"
        );
        // The same script in a module capsule names Override as the copy
        // the engine loads.
        for d in fs::read_dir(&decompiled).unwrap().filter_map(Result::ok) {
            let twin = d.path().join(format!("{name}.nss"));
            if d.file_name() != "override" && twin.is_file() {
                let h = header(&twin);
                let loaded = h.iter().find(|l| l.starts_with("loaded: ")).unwrap();
                assert!(
                    loaded.contains("Override/") && loaded.ends_with(", not this copy"),
                    "{}: {loaded}",
                    twin.display()
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no Override script is also in a capsule");
    fs::remove_dir_all(&out).unwrap();
}
