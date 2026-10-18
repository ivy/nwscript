//! The round-trip suite over a game tree: every file a registered
//! [`Format`] reads is read, written again, and classified as identical,
//! equal after re-read (another writer's layout, named through the known
//! variants file), or different (a defect). Files no format reads are
//! counted as "no reader", so the suite says how much of a tree it covers.
//!
//! A format registers once, here, through [`Registry::builtin`]; the
//! archives are the first. Archives are walked recursively: a member of a
//! `.mod` is classified by its own format, and the archive by the archive
//! format.
//!
//! `rakata conformance` is a thin command over [`run`]; a test that
//! registers a deliberately wrong writer goes through the same function.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::codes;
use crate::diag::{Diagnostic, Exit};

/// Why a reader or writer refused: a registered diagnostic code and the
/// message the format rendered for this file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The code, registered in [`crate::codes`].
    pub code: &'static str,
    pub message: String,
}

impl Problem {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Problem {
            code,
            message: message.into(),
        }
    }
}

/// A resource held inside another: a member of an archive, classified by
/// its own extension.
pub struct Nested<'a> {
    /// How the member is named inside its container: `name.ext`, or
    /// `#index.ext` where the container holds no names.
    pub name: String,
    /// The member's extension, lowercase; `None` when the container's type
    /// id names no known type.
    pub extension: Option<String>,
    pub bytes: &'a [u8],
}

/// A format the suite exercises: a reader, a writer, and how two readings
/// compare. The model borrows from the bytes it was read from, as the
/// archive types do; a format whose model owns its data ignores the
/// lifetime.
pub trait Format: Send + Sync {
    /// What the reader produces.
    type Model<'a>;

    /// The format's name, as `--only` selects it and the report lists it.
    fn name(&self) -> &'static str;

    /// The extensions this format reads, lowercase, without the dot.
    fn extensions(&self) -> &'static [&'static str];

    fn read<'a>(&self, bytes: &'a [u8]) -> Result<Self::Model<'a>, Problem>;

    fn write(&self, model: &Self::Model<'_>) -> Result<Vec<u8>, Problem>;

    /// Whether two readings hold the same data, layout aside.
    fn semantic_eq(&self, a: &Self::Model<'_>, b: &Self::Model<'_>) -> bool;

    /// The resources a model holds, for a container format; none by
    /// default.
    fn members<'m>(&self, _model: &'m Self::Model<'_>) -> Vec<Nested<'m>> {
        Vec::new()
    }

    /// The layout facts of a file as written, one line, for a file whose
    /// bytes the writer did not reproduce: the offsets and header fields
    /// another writer chose. The known variants file names a layout by a
    /// glob over this line. None by default.
    fn layout(&self, _bytes: &[u8], _model: &Self::Model<'_>) -> Option<String> {
        None
    }
}

/// How one file's round trip came out.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// The writer reproduced the file byte for byte.
    Identical,
    /// The bytes differ, and the written file reads back to the same data:
    /// a layout another writer chose.
    Equal,
    /// The bytes differ, and so does the data read back; or the writer
    /// failed, or its output does not read.
    Different,
    /// The reader refused the file.
    Failed,
}

/// The result of one round trip, before the known variants are applied.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub class: Class,
    /// What happened, for a file that is not identical.
    pub detail: Option<String>,
    /// The diagnostic code, for a failed read or a write that does not
    /// read back.
    pub code: Option<&'static str>,
    /// The layout line of a file that is not identical, from
    /// [`Format::layout`].
    pub layout: Option<String>,
}

impl Outcome {
    const IDENTICAL: Outcome = Outcome {
        class: Class::Identical,
        detail: None,
        code: None,
        layout: None,
    };
}

