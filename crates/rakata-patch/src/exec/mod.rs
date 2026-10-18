//! The executor: a [`Plan`] applied to a game install in TSLPatcher's phase
//! order, every write read back.
//!
//! **The report is the tree.** After each operation the executor re-reads
//! the file or capsule member it wrote and records the hash it finds there
//! beside the hash of what it wrote; when they differ, or nothing is there,
//! the operation fails under [`codes::READBACK_MISMATCH`] naming both, so a
//! claim that did not land is the installer's own error. A [`Report`] holds
//! one [`Record`] per operation, its target and that hash.
//!
//! What each list reads its target from and where it writes is recorded in
//! `docs/formats/changes-ini.md`, "Where each list reads and writes": the
//! destination's copy first, then the installer's own file, then the copy
//! the game loads (through `rakata-game`), so a table not yet in `Override`
//! starts from the BIF's. Names compare without regard to case.

mod files;
mod gff;
mod script;
mod table;
mod tlk;

pub use files::{Dest, Target};

use crate::changes::{GffEdit, HackWrite, SsfEntry};
use crate::issue::codes;
use crate::load::Loaded;
use crate::plan::{Operation, Phase, Plan, Step};
use crate::ssf::Ssf;
use crate::value::Value;
use files::{Files, find_ci};
use rakata_game::{Install, Profile};
use rakata_res::{ContentHash, ResId};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Why an operation, or the install, did not do what it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Failure {
    /// The registered code: `RE03xx`, `RW03xx`, or a format crate's.
    pub code: &'static str,
    pub message: String,
}

impl Failure {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Failure {
            code,
            message: message.into(),
        }
    }

    /// Warnings carry an `RW` code.
    pub fn is_error(&self) -> bool {
        !self.code.starts_with("RW")
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// The memory tokens as the install left them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Tokens {
    /// `2DAMEMORY#`: what an operation stored in slot `#`.
    pub memory: BTreeMap<u32, String>,
    /// `StrRef#`: the dialog.tlk index entry `#` was appended at.
    pub strref: BTreeMap<u32, u32>,
}

impl Tokens {
    /// A value's text with its token filled. `high()` means something only
    /// in a 2DA cell, which [`table`] handles before asking.
    pub fn resolve(&self, v: &Value) -> Result<String, Failure> {
        match v {
            Value::Literal(s) => Ok(s.clone()),
            Value::Memory(n) => self.memory.get(n).cloned().ok_or_else(|| {
                Failure::new(
                    codes::TOKEN_NO_VALUE,
                    format!("2DAMEMORY{n} is read, and no operation before this one stored it"),
                )
            }),
            Value::StrRef(n) => self.strref.get(n).map(u32::to_string).ok_or_else(|| {
                Failure::new(
                    codes::TOKEN_NO_VALUE,
                    format!("StrRef{n} is read, and [TLKList] appended no entry for it"),
                )
            }),
            Value::High => Err(Failure::new(
                codes::CANNOT_APPLY,
                "high() stands only in a 2DA cell or row label",
            )),
        }
    }
}

/// What became of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Written, and the read-back found the bytes written.
    Written,
    /// Not written, as TSLPatcher skips it (a `File#` whose file exists).
    Skipped,
    /// Something did not apply or did not land; `issues` says what.
    Failed,
}

/// One operation of the plan, what it targeted and what the tree holds
/// there afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Record {
    pub phase: Phase,
    /// The line of the list entry in the changes file.
    pub line: u32,
    /// The operation, as `ChangeRow0 on baseitems.2da`.
    pub operation: String,
    /// What it wrote: `Override/baseitems.2da`, `modules/danm13.mod:x.utc`.
    pub target: String,
    /// Where the bytes it started from came from.
    pub source: Option<String>,
    pub outcome: Outcome,
    /// SHA-256 of the bytes the installer wrote.
    pub written: Option<String>,
    /// SHA-256 of what the read-back found in the target.
    pub found: Option<String>,
    /// What it did, in a few words.
    pub detail: Option<String>,
    pub issues: Vec<Failure>,
}

