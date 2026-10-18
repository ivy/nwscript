//! The round-trip census over a KotOR 1 install: every archive lists, and
//! every archive read and written again is compared to its source, with the
//! counts of identical and different printed and each difference named with
//! its cause. Also the zero-copy check on the largest BIF and the timing of
//! a listing of every archive, read whole and memory-mapped.
//!
//! Reads the user's own install from `$RAKATA_GAME_K1` (the folder holding
//! `chitin.key`) and skips, naming the variable, when it is unset. No game
//! bytes are committed.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rakata_archive::{Bif, Erf, Key, Rim, Source};

const GAME_VAR: &str = "RAKATA_GAME_K1";

fn game() -> Option<PathBuf> {
    match std::env::var(GAME_VAR) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${GAME_VAR} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

/// A child of `dir` named `name` in any case, as the game finds it.
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// Files in `dir` whose extension is one of `exts`, sorted by name.
fn files_with(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
        })
        .collect();
    files.sort();
    files
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Key,
    Bif,
    Erf,
    Rim,
}

/// Every archive of the install with its kind: `chitin.key` and the BIFs it
/// names, `modules/*.rim|*.mod`, `rims/*.rim`, `lips/*.mod`, `patch.erf`
/// and `texturepacks/*.erf`.
fn archives(root: &Path) -> Vec<(Kind, PathBuf)> {
    let key_path = child_ci(root, "chitin.key").expect("chitin.key");
    let key = Key::parse(&fs::read(&key_path).unwrap()).unwrap();
    let mut out = vec![(Kind::Key, key_path)];
    for b in key.bifs() {
        let mut p = root.to_path_buf();
        for part in b.components() {
            p = child_ci(&p, &part)
                .unwrap_or_else(|| panic!("{} names missing {}", b.name_str(), part));
        }
        out.push((Kind::Bif, p));
    }
    let dir = |name: &str| child_ci(root, name).unwrap_or_else(|| root.join(name));
    for p in files_with(&dir("modules"), &["rim"])
        .into_iter()
        .chain(files_with(&dir("rims"), &["rim"]))
    {
        out.push((Kind::Rim, p));
    }
    for p in files_with(&dir("modules"), &["mod"])
        .into_iter()
        .chain(files_with(&dir("lips"), &["mod"]))
        .chain(child_ci(root, "patch.erf"))
        .chain(files_with(&dir("texturepacks"), &["erf"]))
    {
        out.push((Kind::Erf, p));
    }
    out
}

/// The archive in `bytes` listed and written again: the member count and
/// the bytes written.
fn round_trip(kind: Kind, bytes: &[u8]) -> (usize, Vec<u8>) {
    match kind {
        Kind::Key => {
            let k = Key::parse(bytes).unwrap();
            (k.entries().len(), k.to_bytes().unwrap())
        }
        Kind::Bif => {
            let b = Bif::parse(bytes).unwrap();
            (b.len(), b.to_bytes().unwrap())
        }
        Kind::Erf => {
            let e = Erf::parse(bytes).unwrap();
            (e.len(), e.to_bytes().unwrap())
        }
        Kind::Rim => {
            let r = Rim::parse(bytes).unwrap();
            (r.len(), r.to_bytes().unwrap())
        }
    }
}

/// Why a written archive differs from its source, from the bytes: where
/// the first difference lies and how the member layout differs.
fn cause(kind: Kind, source: &[u8], written: &[u8]) -> String {
    let first = source
        .iter()
        .zip(written)
        .position(|(a, b)| a != b)
        .unwrap_or(source.len().min(written.len()));
    let layout = match kind {
        Kind::Rim => {
            let r = Rim::parse(source).unwrap();
            let count = r.len();
            // The gaps between members as read, from the table.
            let table = 120;
            let mut offsets: Vec<(usize, usize)> = (0..count)
                .map(|i| {
                    let e = table + i * 32;
                    let at = |o: usize| {
                        u32::from_le_bytes(source[e + o..e + o + 4].try_into().unwrap()) as usize
                    };
                    (at(24), at(28))
                })
                .collect();
            offsets.sort();
            let first_gap = offsets.first().map(|&(o, _)| o - (table + count * 32));
            let mut gaps: Vec<usize> = offsets
                .windows(2)
                .map(|w| w[1].0 - (w[0].0 + w[0].1))
                .collect();
            gaps.sort_unstable();
            gaps.dedup();
            format!(
                "flag {}, {count} members, {} after the table, member gaps {gaps:?}, {} trailing",
                r.flag,
                first_gap.map_or("nothing".to_string(), |g| format!("{g} bytes")),
                offsets.last().map_or(0, |&(o, l)| source.len() - (o + l))
            )
        }
        _ => String::new(),
    };
    format!(
        "{} vs {} bytes, first difference at {first}; {layout}",
        source.len(),
        written.len()
    )
}