/// A [`Format`] with its model erased: what the [`Registry`] holds. Every
/// `Format` is one through the blanket implementation.
pub trait Driver: Send + Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    /// Read, write and compare, handing each member the model holds to
    /// `nested` before the comparison.
    fn classify(&self, bytes: &[u8], nested: &mut dyn FnMut(Nested<'_>)) -> Outcome;
    /// Read and hand each member to `nested` without writing or comparing:
    /// how a container the run does not score is still walked into.
    fn descend(&self, bytes: &[u8], nested: &mut dyn FnMut(Nested<'_>)) -> Result<(), Problem>;
    /// The format as a pair, for one whose resource is two files of one
    /// name ([`PairFormat`]); none by default.
    fn pair(&self) -> Option<&dyn PairDriver> {
        None
    }
}

/// A format whose resource is two files of one name, read and written
/// together: a model's MDL and MDX. The suite gathers both halves from
/// across the tree (a BIF member by the name the KEY gives it, a RIM's
/// companion from its `dx` partner) and scores each half from one round
/// trip of the pair.
pub trait PairFormat: Send + Sync {
    /// What the reader produces.
    type Model<'a>;

    /// The format's name, as `--only` selects it and the report lists it.
    fn name(&self) -> &'static str;

    /// The two extensions, lowercase: the first half's and the second's.
    fn extensions(&self) -> &'static [&'static str];

    fn read<'a>(&self, first: &'a [u8], second: &'a [u8]) -> Result<Self::Model<'a>, Problem>;

    fn write(&self, model: &Self::Model<'_>) -> Result<[Vec<u8>; 2], Problem>;

    /// Whether two readings hold the same data, layout aside.
    fn semantic_eq(&self, a: &Self::Model<'_>, b: &Self::Model<'_>) -> bool;

    /// The layout facts of each half as written, one line each, for a pair
    /// whose bytes the writer did not reproduce (see [`Format::layout`]).
    fn layout(&self, _halves: [&[u8]; 2], _model: &Self::Model<'_>) -> [Option<String>; 2] {
        [None, None]
    }
}

/// A [`PairFormat`] with its model erased.
pub trait PairDriver: Send + Sync {
    /// Read, write and compare the pair: the outcome of each half.
    fn classify_pair(&self, first: &[u8], second: &[u8]) -> [Outcome; 2];
}

/// A [`PairFormat`] as the [`Registry`] holds it.
pub struct Paired<P>(pub P);

impl<P: PairFormat> PairDriver for Paired<P> {
    fn classify_pair(&self, first: &[u8], second: &[u8]) -> [Outcome; 2] {
        let f = &self.0;
        let both = |class: Class, detail: String, code: Option<&'static str>| {
            let o = Outcome {
                class,
                detail: Some(detail),
                code,
                layout: None,
            };
            [o.clone(), o]
        };
        let model = match f.read(first, second) {
            Ok(m) => m,
            Err(p) => return both(Class::Failed, p.message, Some(p.code)),
        };
        let written = match f.write(&model) {
            Ok(w) => w,
            Err(p) => {
                return both(
                    Class::Different,
                    format!("the writer failed: {}", p.message),
                    Some(p.code),
                );
            }
        };
        let halves = [first, second];
        if written[0] == first && written[1] == second {
            return [Outcome::IDENTICAL, Outcome::IDENTICAL];
        }
        let layout = f.layout(halves, &model);
        let again = f.read(&written[0], &written[1]);
        let mut out = [Outcome::IDENTICAL, Outcome::IDENTICAL];
        for (i, o) in out.iter_mut().enumerate() {
            if written[i] == halves[i] {
                continue;
            }
            let shape = difference(halves[i], &written[i]);
            *o = match &again {
                Err(p) => Outcome {
                    class: Class::Different,
                    detail: Some(format!(
                        "the written bytes do not re-read: {}: {}; {shape}",
                        p.code, p.message
                    )),
                    code: Some(codes::RE0502.id),
                    layout: layout[i].clone(),
                },
                Ok(a) if f.semantic_eq(&model, a) => Outcome {
                    class: Class::Equal,
                    detail: Some(shape),
                    code: None,
                    layout: layout[i].clone(),
                },
                Ok(_) => Outcome {
                    class: Class::Different,
                    detail: Some(shape),
                    code: None,
                    layout: layout[i].clone(),
                },
            };
        }
        out
    }
}

impl<P: PairFormat> Driver for Paired<P> {
    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn extensions(&self) -> &'static [&'static str] {
        self.0.extensions()
    }

    /// A half met alone is read as the first half with no second; the
    /// suite pairs halves before it classifies them.
    fn classify(&self, bytes: &[u8], _nested: &mut dyn FnMut(Nested<'_>)) -> Outcome {
        let [o, _] = self.classify_pair(bytes, &[]);
        o
    }

    fn descend(&self, _bytes: &[u8], _nested: &mut dyn FnMut(Nested<'_>)) -> Result<(), Problem> {
        Ok(())
    }

    fn pair(&self) -> Option<&dyn PairDriver> {
        Some(self)
    }
}

impl<F: Format> Driver for F {
    fn name(&self) -> &'static str {
        Format::name(self)
    }

    fn extensions(&self) -> &'static [&'static str] {
        Format::extensions(self)
    }

    fn descend(&self, bytes: &[u8], nested: &mut dyn FnMut(Nested<'_>)) -> Result<(), Problem> {
        let model = self.read(bytes)?;
        for member in self.members(&model) {
            nested(member);
        }
        Ok(())
    }

    fn classify(&self, bytes: &[u8], nested: &mut dyn FnMut(Nested<'_>)) -> Outcome {
        let model = match self.read(bytes) {
            Ok(m) => m,
            Err(p) => {
                return Outcome {
                    class: Class::Failed,
                    detail: Some(p.message),
                    code: Some(p.code),
                    layout: None,
                };
            }
        };
        for member in self.members(&model) {
            nested(member);
        }
        let written = match self.write(&model) {
            Ok(w) => w,
            Err(p) => {
                return Outcome {
                    class: Class::Different,
                    detail: Some(format!("the writer failed: {}", p.message)),
                    code: Some(p.code),
                    layout: self.layout(bytes, &model),
                };
            }
        };
        if written == bytes {
            return Outcome::IDENTICAL;
        }
        let shape = difference(bytes, &written);
        let layout = self.layout(bytes, &model);
        match self.read(&written) {
            Err(p) => Outcome {
                class: Class::Different,
                detail: Some(format!(
                    "the written bytes do not re-read: {}: {}; {shape}",
                    p.code, p.message
                )),
                code: Some(codes::RE0502.id),
                layout,
            },
            Ok(again) if self.semantic_eq(&model, &again) => Outcome {
                class: Class::Equal,
                detail: Some(shape),
                code: None,
                layout,
            },
            Ok(_) => Outcome {
                class: Class::Different,
                detail: Some(shape),
                code: None,
                layout,
            },
        }
    }
}

/// Where two byte strings part: their lengths and the first differing
/// offset.
fn difference(source: &[u8], written: &[u8]) -> String {
    let first = source
        .iter()
        .zip(written)
        .position(|(a, b)| a != b)
        .unwrap_or(source.len().min(written.len()));
    format!(
        "{} bytes read, {} written, first difference at offset {first}",
        source.len(),
        written.len()
    )
}

/// The formats the suite runs, each claiming its extensions.
#[derive(Default)]
pub struct Registry {
    formats: Vec<Box<dyn Driver>>,
    /// The formats the run scores, when `only` narrowed it; every format
    /// still reads its files, so a selected format inside an unselected
    /// container is reached.
    scored: Option<Vec<String>>,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.names()).finish()
    }
}

impl Registry {
    /// No formats.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Every format the workspace has: the archives today, each format
    /// crate as it lands.
    pub fn builtin() -> Self {
        let mut r = Self::empty();
        r.register(Box::new(crate::formats::Archive));
        r.register(Box::new(crate::formats::Tlk));
        r.register(Box::new(crate::formats::Lip));
        r.register(Box::new(crate::formats::Lyt));
        r.register(Box::new(crate::formats::Vis));
        r.register(Box::new(crate::formats::Tpc));
        r.register(Box::new(crate::formats::Txi));
        r.register(Box::new(crate::formats::Tga));
        r.register(Box::new(crate::formats::Walkmesh));
        r.register(Box::new(crate::formats::TwoDa));
        r.register(Box::new(crate::formats::Gff));
        r.register(Box::new(Paired(crate::formats::Mdl)));
        r
    }

    /// Add a format. A format claiming an extension another holds is a
    /// programming error, caught here.
    pub fn register(&mut self, format: Box<dyn Driver>) {
        for ext in format.extensions() {
            assert!(
                self.by_extension(ext).is_none(),
                "two formats read `.{ext}`: {} and {}",
                self.by_extension(ext).map_or("", Driver::name),
                format.name()
            );
        }
        self.formats.push(format);
    }

    /// Score only the formats named; the others still read their files
    /// and are walked into, so `--only gff` reaches the GFFs inside every
    /// RIM, MOD and BIF. An unknown name is an error naming the formats
    /// there are.
    pub fn only(mut self, names: &[String]) -> Result<Self, String> {
        for n in names {
            if !self.formats.iter().any(|f| f.name() == n) {
                return Err(format!(
                    "no format `{n}`; the formats are {}",
                    self.names().join(", ")
                ));
            }
        }
        self.scored = Some(names.to_vec());
        Ok(self)
    }

    /// Every registered format's name, scored or not.
    pub fn names(&self) -> Vec<&'static str> {
        self.formats.iter().map(|f| f.name()).collect()
    }

    /// Whether the run scores this format's files, or only walks them.
    pub fn is_scored(&self, name: &str) -> bool {
        self.scored
            .as_ref()
            .is_none_or(|s| s.iter().any(|n| n == name))
    }

    /// The formats the run scores and the report lists.
    pub fn formats(&self) -> impl Iterator<Item = &dyn Driver> {
        self.formats
            .iter()
            .map(Box::as_ref)
            .filter(|f| self.is_scored(f.name()))
    }

    /// The format reading files with this extension, in any case.
    pub fn by_extension(&self, ext: &str) -> Option<&dyn Driver> {
        self.formats
            .iter()
            .find(|f| f.extensions().iter().any(|e| e.eq_ignore_ascii_case(ext)))
            .map(Box::as_ref)
    }
}