impl Record {
    fn new(step: &Step, operation: String) -> Self {
        Record {
            phase: step.phase,
            line: step.line,
            operation,
            target: String::new(),
            source: None,
            outcome: Outcome::Written,
            written: None,
            found: None,
            detail: None,
            issues: Vec::new(),
        }
    }

    fn fail(&mut self, f: Failure) {
        if f.is_error() {
            self.outcome = Outcome::Failed;
        }
        self.issues.push(f);
    }

    /// Whether the operation landed as the plan says (a skip included).
    pub fn ok(&self) -> bool {
        self.outcome != Outcome::Failed
    }

    /// How a message names the operation: `[2DAList] line 12, ChangeRow0 on baseitems.2da`.
    pub fn name(&self) -> String {
        format!(
            "[{}] line {}, {}",
            self.phase.section(),
            self.line,
            self.operation
        )
    }
}

/// The outcome of an install.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    pub records: Vec<Record>,
    pub tokens: Tokens,
    /// Capsules the install made, from the module's RIMs, to write into.
    pub created: Vec<String>,
    /// What stopped the install before any operation, if anything did.
    pub failure: Option<Failure>,
}

impl Report {
    /// No operation failed and nothing stopped the install.
    pub fn ok(&self) -> bool {
        self.failure.is_none() && self.records.iter().all(Record::ok)
    }

    /// The records that failed.
    pub fn failed(&self) -> impl Iterator<Item = &Record> {
        self.records.iter().filter(|r| !r.ok())
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(e) = &self.failure {
            writeln!(f, "not installed: {e}")?;
        }
        let mut phase = None;
        for r in &self.records {
            if phase != Some(r.phase) {
                writeln!(f, "[{}]", r.phase.section())?;
                phase = Some(r.phase);
            }
            let state = match r.outcome {
                Outcome::Written => "ok     ",
                Outcome::Skipped => "skipped",
                Outcome::Failed => "FAILED ",
            };
            let hash = r.found.as_deref().map_or("-", |h| &h[..h.len().min(16)]);
            writeln!(f, "  {state} {hash:<16} {:<44} {}", r.target, r.operation)?;
            for i in &r.issues {
                writeln!(f, "          {i}")?;
            }
        }
        for c in &self.created {
            writeln!(f, "made {c} from the module's RIMs")?;
        }
        let failed = self.failed().count();
        let written = self
            .records
            .iter()
            .filter(|r| r.outcome == Outcome::Written)
            .count();
        let skipped = self
            .records
            .iter()
            .filter(|r| r.outcome == Outcome::Skipped)
            .count();
        write!(
            f,
            "{} operations: {written} written and read back, {skipped} skipped, {failed} failed",
            self.records.len()
        )
    }
}

/// Called after each write and before its read-back, with the target.
pub type AfterWrite<'h> = Box<dyn FnMut(&Target) + 'h>;

/// Applies a plan to one install.
pub struct Executor<'h> {
    files: Files,
    install: Install,
    /// The install's listing predates a write under it.
    stale: bool,
    game: nwscript::Game,
    tokens: Tokens,
    created: Vec<String>,
    after_write: Option<AfterWrite<'h>>,
}

impl<'h> Executor<'h> {
    /// An executor for the files `loaded` read from `tslpatchdata`, against
    /// the install at `game_dir`.
    pub fn open(game_dir: &Path, tslpatchdata: &Path, loaded: &Loaded) -> Result<Self, Failure> {
        let k2 = loaded.changes.settings.lookup_game_number == Some(2);
        let (profile, game) = if k2 {
            (Profile::K2, nwscript::Game::K2)
        } else {
            (Profile::K1, nwscript::Game::K1)
        };
        let install =
            Install::open(game_dir, profile).map_err(|e| Failure::new(e.code(), e.to_string()))?;
        let mut data_dirs = vec![loaded.data_dir.clone()];
        if loaded.data_dir != tslpatchdata {
            data_dirs.push(tslpatchdata.to_path_buf());
        }
        Ok(Executor {
            files: Files {
                root: game_dir.to_path_buf(),
                data_dirs,
                profile,
            },
            install,
            stale: false,
            game,
            tokens: Tokens::default(),
            created: Vec::new(),
            after_write: None,
        })
    }

