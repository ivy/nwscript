//! The round-trip census over the game's own lip-sync files: every member of
//! `lips/*.mod` in a KotOR 1 install is read and written again and compared
//! to its source, and the facts the layout leaves open are counted (time
//! order, duplicated times, the duration against the last keyframe, empty
//! files, the shapes in use). The loose `.lip` files of a modded build's
//! `Override/` are measured the same way.
//!
//! Reads the user's own trees from `$RAKATA_GAME_K1` (the folder holding
//! `chitin.key`) and `$RAKATA_BUILD`, and skips, naming the variable, when
//! one is unset. No game bytes are committed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rakata_archive::Erf;
use rakata_lip::{Lip, Shape};

fn tree(var: &str, what: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${var} is unset; set it to {what}");
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

/// Files in `dir` with extension `ext` in any case, sorted by name.
fn files_with(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .collect();
    files.sort();
    files
}

/// What a set of files teaches, counted.
#[derive(Default)]
struct Census {
    files: usize,
    identical: usize,
    different: Vec<String>,
    failed: Vec<String>,
    unsorted: Vec<String>,
    duplicated_time: Vec<String>,
    empty: Vec<String>,
    last_after_length: Vec<String>,
    last_before_length: Vec<String>,
    last_at_length: usize,
    first_not_zero: usize,
    first_not_rest: usize,
    last_not_rest: Vec<String>,
    keys: usize,
    shapes: BTreeMap<Shape, usize>,
    bytes: usize,
}

impl Census {
    fn add(&mut self, name: &str, bytes: &[u8]) {
        self.files += 1;
        self.bytes += bytes.len();
        let lip = match Lip::parse(bytes) {
            Ok(l) => l,
            Err(e) => {
                self.failed.push(format!("{name}: {e}"));
                return;
            }
        };
        match lip.to_bytes() {
            Ok(w) if w == bytes => self.identical += 1,
            Ok(w) => self.different.push(format!(
                "{name}: {} bytes written for {}",
                w.len(),
                bytes.len()
            )),
            Err(e) => self.different.push(format!("{name}: {e}")),
        }
        if !lip.is_sorted() {
            self.unsorted.push(name.to_string());
        }
        if let Some(i) = lip
            .keys
            .windows(2)
            .position(|w| w[0].time.to_bits() == w[1].time.to_bits())
        {
            let (a, b) = (lip.keys[i], lip.keys[i + 1]);
            self.duplicated_time.push(format!(
                "{name}: keys {i} and {} (offsets {:#x} and {:#x}) both at {} ({:#010x}), shapes {} then {}",
                i + 1,
                rakata_lip::HEADER_LEN + i * rakata_lip::KEY_LEN,
                rakata_lip::HEADER_LEN + (i + 1) * rakata_lip::KEY_LEN,
                a.time,
                a.time.to_bits(),
                a.shape,
                b.shape
            ));
        }
        self.keys += lip.keys.len();
        for k in &lip.keys {
            *self.shapes.entry(k.shape).or_default() += 1;
        }
        match (lip.keys.first(), lip.keys.last()) {
            (None, _) | (_, None) => self.empty.push(name.to_string()),
            (Some(first), Some(last)) => {
                if first.time != 0.0 {
                    self.first_not_zero += 1;
                }
                if first.shape != Shape::Rest {
                    self.first_not_rest += 1;
                }
                if last.shape != Shape::Rest {
                    self.last_not_rest
                        .push(format!("{name}: ends on {}", last.shape));
                }
                if last.time > lip.length {
                    self.last_after_length.push(format!(
                        "{name}: last {} > length {}",
                        last.time, lip.length
                    ));
                } else if last.time < lip.length {
                    self.last_before_length.push(format!(
                        "{name}: last {} < length {}",
                        last.time, lip.length
                    ));
                } else {
                    self.last_at_length += 1;
                }
            }
        }
    }

    fn report(&self, what: &str) {
        let show = |label: &str, v: &[String]| {
            println!("  {label}: {}", v.len());
            for s in v.iter().take(8) {
                println!("    {s}");
            }
            if v.len() > 8 {
                println!("    ... and {} more", v.len() - 8);
            }
        };
        println!(
            "{what}: {} files, {} bytes, {} keyframes",
            self.files, self.bytes, self.keys
        );
        println!("  identical: {}", self.identical);
        show("different", &self.different);
        show("failed", &self.failed);
        show("not in time order", &self.unsorted);
        show("with a duplicated time", &self.duplicated_time);
        show("with zero keyframes", &self.empty);
        show("last keyframe after the duration", &self.last_after_length);
        show(
            "last keyframe before the duration",
            &self.last_before_length,
        );
        println!("  last keyframe at the duration: {}", self.last_at_length);
        println!(
            "  first keyframe not at 0: {}; first not rest: {}",
            self.first_not_zero, self.first_not_rest
        );
        show("last keyframe not rest", &self.last_not_rest);
        println!("  shapes:");
        for (s, n) in &self.shapes {
            println!("    {:>2} {:<4} {n}", s.byte(), s.label());
        }
    }
}

#[test]
fn every_vanilla_lip_round_trips_byte_for_byte() {
    let Some(root) = tree("RAKATA_GAME_K1", "a KotOR 1 install") else {
        return;
    };
    let lips = child_ci(&root, "lips").expect("the install has a lips folder");
    let mut census = Census::default();
    let mut archives = 0;
    for path in files_with(&lips, "mod") {
        let bytes = fs::read(&path).unwrap();
        let erf = Erf::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        archives += 1;
        let stem = path.file_name().unwrap().to_string_lossy().into_owned();
        for m in erf.members() {
            assert_eq!(m.id.ty.extension(), Some("lip"), "{stem} holds {}", m.id);
            census.add(&format!("{stem}/{}", m.id), m.data());
        }
    }
    println!("{archives} archives under lips/");
    census.report("vanilla lips/*.mod members");
    assert!(census.files > 0, "no LIP members found");
    assert!(
        census.failed.is_empty(),
        "files refused: {:#?}",
        census.failed
    );
    assert!(
        census.different.is_empty(),
        "files not reproduced: {:#?}",
        census.different
    );
    assert_eq!(census.identical, census.files);
}

#[test]
fn every_loose_lip_in_the_build_round_trips_byte_for_byte() {
    let Some(root) = tree("RAKATA_BUILD", "a modded KotOR 1 build") else {
        return;
    };
    let override_dir = child_ci(&root, "override").expect("the build has an Override folder");
    let mut census = Census::default();
    for path in files_with(&override_dir, "lip") {
        let bytes = fs::read(&path).unwrap();
        census.add(&path.file_name().unwrap().to_string_lossy(), &bytes);
    }
    census.report("build Override/*.lip");
    assert!(census.files > 0, "no loose LIP files found");
    assert!(
        census.failed.is_empty(),
        "files refused: {:#?}",
        census.failed
    );
    assert!(
        census.different.is_empty(),
        "files not reproduced: {:#?}",
        census.different
    );
}