/// A named layout another writer chooses: what `conformance/rakata-known.toml`
/// records for a file that is equal after re-read.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    /// The format whose files show it.
    pub format: String,
    /// The tool that writes it.
    pub writer: String,
    /// How the layout differs from BioWare's.
    pub description: String,
    /// How a file is recognised: a glob (`*` and `?`) over the layout line
    /// the format reports for it ([`Format::layout`]).
    pub recognised: String,
    /// Globs over paths within the tree, `/`-separated with `!` before a
    /// member's name, for a variant the layout line alone does not tell;
    /// a file matching one is this variant.
    #[serde(default)]
    pub files: Vec<String>,
}

/// The known variants file.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Known {
    #[serde(default)]
    pub variant: BTreeMap<String, Variant>,
}

impl Known {
    /// The file at `path`; an absent file is no variants, a malformed one
    /// an error naming it.
    pub fn load(path: &Path) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(t) => toml::from_str(&t).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    /// The variant a file of `format` matches, by its layout line or its
    /// path, if any.
    pub fn variant_of(&self, format: &str, path: &str, layout: Option<&str>) -> Option<&str> {
        self.variant
            .iter()
            .find(|(_, v)| {
                v.format == format
                    && (layout.is_some_and(|l| glob_match(&v.recognised, l))
                        || v.files.iter().any(|g| glob_match(g, path)))
            })
            .map(|(k, _)| k.as_str())
    }
}