    /// Run `f` after every write, before the read-back: the point at which
    /// another process could change what was written.
    pub fn after_write(&mut self, f: impl FnMut(&Target) + 'h) {
        self.after_write = Some(Box::new(f));
    }

    /// Apply the plan: the requirements first, then every step in order.
    pub fn run(mut self, plan: &Plan) -> Report {
        let mut report = Report::default();
        for r in &plan.requirements {
            let dir = self.override_dir();
            if find_ci(&dir, &r.file).is_none() {
                let msg = r.message.clone().unwrap_or_else(|| {
                    format!("this mod requires `{}`, which is not installed", r.file)
                });
                report.failure = Some(Failure::new(
                    codes::REQUIRED_MISSING,
                    format!("override\\{} is not there: {msg}", r.file),
                ));
                return report;
            }
        }
        let steps = &plan.steps;
        let mut i = 0;
        while i < steps.len() {
            if let Operation::Table { file, .. } = &steps[i].op {
                let j = i + steps[i..]
                    .iter()
                    .take_while(|s| matches!(&s.op, Operation::Table { file: f, .. } if f.eq_ignore_ascii_case(file)))
                    .count();
                self.tables(file, &steps[i..j], &mut report.records);
                i = j;
                continue;
            }
            let rec = self.step(&steps[i]);
            report.records.push(rec);
            i += 1;
        }
        report.tokens = self.tokens;
        report.created = self.created;
        report
    }

    fn override_dir(&self) -> PathBuf {
        match self.files.destination("override") {
            Dest::Folder(p) | Dest::Capsule(p) => p,
        }
    }

    fn install(&mut self) -> Result<&mut Install, Failure> {
        if self.stale {
            self.install = Install::open(&self.files.root, self.files.profile)
                .map_err(|e| Failure::new(e.code(), e.to_string()))?;
            self.stale = false;
        }
        Ok(&mut self.install)
    }

    /// Write, then read back. Fills the record's hashes; a mismatch fails it.
    fn commit(&mut self, target: &Target, bytes: &[u8], rec: &mut Record) {
        let written = ContentHash::of(bytes).to_hex();
        rec.written = Some(written.clone());
        match self.files.write(target, bytes) {
            Ok(Some(c)) => self.created.push(c),
            Ok(None) => {}
            Err(f) => {
                rec.fail(f);
                return;
            }
        }
        self.stale = true;
        if let Some(hook) = &mut self.after_write {
            hook(target);
        }
        match self.files.read(target) {
            Ok(Some(b)) => {
                let found = ContentHash::of(&b).to_hex();
                rec.found = Some(found.clone());
                if found != written {
                    rec.fail(Failure::new(
                        codes::READBACK_MISMATCH,
                        format!(
                            "{}: wrote {written} to {}, and the read-back found {found}",
                            rec.name(),
                            rec.target
                        ),
                    ));
                }
            }
            Ok(None) => rec.fail(Failure::new(
                codes::READBACK_MISMATCH,
                format!(
                    "{}: wrote {written} to {}, and the read-back found nothing there",
                    rec.name(),
                    rec.target
                ),
            )),
            Err(f) => rec.fail(f),
        }
    }

    /// The bytes an edit starts from and where they came from: the target
    /// as it stands, the installer's file, then the copy the game loads.
    /// `fresh` (`!ReplaceFile=1`, `Replace#`) puts the installer's file first.
    /// The installer's file is `source_name`, else the target's own name (a
    /// `!Filename` that names the file where `File#` names only the section).
    fn edit_source(
        &mut self,
        target: &Target,
        dest: &Dest,
        source_name: &str,
        fresh: bool,
    ) -> Result<(String, Vec<u8>), Failure> {
        let target_name = match target {
            Target::File(p) => p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            Target::Member { id, .. } => id.to_string(),
        };
        let data = || {
            self.files
                .data_file(source_name)
                .or_else(|| self.files.data_file(&target_name))
                .map(|p| std::fs::read(&p).map(|b| (self.files.shown_data(&p), b)))
        };
        if fresh && let Some(r) = data() {
            return r.map_err(|e| read_failure(source_name, e));
        }
        if let Some(b) = self.files.read(target)? {
            return Ok((self.files.shown(target), b));
        }
        if let Some(r) = data() {
            return r.map_err(|e| read_failure(source_name, e));
        }
        // The BIFs never change, and Override was looked at above, so the
        // listing the install was opened with serves however stale it is.
        if let (Dest::Folder(_), Target::File(_)) = (dest, target)
            && let Ok(id) = ResId::parse(&target_name)
            && let Some((loc, bytes)) = self
                .install
                .read(id, None)
                .map_err(|e| Failure::new(e.code(), e.to_string()))?
        {
            return Ok((self.files.shown_path(loc.path()) + &bif_suffix(&loc), bytes));
        }
        Err(Failure::new(
            codes::SOURCE_MISSING,
            format!(
                "`{source_name}` is in neither {}, tslpatchdata nor the game's files",
                self.files.shown(target)
            ),
        ))
    }

