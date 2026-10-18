//! The install section: each installer of `conformance/bench.toml` on each
//! install case, onto a fresh reflink copy of the case's tree, timed; the
//! first run's tree is then compared with `rakata install`'s, path by path
//! and, for a format Rakata reads, by its data.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::formats;
use super::host::{self, Stats};
use super::report::{InstallRow, Listed};
use super::{BenchFile, InstallCase, RunOpts, Subst, identities, progress};

const LISTED: usize = 40;

pub fn run(bench: &BenchFile, subst: &Subst, opts: &RunOpts) -> Vec<InstallRow> {
    let mut rows = Vec::new();
    let root = opts.work.join("install");
    for (name, case) in &bench.install {
        if !RunOpts::wants(&opts.cases, name) {
            continue;
        }
        let installers: Vec<_> = bench
            .installer
            .iter()
            .filter(|(i, _)| RunOpts::wants(&opts.installers, i))
            .collect();
        let first = rows.len();
        for (inst, def) in &installers {
            rows.push(InstallRow {
                case: name.clone(),
                case_description: case.description.clone(),
                installer: (*inst).clone(),
                description: def.description.clone(),
                command: def.command.clone(),
                identity: identities(subst, &def.command, &def.identity),
                option: case.option,
                ..Default::default()
            });
        }
        let prepared = prepare(name, case, subst, &root);
        let (mod_dir, data_rel, tree, number) = match prepared {
            Ok(p) => p,
            Err(e) => {
                for r in &mut rows[first..] {
                    r.skipped = Some(e.clone());
                }
                continue;
            }
        };
        let mut results: BTreeMap<String, PathBuf> = BTreeMap::new();
        for (k, (inst, def)) in installers.iter().enumerate() {
            let row = &mut rows[first + k];
            let template = match subst.all(&def.command) {
                Ok(t) => t,
                Err(e) => {
                    row.skipped = Some(e);
                    continue;
                }
            };
            let option_args: Vec<String> = match (number, case.option) {
                (Some(n), Some(o)) => def
                    .option_args
                    .iter()
                    .map(|a| {
                        a.replace("{option}", &o.to_string())
                            .replace("{option_number}", &n.to_string())
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let (mut times, mut rss) = (Vec::new(), None::<u64>);
            for rep in 0..opts.install_reps {
                progress(format!(
                    "install {name} with {inst}, run {}/{}",
                    rep + 1,
                    opts.install_reps
                ));
                let run_dir = root.join("run").join(format!("{name}-{inst}"));
                let _ = fs::remove_dir_all(&run_dir);
                let game = run_dir.join("game");
                let moddir = run_dir.join("mod");
                let log = root.join("logs").join(name).join(format!("{inst}-{rep}"));
                let _ = fs::remove_dir_all(&log);
                let copied = fs::create_dir_all(&run_dir)
                    .map_err(|e| e.to_string())
                    .and_then(|()| copy_tree(&tree, &game))
                    .and_then(|()| copy_tree(&mod_dir, &moddir))
                    .and_then(|()| fs::create_dir_all(&log).map_err(|e| e.to_string()));
                if let Err(e) = copied {
                    row.skipped = Some(format!("preparing the run: {e}"));
                    break;
                }
                let data = moddir.join(&data_rel);
                let mut argv = Vec::new();
                for a in &template {
                    if a == "{option_args}" {
                        argv.extend(option_args.iter().cloned());
                        continue;
                    }
                    argv.push(
                        a.replace("{data}", &data.display().to_string())
                            .replace("{game}", &game.display().to_string())
                            .replace("{option}", &case.option.unwrap_or(0).to_string())
                            .replace("{log}", &log.display().to_string()),
                    );
                }
                let m = host::run(
                    &argv,
                    Some(&run_dir),
                    opts.time.as_deref(),
                    &opts.work,
                    opts.timeout,
                );
                times.push(m.wall);
                rss = rss.max(m.rss_kb);
                row.exits.push(m.code);
                if m.code != Some(0) && !m.stderr.is_empty() && rep == 0 {
                    row.notes.push(format!("exit {:?}: {}", m.code, m.stderr));
                }
                if rep == 0 {
                    let keep = root.join("result").join(name).join(inst);
                    let _ = fs::remove_dir_all(&keep);
                    let _ = fs::create_dir_all(keep.parent().expect("has a parent"));
                    if fs::rename(&game, &keep).is_ok() {
                        let changed = changed(&tree, &keep);
                        row.changed = changed.len();
                        results.insert((*inst).clone(), keep);
                    }
                }
                let _ = fs::remove_dir_all(&run_dir);
            }
            row.time = Stats::of(&times);
            row.rss_mb = rss.map(|k| k as f64 / 1024.0);
        }
        // Each result against rakata's.
        if let Some(ours) = results.get("rakata") {
            let ours_changed: BTreeSet<String> = changed(&tree, ours).into_iter().collect();
            for row in rows[first..].iter_mut() {
                let Some(theirs) = results.get(&row.installer) else {
                    continue;
                };
                if row.installer == "rakata" {
                    continue;
                }
                let mut paths = ours_changed.clone();
                paths.extend(changed(&tree, theirs));
                let mut cmp = Comparison::default();
                for p in &paths {
                    compare_path(&mut cmp, p, &ours.join(p), &theirs.join(p));
                }
                row.against_rakata = Some(cmp.counts);
                row.listed = cmp.listed;
            }
        }
        for p in results.values() {
            let _ = fs::remove_dir_all(p);
        }
    }
    rows
}

/// Unpack the case's archive once, find its `tslpatchdata`, and resolve
/// the option's number.
fn prepare(
    name: &str,
    case: &InstallCase,
    subst: &Subst,
    root: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf, Option<u32>), String> {
    let archive = PathBuf::from(subst.fixed(&case.archive)?);
    let tree = PathBuf::from(subst.fixed(&case.tree)?);
    if !archive.is_file() {
        return Err(format!("no archive at {}", case.archive));
    }
    if !tree.join("chitin.key").is_file() {
        return Err(format!("{} is not a game tree", case.tree));
    }
    let dir = root.join("mods").join(name);
    if !dir.is_dir() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let ok = Command::new("7zz")
            .arg("x")
            .arg("-y")
            .arg(format!("-o{}", dir.display()))
            .arg(&archive)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            let _ = fs::remove_dir_all(&dir);
            return Err(format!("7zz could not unpack {}", case.archive));
        }
    }
    let data_rel = PathBuf::from(&case.data);
    let data = dir.join(&data_rel);
    if !data.is_dir() {
        return Err(format!(
            "no {} in the unpacked archive {}",
            case.data,
            dir.display()
        ));
    }
    let number = match case.option {
        Some(o) => option_number(&data, o)?,
        None => None,
    };
    Ok((dir, data_rel, tree, number))
}

/// The `Namespace#` number of the option at 0-based `index` in
/// `namespaces.ini`'s `[Namespaces]`; `None` without a namespaces.ini.
fn option_number(data: &Path, index: u32) -> Result<Option<u32>, String> {
    let Some(ns) = fs::read_dir(data).ok().and_then(|d| {
        d.flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .is_some_and(|n| n.eq_ignore_ascii_case("namespaces.ini"))
            })
    }) else {
        return Ok(None);
    };
    let text = String::from_utf8_lossy(&fs::read(&ns).map_err(|e| e.to_string())?).into_owned();
    let mut in_section = false;
    let mut numbers = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_section = line.eq_ignore_ascii_case("[namespaces]");
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((k, _)) = line.split_once('=') {
            let k = k.trim().to_ascii_lowercase();
            if let Some(n) = k.strip_prefix("namespace").and_then(|n| n.parse().ok()) {
                numbers.push(n);
            }
        }
    }
    numbers
        .get(index as usize)
        .copied()
        .map(Some)
        .ok_or_else(|| {
            format!(
                "namespaces.ini lists {} options; there is no index {index}",
                numbers.len()
            )
        })
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let ok = Command::new("cp")
        .arg("-a")
        .arg("--reflink=auto")
        .arg(from)
        .arg(to)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(format!("cp -a {} {}", from.display(), to.display()))
    }
}