/// `*` matches any run of characters, `?` one; case does not matter, as
/// the engine finds files.
pub fn glob_match(pat: &str, s: &str) -> bool {
    fn go(p: &[char], s: &[char]) -> bool {
        match (p.first(), s.first()) {
            (None, None) => true,
            (Some('*'), _) => go(&p[1..], s) || (!s.is_empty() && go(p, &s[1..])),
            (Some('?'), Some(_)) => go(&p[1..], &s[1..]),
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => go(&p[1..], &s[1..]),
            _ => false,
        }
    }
    let p: Vec<char> = pat.chars().collect();
    let s: Vec<char> = s.chars().collect();
    go(&p, &s)
}

/// One file that is not identical, in the report.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub class: Class,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The diagnostic code of a failed read, or of a write that does not
    /// read back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// The known variant an equal-after-re-read file matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// The layout line the format reported for the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub files: usize,
    pub identical: usize,
    pub equal: usize,
    pub different: usize,
    pub failed: usize,
}

impl Counts {
    fn add(&mut self, class: Class) {
        self.files += 1;
        match class {
            Class::Identical => self.identical += 1,
            Class::Equal => self.equal += 1,
            Class::Different => self.different += 1,
            Class::Failed => self.failed += 1,
        }
    }
}

/// One format's part of the report.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FormatReport {
    pub name: String,
    pub extensions: Vec<String>,
    pub counts: Counts,
    /// Time spent reading, writing and comparing this format's files,
    /// summed over the threads that did it.
    pub busy_seconds: f64,
    /// Every file that is not identical, by its path within the tree.
    pub entries: BTreeMap<String, Entry>,
}

/// The files no registered format reads.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct NoReader {
    pub total: usize,
    /// By extension; a member whose type id names no known type counts
    /// under `type<id>`.
    pub by_extension: BTreeMap<String, usize>,
}

/// The report of one run.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Report {
    /// The tree, as named on the command line.
    pub tree: String,
    /// Every file and member seen.
    pub files: usize,
    pub wall_seconds: f64,
    pub threads: usize,
    pub formats: Vec<FormatReport>,
    pub no_reader: NoReader,
    /// The known variants file's contents, so the report explains its
    /// own variant names.
    pub variants: BTreeMap<String, Variant>,
}

impl Report {
    /// The exit code the run earns: 3 when any file is different, 1 when
    /// any file failed to read, else 0.
    pub fn exit(&self) -> Exit {
        if self.formats.iter().any(|f| f.counts.different > 0) {
            Exit::Different
        } else if self.formats.iter().any(|f| f.counts.failed > 0) {
            Exit::Errors
        } else {
            Exit::Ok
        }
    }

