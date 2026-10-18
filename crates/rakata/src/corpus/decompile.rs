//! Every script an install ships only as bytecode, recovered to source:
//! `decompiled/<capsule>/<script>.nss` for each NCS in each module capsule,
//! each capsule under `rims/` and `Override/`, with a header naming where
//! the script lies, which copy the engine loads and how many functions
//! verified; and `decompiled/calls.jsonl`, one line per engine call.
//!
//! A recovered function that is byte for byte a function of an include
//! library the script could have included is replaced by the `#include`
//! line, so a grep hit is in the script's own logic. The match is by
//! bytecode and settled by recompiling: a library's functions are compiled
//! once through a driver script and their shapes indexed
//! ([`nwscript_decomp::shape`]); a script whose functions match and whose
//! leading globals are the library's gets a [`Fold`], which the decompiler
//! applies only when the folded source still recompiles to the original
//! bytes. A function the script changed by one call does not match and
//! stays in the script's own source.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use nwscript::decompile::{Decompiled, Fold, FoldStruct, Global, Kind, ProjectCompiler, Status};
use nwscript::diag::{Diagnostic, SourceMap};
use nwscript::ncs::{Instruction, Opcode, Operands, Program};
use nwscript::sema::{EngineTable, Type};
use nwscript::{Checker, Declarations, Game, MemResolver};
use nwscript_decomp::{function_ranges, globals_bytes, shape};
use rakata_archive::{Erf, Rim, Source};
use rakata_game::{Install, Location, ModuleName, Profile};
use rakata_res::{ContentHash, ResId, ResRef, ResType};
use serde::Serialize;

use super::{Error, Result, clear, find_ci, listed, write};
use crate::codes;

/// What [`decompile`] takes.
#[derive(Debug, Clone)]
pub struct Options {
    /// The install: the folder holding `chitin.key`.
    pub game: PathBuf,
    /// The corpus directory; the tree goes under `decompiled/` in it.
    pub out: PathBuf,
    pub profile: Profile,
    /// Worker threads; 0 for the number of CPUs.
    pub jobs: usize,
}

/// Scripts and functions by outcome.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Tally {
    pub scripts: usize,
    /// Scripts the decompiler could not read.
    pub failed: usize,
    pub functions: usize,
    pub verified: usize,
    /// Functions replaced by an include's.
    pub folded_functions: usize,
    /// Scripts with at least one `#include` folded in.
    pub folded_scripts: usize,
    /// Scripts whose fold recompiled differently and were written unfolded.
    pub unfolded: usize,
    /// Engine calls indexed.
    pub calls: usize,
}

impl Tally {
    fn add(&mut self, o: &Tally) {
        self.scripts += o.scripts;
        self.failed += o.failed;
        self.functions += o.functions;
        self.verified += o.verified;
        self.folded_functions += o.folded_functions;
        self.folded_scripts += o.folded_scripts;
        self.unfolded += o.unfolded;
        self.calls += o.calls;
    }

    /// The verified fraction as `99.92%`, or `-` with no functions.
    pub fn percent(&self) -> String {
        if self.functions == 0 {
            "-".into()
        } else {
            format!(
                "{:.2}%",
                self.verified as f64 * 100.0 / self.functions as f64
            )
        }
    }
}

/// What a run produced.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The `decompiled/` directory written.
    pub out: PathBuf,
    /// Include libraries whose functions were indexed.
    pub libraries: Vec<String>,
    /// By source: `modules`, `rims`, `override`.
    pub sources: BTreeMap<String, Tally>,
    pub total: Tally,
    /// Wall time of the whole run.
    pub seconds: f64,
}

impl Report {
    /// The summary as text, the wall time last.
    pub fn text(&self) -> String {
        let mut s = format!(
            "{:<10} {:>7} {:>7} {:>9} {:>9} {:>9} {:>7} {:>8}\n",
            "source", "scripts", "failed", "functions", "verified", "fraction", "folded", "calls"
        );
        for (name, t) in self
            .sources
            .iter()
            .chain(std::iter::once((&"total".to_string(), &self.total)))
        {
            s.push_str(&format!(
                "{name:<10} {:>7} {:>7} {:>9} {:>9} {:>9} {:>7} {:>8}\n",
                t.scripts,
                t.failed,
                t.functions,
                t.verified,
                t.percent(),
                t.folded_functions,
                t.calls
            ));
        }
        s.push_str(&format!(
            "{} include libraries indexed; {} scripts folded, {} written unfolded after a fold recompiled differently\n",
            self.libraries.len(),
            self.total.folded_scripts,
            self.total.unfolded
        ));
        s.push_str(&format!(
            "written to {}\nwall time {:.1}s\n",
            self.out.display(),
            self.seconds
        ));
        s
    }
}

/// An include library as compiled through its driver.
struct Library {
    /// Its name, as `#include` spells it.
    name: String,
    /// Every file its inclusion brings in, itself included, lowercase
    /// without extension.
    files: BTreeSet<String>,
    /// The instructions that declare and initialise its globals, as a
    /// script including it starts.
    globals_init: Vec<Instruction>,
    /// Its globals, in declaration order.
    globals: Vec<LibGlobal>,
    functions: Vec<LibFunction>,
}

/// A library's global: its name, its size on the stack, and for a struct
/// each field's name and size, since the decompiler may recover a struct
/// global as one variable per slot.
struct LibGlobal {
    name: String,
    size: u32,
    /// The struct's name, for a struct global.
    struct_name: Option<String>,
    fields: Vec<(String, u32)>,
}

/// The names the libraries give the script's leading globals, one per
/// recovered whole variable, and the recovered structs the libraries
/// declare: a global recovered at its full size takes its name (a struct
/// then takes the library's struct and field names too); a struct
/// recovered slot by slot takes `name.field` for each slot. `None` when the
/// recovered sizes do not line up with the libraries'.
fn align_globals(
    recovered: &[Global],
    globals: &[&LibGlobal],
) -> Option<(Vec<String>, Vec<FoldStruct>)> {
    let mut names = Vec::new();
    let mut structs: Vec<FoldStruct> = Vec::new();
    let mut r = recovered.iter();
    for g in globals {
        let got = r.next()?;
        if got.size == g.size {
            names.push(g.name.clone());
            if let (Some(index), Some(name)) = (got.struct_index, &g.struct_name) {
                if got.fields != g.fields.len() {
                    return None;
                }
                if !structs.iter().any(|s| s.index == index) {
                    structs.push(FoldStruct {
                        index,
                        name: name.clone(),
                        fields: g.fields.iter().map(|(f, _)| f.clone()).collect(),
                    });
                }
            }
            continue;
        }
        if g.fields.is_empty() {
            return None;
        }
        let mut covered = 0;
        let mut fields = g.fields.iter();
        let mut size = got.size;
        loop {
            let (field, field_size) = fields.next()?;
            if *field_size != size {
                return None;
            }
            names.push(format!("{}.{field}", g.name));
            covered += size;
            if covered == g.size {
                break;
            }
            size = r.next()?.size;
        }
    }
    Some((names, structs))
}

