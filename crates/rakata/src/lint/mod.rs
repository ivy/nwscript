//! The tree lint, `rakata lint`: faults a crash or a patcher error would
//! otherwise find, visible in the files of an install.
//!
//! - RW1901: a reference that resolves to nothing: a GFF's `CResRef` field,
//!   a 2DA cell of a resref column, or a string literal a script passes to
//!   an engine routine's resref parameter, naming no resource of the type
//!   the site names in the scope it is loaded in.
//! - RW1902: a strref in a GFF or a 2DA strref column at or past the end
//!   of `dialog.tlk`.
//! - RW1903: two `Override/` files whose names differ only in case.
//! - RE1901: a `.tpc` in `Override/` beside a `.tga` or `.dds` of the same
//!   name, which crashes the game.
//!
//! Which type a GFF field or 2DA column names, and which columns hold
//! strrefs, is the measured table in `sites.toml`; which routine
//! parameters take a resref is the engine's table from the install's own
//! `nwscript.nss`, read by the name of each parameter as the index reads
//! it ([`rakata_index::ref_kind`]). A run checks every resource, or only
//! those in the paths it is given, while resolution always sees the whole
//! install; the contents come from a walk of the tree or from its index.

mod known;
mod rules;
mod sites;
mod source;
mod universe;

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::Instant;

use nwscript::diag::{Diagnostic, SourceMap};
use rakata_game::{Install, Profile};
use serde::Serialize;

pub use known::Known;

use rules::Rules;
use universe::{Universe, child_ci};

/// The codes the rules report under.
pub mod codes {
    /// A reference that resolves to nothing.
    pub const DANGLING: &str = "RW1901";
    /// A strref at or past the end of the talk table.
    pub const STRREF: &str = "RW1902";
    /// `Override/` files whose names differ only in case.
    pub const CASE: &str = "RW1903";
    /// A `.tpc` beside a `.tga` or `.dds` of the same name in `Override/`.
    pub const TPC_SHADOW: &str = "RE1901";
    /// A paths file or known list that cannot be read or does not parse.
    pub const INPUT: &str = "RE1902";
    /// An index database holding no tree at the install's folder.
    pub const NO_TREE: &str = "RE1903";
    /// A rule skipped: no `dialog.tlk`, or no `nwscript.nss`.
    pub const SKIPPED: &str = "RW1904";
    /// Resources that could not be read, so what they name went unchecked.
    pub const UNREAD: &str = "RW1905";
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub code: &'static str,
    /// The file relative to the tree, `PATH!MEMBER` for an archive member.
    pub path: String,
    /// Where in it: a GFF field path, `row N, column C` in a 2DA,
    /// `@OFFSET Routine.param` in a script; empty for a file of `Override/`.
    pub site: String,
    /// What is named: the resref or the strref; for an `Override/` rule,
    /// the other files.
    pub value: String,
    /// The type the site names (`ncs`, `module`, `blueprint`, `any`), for
    /// a dangling reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub want: Option<String>,
    /// The other files an `Override/` finding involves, for `--paths`.
    #[serde(skip)]
    pub related: Vec<String>,
}

impl Finding {
    /// The message, without the code.
    pub fn message(&self, talk_table: Option<u32>) -> String {
        let at = if self.site.is_empty() {
            format!("`{}`", self.path)
        } else {
            format!("`{}` {}", self.path, self.site)
        };
        match self.code {
            codes::DANGLING => match self.want.as_deref() {
                Some("any") | None => {
                    format!(
                        "{at} names `{}`, which no layer of the install holds",
                        self.value
                    )
                }
                Some("module") => format!(
                    "{at} names the module `{}`, which the install does not have",
                    self.value
                ),
                Some(t) => format!(
                    "{at} names `{}` as a {t}, and no layer of the install holds one",
                    self.value
                ),
            },
            codes::STRREF => format!(
                "{at} shows strref {}, at or past the end of the talk table ({} strings)",
                self.value,
                talk_table.map_or_else(|| "?".to_string(), |n| n.to_string())
            ),
            codes::CASE => format!("{at} differs only in case from {}", self.value),
            _ => format!(
                "{at} lies beside {} in Override: the game crashes on a texture held as both",
                self.value
            ),
        }
    }

    fn sort_key(&self) -> (String, &str, &str, &str) {
        (
            self.path.to_ascii_lowercase(),
            self.path.as_str(),
            self.site.as_str(),
            self.code,
        )
    }
}