    /// One diagnostic per file whose entry carries a code.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for f in &self.formats {
            for (path, e) in &f.entries {
                let Some(code) = e.code.as_deref().and_then(codes::lookup) else {
                    continue;
                };
                out.push(Diagnostic::new(
                    code,
                    format!("{path}: {}", e.detail.as_deref().unwrap_or("")),
                ));
            }
        }
        out
    }

    /// Per-format counts, in the five columns the README table shows.
    pub fn markdown_table(&self) -> String {
        let mut s = String::from(
            "| Format | Files | Identical | Equal after re-read | Different | No reader |\n| --- | ---: | ---: | ---: | ---: | ---: |\n",
        );
        for f in &self.formats {
            let c = &f.counts;
            let different = match c.failed {
                0 => c.different.to_string(),
                n => format!("{} ({n} unreadable)", c.different),
            };
            s.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | |\n",
                f.name, c.files, c.identical, c.equal, different
            ));
        }
        s.push_str(&format!(
            "| no format | {} | | | | {} |\n",
            self.no_reader.total, self.no_reader.total
        ));
        s
    }

    /// The summary: the table, the timings, each file that is not
    /// identical by format and class, and the extensions with no reader.
    pub fn markdown(&self) -> String {
        let mut s = format!(
            "# Round-trip conformance\n\n{} files in `{}`, {} with a registered format, in {:.2} s on {} threads.\n\n",
            self.files,
            self.tree,
            self.files - self.no_reader.total,
            self.wall_seconds,
            self.threads
        );
        s.push_str(&self.markdown_table());
        for f in &self.formats {
            s.push_str(&format!(
                "\n## {}\n\n`.{}`: {} files in {:.2} s.\n",
                f.name,
                f.extensions.join("`, `."),
                f.counts.files,
                f.busy_seconds
            ));
            for class in [Class::Different, Class::Failed, Class::Equal] {
                let list: Vec<_> = f.entries.iter().filter(|(_, e)| e.class == class).collect();
                if list.is_empty() {
                    continue;
                }
                let heading = match class {
                    Class::Identical => unreachable!(),
                    Class::Equal => "Equal after re-read",
                    Class::Different => "Different",
                    Class::Failed => "Failed to read",
                };
                s.push_str(&format!("\n{heading} ({}):\n\n", list.len()));
                for (path, e) in list {
                    let variant = match (&e.variant, class) {
                        (Some(v), _) => format!(" [{v}]"),
                        (None, Class::Equal) => " [no known variant]".to_string(),
                        _ => String::new(),
                    };
                    let code = e
                        .code
                        .as_deref()
                        .map_or(String::new(), |c| format!("{c}: "));
                    let layout = e
                        .layout
                        .as_deref()
                        .map_or(String::new(), |l| format!("; {l}"));
                    s.push_str(&format!(
                        "- `{path}`{variant}: {code}{}{layout}\n",
                        e.detail.as_deref().unwrap_or("")
                    ));
                }
            }
        }
        if self.no_reader.total > 0 {
            s.push_str(&format!(
                "\n## No reader\n\n{} files, by extension:\n\n",
                self.no_reader.total
            ));
            let mut by: Vec<_> = self.no_reader.by_extension.iter().collect();
            by.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            for (ext, n) in by {
                s.push_str(&format!("- `.{ext}`: {n}\n"));
            }
        }
        s
    }
}

/// A child of `dir` named `name` in any case, as the engine finds it. When
/// several spellings exist (a build that symlinks `override` to
/// `Override`), the one that is not a symlink is preferred, then the exact
/// spelling.
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut matches: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
        .collect();
    matches.sort_by_key(|p| {
        let link = p
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink());
        let exact = p.file_name().is_some_and(|f| f == name);
        (link, !exact)
    });
    matches.into_iter().next()
}

/// The files of `dir` (not its subdirectories), sorted; nothing when it is
/// absent.
fn files_of(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| !t.is_dir()))
        .map(|e| e.path())
        .collect();
    files.sort();
    files
}

/// Every file under `dir`, at any depth, sorted.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    fn go(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                go(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    go(dir, &mut out);
    out.sort();
    out
}

/// The files of a tree the suite classifies: `chitin.key` and the BIFs it
/// names, every other `data/*.bif`, `Override/` at any depth, `modules/`,
/// `rims/`, `lips/`, `patch.erf`, `TexturePacks/*.erf`, `streamwaves/**/*.lip`
/// and `dialog.tlk`. Folder names are matched in any case, and a file
/// reached twice (a symlinked folder, a BIF both named and present) is
/// listed once. Fails when `root` has no `chitin.key`.
pub fn walk(root: &Path) -> Result<Vec<PathBuf>, Diagnostic> {
    let key_path = child_ci(root, "chitin.key").ok_or_else(|| {
        Diagnostic::new(
            &codes::RE0501,
            format!(
                "`{}` is not a game install: it has no chitin.key",
                root.display()
            ),
        )
    })?;
    let mut out = vec![key_path.clone()];
    if let Ok(bytes) = fs::read(&key_path)
        && let Ok(key) = rakata_archive::Key::parse(&bytes)
    {
        for b in key.bifs() {
            let mut p = root.to_path_buf();
            let mut found = true;
            for part in b.components() {
                match child_ci(&p, &part) {
                    Some(next) => p = next,
                    None => {
                        found = false;
                        break;
                    }
                }
            }
            if found {
                out.push(p);
            }
        }
    }
    let dir = |name: &str| child_ci(root, name).unwrap_or_else(|| root.join(name));
    out.extend(
        files_of(&dir("data"))
            .into_iter()
            .filter(|p| has_extension(p, "bif")),
    );
    out.extend(files_under(&dir("override")));
    out.extend(files_of(&dir("modules")));
    out.extend(files_of(&dir("rims")));
    out.extend(files_of(&dir("lips")));
    out.extend(child_ci(root, "patch.erf"));
    out.extend(
        files_of(&dir("texturepacks"))
            .into_iter()
            .filter(|p| has_extension(p, "erf")),
    );
    out.extend(
        files_under(&dir("streamwaves"))
            .into_iter()
            .filter(|p| has_extension(p, "lip")),
    );
    out.extend(child_ci(root, "dialog.tlk"));

    let mut seen = std::collections::HashSet::new();
    out.retain(|p| {
        let id = p.canonicalize().unwrap_or_else(|_| p.clone());
        seen.insert(id)
    });
    Ok(out)
}