    fn step(&mut self, step: &Step) -> Record {
        match &step.op {
            Operation::TlkAppend { file, tokens } => {
                let mut rec =
                    Record::new(step, format!("append {} entries of {file}", tokens.len()));
                self.tlk(&mut rec, file, |dialog, src, tok| {
                    tlk::append(dialog, src, file, tokens, tok)
                });
                rec
            }
            Operation::TlkReplace { file, entries } => {
                let mut rec = Record::new(
                    step,
                    format!("replace {} entries from {file}", entries.len()),
                );
                self.tlk(&mut rec, file, |dialog, src, _| {
                    tlk::replace(dialog, src, file, entries)
                });
                rec
            }
            Operation::InstallFile {
                source,
                destination,
                name,
                replace,
            } => {
                let verb = if *replace { "Replace" } else { "File" };
                let mut rec = Record::new(step, format!("{verb} {name} into {destination}"));
                self.install_file(&mut rec, source, destination, name, *replace);
                rec
            }
            Operation::Gff {
                source,
                destination,
                name,
                replace_file,
                edits,
                adds,
                ..
            } => {
                let mut rec = Record::new(step, format!("edit {name}"));
                self.gff(
                    &mut rec,
                    source,
                    destination,
                    name,
                    *replace_file,
                    edits,
                    adds,
                );
                rec
            }
            Operation::Hack {
                file,
                destination,
                replace_file,
                writes,
            } => {
                let mut rec = Record::new(step, format!("patch bytes of {file}"));
                self.hack(&mut rec, file, destination, *replace_file, writes);
                rec
            }
            Operation::Compile {
                file,
                destination,
                name,
                replace,
            } => {
                let mut rec = Record::new(step, format!("compile {file}"));
                self.compile(&mut rec, file, destination, name, *replace);
                rec
            }
            Operation::Ssf {
                file,
                replace,
                entries,
            } => {
                let mut rec = Record::new(step, format!("edit {file}"));
                self.ssf(&mut rec, file, *replace, entries);
                rec
            }
            Operation::Table { .. } => unreachable!("tables run in groups"),
        }
    }

    fn tlk(
        &mut self,
        rec: &mut Record,
        file: &str,
        apply: impl FnOnce(
            &mut rakata_tlk::Tlk,
            &rakata_tlk::Tlk,
            &mut Tokens,
        ) -> Result<String, Failure>,
    ) {
        let Some(dialog_path) = find_ci(&self.files.root, "dialog.tlk") else {
            rec.target = "dialog.tlk".into();
            rec.fail(Failure::new(
                codes::SOURCE_MISSING,
                "the game has no dialog.tlk",
            ));
            return;
        };
        let target = Target::File(dialog_path.clone());
        rec.target = self.files.shown(&target);
        let Some(src_path) = self.files.data_file(file) else {
            rec.fail(Failure::new(
                codes::SOURCE_MISSING,
                format!("tslpatchdata has no `{file}`"),
            ));
            return;
        };
        rec.source = Some(self.files.shown_data(&src_path));
        let read = |p: &Path| -> Result<rakata_tlk::Tlk, Failure> {
            let b = std::fs::read(p).map_err(|e| read_failure(&p.display().to_string(), e))?;
            rakata_tlk::Tlk::parse(&b).map_err(|e| {
                Failure::new(
                    codes::TARGET_UNREADABLE,
                    format!("`{}` does not read as a talk table: {e}", p.display()),
                )
            })
        };
        let (mut dialog, src) = match (read(&dialog_path), read(&src_path)) {
            (Ok(d), Ok(s)) => (d, s),
            (Err(f), _) | (_, Err(f)) => {
                rec.fail(f);
                return;
            }
        };
        match apply(&mut dialog, &src, &mut self.tokens) {
            Ok(d) => rec.detail = Some(d),
            Err(f) => {
                rec.fail(f);
                return;
            }
        }
        match dialog.to_bytes() {
            Ok(bytes) => self.commit(&target, &bytes, rec),
            Err(e) => rec.fail(Failure::new(codes::CANNOT_APPLY, e.to_string())),
        }
    }