#[test]
fn every_vanilla_archive_lists_and_writes_back_byte_identical() {
    let Some(root) = game() else { return };
    let list = archives(&root);
    let mut per_kind = std::collections::BTreeMap::new();
    let mut members = 0usize;
    let mut identical = 0usize;
    let mut different = Vec::new();
    for (kind, path) in &list {
        let source = Source::map(path).unwrap();
        let (n, written) = round_trip(*kind, &source);
        members += n;
        *per_kind.entry(format!("{kind:?}")).or_insert(0usize) += 1;
        if written == *source {
            identical += 1;
        } else {
            different.push((
                path.strip_prefix(&root).unwrap().display().to_string(),
                cause(*kind, &source, &written),
            ));
        }
    }
    println!("{} archives, {members} members: {per_kind:?}", list.len());
    println!(
        "written back: {identical} identical, {} different",
        different.len()
    );
    for (path, cause) in &different {
        println!("  {path}: {cause}");
    }
    assert!(
        different.is_empty(),
        "written archives differ from their source: {different:#?}"
    );
    assert_eq!(list.len(), identical);
}

/// Listing a BIF of a hundred megabytes copies no member: every member's
/// bytes lie inside the mapped source.
#[test]
fn listing_the_largest_bif_copies_no_member() {
    let Some(root) = game() else { return };
    let (_, path) = archives(&root)
        .into_iter()
        .filter(|(k, _)| *k == Kind::Bif)
        .max_by_key(|(_, p)| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .unwrap();
    let size = fs::metadata(&path).unwrap().len();
    let start = Instant::now();
    let source = Source::map(&path).unwrap();
    let bif = Bif::parse(&source).unwrap();
    let elapsed = start.elapsed();
    let range = source.as_ptr_range();
    let mut listed = 0usize;
    for m in bif.members() {
        assert!(m.is_borrowed());
        assert!(range.contains(&m.data().as_ptr()) || m.is_empty());
        listed += m.len();
    }
    println!(
        "{}: {} bytes, {} members ({listed} bytes of members) listed in {elapsed:?} with no copies",
        path.display(),
        size,
        bif.len()
    );
    assert!(size > 100 << 20, "{} is under 100 MB", path.display());
}

/// Cold open plus a full listing of every vanilla archive, read whole and
/// memory-mapped: the figures that decide whether the mapped `Source` earns
/// its `unsafe`. Printed for `--nocapture`; the test only runs both ways.
#[test]
fn listing_every_archive_read_whole_and_mapped() {
    let Some(root) = game() else { return };
    let list = archives(&root);
    let mut bytes = 0u64;
    let time = |open: &dyn Fn(&Path) -> Source| -> Duration {
        let start = Instant::now();
        for (kind, path) in &list {
            let source = open(path);
            match kind {
                Kind::Key => drop(Key::parse(&source).unwrap()),
                Kind::Bif => drop(Bif::parse(&source).unwrap()),
                Kind::Erf => drop(Erf::parse(&source).unwrap()),
                Kind::Rim => drop(Rim::parse(&source).unwrap()),
            }
        }
        start.elapsed()
    };
    for (_, path) in &list {
        bytes += fs::metadata(path).unwrap().len();
    }
    let mapped = time(&|p| Source::map(p).unwrap());
    let read = time(&|p| Source::read(p).unwrap());
    let mapped_again = time(&|p| Source::map(p).unwrap());
    println!(
        "{} archives, {} MB: listed in {read:?} read whole, {mapped:?} mapped (first), {mapped_again:?} mapped (again)",
        list.len(),
        bytes >> 20
    );
}