/// Every file under `dir`, relative, `/`-separated. A symbolic link is
/// not followed: a tree whose `override` links to `Override` holds each
/// file once.
fn files(dir: &Path) -> BTreeMap<String, fs::Metadata> {
    fn go(base: &Path, dir: &Path, out: &mut BTreeMap<String, fs::Metadata>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(m) = fs::symlink_metadata(&p) else {
                continue;
            };
            if m.is_dir() {
                go(base, &p, out);
            } else if m.is_file() {
                let rel = p
                    .strip_prefix(base)
                    .unwrap_or(&p)
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(rel, m);
            }
        }
    }
    let mut out = BTreeMap::new();
    go(dir, dir, &mut out);
    out
}

/// The paths of `after` that differ from `before`: added, removed, or
/// with other bytes.
fn changed(before: &Path, after: &Path) -> Vec<String> {
    let (a, b) = (files(before), files(after));
    let mut out = Vec::new();
    for (p, mb) in &b {
        match a.get(p) {
            None => out.push(p.clone()),
            Some(ma) => {
                let same_stat = ma.len() == mb.len()
                    && ma.modified().ok() == mb.modified().ok();
                if !same_stat && fs::read(before.join(p)).ok() != fs::read(after.join(p)).ok() {
                    out.push(p.clone());
                }
            }
        }
    }
    out.extend(a.keys().filter(|p| !b.contains_key(*p)).cloned());
    out
}