fn has_extension(p: &Path, ext: &str) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// Extension of a path or member name, lowercase; the text after the last
/// dot.
fn extension_of(name: &str) -> Option<String> {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .filter(|e| !e.is_empty())
}

/// What one thread accumulates.
#[derive(Default)]
struct Tally {
    counts: BTreeMap<&'static str, (Counts, Duration)>,
    entries: BTreeMap<&'static str, BTreeMap<String, Entry>>,
    no_reader: NoReader,
    /// The halves of pair formats' resources, classified once the walk
    /// has met them all.
    halves: Vec<Half>,
}

/// One half of a [`PairFormat`]'s resource, held until the walk ends.
struct Half {
    format: &'static str,
    /// Where its other half is looked for: the container or folder, and
    /// the resref (see [`pair_key`]).
    key: (String, String),
    /// Which extension of the format's it has: 0 the first, 1 the second.
    role: usize,
    path: String,
    bytes: Vec<u8>,
}

/// The names the KEY gives BIF members: by the BIF's path within the tree
/// and the member's index, lowercase.
type BifNames = std::collections::HashMap<(String, usize), String>;

fn bif_names(root: &Path) -> BifNames {
    let mut out = BifNames::new();
    let Some(key) = child_ci(root, "chitin.key")
        .and_then(|p| fs::read(p).ok())
        .and_then(|b| rakata_archive::Key::parse(&b).ok())
    else {
        return out;
    };
    let paths: Vec<String> = key
        .bifs()
        .iter()
        .map(|b| {
            b.components()
                .collect::<Vec<_>>()
                .join("/")
                .to_ascii_lowercase()
        })
        .collect();
    for e in key.entries() {
        if let Some(p) = paths.get(e.bif as usize) {
            out.insert(
                (p.clone(), e.index as usize),
                e.id.name.to_string().to_ascii_lowercase(),
            );
        }
    }
    out
}

/// Where a half's other half lives, as `(container, resref)`, lowercase:
/// the archive holding it (a RIM's `dx` partner named as the RIM, which
/// holds the first halves) or the folder of a loose file, and the
/// resource's name (a BIF member's from the KEY).
fn pair_key(path: &str, names: &BifNames) -> (String, String) {
    let lower = path.to_ascii_lowercase();
    let (container, member) = lower
        .rsplit_once('!')
        .or_else(|| lower.rsplit_once('/'))
        .unwrap_or(("", &lower));
    let stem = member.rsplit_once('.').map_or(member, |(s, _)| s);
    let resref = stem
        .strip_prefix('#')
        .and_then(|i| i.parse::<usize>().ok())
        .and_then(|i| names.get(&(container.to_string(), i)))
        .map_or_else(|| stem.to_string(), String::clone);
    let container = match container.strip_suffix("dx.rim") {
        Some(c) => format!("{c}.rim"),
        None => container.to_string(),
    };
    (container, resref)
}

impl Tally {
    fn merge(&mut self, other: Tally) {
        for (name, (c, d)) in other.counts {
            let e = self.counts.entry(name).or_default();
            e.0.files += c.files;
            e.0.identical += c.identical;
            e.0.equal += c.equal;
            e.0.different += c.different;
            e.0.failed += c.failed;
            e.1 += d;
        }
        for (name, entries) in other.entries {
            self.entries.entry(name).or_default().extend(entries);
        }
        self.no_reader.total += other.no_reader.total;
        for (ext, n) in other.no_reader.by_extension {
            *self.no_reader.by_extension.entry(ext).or_default() += n;
        }
        self.halves.extend(other.halves);
    }

    fn no_reader(&mut self, ext: Option<String>) {
        self.no_reader.total += 1;
        *self
            .no_reader
            .by_extension
            .entry(ext.unwrap_or_else(|| "(none)".to_string()))
            .or_default() += 1;
    }

    /// Count one file's outcome, and list it when it is not identical.
    fn record(
        &mut self,
        known: &Known,
        name: &'static str,
        path: &str,
        outcome: Outcome,
        busy: Duration,
    ) {
        let slot = self.counts.entry(name).or_default();
        slot.0.add(outcome.class);
        slot.1 += busy;
        if outcome.class != Class::Identical {
            let variant = (outcome.class == Class::Equal)
                .then(|| known.variant_of(name, path, outcome.layout.as_deref()))
                .flatten()
                .map(str::to_string);
            self.entries.entry(name).or_default().insert(
                path.to_string(),
                Entry {
                    class: outcome.class,
                    detail: outcome.detail,
                    code: outcome.code.map(str::to_string),
                    variant,
                    layout: outcome.layout,
                },
            );
        }
    }