struct LibFunction {
    name: String,
    shape: Vec<u8>,
    /// The return type and parameter types as source spells them, to tell
    /// apart functions whose bodies compile alike.
    ret: String,
    params: Vec<String>,
    /// The library functions it calls, by index, in the order of its `JSR`s;
    /// `None` for a call to a function the driver did not name.
    callees: Vec<Option<usize>>,
}

impl LibFunction {
    /// Whether a recovered function's signature could be this one's: the
    /// same return type and arity, and each parameter whose type the
    /// bytecode shows the same.
    fn accepts(&self, ret: &str, params: &[Option<String>]) -> bool {
        self.ret == ret
            && self.params.len() == params.len()
            && self
                .params
                .iter()
                .zip(params)
                .all(|(a, b)| b.as_ref().is_none_or(|b| a == b))
    }
}

/// A type as source spells it, with engine structures by name.
fn type_text(ty: &Type, table: &EngineTable) -> String {
    match ty {
        Type::Engine(n) => table
            .structures
            .get(usize::from(*n))
            .cloned()
            .unwrap_or_else(|| format!("engine{n}")),
        other => other.to_string(),
    }
}

/// One script to decompile, with everything the header needs resolved.
struct Job {
    /// `modules`, `rims` or `override`.
    source: &'static str,
    /// The directory under `decompiled/`.
    capsule: String,
    /// The script's name, lowercase.
    name: String,
    /// Where this copy lies, relative to the install.
    here: String,
    module: Option<ModuleName>,
    bytes: Vec<u8>,
}

struct Outcome {
    tally: Tally,
    calls: Vec<Call>,
    warnings: Vec<Diagnostic>,
}

#[derive(Serialize)]
struct Call {
    script: String,
    capsule: String,
    routine: String,
    /// One entry per argument: the literal where the operand is a
    /// constant, `null` where it is computed.
    args: Vec<serde_json::Value>,
    offset: u32,
}

fn game_of(profile: Profile) -> Game {
    match profile {
        Profile::K1 => Game::K1,
        Profile::K2 => Game::K2,
    }
}

fn warn(code: &str, message: String) -> Diagnostic {
    let code =
        codes::lookup(code).unwrap_or_else(|| panic!("corpus code {code} is not registered"));
    Diagnostic::new(code, message)
}

/// What recovering any one script of an install needs, built once: every
/// NSS the install holds (the copy the engine loads), the engine's
/// declarations from its `nwscript.nss`, and the include libraries compiled
/// and indexed by function shape for folding. `rakata corpus decompile`
/// runs it over every script; a dialogue's transcript over its conditions.
pub struct Recovery {
    game: Game,
    declarations: Declarations,
    sources: Vec<(String, Vec<u8>)>,
    table: EngineTable,
    libs: Vec<Library>,
    index: HashMap<Vec<u8>, Vec<(usize, usize)>>,
}

/// A script recovered by [`Recovery::recover`]: the source, each function
/// verified or not, and what folding did.
pub struct Recovered<'t> {
    pub decompiled: Decompiled<'t>,
    pub fold: FoldOutcome,
}

/// What folding include libraries out of a recovered script did.
pub enum FoldOutcome {
    /// No function matched a library's.
    None,
    /// `functions` were replaced by the `includes`; `ranges` are their byte
    /// ranges in the original program.
    Folded {
        functions: usize,
        includes: Vec<String>,
        ranges: Vec<(u32, u32)>,
    },
    /// The folded source recompiled differently (`why`), so none was
    /// replaced. `attempt` is that source when `RAKATA_CORPUS_DUMP` is set.
    Refused {
        functions: usize,
        includes: Vec<String>,
        why: String,
        attempt: Option<Vec<u8>>,
    },
}

impl Recovery {
    /// Read the install's NSS, its declarations and its include libraries.
    /// Libraries that do not compile are left out with an RW1002 warning;
    /// an install without `nwscript.nss` is RE1002.
    pub fn new(install: &Install, warnings: &mut Vec<Diagnostic>) -> Result<Self> {
        let game = game_of(install.profile());
        let declarations = Declarations::BioWare;
        // Every NSS the install holds, the copy the engine loads: the
        // includes a script could have included, and nwscript.nss.
        let mut nss_ids: BTreeSet<ResId> = install
            .key()
            .entries()
            .iter()
            .filter(|e| e.id.ty == ResType::Nss)
            .map(|e| e.id)
            .collect();
        nss_ids.extend(
            install
                .override_ids()
                .into_iter()
                .filter(|id| id.ty == ResType::Nss),
        );
        let mut sources: Vec<(String, Vec<u8>)> = Vec::new();
        for id in &nss_ids {
            if let Some((_, bytes)) = install.read(*id, None)? {
                sources.push((id.to_string().to_ascii_lowercase(), bytes));
            }
        }
        let mut recovery = Recovery {
            game,
            declarations,
            sources,
            table: EngineTable::default(),
            libs: Vec::new(),
            index: HashMap::new(),
        };

        let mut map = SourceMap::new();
        let mut table = match nwscript::engine::load_table(&mut recovery.resolver(), &mut map) {
            Ok(Some((t, _))) => t,
            Ok(None) => {
                return Err(Error::NoNwscript {
                    dir: install.root().to_path_buf(),
                });
            }
            Err(source) => {
                return Err(Error::Read {
                    path: install.root().join("nwscript.nss"),
                    source,
                });
            }
        };
        if let Some((name, text)) = declarations.overlay(game) {
            let file = map.add(name, text.as_bytes().to_vec());
            let _ = table.overlay(file, text.as_bytes());
        }

        let libs = libraries(
            &recovery.sources,
            &|| recovery.resolver(),
            game,
            declarations,
            &table,
            warnings,
        );
        let mut index: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();
        for (li, lib) in libs.iter().enumerate() {
            for (fi, f) in lib.functions.iter().enumerate() {
                index.entry(f.shape.clone()).or_default().push((li, fi));
            }
        }
        recovery.table = table;
        recovery.libs = libs;
        recovery.index = index;
        Ok(recovery)
    }

    /// The engine's declarations, from the install's `nwscript.nss`.
    pub fn table(&self) -> &EngineTable {
        &self.table
    }

    /// The include libraries indexed for folding, by name.
    pub fn libraries(&self) -> impl Iterator<Item = &str> {
        self.libs.iter().map(|l| l.name.as_str())
    }

