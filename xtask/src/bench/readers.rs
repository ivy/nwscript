//! The readers section: every comparator over every format's list, first
//! once for correctness (`check`), then timed (`start`, `read`, `write`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::formats;
use super::host::{self, Stats};
use super::report::{Listed, ReaderRow, WriteCounts};
use super::{BenchFile, ReaderDef, RunOpts, Subst, identities, progress};

/// Files listed in a row, at most; the counts cover the rest.
const LISTED: usize = 40;

pub fn run(bench: &BenchFile, subst: &Subst, opts: &RunOpts) -> Vec<ReaderRow> {
    let mut rows = Vec::new();
    for format in formats::ALL {
        if !RunOpts::wants(&opts.formats, format.name) {
            continue;
        }
        let list_path = opts.files.join(format!("{}.list", format.name));
        let files: Option<Vec<PathBuf>> = fs::read_to_string(&list_path).ok().map(|t| {
            t.lines()
                .filter(|l| !l.is_empty())
                .take(opts.max_files.unwrap_or(usize::MAX))
                .map(PathBuf::from)
                .collect()
        });
        let mut read_ok: BTreeMap<String, Vec<bool>> = BTreeMap::new();
        let first_row = rows.len();
        for (tool, def) in &bench.reader {
            if !RunOpts::wants(&opts.readers, tool) {
                continue;
            }
            let mut row = ReaderRow::new(tool, format.name, def, subst);
            match &files {
                None => {
                    row.skipped = Some(format!(
                        "no list at {}: run `cargo xtask bench extract` first",
                        list_path.display()
                    ))
                }
                Some(files) if files.is_empty() => {
                    row.skipped = Some("the tree holds no file of this format".into())
                }
                Some(files) => {
                    row.files = files.len();
                    progress(format!("{tool} on {} ({} files)", format.name, files.len()));
                    let oks = if def.per_file {
                        per_file(&mut row, def, subst, opts, format.name, files)
                    } else {
                        batch(&mut row, def, subst, opts, format.name, files)
                    };
                    if let Some(oks) = oks {
                        read_ok.insert(tool.clone(), oks);
                    }
                }
            }
            rows.push(row);
        }
        // Where a comparator and Rakata disagree on whether a file reads.
        if let (Some(files), Some(ours)) = (&files, read_ok.get("rakata")) {
            for row in rows[first_row..].iter_mut() {
                let Some(theirs) = read_ok.get(&row.tool).filter(|_| row.tool != "rakata") else {
                    continue;
                };
                // A file only Rakata reads is already listed as this
                // tool's failure; one only this tool reads is named here.
                for (i, (a, b)) in ours.iter().zip(theirs).enumerate() {
                    if *a && !b {
                        row.read_by_rakata_only += 1;
                    } else if !a && *b {
                        row.read_by_this_only += 1;
                        row.list(Listed {
                            file: short(&files[i], opts),
                            class: "read by this tool, not by rakata".into(),
                            detail: String::new(),
                        });
                    }
                }
            }
        }
    }
    rows
}

/// A path as the report shows it: within the extraction or the tree.
pub fn short(p: &Path, opts: &RunOpts) -> String {
    if let Ok(r) = p.strip_prefix(&opts.files) {
        return r.display().to_string();
    }
    let files_abs = std::env::current_dir()
        .map(|d| d.join(&opts.files))
        .unwrap_or_default();
    if let Ok(r) = p.strip_prefix(&files_abs) {
        return r.display().to_string();
    }
    // A file of the tree: within the tree.
    if let Some(r) = opts.tree.as_deref().and_then(|t| p.strip_prefix(t).ok()) {
        return r.display().to_string();
    }
    p.display().to_string()
}

fn fill(template: &[String], mode: &str, format: &str, list: &Path, out: &Path) -> Vec<String> {
    template
        .iter()
        .map(|a| {
            a.replace("{mode}", mode)
                .replace("{format}", format)
                .replace("{list}", &list.display().to_string())
                .replace("{out}", &out.display().to_string())
        })
        .collect()
}