    /// Classify `bytes` at `path` (within the tree) and every member it
    /// holds, recursively; hold a pair format's half for [`pairs`].
    fn classify(
        &mut self,
        registry: &Registry,
        known: &Known,
        names: &BifNames,
        path: &str,
        bytes: &[u8],
    ) {
        let ext = extension_of(path);
        let Some(format) = ext.as_deref().and_then(|e| registry.by_extension(e)) else {
            self.no_reader(ext);
            return;
        };
        let name = format.name();
        if format.pair().is_some() {
            if registry.is_scored(name) {
                let role = format
                    .extensions()
                    .iter()
                    .position(|e| Some(*e) == ext.as_deref())
                    .unwrap_or(0);
                self.halves.push(Half {
                    format: name,
                    key: pair_key(path, names),
                    role,
                    path: path.to_string(),
                    bytes: bytes.to_vec(),
                });
            }
            return;
        }
        let start = Instant::now();
        let mut members: Vec<(String, Vec<u8>)> = Vec::new();
        let mut collect = |m: Nested<'_>| {
            let member_path = format!("{path}!{}", m.name);
            let readable = m
                .extension
                .as_deref()
                .is_some_and(|e| registry.by_extension(e).is_some());
            if readable {
                members.push((member_path, m.bytes.to_vec()));
            } else {
                self.no_reader(m.extension.clone());
            }
        };
        if !registry.is_scored(name) {
            // Walked, not scored: a container holding files of a scored
            // format, or a file the run was not asked about.
            let _ = format.descend(bytes, &mut collect);
            for (member_path, data) in members {
                self.classify(registry, known, names, &member_path, &data);
            }
            return;
        }
        let outcome = format.classify(bytes, &mut collect);
        self.record(known, name, path, outcome, start.elapsed());
        for (member_path, data) in members {
            self.classify(registry, known, names, &member_path, &data);
        }
    }
}

/// One round trip of a pair: the halves read together, and which of them
/// the run scores (a half read with a partner from elsewhere in the tree
/// is not scored again).
struct Job {
    format: &'static str,
    halves: [Option<usize>; 2],
    scored: [bool; 2],
}

/// Pair the halves the walk held and classify each pair, on `threads`
/// threads. Halves pair within their container or folder; a half with no
/// partner there is read with the partner of its name the tree holds
/// elsewhere (the engine resolves each half by name), and a first half
/// with none anywhere with an empty second. A second half no first half
/// of its name reads counts as having no reader.
fn pairs(registry: &Registry, known: &Known, halves: &[Half], threads: usize) -> Tally {
    let mut tally = Tally::default();
    let mut order: Vec<usize> = (0..halves.len()).collect();
    order.sort_by(|&a, &b| halves[a].path.cmp(&halves[b].path));
    type Group = [Vec<usize>; 2];
    let mut groups: BTreeMap<(&'static str, &(String, String)), Group> = BTreeMap::new();
    let mut by_name: BTreeMap<(&str, &str, usize), usize> = BTreeMap::new();
    for &i in &order {
        let h = &halves[i];
        groups.entry((h.format, &h.key)).or_default()[h.role].push(i);
        by_name.entry((h.format, &h.key.1, h.role)).or_insert(i);
    }
    let mut jobs = Vec::new();
    for (&(format, key), [firsts, seconds]) in &groups {
        let elsewhere = |role: usize| by_name.get(&(format, key.1.as_str(), role)).copied();
        for i in 0..firsts.len().max(seconds.len()) {
            match (firsts.get(i), seconds.get(i)) {
                (Some(&a), Some(&b)) => jobs.push(Job {
                    format,
                    halves: [Some(a), Some(b)],
                    scored: [true, true],
                }),
                (Some(&a), None) => jobs.push(Job {
                    format,
                    halves: [Some(a), elsewhere(1)],
                    scored: [true, false],
                }),
                (None, Some(&b)) => match elsewhere(0) {
                    Some(a) => jobs.push(Job {
                        format,
                        halves: [Some(a), Some(b)],
                        scored: [false, true],
                    }),
                    None => tally.no_reader(extension_of(&halves[b].path)),
                },
                (None, None) => {}
            }
        }
    }
    let next = AtomicUsize::new(0);
    let total = Mutex::new(tally);
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, jobs.len().max(1)) {
            s.spawn(|| {
                let mut t = Tally::default();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(i) else { break };
                    let Some(driver) = registry
                        .formats
                        .iter()
                        .find(|f| f.name() == job.format)
                        .and_then(|f| f.pair())
                    else {
                        continue;
                    };
                    let bytes = |k: usize| job.halves[k].map_or(&[][..], |h| &halves[h].bytes);
                    let start = Instant::now();
                    let outcomes = driver.classify_pair(bytes(0), bytes(1));
                    let busy = start.elapsed() / 2;
                    for (k, outcome) in outcomes.into_iter().enumerate() {
                        if let (true, Some(h)) = (job.scored[k], job.halves[k]) {
                            t.record(known, job.format, &halves[h].path, outcome, busy);
                        }
                    }
                }
                total.lock().expect("no thread panicked").merge(t);
            });
        }
    });
    total.into_inner().expect("no thread panicked")
}

/// Files this large are memory-mapped rather than read whole.
const MAP_THRESHOLD: u64 = 1 << 20;

