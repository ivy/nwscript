//! What the benchmark records of the machine it ran on, and how it times
//! one process: wall time from spawn to exit, peak resident memory from
//! GNU time's `%M` when GNU time is installed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// The machine's state at one moment. No host name: a report may be
/// shared.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HostState {
    /// The 1, 5 and 15 minute load averages.
    pub load: [f64; 3],
    pub mem_available_mb: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Host {
    pub cpu_model: String,
    pub cpus_online: usize,
    /// The CPUs this run was allowed, as the kernel lists them (`0-3`).
    pub cpus_allowed: String,
    pub cpus_allowed_count: usize,
    pub mem_total_mb: u64,
    pub kernel: String,
    pub before: HostState,
    pub after: HostState,
}

fn read(p: &str) -> String {
    fs::read_to_string(p).unwrap_or_default()
}

fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix(':'))
        .map(str::trim)
}

fn kb(text: &str, key: &str) -> u64 {
    field(text, key)
        .and_then(|v| v.split_whitespace().next()?.parse().ok())
        .unwrap_or(0)
}

/// Count the CPUs in a kernel list such as `0-3,8`.
fn count_list(list: &str) -> usize {
    list.split(',')
        .filter(|s| !s.is_empty())
        .map(|r| match r.split_once('-') {
            Some((a, b)) => match (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
                (Ok(a), Ok(b)) if b >= a => b - a + 1,
                _ => 0,
            },
            None => 1,
        })
        .sum()
}

pub fn state() -> HostState {
    let load: Vec<f64> = read("/proc/loadavg")
        .split_whitespace()
        .take(3)
        .filter_map(|v| v.parse().ok())
        .collect();
    HostState {
        load: [
            load.first().copied().unwrap_or(0.0),
            load.get(1).copied().unwrap_or(0.0),
            load.get(2).copied().unwrap_or(0.0),
        ],
        mem_available_mb: kb(&read("/proc/meminfo"), "MemAvailable") / 1024,
    }
}

pub fn host() -> Host {
    let cpuinfo = read("/proc/cpuinfo");
    let status = read("/proc/self/status");
    let allowed = field(&status, "Cpus_allowed_list")
        .unwrap_or("")
        .to_string();
    Host {
        cpu_model: field(&cpuinfo, "model name").unwrap_or("").to_string(),
        cpus_online: cpuinfo
            .lines()
            .filter(|l| l.starts_with("processor"))
            .count(),
        cpus_allowed_count: count_list(&allowed),
        cpus_allowed: allowed,
        mem_total_mb: kb(&read("/proc/meminfo"), "MemTotal") / 1024,
        kernel: read("/proc/sys/kernel/osrelease").trim().to_string(),
        before: state(),
        after: HostState::default(),
    }
}

/// GNU time, when it is installed: the one way to a child's peak memory
/// without unsafe code.
pub fn gnu_time() -> Option<PathBuf> {
    let p = PathBuf::from("/usr/bin/time");
    let out = Command::new(&p)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned()
        + &String::from_utf8_lossy(&out.stderr);
    text.contains("GNU").then_some(p)
}

/// One process's run.
#[derive(Debug, Clone)]
pub struct Measured {
    pub wall: f64,
    pub rss_kb: Option<u64>,
    /// The exit code; `None` when it was killed or timed out.
    pub code: Option<i32>,
    pub timed_out: bool,
    /// The last lines of its standard error.
    pub stderr: String,
}

/// Run `argv` in `cwd` with stdin closed, stdout discarded and stderr
/// kept, and time it. `scratch` is a directory for GNU time's output.
pub fn run(
    argv: &[String],
    cwd: Option<&Path>,
    time: Option<&Path>,
    scratch: &Path,
    timeout: Duration,
) -> Measured {
    let rss_file = scratch.join(format!("rss-{}", std::process::id()));
    let err_file = scratch.join(format!("stderr-{}", std::process::id()));
    let _ = fs::create_dir_all(scratch);
    let mut cmd = match time {
        Some(t) => {
            let mut c = Command::new(t);
            c.args(["-f", "%M", "-o"]).arg(&rss_file).arg("--");
            c.args(argv);
            c
        }
        None => {
            let mut c = Command::new(&argv[0]);
            c.args(&argv[1..]);
            c
        }
    };
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let failed = |e: String| Measured {
        wall: 0.0,
        rss_kb: None,
        code: None,
        timed_out: false,
        stderr: e,
    };
    let err = match fs::File::create(&err_file) {
        Ok(f) => f,
        Err(e) => return failed(format!("{}: {e}", err_file.display())),
    };
    let start = Instant::now();
    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(err)
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return failed(format!("could not start `{}`: {e}", argv[0])),
    };
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_micros(200)),
            Err(e) => return failed(format!("waiting: {e}")),
        }
    };
    let wall = start.elapsed().as_secs_f64();
    let rss_kb = fs::read_to_string(&rss_file).ok().and_then(|t| {
        t.lines()
            .rev()
            .find_map(|l| l.trim().parse::<u64>().ok())
    });
    let stderr = fs::read(&err_file)
        .map(|b| {
            let t = strip_ansi(&String::from_utf8_lossy(&b));
            let lines: Vec<&str> = t.lines().filter(|l| !l.trim().is_empty()).collect();
            lines[lines.len().saturating_sub(3)..].join("\n")
        })
        .unwrap_or_default();
    let _ = fs::remove_file(&rss_file);
    let _ = fs::remove_file(&err_file);
    Measured {
        wall,
        rss_kb,
        code: status.and_then(|s| s.code()),
        timed_out,
        stderr,
    }
}

/// Text without terminal colour sequences (`ESC [ ... letter`).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Median, extremes and count of a set of timings.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Stats {
    pub n: usize,
    pub median: f64,
    pub min: f64,
    pub max: f64,
}

impl Stats {
    pub fn of(values: &[f64]) -> Option<Stats> {
        if values.is_empty() {
            return None;
        }
        let mut v = values.to_vec();
        v.sort_by(f64::total_cmp);
        let n = v.len();
        let median = if n % 2 == 1 {
            v[n / 2]
        } else {
            (v[n / 2 - 1] + v[n / 2]) / 2.0
        };
        Some(Stats {
            n,
            median,
            min: v[0],
            max: v[n - 1],
        })
    }

    /// The spread as a share of the median: (max - min) / median.
    pub fn spread(&self) -> f64 {
        if self.median > 0.0 {
            (self.max - self.min) / self.median
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_lists_count() {
        assert_eq!(count_list("0-3"), 4);
        assert_eq!(count_list("0-3,8,10-11"), 7);
        assert_eq!(count_list(""), 0);
    }

    #[test]
    fn colour_sequences_are_dropped() {
        assert_eq!(strip_ansi("\u{1b}[31m[Error]\u{1b}[0m x"), "[Error] x");
    }

    #[test]
    fn medians_and_spread() {
        let s = Stats::of(&[3.0, 1.0, 2.0, 10.0]).unwrap();
        assert_eq!((s.n, s.median, s.min, s.max), (4, 2.5, 1.0, 10.0));
        assert!((s.spread() - 3.6).abs() < 1e-9);
        assert!(Stats::of(&[]).is_none());
    }
}
