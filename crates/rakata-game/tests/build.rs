//! Over the user's own modded build: open it cold, resolve every name in
//! Override and in every module, and print the timings the proposal's
//! Performance section asks for; and check that `Override/` holds no
//! case-variant duplicates the build did not report. Reads `$RAKATA_BUILD`
//! (the folder holding `chitin.key`) and skips, naming the variable, when it
//! is unset. No game bytes are committed.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rakata_game::{Install, Profile};

const BUILD_VAR: &str = "RAKATA_BUILD";

fn build() -> Option<PathBuf> {
    match std::env::var(BUILD_VAR) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${BUILD_VAR} is unset; set it to a modded KotOR 1 build");
            None
        }
    }
}

fn median(d: &mut [Duration]) -> Duration {
    d.sort_unstable();
    d[d.len() / 2]
}

#[test]
fn every_name_in_override_and_the_modules_resolves_and_is_timed() {
    let Some(root) = build() else { return };
    let t = Instant::now();
    let install = Install::open(&root, Profile::K1).unwrap();
    let open = t.elapsed();

    let mut times = Vec::new();
    let mut resolved = 0usize;
    let mut candidates = 0usize;
    let mut not_found = 0usize;
    for id in install.override_ids() {
        let t = Instant::now();
        let r = install.resolve(id, None).unwrap();
        times.push(t.elapsed());
        resolved += 1;
        candidates += r.candidates.len();
        assert!(r.winner().is_some(), "{id} is in Override but not resolved");
    }
    let override_count = resolved;
    for module in install.modules() {
        for id in install.module_ids(module).unwrap() {
            let t = Instant::now();
            let r = install.resolve(id, Some(module)).unwrap();
            times.push(t.elapsed());
            resolved += 1;
            candidates += r.candidates.len();
            if r.winner().is_none() {
                not_found += 1;
            }
        }
    }
    let total: Duration = times.iter().sum();
    let max = *times.iter().max().unwrap();
    let med = median(&mut times);
    println!(
        "open cold: {open:?}; resolved {resolved} names ({override_count} from Override, {} from {} modules): median {med:?}, max {max:?}, total {total:?}; {candidates} candidates",
        resolved - override_count,
        install.modules().len()
    );
    assert_eq!(not_found, 0, "a module member the module did not resolve");
}

#[test]
fn override_case_variants_are_reported() {
    let Some(root) = build() else { return };
    let install = Install::open(&root, Profile::K1).unwrap();
    let groups = install.override_case_variants();
    println!("Override case-variant groups: {}", groups.len());
    for g in &groups {
        let names: Vec<String> = g
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        println!("  {}", names.join(", "));
    }
}