    fn install_file(
        &mut self,
        rec: &mut Record,
        source: &str,
        destination: &str,
        name: &str,
        replace: bool,
    ) {
        let dest = self.files.destination(destination);
        let target = match self.files.target(&dest, name) {
            Ok(t) => t,
            Err(f) => {
                rec.target = format!("{destination}\\{name}");
                rec.fail(f);
                return;
            }
        };
        rec.target = self.files.shown(&target);
        let Some(src) = self.files.data_file(source) else {
            rec.fail(Failure::new(
                codes::SOURCE_MISSING,
                format!("tslpatchdata has no `{source}` to install"),
            ));
            return;
        };
        rec.source = Some(self.files.shown_data(&src));
        if !replace {
            match self.files.read(&target) {
                Ok(Some(b)) => {
                    rec.outcome = Outcome::Skipped;
                    rec.found = Some(ContentHash::of(&b).to_hex());
                    rec.fail(Failure::new(
                        codes::SKIPPED_EXISTS,
                        format!("{} exists; `File#` does not overwrite it", rec.target),
                    ));
                    return;
                }
                Ok(None) => {}
                Err(f) => {
                    rec.fail(f);
                    return;
                }
            }
        }
        match std::fs::read(&src) {
            Ok(bytes) => self.commit(&target, &bytes, rec),
            Err(e) => rec.fail(read_failure(source, e)),
        }
    }

    /// One table's run of operations: read once, apply each, write once.
    fn tables(&mut self, file: &str, steps: &[Step], out: &mut Vec<Record>) {
        let dest = self.files.destination("override");
        let mut recs: Vec<Record> = steps
            .iter()
            .map(|s| {
                let key = match &s.op {
                    Operation::Table { key, .. } => key.as_str(),
                    _ => "",
                };
                Record::new(s, format!("{key} on {file}"))
            })
            .collect();
        let start = (|| {
            let target = self.files.target(&dest, file)?;
            let (source, bytes) = self.edit_source(&target, &dest, file, false)?;
            let table = rakata_2da::TwoDa::parse(&bytes).map_err(|e| {
                Failure::new(
                    codes::TARGET_UNREADABLE,
                    format!("{source} does not read as a 2DA: {e}"),
                )
            })?;
            Ok::<_, Failure>((target, source, table))
        })();
        let (target, source, mut table) = match start {
            Ok(x) => x,
            Err(f) => {
                for r in &mut recs {
                    r.target = format!("override\\{file}");
                    r.fail(f.clone());
                }
                out.extend(recs);
                return;
            }
        };
        let shown = self.files.shown(&target);
        let mut applied = 0;
        for (r, s) in recs.iter_mut().zip(steps) {
            r.target = shown.clone();
            r.source = Some(source.clone());
            let Operation::Table { kind, .. } = &s.op else {
                continue;
            };
            match table::apply(&mut table, kind, &mut self.tokens) {
                Ok(d) => {
                    r.detail = Some(d);
                    applied += 1;
                }
                Err(f) => r.fail(f),
            }
        }
        if applied > 0 {
            let mut probe = Record::new(&steps[0], format!("write {file}"));
            probe.target = shown.clone();
            match table.to_bytes() {
                Ok(bytes) => self.commit(&target, &bytes, &mut probe),
                Err(e) => probe.fail(Failure::new(e.code(), e.to_string())),
            }
            for r in &mut recs {
                r.written = probe.written.clone();
                r.found = probe.found.clone();
                for i in &probe.issues {
                    let mut i = i.clone();
                    i.message = i.message.replace(&probe.name(), &r.name());
                    r.fail(i);
                }
            }
        }
        out.extend(recs);
    }

