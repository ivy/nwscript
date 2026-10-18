//! The round trip over the user's own talk tables: the vanilla `dialog.tlk`
//! under `$RAKATA_GAME_K1` read and written again must be the file it came
//! from; the modded build's under `$RAKATA_BUILD`, written by another tool,
//! must at least read back to the same table, and its strref 42724 (the
//! area-transition refusal) must carry text. Each test skips, naming its
//! variable, when it is unset. No game bytes are committed.

use std::path::PathBuf;

use rakata_res::StrRef;
use rakata_tlk::Tlk;

fn tree(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${var} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

fn dialog_tlk(root: &std::path::Path) -> PathBuf {
    std::fs::read_dir(root)
        .ok()
        .and_then(|d| {
            d.filter_map(Result::ok)
                .find(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case("dialog.tlk")
                })
                .map(|e| e.path())
        })
        .unwrap_or_else(|| root.join("dialog.tlk"))
}

#[test]
fn vanilla_dialog_tlk_is_identical_after_a_round_trip() {
    let Some(root) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let path = dialog_tlk(&root);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let tlk = Tlk::parse(&bytes).unwrap_or_else(|e| panic!("{e}"));
    eprintln!(
        "{}: {} entries, language {:?}",
        path.display(),
        tlk.len(),
        tlk.language
    );
    let written = tlk.to_bytes().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(written.len(), bytes.len(), "written length differs");
    let first = written.iter().zip(&bytes).position(|(a, b)| a != b);
    assert_eq!(first, None, "first differing byte");
}

#[test]
fn build_dialog_tlk_reads_back_and_names_the_refusal() {
    let Some(root) = tree("RAKATA_BUILD") else {
        return;
    };
    let path = dialog_tlk(&root);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let tlk = Tlk::parse(&bytes).unwrap_or_else(|e| panic!("{e}"));
    let refusal = tlk
        .text(StrRef::new(42724))
        .expect("strref 42724 is in the build's table");
    eprintln!("strref 42724: {refusal:?}");
    assert!(!refusal.is_empty(), "strref 42724 has no text");
    let written = tlk.to_bytes().unwrap_or_else(|e| panic!("{e}"));
    let again = Tlk::parse(&written).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(again, tlk, "the written table reads back differently");
    match written.iter().zip(&bytes).position(|(a, b)| a != b) {
        None if written.len() == bytes.len() => eprintln!("{}: identical", path.display()),
        at => eprintln!(
            "{}: equal after re-read; first difference at {:?}",
            path.display(),
            at.or(Some(bytes.len().min(written.len())))
        ),
    }
}
