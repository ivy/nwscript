//! The benchmark's report: what was run, on what, and what came out, as
//! JSON and as Markdown.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use super::Identity;
use super::extract::Manifest;
use super::host::{Host, Stats};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Report {
    pub label: String,
    pub manifest_sha256: String,
    pub host: Host,
    pub method: Method,
    /// The extraction the readers ran over.
    pub files: Option<Manifest>,
    pub readers: Vec<ReaderRow>,
    pub compile: Vec<CompileRow>,
    pub install: Vec<InstallRow>,
    /// Against an earlier report, when one was given.
    #[serde(default)]
    pub compared: Vec<Rerun>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Method {
    pub reps: usize,
    pub install_reps: usize,
    pub warmup: usize,
    pub max_files: Option<usize>,
    pub max_scripts: Option<usize>,
    pub peak_memory: String,
}

/// One file named in a row: what happened to it.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Listed {
    pub file: String,
    pub class: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub detail: String,
}

/// How a tool's written files compare with the originals.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WriteCounts {
    /// Byte for byte the original.
    pub identical: usize,
    /// Other bytes, the same data as Rakata reads both.
    pub equal: usize,
    /// Other data.
    pub different: usize,
    /// Written, and Rakata does not read what was written.
    pub unreadable: usize,
    /// The writer refused.
    pub failed: usize,
    /// Read, not written: the tool does not write this format.
    pub not_written: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ReaderRow {
    pub tool: String,
    pub format: String,
    pub description: String,
    pub command: Vec<String>,
    pub identity: Vec<Identity>,
    /// Why the row did not run.
    pub skipped: Option<String>,
    pub files: usize,
    pub read_ok: usize,
    pub read_failed: usize,
    /// Read failures by their message, the first 120 characters.
    pub failure_kinds: BTreeMap<String, usize>,
    /// Files Rakata reads and this tool does not, and the reverse.
    pub read_by_rakata_only: usize,
    pub read_by_this_only: usize,
    pub write: Option<WriteCounts>,
    /// Files named, at most forty, and how many of each class there were.
    pub listed: Vec<Listed>,
    pub listed_counts: BTreeMap<String, usize>,
    /// Run once per file: `read` is the sum, `start` one process.
    pub per_file: bool,
    pub start: Option<Stats>,
    pub read: Option<Stats>,
    pub write_time: Option<Stats>,
    pub rss_read_mb: Option<f64>,
    pub rss_write_mb: Option<f64>,
    pub notes: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CompileRow {
    pub compiler: String,
    pub description: String,
    pub command: Vec<String>,
    pub identity: Vec<Identity>,
    pub skipped: Option<String>,
    pub scripts: usize,
    /// Byte for byte BioWare's bytecode.
    pub identical: usize,
    pub different: usize,
    pub failed: usize,
    /// One process doing nothing.
    pub start: Option<Stats>,
    /// Every script, one process each, in sequence.
    pub total: Option<Stats>,
    pub rss_mb: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstallRow {
    pub case: String,
    pub case_description: String,
    pub installer: String,
    pub description: String,
    pub command: Vec<String>,
    pub identity: Vec<Identity>,
    pub skipped: Option<String>,
    pub option: Option<u32>,
    /// Each run's exit code.
    pub exits: Vec<Option<i32>>,
    pub time: Option<Stats>,
    pub rss_mb: Option<f64>,
    /// Paths the first run changed in the tree.
    pub changed: usize,
    /// Its tree against `rakata install`'s, by class.
    pub against_rakata: Option<BTreeMap<String, usize>>,
    pub listed: Vec<Listed>,
    pub notes: Vec<String>,
}

/// One timing of this run against the same timing of an earlier one.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Rerun {
    pub what: String,
    pub before: Stats,
    pub now: Stats,
    /// This run's median lies within the earlier run's range.
    pub within: bool,
}

pub fn compare(old: &Report, new: &Report) -> Vec<Rerun> {
    let mut out = Vec::new();
    let mut push = |what: String, a: &Option<Stats>, b: &Option<Stats>| {
        if let (Some(a), Some(b)) = (a, b) {
            out.push(Rerun {
                what,
                before: a.clone(),
                now: b.clone(),
                within: b.median >= a.min && b.median <= a.max,
            });
        }
    };
    for r in &new.readers {
        if let Some(o) = old
            .readers
            .iter()
            .find(|o| o.tool == r.tool && o.format == r.format)
        {
            push(format!("{} {} start", r.tool, r.format), &o.start, &r.start);
            push(format!("{} {} read", r.tool, r.format), &o.read, &r.read);
            push(
                format!("{} {} write", r.tool, r.format),
                &o.write_time,
                &r.write_time,
            );
        }
    }
    for c in &new.compile {
        if let Some(o) = old.compile.iter().find(|o| o.compiler == c.compiler) {
            push(format!("{} compile", c.compiler), &o.total, &c.total);
        }
    }
    for i in &new.install {
        if let Some(o) = old
            .install
            .iter()
            .find(|o| o.case == i.case && o.installer == i.installer)
        {
            push(
                format!("{} {} install", i.installer, i.case),
                &o.time,
                &i.time,
            );
        }
    }
    out
}

fn secs(s: &Option<Stats>) -> String {
    match s {
        None => String::new(),
        Some(s) => format!(
            "{} ({}–{}, ±{:.0}%)",
            fmt_s(s.median),
            fmt_s(s.min),
            fmt_s(s.max),
            s.spread() * 50.0
        ),
    }
}

fn fmt_s(v: f64) -> String {
    if v >= 10.0 {
        format!("{v:.1}")
    } else if v >= 0.1 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

/// The work of a run: its median less the median of a process that does
/// nothing (times the files, for a tool run once per file).
fn work(r: &ReaderRow, run: &Option<Stats>) -> String {
    match (run, &r.start) {
        (Some(run), Some(start)) => {
            let starts = if r.per_file {
                start.median * r.files as f64
            } else {
                start.median
            };
            fmt_s((run.median - starts).max(0.0))
        }
        _ => String::new(),
    }
}

fn mb(v: Option<f64>) -> String {
    v.map(|v| format!("{v:.0}")).unwrap_or_default()
}

pub fn markdown(r: &Report) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Benchmark\n");
    if !r.label.is_empty() {
        let _ = writeln!(s, "**{}**\n", r.label);
    }
    let h = &r.host;
    let _ = writeln!(
        s,
        "Host: {} CPUs online ({}), this run allowed {} (`{}`); {} MB of memory, {} MB available before and {} MB after; kernel {}.\n",
        h.cpus_online,
        h.cpu_model,
        h.cpus_allowed_count,
        h.cpus_allowed,
        h.mem_total_mb,
        h.before.mem_available_mb,
        h.after.mem_available_mb,
        h.kernel
    );
    let busy = h.before.load[0].max(h.after.load[0]);
    let _ = writeln!(
        s,
        "Load average before: {:.2} {:.2} {:.2}; after: {:.2} {:.2} {:.2}.{}\n",
        h.before.load[0],
        h.before.load[1],
        h.before.load[2],
        h.after.load[0],
        h.after.load[1],
        h.after.load[2],
        if busy > 1.0 {
            format!(
                " **The host was busy (load {busy:.2} on {} CPUs): these timings are not a measurement.**",
                h.cpus_online
            )
        } else {
            String::new()
        }
    );
    let m = &r.method;
    let _ = writeln!(
        s,
        "Method: release builds; each timing is {} repetitions after {} warm-up run (the page cache warm), as median (min–max, ± half the range as a share of the median); a process that starts and does nothing is timed apart and subtracted for the *work* column. Readers run one thread over the files in list order. Peak memory: {}.{}{}\n",
        m.reps,
        m.warmup,
        m.peak_memory,
        m.max_files
            .map(|n| format!(" Only the first {n} files of each list."))
            .unwrap_or_default(),
        m.max_scripts
            .map(|n| format!(" Only the first {n} scripts."))
            .unwrap_or_default()
    );
    if let Some(f) = &r.files {
        let _ = writeln!(
            s,
            "## Files\n\n| Format | Files | MB | Duplicates |\n| --- | --- | --- | --- |"
        );
        for (name, c) in &f.formats {
            let _ = writeln!(
                s,
                "| `{name}` | {} | {:.1} | {} |",
                c.files,
                c.bytes as f64 / 1e6,
                c.duplicates
            );
        }
        s.push('\n');
    }

    if !r.readers.is_empty() {
        let _ = writeln!(s, "## Readers and writers\n");
        let _ = writeln!(
            s,
            "Written back: identical bytes / equal data in other bytes / different data / unreadable by Rakata / writer failed. Times in seconds over the whole list.\n"
        );
        let _ = writeln!(
            s,
            "| Format | Tool | Files | Read | Failed | Written back | Start s | Read s | Read work s | Write s | Write work s | Peak MB read / write |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for row in &r.readers {
            if let Some(why) = &row.skipped {
                let _ = writeln!(
                    s,
                    "| `{}` | {} | | | | skipped: {} | | | | | | |",
                    row.format,
                    row.tool,
                    why.replace('|', "\\|").replace('\n', " ")
                );
                continue;
            }
            let written = row
                .write
                .as_ref()
                .map(|w| {
                    format!(
                        "{} / {} / {} / {} / {}",
                        w.identical, w.equal, w.different, w.unreadable, w.failed
                    )
                })
                .unwrap_or_else(|| "does not write".into());
            let _ = writeln!(
                s,
                "| `{}` | {}{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} / {} |",
                row.format,
                row.tool,
                if row.per_file { " (per file)" } else { "" },
                row.files,
                row.read_ok,
                row.read_failed,
                written,
                secs(&row.start),
                secs(&row.read),
                work(row, &row.read),
                secs(&row.write_time),
                work(row, &row.write_time),
                mb(row.rss_read_mb),
                mb(row.rss_write_mb),
            );
        }
        s.push('\n');
        for row in &r.readers {
            if row.listed.is_empty() && row.notes.is_empty() {
                continue;
            }
            let _ = writeln!(s, "### `{}`, {}\n", row.format, row.tool);
            if row.tool != "rakata" && row.skipped.is_none() {
                let _ = writeln!(
                    s,
                    "- read by rakata, not by this tool: {}",
                    row.read_by_rakata_only
                );
            }
            for (class, n) in &row.listed_counts {
                let _ = writeln!(s, "- {class}: {n}");
            }
            if !row.failure_kinds.is_empty() {
                let _ = writeln!(s, "\nRead failures by message:\n");
                let mut kinds: Vec<_> = row.failure_kinds.iter().collect();
                kinds.sort_by(|a, b| b.1.cmp(a.1));
                for (k, n) in kinds.iter().take(10) {
                    let _ = writeln!(s, "- {n}: `{}`", k.replace('`', "'"));
                }
            }
            let _ = writeln!(s, "\n<details><summary>Files</summary>\n");
            for l in &row.listed {
                let _ = writeln!(
                    s,
                    "- `{}`: {}{}",
                    l.file,
                    l.class,
                    if l.detail.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", l.detail.replace('\n', " "))
                    }
                );
            }
            for n in &row.notes {
                let _ = writeln!(s, "- note: {n}");
            }
            let _ = writeln!(s, "\n</details>\n");
        }
    }

    if !r.compile.is_empty() {
        let _ = writeln!(
            s,
            "## Compilers\n\nEach script compiled in its own folder, one process per script, in sequence; *identical* is byte for byte BioWare's bytecode.\n\n| Compiler | Scripts | Identical | Different | Failed | Start s | All scripts s | Per script ms | Peak MB |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for c in &r.compile {
            if let Some(why) = &c.skipped {
                let _ = writeln!(s, "| {} | skipped: {} | | | | | | | |", c.compiler, why);
                continue;
            }
            let per = c
                .total
                .as_ref()
                .filter(|_| c.scripts > 0)
                .map(|t| format!("{:.1}", t.median * 1000.0 / c.scripts as f64))
                .unwrap_or_default();
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                c.compiler,
                c.scripts,
                c.identical,
                c.different,
                c.failed,
                secs(&c.start),
                secs(&c.total),
                per,
                mb(c.rss_mb)
            );
        }
        s.push('\n');
    }

    if !r.install.is_empty() {
        let _ = writeln!(
            s,
            "## Installers\n\nEach run onto a fresh reflink copy of the case's tree; the time is the installer's process (for TSLPatcher, the Wine container and the driver's waits included). The first run's tree is compared with `rakata install`'s.\n\n| Case | Installer | Option | Exits | Time s | Peak MB | Paths changed | Against rakata's tree |\n| --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        for i in &r.install {
            if let Some(why) = &i.skipped {
                let _ = writeln!(
                    s,
                    "| `{}` | {} | | skipped: {} | | | | |",
                    i.case, i.installer, why
                );
                continue;
            }
            let exits: Vec<String> = i
                .exits
                .iter()
                .map(|e| e.map_or("killed".into(), |c| c.to_string()))
                .collect();
            let against = i
                .against_rakata
                .as_ref()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| format!("{k} {v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let _ = writeln!(
                s,
                "| `{}` | {} | {} | {} | {} | {} | {} | {} |",
                i.case,
                i.installer,
                i.option.map(|o| o.to_string()).unwrap_or_default(),
                exits.join(" "),
                secs(&i.time),
                mb(i.rss_mb),
                i.changed,
                against
            );
        }
        s.push('\n');
        let mut cases = std::collections::BTreeSet::new();
        for i in &r.install {
            if cases.insert(&i.case) && !i.case_description.is_empty() {
                let _ = writeln!(s, "- `{}`: {}", i.case, i.case_description);
            }
        }
        s.push('\n');
        for i in &r.install {
            if i.listed.is_empty() && i.notes.is_empty() {
                continue;
            }
            let _ = writeln!(s, "### `{}`, {}\n", i.case, i.installer);
            for l in &i.listed {
                let _ = writeln!(
                    s,
                    "- `{}`: {}{}",
                    l.file,
                    l.class,
                    if l.detail.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", l.detail)
                    }
                );
            }
            for n in &i.notes {
                let _ = writeln!(s, "- note: {}", n.replace('\n', " "));
            }
            s.push('\n');
        }
    }

    if !r.compared.is_empty() {
        let within = r.compared.iter().filter(|c| c.within).count();
        let _ = writeln!(
            s,
            "## Against the earlier run\n\n{within} of {} medians lie within the earlier run's range.\n\n| Timing | Earlier | Now | Within |\n| --- | --- | --- | --- |",
            r.compared.len()
        );
        for c in &r.compared {
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} |",
                c.what,
                secs(&Some(c.before.clone())),
                secs(&Some(c.now.clone())),
                if c.within { "yes" } else { "no" }
            );
        }
        s.push('\n');
    }

    let _ = writeln!(s, "## Tools\n");
    let mut seen = std::collections::BTreeSet::new();
    let ids = r
        .readers
        .iter()
        .map(|x| (&x.tool, &x.description, &x.command, &x.identity))
        .chain(
            r.compile
                .iter()
                .map(|x| (&x.compiler, &x.description, &x.command, &x.identity)),
        )
        .chain(
            r.install
                .iter()
                .map(|x| (&x.installer, &x.description, &x.command, &x.identity)),
        );
    for (name, desc, cmd, id) in ids {
        if !seen.insert(name.clone()) {
            continue;
        }
        let _ = writeln!(s, "- **{name}**: {desc}\n  - command: `{}`", cmd.join(" "));
        for i in id {
            let _ = writeln!(
                s,
                "  - `{}`: {}",
                i.path,
                i.sha256.as_deref().unwrap_or("not found")
            );
        }
    }
    s
}