/// What to lint.
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// The install: the folder holding `chitin.key`.
    pub game: &'a Path,
    pub profile: Profile,
    /// Check only these files, relative to the tree ([`crate::index::parse_changed`]);
    /// resolution still sees the whole install.
    pub paths: Option<&'a [String]>,
    /// Read the contents from this index instead of walking the tree.
    pub db: Option<&'a Path>,
    /// Findings the tree is accepted with.
    pub known: Option<&'a Known>,
    /// Threads for the walk; 0 for one per CPU.
    pub threads: usize,
}

/// What a run found.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Report {
    /// `walk`, or `index` with the tree's label.
    pub source: String,
    /// Resources whose contents were checked: GFFs, 2DAs and scripts.
    pub checked: usize,
    /// Resources of those types that did not read.
    pub unread: usize,
    /// The talk table's length, when the install has one.
    pub talk_table: Option<u32>,
    /// Findings the known list does not hold, sorted by path, site and code.
    pub findings: Vec<Finding>,
    /// Those findings by code.
    pub counts: BTreeMap<&'static str, usize>,
    /// Findings the known list holds, by code; not listed.
    pub known: BTreeMap<&'static str, usize>,
    /// Every finding, known or not, for writing a known list.
    #[serde(skip)]
    pub all: Vec<Finding>,
    pub seconds: f64,
    #[serde(skip)]
    pub warnings: Vec<Diagnostic>,
}

impl Report {
    /// The findings as diagnostics.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.findings
            .iter()
            .map(|f| crate::index::diagnostic(f.code, f.message(self.talk_table)))
            .collect()
    }

    /// One line: the counts and the time.
    pub fn summary(&self) -> String {
        let n = self.findings.len();
        let by: Vec<String> = self
            .counts
            .iter()
            .map(|(c, k)| format!("{c} {k}"))
            .collect();
        let known: usize = self.known.values().sum();
        format!(
            "lint: {n} finding{}{} and {known} known, {} resources checked from the {} in {:.2} s",
            if n == 1 { "" } else { "s" },
            if by.is_empty() {
                String::new()
            } else {
                format!(" ({})", by.join(", "))
            },
            self.checked,
            self.source,
            self.seconds
        )
    }
}

/// The `Override/` rules: case variants and `.tpc` shadows.
fn override_rules(install: &Install, root: &Path, out: &mut Vec<Finding>) {
    let rel = |p: &Path| {
        p.strip_prefix(root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    };
    let file_name = |p: &Path| {
        p.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    };
    for group in install.override_case_variants() {
        for p in &group {
            let others: Vec<&std::path::PathBuf> = group.iter().filter(|q| *q != p).collect();
            out.push(Finding {
                code: codes::CASE,
                path: rel(p),
                site: String::new(),
                value: others
                    .iter()
                    .map(|q| format!("`{}`", file_name(q)))
                    .collect::<Vec<_>>()
                    .join(", "),
                want: None,
                related: others.iter().map(|q| rel(q)).collect(),
            });
        }
    }
    let Some(dir) = child_ci(root, "override") else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut files: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| !t.is_dir()))
        .map(|e| e.path())
        .collect();
    files.sort();
    let mut by_stem: BTreeMap<String, Vec<&std::path::PathBuf>> = BTreeMap::new();
    for f in &files {
        let n = file_name(f).to_ascii_lowercase();
        if let Some((stem, ext)) = n.rsplit_once('.')
            && matches!(ext, "tpc" | "tga" | "dds")
        {
            by_stem.entry(stem.to_string()).or_default().push(f);
        }
    }
    for group in by_stem.values() {
        let is = |p: &&&std::path::PathBuf, e: &str| {
            file_name(p)
                .to_ascii_lowercase()
                .ends_with(&format!(".{e}"))
        };
        let shadows: Vec<&&std::path::PathBuf> = group
            .iter()
            .filter(|p| is(p, "tga") || is(p, "dds"))
            .collect();
        if shadows.is_empty() {
            continue;
        }
        for tpc in group.iter().filter(|p| is(p, "tpc")) {
            out.push(Finding {
                code: codes::TPC_SHADOW,
                path: rel(tpc),
                site: String::new(),
                value: shadows
                    .iter()
                    .map(|q| format!("`{}`", file_name(q)))
                    .collect::<Vec<_>>()
                    .join(", "),
                want: None,
                related: shadows.iter().map(|q| rel(q)).collect(),
            });
        }
    }
}