/// A batch driver: one process per mode over the whole list.
fn batch(
    row: &mut ReaderRow,
    def: &ReaderDef,
    subst: &Subst,
    opts: &RunOpts,
    format: &str,
    files: &[PathBuf],
) -> Option<Vec<bool>> {
    let template = match subst.all(&def.command) {
        Ok(t) => t,
        Err(e) => {
            row.skipped = Some(e);
            return None;
        }
    };
    let dir = opts.work.join(&row.tool);
    if let Err(e) = fs::create_dir_all(&dir) {
        row.skipped = Some(format!("{}: {e}", dir.display()));
        return None;
    }
    let list = dir.join(format!("{format}.list"));
    let text: String = files.iter().map(|p| format!("{}\n", p.display())).collect();
    if let Err(e) = fs::write(&list, text) {
        row.skipped = Some(format!("{}: {e}", list.display()));
        return None;
    }
    let out = dir.join(format!("{format}.jsonl"));
    let go = |mode: &str| {
        host::run(
            &fill(&template, mode, format, &list, &out),
            None,
            opts.time.as_deref(),
            &opts.work,
            opts.timeout,
        )
    };

    let start = go("start");
    if start.code == Some(i32::from(super::driver::UNSUPPORTED)) {
        row.skipped = Some(start.stderr.trim().to_string());
        return None;
    }
    if start.code != Some(0) {
        row.skipped = Some(format!(
            "the driver did not start (exit {:?}): {}",
            start.code, start.stderr
        ));
        return None;
    }

    // Correctness, untimed.
    let check = go("check");
    if check.code != Some(0) {
        row.skipped = Some(format!(
            "the check run failed (exit {:?}{}): {}",
            check.code,
            if check.timed_out { ", timed out" } else { "" },
            check.stderr
        ));
        return None;
    }
    let records = match fs::read_to_string(&out) {
        Ok(t) => t,
        Err(e) => {
            row.skipped = Some(format!("{}: {e}", out.display()));
            return None;
        }
    };
    let mut oks = vec![false; files.len()];
    let mut writes_any = false;
    let mut counts = WriteCounts::default();
    for line in records.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(i) = v["i"].as_u64().map(|i| i as usize).filter(|&i| i < files.len()) else {
            continue;
        };
        let file = &files[i];
        if let Some(e) = v["error"].as_str() {
            row.fail(short(file, opts), e, file);
            continue;
        }
        oks[i] = true;
        row.read_ok += 1;
        if let Some(e) = v["write_error"].as_str() {
            writes_any = true;
            counts.failed += 1;
            row.list(Listed {
                file: short(file, opts),
                class: "write failed".into(),
                detail: e.to_string(),
            });
            continue;
        }
        match v["write"].as_str() {
            Some("identical") => {
                writes_any = true;
                counts.identical += 1;
            }
            Some("different") => {
                writes_any = true;
                let theirs = v["out"].as_str().map(PathBuf::from);
                classify_written(row, &mut counts, format, file, theirs.as_deref(), opts);
            }
            _ => counts.not_written += 1,
        }
    }
    let _ = fs::remove_dir_all(out.with_extension("d"));
    if writes_any {
        row.write = Some(counts);
    }

    // Speed: a warm-up read, then the repetitions, interleaved.
    let _ = go("read");
    let (mut s, mut r, mut w) = (Vec::new(), Vec::new(), Vec::new());
    let (mut rss_r, mut rss_w) = (None::<u64>, None::<u64>);
    for rep in 0..opts.reps {
        progress(format!(
            "  {} {format} repetition {}/{}",
            row.tool,
            rep + 1,
            opts.reps
        ));
        let m = go("start");
        s.push(m.wall);
        let m = go("read");
        if m.code != Some(0) {
            row.notes.push(format!("a timed read exited {:?}: {}", m.code, m.stderr));
        }
        r.push(m.wall);
        rss_r = rss_r.max(m.rss_kb);
        if writes_any {
            let m = go("write");
            if m.code == Some(0) {
                w.push(m.wall);
                rss_w = rss_w.max(m.rss_kb);
            }
        }
    }
    row.start = Stats::of(&s);
    row.read = Stats::of(&r);
    row.write_time = Stats::of(&w);
    row.rss_read_mb = rss_r.map(|k| k as f64 / 1024.0);
    row.rss_write_mb = rss_w.map(|k| k as f64 / 1024.0);
    let _ = fs::remove_file(&out);
    Some(oks)
}