    #[allow(clippy::too_many_arguments)]
    fn gff(
        &mut self,
        rec: &mut Record,
        source: &str,
        destination: &str,
        name: &str,
        replace_file: bool,
        edits: &[GffEdit],
        adds: &[crate::changes::AddField],
    ) {
        let dest = self.files.destination(destination);
        let r = (|| {
            let target = self.files.target(&dest, name)?;
            rec.target = self.files.shown(&target);
            let (src, bytes) = self.edit_source(&target, &dest, source, replace_file)?;
            rec.source = Some(src.clone());
            let gff = rakata_gff::Gff::parse(&bytes).map_err(|e| {
                Failure::new(
                    codes::TARGET_UNREADABLE,
                    format!("{src} does not read as a GFF: {e}"),
                )
            })?;
            Ok::<_, Failure>((target, gff))
        })();
        let (target, mut gff) = match r {
            Ok(x) => x,
            Err(f) => {
                if rec.target.is_empty() {
                    rec.target = format!("{destination}\\{name}");
                }
                rec.fail(f);
                return;
            }
        };
        let failures = gff::apply(&mut gff, edits, adds, &mut self.tokens);
        rec.detail = Some(format!(
            "{} edits, {} additions, {} did not apply",
            edits.len(),
            adds.len(),
            failures.len()
        ));
        for f in failures {
            rec.fail(f);
        }
        match gff.to_bytes() {
            Ok(bytes) => self.commit(&target, &bytes, rec),
            Err(e) => rec.fail(Failure::new(e.code(), e.to_string())),
        }
    }

    fn hack(
        &mut self,
        rec: &mut Record,
        file: &str,
        destination: &str,
        replace_file: bool,
        writes: &[HackWrite],
    ) {
        let dest = self.files.destination(destination);
        let r = (|| {
            let target = self.files.target(&dest, file)?;
            rec.target = self.files.shown(&target);
            let (src, bytes) = self.edit_source(&target, &dest, file, replace_file)?;
            rec.source = Some(src);
            Ok::<_, Failure>((target, bytes))
        })();
        let (target, mut bytes) = match r {
            Ok(x) => x,
            Err(f) => {
                if rec.target.is_empty() {
                    rec.target = format!("{destination}\\{file}");
                }
                rec.fail(f);
                return;
            }
        };
        for w in writes {
            let r = self.tokens.resolve(&w.value).and_then(|t| {
                hack_value(&t).ok_or_else(|| {
                    Failure::new(
                        codes::CANNOT_APPLY,
                        format!("line {}: `{t}` is not an unsigned 32-bit value", w.line),
                    )
                })
            });
            let at = w.offset as usize;
            match r {
                Ok(v) if at + 4 <= bytes.len() => {
                    bytes[at..at + 4].copy_from_slice(&v.to_be_bytes())
                }
                Ok(_) => rec.fail(Failure::new(
                    codes::CANNOT_APPLY,
                    format!(
                        "line {}: offset {at} is past the end of the file's {} bytes",
                        w.line,
                        bytes.len()
                    ),
                )),
                Err(f) => rec.fail(f),
            }
        }
        rec.detail = Some(format!("{} writes", writes.len()));
        self.commit(&target, &bytes, rec);
    }

