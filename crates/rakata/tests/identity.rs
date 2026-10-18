//! The build identity, checked against a repository the test makes: the
//! same code `build.rs` runs, included from `build/identity.rs`.

#[path = "../build/identity.rs"]
#[allow(dead_code)]
mod identity;

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let st = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .status()
        .unwrap();
    assert!(st.success(), "git {args:?} failed");
}

/// A fresh repository with one committed file, in a directory of its own.
fn repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rakata-identity-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    std::fs::write(dir.join("a.txt"), "one\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-q", "-m", "one"]);
    dir
}

#[test]
fn a_clean_checkout_is_version_plus_describe() {
    let dir = repo("clean");
    let co = identity::checkout(&dir).unwrap();
    let id = identity::identity("0.0.0", Some(&co));
    let head = String::from_utf8(
        Command::new("git")
            .args(["-C", dir.to_str().unwrap(), "rev-parse", "--short", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert_eq!(id, format!("0.0.0+{}", head.trim()));
    assert!(!id.contains("dirty"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_changed_file_changes_the_identity_and_marks_it_dirty() {
    let dir = repo("dirty");
    let co = identity::checkout(&dir).unwrap();
    let clean = identity::identity("0.0.0", Some(&co));
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    let changed = identity::identity("0.0.0", Some(&co));
    assert_ne!(clean, changed);
    assert!(changed.starts_with(&format!("{clean}.dirty.")), "{changed}");
    let hash = changed.rsplit('.').next().unwrap();
    assert_eq!(hash.len(), 8);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    // Another change, another hash; the same change, the same hash.
    std::fs::write(dir.join("a.txt"), "three\n").unwrap();
    let again = identity::identity("0.0.0", Some(&co));
    assert_ne!(changed, again);
    std::fs::write(dir.join("a.txt"), "two\n").unwrap();
    assert_eq!(identity::identity("0.0.0", Some(&co)), changed);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_new_untracked_file_changes_the_identity() {
    let dir = repo("untracked");
    let co = identity::checkout(&dir).unwrap();
    let clean = identity::identity("0.0.0", Some(&co));
    std::fs::write(dir.join("b.txt"), "new\n").unwrap();
    let with_file = identity::identity("0.0.0", Some(&co));
    assert_ne!(clean, with_file);
    std::fs::write(dir.join("b.txt"), "other\n").unwrap();
    assert_ne!(with_file, identity::identity("0.0.0", Some(&co)));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn outside_a_checkout_the_version_stands_alone() {
    assert_eq!(identity::identity("0.0.0", None), "0.0.0");
}

#[test]
fn watched_paths_name_the_tracked_files_and_the_metadata() {
    let dir = repo("watched");
    let co = identity::checkout(&dir).unwrap();
    let paths = identity::watched_paths(&co);
    assert!(paths.contains(&co.root.join("a.txt")));
    assert!(paths.contains(&co.git_dir.join("HEAD")));
    assert!(paths.contains(&co.git_dir.join("index")));
    std::fs::remove_dir_all(dir).unwrap();
}