    /// A resolver over the install's NSS.
    fn resolver(&self) -> MemResolver {
        let mut r = MemResolver::new();
        for (name, bytes) in &self.sources {
            r = r.with(name, bytes.clone());
        }
        r
    }

    /// A compiler for verifying, one per thread.
    pub fn compiler(&self) -> ProjectCompiler<MemResolver> {
        ProjectCompiler::new(self.resolver(), self.game, self.declarations)
    }

    /// Decompile `bytes` (decoded as `program`), verify every function by
    /// recompiling, and fold out the include libraries whose functions it
    /// embeds when the folded source recompiles to the same bytes.
    /// `display` names the script in messages. Fails with the decompiler's
    /// diagnostic when the bytecode cannot be read back.
    pub fn recover(
        &self,
        display: &str,
        bytes: &[u8],
        program: &Program,
        compiler: &mut ProjectCompiler<MemResolver>,
    ) -> std::result::Result<Recovered<'_>, Diagnostic> {
        let mut map = SourceMap::new();
        let (mut d, _) = nwscript::decompile::decompile(
            display,
            bytes,
            &self.table,
            Some(&mut *compiler),
            &mut map,
        )?;
        let Some(fold) = plan_fold(&d, program, &self.libs, &self.index) else {
            return Ok(Recovered {
                decompiled: d,
                fold: FoldOutcome::None,
            });
        };
        let functions = fold.functions.len();
        let includes = fold.includes.clone();
        let ranges: Vec<(u32, u32)> = fold
            .functions
            .iter()
            .map(|(i, _)| (d.functions[*i].start, d.functions[*i].end))
            .collect();
        let attempt =
            std::env::var_os("RAKATA_CORPUS_DUMP").and_then(|_| d.folded_source(&fold).ok());
        let fold = match d.fold(fold, program, compiler) {
            Ok(()) => FoldOutcome::Folded {
                functions,
                includes,
                ranges,
            },
            Err(why) => FoldOutcome::Refused {
                functions,
                includes,
                why: why.to_string(),
                attempt,
            },
        };
        Ok(Recovered {
            decompiled: d,
            fold,
        })
    }
}

/// Decompile every bytecode-only script of the install at `o.game` into
/// `o.out/decompiled/`. The warnings name scripts the decompiler could not
/// read (RW1001) and libraries that did not compile (RW1002).
pub fn decompile(o: &Options) -> Result<(Report, Vec<Diagnostic>)> {
    let start = Instant::now();
    let install = Install::open(&o.game, o.profile)?;
    let mut warnings = Vec::new();
    let recovery = Recovery::new(&install, &mut warnings)?;

    let jobs = jobs(&install)?;
    let out = o.out.join("decompiled");
    clear(&out)?;
    fs::create_dir_all(&out).map_err(|source| Error::Write {
        path: out.clone(),
        source,
    })?;

    // Which copy the engine loads, and every other, resolved before the
    // workers start.
    let copies = copies(&install, &jobs)?;

    let outcomes: Mutex<Vec<Option<Outcome>>> = Mutex::new((0..jobs.len()).map(|_| None).collect());
    let next = AtomicUsize::new(0);
    let workers = match o.jobs {
        0 => std::thread::available_parallelism().map_or(4, |n| n.get()),
        n => n,
    };
    std::thread::scope(|s| {
        for _ in 0..workers {
            let (outcomes, next, jobs, recovery, copies, out) =
                (&outcomes, &next, &jobs, &recovery, &copies, &out);
            s.spawn(move || {
                let mut compiler = recovery.compiler();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(i) else {
                        break;
                    };
                    let outcome = one(job, &copies[i], recovery, &mut compiler, out);
                    outcomes.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(outcome);
                }
            });
        }
    });

    let outcomes = outcomes.into_inner().unwrap_or_else(|e| e.into_inner());
    let mut sources_tally: BTreeMap<String, Tally> = BTreeMap::new();
    let mut total = Tally::default();
    let mut calls = Vec::new();
    for (job, outcome) in jobs.iter().zip(outcomes) {
        let Some(outcome) = outcome else {
            continue;
        };
        sources_tally
            .entry(job.source.to_string())
            .or_default()
            .add(&outcome.tally);
        total.add(&outcome.tally);
        calls.extend(outcome.calls);
        warnings.extend(outcome.warnings);
    }
    calls.sort_by(|a, b| (&a.capsule, &a.script, a.offset).cmp(&(&b.capsule, &b.script, b.offset)));
    let mut jsonl = String::new();
    for c in &calls {
        jsonl.push_str(&serde_json::to_string(c).expect("a call serializes"));
        jsonl.push('\n');
    }
    write(&out.join("calls.jsonl"), jsonl.as_bytes())?;

    Ok((
        Report {
            out,
            libraries: recovery.libraries().map(str::to_owned).collect(),
            sources: sources_tally,
            total,
            seconds: start.elapsed().as_secs_f64(),
        },
        warnings,
    ))
}

/// A local variable's declaration for a parameter type, for the driver.
fn local_decl(ty: &Type, table: &EngineTable) -> Option<(String, String)> {
    let (type_text, var) = match ty {
        Type::Int => ("int".to_string(), "i".to_string()),
        Type::Float => ("float".into(), "f".into()),
        Type::String => ("string".into(), "s".into()),
        Type::Object => ("object".into(), "o".into()),
        Type::Vector => ("vector".into(), "v".into()),
        Type::Engine(n) => (
            table.structures.get(usize::from(*n))?.clone(),
            format!("e{n}"),
        ),
        Type::Struct(name) => (format!("struct {name}"), format!("st_{name}")),
        Type::Void | Type::Action => return None,
    };
    Some((type_text, format!("arg_{var}")))
}

/// Compile every include library once, through a driver script that calls
/// each of its functions, and index each function's shape.
fn libraries(
    sources: &[(String, Vec<u8>)],
    resolver: &(dyn Fn() -> MemResolver + Sync),
    game: Game,
    declarations: Declarations,
    table: &EngineTable,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<Library> {
    let slots: Mutex<Vec<Option<LibraryOutcome>>> =
        Mutex::new((0..sources.len()).map(|_| None).collect());
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(sources.len())
        .max(1);
    std::thread::scope(|s| {
        for _ in 0..workers {
            let (slots, next) = (&slots, &next);
            s.spawn(move || {
                let mut checker = Checker::with_declarations(declarations, game);
                let mut resolver = resolver();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((file, bytes)) = sources.get(i) else {
                        break;
                    };
                    let lib = library(file, bytes, &mut checker, &mut resolver, game, table);
                    slots.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(lib);
                }
            });
        }
    });
    let mut libs = Vec::new();
    for slot in slots.into_inner().unwrap_or_else(|e| e.into_inner()) {
        match slot {
            Some(Ok(Some(lib))) => libs.push(lib),
            Some(Err(w)) => warnings.push(w),
            _ => {}
        }
    }
    libs
}