/// Run the suite over the install at `root` with the formats in
/// `registry`, naming equal-after-re-read layouts from `known`, on
/// `threads` threads (0 for one per CPU). Fails only when `root` is not an
/// install; every other problem is a classification in the report.
pub fn run(
    root: &Path,
    registry: &Registry,
    known: &Known,
    threads: usize,
) -> Result<Report, Diagnostic> {
    let start = Instant::now();
    let files = walk(root)?;
    let names = bif_names(root);
    let threads = match threads {
        0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
        n => n,
    }
    .min(files.len().max(1));
    let next = AtomicUsize::new(0);
    let total = Mutex::new(Tally::default());
    let rel = |p: &Path| -> String {
        p.strip_prefix(root)
            .unwrap_or(p)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    };
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut tally = Tally::default();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = files.get(i) else { break };
                    let rel_path = rel(path);
                    let size = fs::metadata(path).map_or(0, |m| m.len());
                    let source = if size >= MAP_THRESHOLD {
                        rakata_archive::Source::map(path)
                    } else {
                        rakata_archive::Source::read(path)
                    };
                    match source {
                        Ok(src) => tally.classify(registry, known, &names, &rel_path, &src),
                        Err(e) => {
                            let ext = extension_of(&rel_path);
                            match ext.as_deref().and_then(|x| registry.by_extension(x)) {
                                Some(f) => {
                                    let slot = tally.counts.entry(f.name()).or_default();
                                    slot.0.add(Class::Failed);
                                    tally.entries.entry(f.name()).or_default().insert(
                                        rel_path,
                                        Entry {
                                            class: Class::Failed,
                                            detail: Some(e.to_string()),
                                            code: Some(e.code().to_string()),
                                            variant: None,
                                            layout: None,
                                        },
                                    );
                                }
                                None => tally.no_reader(ext),
                            }
                        }
                    }
                }
                total.lock().expect("no thread panicked").merge(tally);
            });
        }
    });
    let mut tally = total.into_inner().expect("no thread panicked");
    let halves = std::mem::take(&mut tally.halves);
    tally.merge(pairs(registry, known, &halves, threads));
    let formats = registry
        .formats()
        .map(|f| {
            let (counts, busy) = tally.counts.get(f.name()).cloned().unwrap_or_default();
            FormatReport {
                name: f.name().to_string(),
                extensions: f.extensions().iter().map(|e| e.to_string()).collect(),
                counts,
                busy_seconds: busy.as_secs_f64(),
                entries: tally.entries.get(f.name()).cloned().unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    let with_format: usize = formats.iter().map(|f| f.counts.files).sum();
    Ok(Report {
        tree: root.display().to_string(),
        files: with_format + tally.no_reader.total,
        wall_seconds: start.elapsed().as_secs_f64(),
        threads,
        formats,
        no_reader: tally.no_reader,
        variants: known.variant.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob_match("modules/*.mod", "modules/danm13.mod"));
        assert!(glob_match("MODULES/*.MOD", "modules/danm13.mod"));
        assert!(glob_match(
            "modules/*.mod!*.utc",
            "modules/danm13.mod!p_bastila.utc"
        ));
        assert!(!glob_match(
            "modules/*.mod",
            "modules/danm13.mod!p_bastila.utc"
        ));
        assert!(glob_match("k_?", "k_a"));
        assert!(!glob_match("k_?", "k_ab"));
    }

    #[test]
    fn extensions_are_the_text_after_the_last_dot_lowercased() {
        assert_eq!(
            extension_of("Override/P_Bastila.UTC").as_deref(),
            Some("utc")
        );
        assert_eq!(
            extension_of("modules/a.mod!#12.2da").as_deref(),
            Some("2da")
        );
        assert_eq!(extension_of("README"), None);
        assert_eq!(extension_of("trailing."), None);
    }

    #[test]
    fn the_table_has_the_five_columns_and_counts_unreadable_files_as_different() {
        let r = Report {
            tree: "t".into(),
            files: 12,
            wall_seconds: 0.5,
            threads: 1,
            formats: vec![FormatReport {
                name: "archive".into(),
                extensions: vec!["rim".into()],
                counts: Counts {
                    files: 10,
                    identical: 7,
                    equal: 1,
                    different: 1,
                    failed: 1,
                },
                busy_seconds: 0.1,
                entries: BTreeMap::new(),
            }],
            no_reader: NoReader {
                total: 2,
                by_extension: BTreeMap::from([("utc".to_string(), 2)]),
            },
            variants: BTreeMap::new(),
        };
        let t = r.markdown_table();
        assert!(t.starts_with(
            "| Format | Files | Identical | Equal after re-read | Different | No reader |\n"
        ));
        assert!(
            t.contains("| `archive` | 10 | 7 | 1 | 1 (1 unreadable) | |\n"),
            "{t}"
        );
        assert!(t.contains("| no format | 2 | | | | 2 |\n"), "{t}");
        assert_eq!(r.exit(), Exit::Different);
    }
}