    fn compile(
        &mut self,
        rec: &mut Record,
        file: &str,
        destination: &str,
        name: &str,
        replace: bool,
    ) {
        let dest = self.files.destination(destination);
        let target = match self.files.target(&dest, name) {
            Ok(t) => t,
            Err(f) => {
                rec.target = format!("{destination}\\{name}");
                rec.fail(f);
                return;
            }
        };
        rec.target = self.files.shown(&target);
        let Some(src) = self.files.data_file(file) else {
            rec.fail(Failure::new(
                codes::SOURCE_MISSING,
                format!("tslpatchdata has no `{file}` to compile"),
            ));
            return;
        };
        rec.source = Some(self.files.shown_data(&src));
        if !replace {
            match self.files.read(&target) {
                Ok(Some(b)) => {
                    rec.outcome = Outcome::Skipped;
                    rec.found = Some(ContentHash::of(&b).to_hex());
                    rec.fail(Failure::new(
                        codes::SKIPPED_EXISTS,
                        format!("{} exists; `File#` does not overwrite it", rec.target),
                    ));
                    return;
                }
                Ok(None) => {}
                Err(f) => {
                    rec.fail(f);
                    return;
                }
            }
        }
        let source = match std::fs::read(&src) {
            Ok(b) => b,
            Err(e) => {
                rec.fail(read_failure(file, e));
                return;
            }
        };
        let filled = match script::fill_tokens(&source, &self.tokens) {
            Ok(b) => b,
            Err(f) => {
                rec.fail(f);
                return;
            }
        };
        let data_dirs = self.files.data_dirs.clone();
        let game = self.game;
        let ncs = match self.install() {
            Ok(install) => script::compile(file, &filled, &data_dirs, install, game),
            Err(f) => Err(f),
        };
        match ncs {
            Ok(bytes) => {
                rec.detail = Some(format!("{} bytes of bytecode", bytes.len()));
                self.commit(&target, &bytes, rec);
            }
            Err(f) => rec.fail(f),
        }
    }

    fn ssf(&mut self, rec: &mut Record, file: &str, replace: bool, entries: &[SsfEntry]) {
        let dest = self.files.destination("override");
        let r = (|| {
            let target = self.files.target(&dest, file)?;
            rec.target = self.files.shown(&target);
            let (src, bytes) = self.edit_source(&target, &dest, file, replace)?;
            rec.source = Some(src.clone());
            let ssf = Ssf::parse(&bytes)
                .map_err(|e| Failure::new(codes::TARGET_UNREADABLE, format!("{src}: {e}")))?;
            Ok::<_, Failure>((target, ssf))
        })();
        let (target, mut ssf) = match r {
            Ok(x) => x,
            Err(f) => {
                if rec.target.is_empty() {
                    rec.target = format!("override\\{file}");
                }
                rec.fail(f);
                return;
            }
        };
        for e in entries {
            let Some(slot) = crate::ssf::slot(&e.sound) else {
                rec.fail(Failure::new(
                    codes::CANNOT_APPLY,
                    format!(
                        "line {}: `{}` names no sound of a soundset",
                        e.line, e.sound
                    ),
                ));
                continue;
            };
            match self.tokens.resolve(&e.value).and_then(|t| {
                gff_strref(&t).ok_or_else(|| {
                    Failure::new(
                        codes::CANNOT_APPLY,
                        format!("line {}: `{t}` is not a strref", e.line),
                    )
                })
            }) {
                Ok(v) => ssf.set(slot, v),
                Err(f) => rec.fail(f),
            }
        }
        rec.detail = Some(format!("{} sounds", entries.len()));
        self.commit(&target, &ssf.to_bytes(), rec);
    }
}

fn read_failure(name: &str, e: std::io::Error) -> Failure {
    Failure::new(codes::SOURCE_MISSING, format!("cannot read `{name}`: {e}"))
}

/// `[index]` for a BIF location, so a report names the member.
fn bif_suffix(loc: &rakata_game::Location) -> String {
    match loc {
        rakata_game::Location::Bif { index, .. } => format!("[{index}]"),
        rakata_game::Location::Capsule { member, .. } => format!(":{member}"),
        rakata_game::Location::File { .. } => String::new(),
    }
}

/// A HACKList value: an unsigned 32-bit number, written big-endian as
/// TSLPatcher writes it (the byte order of NCS bytecode, what the list
/// patches); a negative value is refused, as TSLPatcher refuses it.
fn hack_value(text: &str) -> Option<u32> {
    text.trim().parse().ok()
}

/// A strref as a soundset or GFF holds it: a number, `-1` for none.
fn gff_strref(text: &str) -> Option<u32> {
    let n: i64 = text.trim().parse().ok()?;
    match n {
        -1 => Some(u32::MAX),
        0..=0xFFFF_FFFF => Some(n as u32),
        _ => None,
    }
}