/// Classify a file another writer wrote differently from the original:
/// the same data as Rakata reads both, other data, or a file Rakata does
/// not read.
fn classify_written(
    row: &mut ReaderRow,
    counts: &mut WriteCounts,
    format: &str,
    original: &Path,
    theirs: Option<&Path>,
    opts: &RunOpts,
) {
    let (Ok(a), Some(Ok(b))) = (fs::read(original), theirs.map(fs::read)) else {
        counts.failed += 1;
        row.list(Listed {
            file: short(original, opts),
            class: "write failed".into(),
            detail: "the written file could not be read back from disk".into(),
        });
        return;
    };
    match formats::same(format, &a, &b) {
        Ok(true) => counts.equal += 1,
        Ok(false) => {
            counts.different += 1;
            let name = original
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            row.list(Listed {
                file: short(original, opts),
                class: "different data".into(),
                detail: formats::first_difference(&name, &a, &b).unwrap_or_default(),
            });
        }
        Err(e) => {
            counts.unreadable += 1;
            row.list(Listed {
                file: short(original, opts),
                class: "written file unreadable by rakata".into(),
                detail: e,
            });
        }
    }
}

/// A tool run once per file: success is its exit status.
fn per_file(
    row: &mut ReaderRow,
    def: &ReaderDef,
    subst: &Subst,
    opts: &RunOpts,
    format: &str,
    files: &[PathBuf],
) -> Option<Vec<bool>> {
    if !def.formats.iter().any(|f| f == format) {
        row.skipped = Some(format!(
            "not among the formats conformance/bench.toml asks {} to read",
            row.tool
        ));
        return None;
    }
    let (template, start) = match (subst.all(&def.command), subst.all(&def.start)) {
        (Ok(t), Ok(s)) => (t, s),
        (Err(e), _) | (_, Err(e)) => {
            row.skipped = Some(e);
            return None;
        }
    };
    let argv = |f: &Path| -> Vec<String> {
        template
            .iter()
            .map(|a| a.replace("{file}", &f.display().to_string()))
            .collect()
    };
    let go = |argv: &[String]| {
        host::run(
            argv,
            None,
            opts.time.as_deref(),
            &opts.work,
            opts.timeout,
        )
    };
    let mut oks = vec![false; files.len()];
    for (i, f) in files.iter().enumerate() {
        let m = go(&argv(f));
        if m.code == Some(0) {
            oks[i] = true;
            row.read_ok += 1;
        } else {
            let detail = if m.stderr.is_empty() {
                format!("exit {:?}", m.code)
            } else {
                m.stderr.lines().last().unwrap_or("").to_string()
            };
            row.fail(short(f, opts), &detail, f);
        }
    }
    let (mut s, mut r) = (Vec::new(), Vec::new());
    let mut rss = None::<u64>;
    for rep in 0..opts.reps {
        progress(format!(
            "  {} {format} repetition {}/{}",
            row.tool,
            rep + 1,
            opts.reps
        ));
        if !start.is_empty() {
            s.push(go(&start).wall);
        }
        let mut total = 0.0;
        for f in files {
            let m = go(&argv(f));
            total += m.wall;
            rss = rss.max(m.rss_kb);
        }
        r.push(total);
    }
    row.per_file = true;
    row.start = Stats::of(&s);
    row.read = Stats::of(&r);
    row.rss_read_mb = rss.map(|k| k as f64 / 1024.0);
    Some(oks)
}

impl ReaderRow {
    pub fn new(tool: &str, format: &str, def: &ReaderDef, subst: &Subst) -> Self {
        ReaderRow {
            tool: tool.to_string(),
            format: format.to_string(),
            description: def.description.clone(),
            command: def.command.clone(),
            identity: identities(subst, &def.command, &def.identity),
            ..Default::default()
        }
    }

    fn fail(&mut self, file: String, error: &str, path: &Path) {
        self.read_failed += 1;
        // The message without the file's name, so failures group by kind.
        let mut kind = error.replace(&path.display().to_string(), "FILE");
        if let Some(n) = path.file_name() {
            kind = kind.replace(&*n.to_string_lossy(), "FILE");
        }
        let kind: String = kind.chars().take(120).collect();
        *self.failure_kinds.entry(kind).or_default() += 1;
        self.list(Listed {
            file,
            class: "read failed".into(),
            detail: error.to_string(),
        });
    }

    fn list(&mut self, l: Listed) {
        *self.listed_counts.entry(l.class.clone()).or_default() += 1;
        if self.listed.len() < LISTED {
            self.listed.push(l);
        }
    }
}