/// An NSS read as an include library: `Ok(None)` when it is not one.
type LibraryOutcome = std::result::Result<Option<Library>, Diagnostic>;

/// One NSS as an include library: `None` when it is not one (it has an
/// entry point or defines no function), a warning when it does not compile.
fn library(
    file: &str,
    bytes: &[u8],
    checker: &mut Checker,
    resolver: &mut MemResolver,
    game: Game,
    table: &EngineTable,
) -> LibraryOutcome {
    let levels = nwscript::modes::Mode::Bioware.levels();
    let Some(name) = file.strip_suffix(".nss") else {
        return Ok(None);
    };
    if name == "nwscript" {
        return Ok(None);
    }
    let mut map = SourceMap::new();
    let checked = checker.check(resolver, &mut map, file, bytes);
    let Some(analysis) = checked.analysis else {
        return Ok(None);
    };
    if analysis.program.entry.is_some() {
        return Ok(None);
    }
    let defined: Vec<_> = analysis
        .program
        .functions
        .iter()
        .filter(|f| f.definition.is_some())
        .collect();
    if defined.is_empty() {
        return Ok(None);
    }

    let mut driver = format!("#include \"{name}\"\nvoid main()\n{{\n");
    let mut locals: BTreeMap<String, String> = BTreeMap::new();
    let mut calls = String::new();
    for f in &defined {
        let mut args = Vec::new();
        for p in &f.params {
            match local_decl(&p.ty, table) {
                Some((ty, var)) => {
                    locals.insert(var.clone(), ty);
                    args.push(var);
                }
                // An action argument is any void call.
                None => args.push("ClearAllActions()".to_string()),
            }
        }
        calls.push_str(&format!("    {}({});\n", f.name, args.join(", ")));
    }
    for (var, ty) in &locals {
        driver.push_str(&format!("    {ty} {var};\n"));
    }
    driver.push_str(&calls);
    driver.push_str("}\n");

    let mut dmap = SourceMap::new();
    let checked = checker.check(resolver, &mut dmap, "driver.nss", driver.as_bytes());
    let diags = nwscript::modes::finish(
        checked.analysis.as_ref(),
        checked.diagnostics,
        &dmap,
        &levels,
    );
    if let Some(e) = diags.iter().find(|d| d.is_error()) {
        return Err(warn(
            "RW1002",
            format!(
                "{name} does not compile as an include library, so its functions are not folded: {}",
                e.message
            ),
        ));
    }
    let Some(driven) = checked.analysis else {
        return Ok(None);
    };
    let program = match nwscript::compile::compile(&driven.program, game) {
        Ok(p) => p,
        Err(d) => {
            return Err(warn(
                "RW1002",
                format!(
                    "{name} does not compile as an include library, so its functions are not folded: {}",
                    d.message
                ),
            ));
        }
    };

    let ins = program.instructions();
    let ranges = function_ranges(&program);
    // With globals the entry stub jumps to their initialisation, which
    // ends in `SAVEBP; JSR main`; without, it jumps to main itself.
    let main_jsr = ins
        .iter()
        .position(|x| x.opcode == Opcode::SaveBp)
        .map_or(0, |save| save + 1);
    let main_range = ins
        .get(main_jsr)
        .and_then(|x| x.jump_target(program.offsets()[main_jsr]))
        .and_then(|t| program.index_at(t))
        .and_then(|t| ranges.iter().position(|&(s, e)| s <= t && t < e));
    let Some(m) = main_range else {
        return Err(warn(
            "RW1002",
            format!(
                "{name} does not compile as an include library, so its functions are not folded: the driver's bytecode has no entry point"
            ),
        ));
    };
    // Without globals main is the first range, which takes in the
    // entry stub's own `JSR`.
    let from = if ranges[m].0 == 0 { 2 } else { ranges[m].0 };
    let mut called: Vec<usize> = Vec::new();
    for (i, x) in ins.iter().enumerate().take(ranges[m].1).skip(from) {
        if x.opcode == Opcode::Jsr
            && let Some(t) = x
                .jump_target(program.offsets()[i])
                .and_then(|t| program.index_at(t))
            && let Some(k) = ranges.iter().position(|&(s, e)| s <= t && t < e)
        {
            called.push(k);
        }
    }
    if called.len() != defined.len() {
        return Err(warn(
            "RW1002",
            format!(
                "{name} does not compile as an include library, so its functions are not folded: the driver calls {} functions but {} compiled",
                defined.len(),
                called.len()
            ),
        ));
    }
    let range_of = |i: usize| -> Option<usize> {
        ins[i]
            .jump_target(program.offsets()[i])
            .and_then(|t| program.index_at(t))
            .and_then(|t| ranges.iter().position(|&(s, e)| s <= t && t < e))
    };
    let gb = globals_bytes(&program);
    let functions = called
        .iter()
        .zip(&defined)
        .map(|(&k, f)| LibFunction {
            name: f.name.clone(),
            shape: shape(&ins[ranges[k].0..ranges[k].1], gb),
            ret: type_text(&f.ret, table),
            params: f.params.iter().map(|p| type_text(&p.ty, table)).collect(),
            callees: (ranges[k].0..ranges[k].1)
                .filter(|&i| ins[i].opcode == Opcode::Jsr)
                .map(|i| range_of(i).and_then(|r| called.iter().position(|&c| c == r)))
                .collect(),
        })
        .collect();
    let files = analysis
        .program
        .files
        .iter()
        .map(|u| {
            u.name
                .to_ascii_lowercase()
                .trim_end_matches(".nss")
                .to_string()
        })
        .collect();
    let globals_init = globals_section(&program).to_vec();
    let program = &analysis.program;
    let globals: Vec<LibGlobal> = program
        .globals
        .iter()
        .map(|g| LibGlobal {
            name: g.name.clone(),
            size: program.size_of(&g.ty),
            struct_name: match &g.ty {
                Type::Struct(s) => Some(s.clone()),
                _ => None,
            },
            fields: match &g.ty {
                Type::Struct(s) => program
                    .structs
                    .iter()
                    .find(|d| d.name == *s)
                    .map(|d| {
                        d.fields
                            .iter()
                            .map(|f| (f.name.clone(), program.size_of(&f.ty)))
                            .collect()
                    })
                    .unwrap_or_default(),
                _ => Vec::new(),
            },
        })
        .collect();
    Ok(Some(Library {
        name: name.to_string(),
        files,
        globals_init,
        globals,
        functions,
    }))
}