/// The talk table's length, from `dialog.tlk`'s header.
fn talk_table_len(root: &Path) -> Option<u32> {
    let p = child_ci(root, "dialog.tlk")?;
    let bytes = std::fs::read(p).ok()?;
    rakata_tlk::Tlk::parse(&bytes).ok().map(|t| t.len() as u32)
}

/// Lint the install `o.game`.
pub fn run(o: &Options) -> Result<Report, Diagnostic> {
    let started = Instant::now();
    let mut install = Install::open(o.game, o.profile)
        .map_err(|e| crate::index::diagnostic(e.code(), e.message()))?;
    let mut report = Report::default();
    let mut map = SourceMap::new();
    let engine = match nwscript::engine::load_table(&mut install, &mut map) {
        Ok(Some((t, _))) => Some(t),
        _ => None,
    };
    let routines = engine
        .as_ref()
        .map(crate::index::routines_of)
        .unwrap_or_default();
    report.talk_table = talk_table_len(o.game);
    if report.talk_table.is_none() {
        report.warnings.push(crate::index::diagnostic(
            codes::SKIPPED,
            format!(
                "`{}` holds no dialog.tlk that reads: strrefs are not checked",
                o.game.display()
            ),
        ));
    }
    if engine.is_none() {
        report.warnings.push(crate::index::diagnostic(
            codes::SKIPPED,
            format!(
                "`{}` holds no nwscript.nss: what scripts pass to the engine is not checked",
                o.game.display()
            ),
        ));
    }
    let universe = Universe::open(&install);
    let listed: Option<HashSet<String>> = o.paths.map(|p| {
        p.iter()
            .map(|s| s.replace('\\', "/").to_ascii_lowercase())
            .collect()
    });

    let read = match o.db {
        Some(db) => {
            let conn = rakata_index::open(db).map_err(|e| crate::index::error(&e))?;
            let Some((tree, label)) = source::tree_of(&conn, o.game)? else {
                return Err(crate::index::diagnostic(
                    codes::NO_TREE,
                    format!(
                        "the index `{}` holds no tree built from `{}`: build one with `rakata index build --game {} --tree LABEL --db {}`",
                        db.display(),
                        o.game.display(),
                        o.game.display(),
                        db.display()
                    ),
                ));
            };
            report.source = format!("index (tree {label})");
            source::index(&conn, tree, &universe, listed.as_ref())?
        }
        None => {
            report.source = "walk".into();
            source::walk(
                o.game,
                install.key(),
                &universe,
                &routines,
                listed.as_ref(),
                o.threads,
            )?
        }
    };
    report.checked = read.items.len() + read.unread.len();
    report.unread = read.unread.len();
    if !read.unread.is_empty() {
        let mut first: Vec<&(String, String)> = read.unread.iter().collect();
        first.sort();
        let names: Vec<String> = first
            .iter()
            .take(3)
            .map(|(p, e)| format!("`{p}` ({e})"))
            .collect();
        report.warnings.push(crate::index::diagnostic(
            codes::UNREAD,
            format!(
                "{} resources did not read, so what they name is not checked: {}{}",
                read.unread.len(),
                names.join(", "),
                if read.unread.len() > 3 { ", …" } else { "" }
            ),
        ));
    }

    let object_types = engine.as_ref().map(rules::object_types).unwrap_or_default();
    let rules = Rules {
        universe: &universe,
        talk_table: report.talk_table,
        engine: engine.as_ref(),
        object_types,
    };
    let mut all = Vec::new();
    for item in &read.items {
        rules.check(item, &mut all);
    }
    let mut over = Vec::new();
    override_rules(&install, o.game, &mut over);
    all.extend(over.into_iter().filter(|f| {
        listed.as_ref().is_none_or(|l| {
            std::iter::once(&f.path)
                .chain(&f.related)
                .any(|p| l.contains(&p.to_ascii_lowercase()))
        })
    }));
    all.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    all.dedup();
    for f in &all {
        if o.known.is_some_and(|k| k.contains(f)) {
            *report.known.entry(f.code).or_default() += 1;
        } else {
            *report.counts.entry(f.code).or_default() += 1;
            report.findings.push(f.clone());
        }
    }
    report.all = all;
    report.seconds = started.elapsed().as_secs_f64();
    Ok(report)
}

#[cfg(test)]
mod tests;