#[derive(Default)]
struct Comparison {
    counts: BTreeMap<String, usize>,
    listed: Vec<Listed>,
}

impl Comparison {
    fn add(&mut self, class: &str, path: &str, detail: String) {
        *self.counts.entry(class.to_string()).or_default() += 1;
        if class != "identical" && self.listed.len() < LISTED {
            self.listed.push(Listed {
                file: path.to_string(),
                class: class.to_string(),
                detail,
            });
        }
    }
}

fn compare_path(cmp: &mut Comparison, rel: &str, ours: &Path, theirs: &Path) {
    let (a, b) = (fs::read(ours).ok(), fs::read(theirs).ok());
    let (a, b) = match (a, b) {
        (Some(a), Some(b)) => (a, b),
        (Some(_), None) => return cmp.add("only in rakata's tree", rel, String::new()),
        (None, Some(_)) => return cmp.add("only in this tree", rel, String::new()),
        (None, None) => return cmp.add("identical", rel, String::new()),
    };
    if a == b {
        return cmp.add("identical", rel, String::new());
    }
    let ext = rel.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    let name = rel.rsplit('/').next().unwrap_or(rel);
    match formats::by_extension(ext).map(|f| formats::same(f.name, &a, &b)) {
        Some(Ok(true)) => cmp.add("equal data", rel, String::new()),
        Some(Ok(false)) => cmp.add(
            "different data",
            rel,
            formats::first_difference(name, &a, &b).unwrap_or_default(),
        ),
        Some(Err(e)) => cmp.add("not comparable", rel, e),
        None => cmp.add(
            "different bytes",
            rel,
            format!("{} bytes against rakata's {}", b.len(), a.len()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_numbers_come_from_the_namespace_keys_in_order() {
        let dir = std::env::temp_dir().join(format!("bench-ns-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("NameSpaces.ini"),
            "[Namespaces]\nNamespace1=a\nNamespace3=b\n\n[a]\nName=x\n",
        )
        .unwrap();
        assert_eq!(option_number(&dir, 0).unwrap(), Some(1));
        assert_eq!(option_number(&dir, 1).unwrap(), Some(3));
        assert!(option_number(&dir, 2).is_err());
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(option_number(&dir, 0).unwrap(), None);
    }
}
