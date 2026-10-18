//! The build identity: the workspace version, then the git describe of the
//! checkout the binary was built from and, when that checkout had
//! uncommitted changes, a short hash of them, as semver build metadata:
//! `0.0.0+49a9b40` from a clean checkout, `0.0.0+49a9b40.dirty.3fa1c2d8` from
//! one with changes. A build that is not from a git checkout (a crates.io
//! tarball, or no `git` on the path) is identified by the version alone.
//!
//! The dirty hash covers `git diff HEAD` (staged and unstaged changes to
//! tracked files) and the contents of untracked, unignored files, so a
//! change in the checkout changes the identity without a commit. This is
//! what a build system keys a step on when it uses a local build.
//!
//! This file is included by `build.rs`, which computes the identity, and by
//! the crate's tests, which check it against a repository they make.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// What `git` knows about a checkout.
pub struct Checkout {
    /// The working tree's root.
    pub root: PathBuf,
    /// The repository's metadata directory (`.git`, or a worktree's).
    pub git_dir: PathBuf,
}

/// The checkout holding `dir`, if `dir` is inside one and `git` runs.
pub fn checkout(dir: &Path) -> Option<Checkout> {
    let root = git(dir, &["rev-parse", "--show-toplevel"])?;
    let git_dir = git(dir, &["rev-parse", "--absolute-git-dir"])?;
    Some(Checkout {
        root: PathBuf::from(String::from_utf8(root).ok()?.trim_end()),
        git_dir: PathBuf::from(String::from_utf8(git_dir).ok()?.trim_end()),
    })
}

/// The identity string for `version` built from `checkout`.
pub fn identity(version: &str, checkout: Option<&Checkout>) -> String {
    let Some(co) = checkout else {
        return version.to_string();
    };
    let describe = git(&co.root, &["describe", "--always", "--tags"])
        .and_then(|b| String::from_utf8(b).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let Some(describe) = describe else {
        // A repository without a commit yet has nothing to describe.
        return version.to_string();
    };
    match dirty_hash(co) {
        Some(h) => format!("{version}+{describe}.dirty.{h}"),
        None => format!("{version}+{describe}"),
    }
}

/// Eight hex digits over the uncommitted changes, or `None` when the tree
/// matches `HEAD`.
fn dirty_hash(co: &Checkout) -> Option<String> {
    let status = git(
        &co.root,
        &["status", "--porcelain", "--untracked-files=all", "-z"],
    )?;
    if status.is_empty() {
        return None;
    }
    let mut h = Sha256::new();
    let diff = git(&co.root, &["diff", "HEAD", "--no-ext-diff", "--binary"])?;
    h.update(&diff);
    for path in untracked(&status) {
        let contents = std::fs::read(co.root.join(&path)).unwrap_or_default();
        h.update(path.as_bytes());
        h.update([0]);
        h.update((contents.len() as u64).to_le_bytes());
        h.update(&contents);
    }
    let digest = h.finalize();
    Some(digest.iter().take(4).map(|b| format!("{b:02x}")).collect())
}

/// The untracked paths in `git status --porcelain -z` output, sorted.
fn untracked(status: &[u8]) -> Vec<String> {
    let mut paths: Vec<String> = status
        .split(|&b| b == 0)
        .filter_map(|entry| entry.strip_prefix(b"?? "))
        .filter_map(|p| String::from_utf8(p.to_vec()).ok())
        .collect();
    paths.sort();
    paths
}

/// The paths whose change should recompute the identity: every tracked
/// file, and the repository metadata a commit or checkout writes.
pub fn watched_paths(co: &Checkout) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(list) = git(&co.root, &["ls-files", "-z"]) {
        paths.extend(
            list.split(|&b| b == 0)
                .filter(|p| !p.is_empty())
                .filter_map(|p| String::from_utf8(p.to_vec()).ok())
                .map(|p| co.root.join(p)),
        );
    }
    for meta in ["HEAD", "index", "packed-refs", "refs"] {
        let p = co.git_dir.join(meta);
        if p.exists() {
            paths.push(p);
        }
    }
    paths
}

/// Run `git` in `dir`; `None` when it fails or is absent.
fn git(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}