/// A capsule's name as a directory: its stem, or its whole file name when
/// another capsule shares the stem.
fn capsule_dir(path: &Path, stems: &HashMap<String, usize>) -> String {
    let file = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if stems.get(&stem).copied().unwrap_or(0) > 1 {
        file
    } else {
        stem
    }
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every NCS in every module capsule, every capsule under `rims/` and
/// `Override/`, in a fixed order.
fn jobs(install: &Install) -> Result<Vec<Job>> {
    let root = install.root();
    let mut capsules: Vec<(&'static str, PathBuf, Option<ModuleName>)> = Vec::new();
    for module in install.modules() {
        for path in install.module_capsules(module)? {
            capsules.push(("modules", path.to_path_buf(), Some(module.clone())));
        }
    }
    if let Some(rims) = find_ci(root, "rims") {
        for path in listed(&rims, &["rim", "mod", "erf"]) {
            capsules.push(("rims", path, None));
        }
    }
    let mut stems: HashMap<String, usize> = HashMap::new();
    for (_, path, _) in &capsules {
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        *stems.entry(stem).or_default() += 1;
    }

    let mut jobs = Vec::new();
    for (source, path, module) in &capsules {
        let bytes = Source::map(path).map_err(|source| Error::Archive {
            path: path.clone(),
            source,
        })?;
        let archive_err = |source| Error::Archive {
            path: path.clone(),
            source,
        };
        let rim;
        let erf;
        let members = if bytes.starts_with(b"RIM ") {
            rim = Rim::parse(&bytes).map_err(archive_err)?;
            rim.members()
        } else {
            erf = Erf::parse(&bytes).map_err(archive_err)?;
            erf.members()
        };
        let capsule = capsule_dir(path, &stems);
        let here = relative(root, path);
        for m in members.iter().filter(|m| m.id.ty == ResType::Ncs) {
            jobs.push(Job {
                source,
                capsule: capsule.clone(),
                name: m.id.name.to_string().to_ascii_lowercase(),
                here: here.clone(),
                module: module.clone(),
                bytes: m.data().to_vec(),
            });
        }
    }
    for id in install.override_ids() {
        if id.ty != ResType::Ncs {
            continue;
        }
        if let Some((location, bytes)) = install.read(id, None)? {
            jobs.push(Job {
                source: "override",
                capsule: "override".into(),
                name: id.name.to_string().to_ascii_lowercase(),
                here: relative(root, location.path()),
                module: None,
                bytes,
            });
        }
    }
    jobs.sort_by(|a, b| {
        (a.source, &a.capsule, &a.name, &a.here).cmp(&(b.source, &b.capsule, &b.name, &b.here))
    });
    Ok(jobs)
}

/// What the header says about the other copies of a script.
struct Copies {
    /// The copy the engine loads, relative to the install, or none found.
    loaded: Option<(String, bool)>,
    /// Every other copy the install holds: where, and whether its bytes
    /// are this copy's.
    others: Vec<(String, bool)>,
}

fn copies(install: &Install, jobs: &[Job]) -> Result<Vec<Copies>> {
    let root = install.root();
    // Every capsule copy by name, from the walk: `rims/` is outside the
    // engine's module search, so the resolver does not list it.
    let mut by_name: HashMap<&str, Vec<(&str, ContentHash)>> = HashMap::new();
    for job in jobs {
        by_name
            .entry(&job.name)
            .or_default()
            .push((&job.here, ContentHash::of(&job.bytes)));
    }
    let mut out = Vec::with_capacity(jobs.len());
    for job in jobs {
        let hash = ContentHash::of(&job.bytes);
        let mut others: Vec<(String, bool)> = Vec::new();
        let mut loaded = None;
        if let Ok(resref) = ResRef::new(&job.name) {
            let id = ResId::new(resref, ResType::Ncs);
            let resolution = install.resolve(id, job.module.as_ref())?;
            for (i, c) in resolution.candidates.iter().enumerate() {
                let where_ = match &c.location {
                    Location::Bif { path, index, .. } => {
                        format!("{}[{index}]", relative(root, path))
                    }
                    other => relative(root, other.path()),
                };
                let same = c.hash == hash;
                if i == 0 {
                    loaded = Some((where_.clone(), same && where_ == job.here));
                }
                if where_ != job.here && !others.iter().any(|(w, _)| *w == where_) {
                    others.push((where_, same));
                }
            }
        }
        for (here, h) in by_name.get(job.name.as_str()).into_iter().flatten() {
            if *here != job.here && !others.iter().any(|(w, _)| w == here) {
                others.push((here.to_string(), *h == hash));
            }
        }
        others.sort();
        out.push(Copies { loaded, others });
    }
    Ok(out)
}

/// Decompile, fold, head and write one script; index its calls.
fn one(
    job: &Job,
    copies: &Copies,
    recovery: &Recovery,
    compiler: &mut ProjectCompiler<MemResolver>,
    out: &Path,
) -> Outcome {
    let mut tally = Tally {
        scripts: 1,
        ..Tally::default()
    };
    let display = format!("{}:{}.ncs", job.here, job.name);
    let mut map = SourceMap::new();
    let program = match nwscript::ncs::decode_named(&display, &job.bytes, &mut map) {
        Ok(p) => p,
        Err(d) => {
            tally.failed = 1;
            return Outcome {
                tally,
                calls: Vec::new(),
                warnings: vec![warn("RW1001", format!("{display}: {}", d.message))],
            };
        }
    };
    let calls = calls(&program, recovery.table(), job);
    tally.calls = calls.len();
    let Recovered {
        mut decompiled,
        fold,
    } = match recovery.recover(&display, &job.bytes, &program, compiler) {
        Ok(r) => r,
        Err(diag) => {
            tally.failed = 1;
            return Outcome {
                tally,
                calls,
                warnings: vec![warn("RW1001", format!("{display}: {}", diag.message))],
            };
        }
    };
    let d = &mut decompiled;
    tally.functions = d.functions.len();
    tally.verified = d.verified();

    let mut fold_note = String::new();
    // The engine calls with string arguments that the fold moved out of the
    // file, so a grep for a script or tag name still finds this script.
    let mut folded_calls: Vec<String> = Vec::new();
    match fold {
        FoldOutcome::None => {}
        FoldOutcome::Folded {
            functions: n,
            includes,
            ranges,
        } => {
            tally.folded_functions = n;
            tally.folded_scripts = 1;
            fold_note = format!(
                "{n} replaced by {}",
                includes
                    .iter()
                    .map(|i| format!("#include \"{i}\""))
                    .collect::<Vec<_>>()
                    .join(" and ")
            );
            for c in &calls {
                let inside = ranges.iter().any(|&(s, e)| s <= c.offset && c.offset < e);
                let strings: Vec<String> = c
                    .args
                    .iter()
                    .filter_map(|a| a.as_str())
                    .map(|s| format!("\"{s}\""))
                    .collect();
                if inside && !strings.is_empty() {
                    folded_calls.push(format!("{}({})", c.routine, strings.join(", ")));
                }
            }
            folded_calls.sort();
            folded_calls.dedup();
        }
        FoldOutcome::Refused {
            functions: n,
            includes,
            why,
            attempt,
        } => {
            tally.unfolded = 1;
            fold_note = format!(
                "{n} matched {} but the fold recompiled differently ({why}), so none replaced",
                includes.join(", ")
            );
            // `RAKATA_CORPUS_DUMP=DIR` keeps every refused fold's source
            // there, for reading why it recompiled differently.
            if let (Some(dir), Some(source)) = (std::env::var_os("RAKATA_CORPUS_DUMP"), attempt) {
                let _ = write(
                    &PathBuf::from(dir).join(format!("{}-{}.folded.nss", job.capsule, job.name)),
                    &source,
                );
            }
        }
    }

    let mut header = vec![match &job.module {
        Some(m) => format!("{}.ncs from {} (module {m})", job.name, job.here),
        None => format!("{}.ncs from {}", job.name, job.here),
    }];
    if !copies.others.is_empty() {
        header.push(format!(
            "also in: {}",
            copies
                .others
                .iter()
                .map(|(w, same)| format!(
                    "{w} ({})",
                    if *same {
                        "same bytes"
                    } else {
                        "different bytes"
                    }
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    header.push(match &copies.loaded {
        Some((w, true)) => format!("loaded: this copy, {w}"),
        Some((w, false)) => format!("loaded: {w}, not this copy"),
        None if job.source == "rims" => {
            "loaded: no copy in Override, patch.erf or the BIFs; rims/ is read outside the module search order".into()
        }
        None => "loaded: no copy found by the engine's search order".into(),
    });
    header.push(match fold_note.is_empty() {
        true => format!(
            "functions: {} of {} verified by recompiling; none replaced by an include",
            tally.verified, tally.functions
        ),
        false => format!(
            "functions: {} of {} verified by recompiling; {fold_note}",
            tally.verified, tally.functions
        ),
    });
    if !folded_calls.is_empty() {
        header.push(format!(
            "in the replaced functions: {}",
            folded_calls.join("; ")
        ));
    }
    d.set_header(header);

    let path = out.join(&job.capsule).join(format!("{}.nss", job.name));
    let mut warnings = Vec::new();
    if let Err(e) = write(&path, &d.source()) {
        warnings.push(warn("RE1001", e.to_string()));
    }
    Outcome {
        tally,
        calls,
        warnings,
    }
}

/// The instructions that initialise a program's globals: from the entry
/// stub's target up to `SAVEBP`, empty for a program without globals. The
/// stub is `JSR; RETN` before `main` and `RSADDI; JSR; RETN` before
/// `StartingConditional`, whose return value it reserves first.
fn globals_section(p: &Program) -> &[Instruction] {
    let ins = p.instructions();
    let Some(save) = ins.iter().position(|x| x.opcode == Opcode::SaveBp) else {
        return &[];
    };
    let start = ins
        .iter()
        .take(2)
        .position(|x| x.opcode == Opcode::Jsr)
        .and_then(|i| ins[i].jump_target(p.offsets()[i]))
        .and_then(|t| p.index_at(t))
        .filter(|&t| t <= save)
        .unwrap_or(save);
    &ins[start..save]
}

/// The includes to fold into a script, from the functions whose shape a
/// library's function has and the globals the libraries would declare.
fn plan_fold(
    d: &Decompiled,
    p: &Program,
    libs: &[Library],
    index: &HashMap<Vec<u8>, Vec<(usize, usize)>>,
) -> Option<Fold> {
    let ins = p.instructions();
    let gb = globals_bytes(p);
    let mut matches: Vec<(usize, Vec<(usize, usize)>)> = Vec::new();
    for (i, f) in d.functions.iter().enumerate() {
        if f.kind != Kind::User || f.status != Status::Verified {
            continue;
        }
        let s = p.index_at(f.start)?;
        let e = p.index_at(f.end).unwrap_or(ins.len());
        let Some(hits) = index.get(&shape(&ins[s..e], gb)) else {
            continue;
        };
        let (ret, params) = d.signature(i)?;
        let hits: Vec<(usize, usize)> = hits
            .iter()
            .copied()
            .filter(|&(l, fi)| libs[l].functions[fi].accepts(&ret, &params))
            .collect();
        if !hits.is_empty() {
            matches.push((i, hits));
        }
    }
    // Closed under calls: a folded function's library version calls the
    // library's versions of its callees, so a callee that did not match
    // (the script embeds an older copy, say) keeps its callers unfolded
    // too; otherwise the folded program would hold two copies of it.
    let starts: HashMap<u32, usize> = d
        .functions
        .iter()
        .enumerate()
        .map(|(i, f)| (f.start, i))
        .collect();
    let callees = |i: usize| -> Vec<usize> {
        let f = &d.functions[i];
        let (Some(s), e) = (p.index_at(f.start), p.index_at(f.end).unwrap_or(ins.len())) else {
            return Vec::new();
        };
        (s..e)
            .filter(|&k| ins[k].opcode == Opcode::Jsr)
            .filter_map(|k| ins[k].jump_target(p.offsets()[k]))
            .filter_map(|t| starts.get(&t).copied())
            .collect()
    };
    loop {
        let matched: Vec<usize> = matches.iter().map(|(i, _)| *i).collect();
        let before = matches.len();
        matches.retain(|(i, _)| callees(*i).iter().all(|c| matched.contains(c)));
        if matches.len() == before {
            break;
        }
    }
    if matches.is_empty() {
        return None;
    }
    let globals = globals_section(p);

    let mut chosen: Vec<usize> = Vec::new();
    let mut files: BTreeSet<&str> = BTreeSet::new();
    let mut consumed = 0;
    let mut covered = vec![false; matches.len()];
    while chosen.len() < 4 && covered.iter().any(|c| !c) {
        let mut best: Option<(usize, usize)> = None;
        for (li, lib) in libs.iter().enumerate() {
            if chosen.contains(&li) || lib.files.iter().any(|f| files.contains(f.as_str())) {
                continue;
            }
            let init = &lib.globals_init;
            if globals.len() < consumed + init.len()
                || globals[consumed..consumed + init.len()] != init[..]
            {
                continue;
            }
            let cover = matches
                .iter()
                .zip(&covered)
                .filter(|((_, hits), c)| !**c && hits.iter().any(|(l, _)| *l == li))
                .count();
            if cover == 0 {
                continue;
            }
            let better = match best {
                None => true,
                Some((b, bc)) => {
                    (
                        cover,
                        std::cmp::Reverse(libs[li].functions.len()),
                        &lib.name,
                    ) > (
                        bc,
                        std::cmp::Reverse(libs[b].functions.len()),
                        &libs[b].name,
                    )
                }
            };
            if better {
                best = Some((li, cover));
            }
        }
        let Some((li, _)) = best else {
            break;
        };
        chosen.push(li);
        consumed += libs[li].globals_init.len();
        files.extend(libs[li].files.iter().map(String::as_str));
        for ((_, hits), c) in matches.iter().zip(covered.iter_mut()) {
            if hits.iter().any(|(l, _)| *l == li) {
                *c = true;
            }
        }
    }
    if chosen.is_empty() {
        return None;
    }
    let lib_globals: Vec<&LibGlobal> = chosen
        .iter()
        .flat_map(|&li| libs[li].globals.iter())
        .collect();
    let (global_names, structs) = align_globals(&d.globals(), &lib_globals)?;
    // Which library function each matched one is. Where several have the
    // same shape and signature (the debug printers, say), a folded caller
    // decides: its library version calls a known function at each `JSR`,
    // and the callee at the same position in the script is that one. So do
    // known callees: a candidate whose library version calls other
    // functions than the ones the script's callees are is not this one
    // (`IsDark` and `IsLight` compile alike but call different functions).
    // The rest take their first candidate.
    let cands: Vec<(usize, Vec<(usize, usize)>)> = matches
        .iter()
        .map(|(i, hits)| {
            let c: Vec<(usize, usize)> = hits
                .iter()
                .copied()
                .filter(|(l, _)| chosen.contains(l))
                .collect();
            (*i, c)
        })
        .filter(|(_, c)| !c.is_empty())
        .collect();
    // The candidates whose library version calls, at each `JSR` to a
    // callee already picked, that callee's library function.
    let fitting = |i: usize,
                   c: &[(usize, usize)],
                   pick: &HashMap<usize, (usize, usize)>|
     -> Vec<(usize, usize)> {
        let called = callees(i);
        c.iter()
            .copied()
            .filter(|&(l, fi)| {
                called
                    .iter()
                    .enumerate()
                    .all(|(k, callee)| match pick.get(callee) {
                        Some(&(pl, pf)) => {
                            pl == l
                                && libs[l].functions[fi].callees.get(k).copied().flatten()
                                    == Some(pf)
                        }
                        None => true,
                    })
            })
            .collect()
    };
    let mut pick: HashMap<usize, (usize, usize)> = cands
        .iter()
        .filter(|(_, c)| c.len() == 1)
        .map(|(i, c)| (*i, c[0]))
        .collect();
    loop {
        let mut changed = false;
        for (i, c) in &cands {
            if pick.contains_key(i) {
                continue;
            }
            let demanded = pick.iter().find_map(|(j, (l, fj))| {
                callees(*j)
                    .iter()
                    .enumerate()
                    .filter(|(_, callee)| *callee == i)
                    .find_map(|(k, _)| libs[*l].functions[*fj].callees.get(k).copied().flatten())
                    .map(|fi| (*l, fi))
                    .filter(|hit| c.contains(hit))
            });
            if let Some(hit) = demanded {
                pick.insert(*i, hit);
                changed = true;
                continue;
            }
            if let [hit] = fitting(*i, c, &pick)[..] {
                pick.insert(*i, hit);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut functions = Vec::new();
    for (i, c) in &cands {
        let (l, fi) = pick
            .get(i)
            .copied()
            .or_else(|| fitting(*i, c, &pick).first().copied())
            .unwrap_or(c[0]);
        functions.push((*i, libs[l].functions[fi].name.clone()));
    }
    Some(Fold {
        includes: chosen.iter().map(|&li| libs[li].name.clone()).collect(),
        globals: global_names,
        functions,
        structs,
    })
}

/// A string literal's bytes as JSON text: Windows-1252, each byte the
/// character of the same number.
fn text(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// Every engine call, with the arguments that are constants. Arguments are
/// pushed last first, so the instruction before `ACTION` is the first
/// argument when it is a constant, and the chain stops at the first
/// computed one.
fn calls(p: &Program, table: &EngineTable, job: &Job) -> Vec<Call> {
    use serde_json::Value;
    let ins = p.instructions();
    let mut out = Vec::new();
    for (i, x) in ins.iter().enumerate() {
        let Operands::Action { routine, argc } = x.operands else {
            continue;
        };
        let f = table.functions.get(usize::from(routine));
        let name = f.map_or_else(|| format!("routine_{routine}"), |f| f.name.clone());
        let mut args: Vec<Value> = Vec::new();
        let mut j = i;
        for k in 0..usize::from(argc) {
            let ty = f.and_then(|f| f.params.get(k)).map(|p| &p.ty);
            let lit = if matches!(ty, Some(Type::Vector)) {
                let comps: Option<Vec<f64>> = (1..=3)
                    .map(|n| match ins.get(j.checked_sub(n)?).map(|x| &x.operands) {
                        Some(Operands::Float(v)) => Some(f64::from(*v)),
                        _ => None,
                    })
                    .collect();
                match comps {
                    Some(zyx) => {
                        j -= 3;
                        Some(Value::from(vec![zyx[2], zyx[1], zyx[0]]))
                    }
                    None => None,
                }
            } else {
                let lit = match j
                    .checked_sub(1)
                    .and_then(|k| ins.get(k))
                    .map(|x| &x.operands)
                {
                    Some(Operands::Int(v)) => Some(Value::from(*v)),
                    Some(Operands::Float(v)) => Some(Value::from(f64::from(*v))),
                    Some(Operands::String(s)) => Some(Value::from(text(s))),
                    Some(Operands::Object(0)) => {
                        Some(serde_json::json!({ "object": "OBJECT_SELF" }))
                    }
                    Some(Operands::Object(1)) => {
                        Some(serde_json::json!({ "object": "OBJECT_INVALID" }))
                    }
                    Some(Operands::Object(v)) => Some(serde_json::json!({ "object": v })),
                    _ => None,
                };
                if lit.is_some() {
                    j -= 1;
                }
                lit
            };
            match lit {
                Some(v) => args.push(v),
                None => break,
            }
        }
        args.resize(usize::from(argc), Value::Null);
        out.push(Call {
            script: job.name.clone(),
            capsule: job.capsule.clone(),
            routine: name,
            args,
            offset: p.offsets()[i],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capsule_dir_is_its_stem_unless_another_capsule_shares_it() {
        let mut stems = HashMap::new();
        stems.insert("danm14ac".to_string(), 2);
        stems.insert("danm14ac_s".to_string(), 1);
        assert_eq!(
            capsule_dir(Path::new("modules/DANM14AC.mod"), &stems),
            "danm14ac.mod"
        );
        assert_eq!(
            capsule_dir(Path::new("modules/danm14ac_s.rim"), &stems),
            "danm14ac_s"
        );
    }

    /// Routines 0 to 2 as a small `nwscript.nss` declares them.
    const ENGINE: &str = "\
#define ENGINE_NUM_STRUCTURES 1
#define ENGINE_STRUCTURE_0 effect
void ExecuteScript(string sScript, object oTarget, int nScriptVar = -1);
void SetFacingPoint(vector vTarget);
int GetHitDice(object oCreature);
";

    fn job(bytes: Vec<u8>) -> Job {
        Job {
            source: "modules",
            capsule: "danm14ac_s".into(),
            name: "k_pdan_juhani27".into(),
            here: "modules/danm14ac_s.rim".into(),
            module: None,
            bytes,
        }
    }

    #[test]
    fn calls_take_the_constant_arguments_before_the_action_and_stop_at_a_computed_one() {
        let (table, diags) =
            EngineTable::from_nwscript(nwscript::diag::FileId(0), ENGINE.as_bytes());
        assert!(diags.is_empty(), "{diags:?}");
        // ExecuteScript("k_pman_aioff", oLocal, -1): the object is computed,
        // so the default after it is not read either; SetFacingPoint takes a
        // vector literal; GetHitDice(OBJECT_SELF) an object constant.
        let program = nwscript::ncs::asm::parse(
            nwscript::diag::FileId(1),
            b"JSR main\nRETN\nmain:\nCONSTI -1\nCPTOPSP -8, 4\nCONSTS \"k_pman_aioff\"\nACTION 0, 3\nCONSTF 1.0\nCONSTF 2.0\nCONSTF 3.0\nACTION 1, 1\nCONSTO 0\nACTION 2, 1\nMOVSP -4\nRETN\n",
        )
        .unwrap();
        let calls = calls(&program, &table, &job(nwscript::ncs::encode(&program)));
        let rows: Vec<serde_json::Value> = calls
            .iter()
            .map(|c| serde_json::to_value(c).unwrap())
            .collect();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["routine"], "ExecuteScript");
        assert_eq!(
            rows[0]["args"],
            serde_json::json!(["k_pman_aioff", null, null])
        );
        assert_eq!(rows[0]["script"], "k_pdan_juhani27");
        assert_eq!(rows[0]["capsule"], "danm14ac_s");
        assert_eq!(rows[1]["routine"], "SetFacingPoint");
        assert_eq!(rows[1]["args"], serde_json::json!([[1.0, 2.0, 3.0]]));
        assert_eq!(rows[2]["routine"], "GetHitDice");
        assert_eq!(
            rows[2]["args"],
            serde_json::json!([{ "object": "OBJECT_SELF" }])
        );
        assert!(rows[0]["offset"].as_u64().unwrap() < rows[1]["offset"].as_u64().unwrap());
    }

    #[test]
    fn the_globals_section_starts_after_either_entry_stub() {
        let asm = |text: &str| {
            nwscript::ncs::asm::parse(nwscript::diag::FileId(0), text.as_bytes()).unwrap()
        };
        let globals = "g:\nRSADDI\nCONSTI 0\nCPDOWNSP -8, 4\nMOVSP -4\nSAVEBP\n";
        // `main` is entered by `JSR; RETN`, `StartingConditional` by
        // `RSADDI; JSR; RETN`: both sections are the same four instructions.
        let main = asm(&format!(
            "JSR g\nRETN\n{globals}JSR main\nRESTOREBP\nMOVSP -4\nRETN\nmain:\nRETN\n"
        ));
        let cond = asm(&format!(
            "RSADDI\nJSR g\nRETN\n{globals}RSADDI\nJSR sc\nCPDOWNSP -12, 4\nMOVSP -4\nRESTOREBP\nMOVSP -4\nRETN\nsc:\nCONSTI 1\nCPDOWNSP -8, 4\nMOVSP -4\nRETN\n"
        ));
        assert_eq!(globals_section(&main).len(), 4);
        assert_eq!(globals_section(&main), globals_section(&cond));
        assert_eq!(globals_section(&cond)[0].opcode, Opcode::RsAdd);
        let bare = asm("JSR main\nRETN\nmain:\nRETN\n");
        assert!(globals_section(&bare).is_empty());
    }

    #[test]
    fn globals_align_whole_variables_and_slot_by_slot_structs() {
        let int = |name: &str| LibGlobal {
            name: name.into(),
            size: 4,
            struct_name: None,
            fields: Vec::new(),
        };
        let st = LibGlobal {
            name: "tPR".into(),
            size: 8,
            struct_name: Some("tLastRound".into()),
            fields: vec![("nA".into(), 4), ("nB".into(), 4)],
        };
        let recovered = |sizes: &[(u32, Option<usize>)]| -> Vec<Global> {
            sizes
                .iter()
                .enumerate()
                .map(|(i, &(size, struct_index))| Global {
                    name: format!("g{i}"),
                    size,
                    struct_index,
                    fields: if struct_index.is_some() { 2 } else { 0 },
                })
                .collect()
        };
        let (a, b) = (int("A"), int("B"));
        // The struct recovered slot by slot: one name per slot.
        let (names, structs) = align_globals(
            &recovered(&[(4, None), (4, None), (4, None), (4, None)]),
            &[&a, &st, &b],
        )
        .unwrap();
        assert_eq!(names, ["A", "tPR.nA", "tPR.nB", "B"]);
        assert!(structs.is_empty());
        // The struct recovered whole: its name, and the struct is folded.
        let (names, structs) = align_globals(
            &recovered(&[(4, None), (8, Some(0)), (4, None)]),
            &[&a, &st, &b],
        )
        .unwrap();
        assert_eq!(names, ["A", "tPR", "B"]);
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "tLastRound");
        assert_eq!(structs[0].fields, ["nA", "nB"]);
        // Too few recovered globals, or sizes that do not line up: no fold.
        assert!(align_globals(&recovered(&[(4, None)]), &[&a, &b]).is_none());
        assert!(align_globals(&recovered(&[(12, None), (4, None)]), &[&a, &b]).is_none());
    }

    #[test]
    fn a_tally_prints_its_fraction() {
        let t = Tally {
            functions: 46511,
            verified: 46475,
            ..Tally::default()
        };
        assert_eq!(t.percent(), "99.92%");
        assert_eq!(Tally::default().percent(), "-");
    }
}
