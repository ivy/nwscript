//! Dialogues against an install: a DLG loaded by name through the engine's
//! search order or from a file, and the [`Lookup`] its transcript needs,
//! answered from the install: text through `dialog.tlk`, each voice-over's
//! duration from the head of its `streamwaves/` file, and every condition
//! decompiled with [`Recovery`] (the corpus's own setup, include libraries
//! folded out) and verified by recompiling. Decompiled scripts are cached
//! by content hash for the life of the [`Session`], so a condition shared
//! by many dialogues is decompiled once.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use nwscript::MemResolver;
use nwscript::decompile::ProjectCompiler;
use nwscript::diag::{Diagnostic, SourceMap};
use rakata_dialog::{Dialog, Lookup, Script, ScriptStatus, Voice, audio};
use rakata_game::{Install, Location, ModuleName, Profile};
use rakata_res::{ContentHash, Gender, Language, LocalizedString, ResId, ResRef, ResType};
use rakata_tlk::Tlk;
use serde::Serialize;

pub use rakata_dialog::{
    Kind, NodeKey, NodeRef, Options as TranscriptOptions, Transcript, codes as dialog_codes,
};

use crate::corpus::{FoldOutcome, Recovered, Recovery, find_ci};

/// Why a dialogue could not be loaded. Each variant has a registered code.
#[derive(Debug)]
pub enum Error {
    /// The install could not be opened, or a resource of it read.
    Game(rakata_game::Error),
    /// The dialogue did not read, or a key did not resolve.
    Dialog(rakata_dialog::Error),
    /// `dialog.tlk` did not read.
    Tlk(rakata_tlk::Error),
    /// A file could not be read.
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The name is not `NAME.dlg`.
    NotDlg { name: String },
    /// No module was named, the global layers do not hold the dialogue,
    /// and several modules do.
    Modules { id: ResId, modules: Vec<String> },
}

impl Error {
    /// The registered diagnostic code.
    pub fn code(&self) -> &'static str {
        match self {
            Error::Game(e) => e.code(),
            Error::Dialog(e) => e.code(),
            Error::Tlk(e) => e.code(),
            Error::Read { .. } => "RE0001",
            Error::NotDlg { .. } => rakata_dialog::codes::NOT_DIALOG,
            Error::Modules { .. } => "RE1105",
        }
    }

    /// The message without the code.
    pub fn message(&self) -> String {
        match self {
            Error::Game(e) => e.message(),
            Error::Dialog(e) => e.message(),
            Error::Tlk(e) => {
                let text = e.to_string();
                text.strip_prefix(e.code())
                    .and_then(|t| t.strip_prefix(": "))
                    .unwrap_or(&text)
                    .to_string()
            }
            Error::Read { path, source } => format!("cannot read {}: {source}", path.display()),
            Error::NotDlg { name } => {
                format!("`{name}` is neither a file nor a dialogue's name as NAME.dlg")
            }
            Error::Modules { id, modules } => format!(
                "{id} is in no global layer and in {} modules: {}; name one with --module",
                modules.len(),
                modules.join(", ")
            ),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for Error {}

impl From<rakata_game::Error> for Error {
    fn from(e: rakata_game::Error) -> Self {
        Error::Game(e)
    }
}

impl From<rakata_dialog::Error> for Error {
    fn from(e: rakata_dialog::Error) -> Self {
        Error::Dialog(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A dialogue loaded, with where it came from.
#[derive(Debug)]
pub struct Loaded {
    pub dialog: Dialog,
    /// The file or the archive member, relative to the install where it
    /// lies inside it.
    pub location: String,
    /// The module whose capsules were searched, named or found.
    pub module: Option<String>,
}

/// What the scripts of a session cost.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Stats {
    /// Scripts decompiled, and of those, the ones whose every function
    /// verified.
    pub decompiled: usize,
    pub verified: usize,
    /// Lookups answered from the cache, by content hash.
    pub cache_hits: usize,
    /// Scripts no layer holds.
    pub missing: usize,
    /// Seconds spent building the include libraries for folding, once.
    pub setup_seconds: f64,
    /// Seconds spent decompiling and verifying.
    pub decompile_seconds: f64,
}

/// An install opened for reading dialogues.
pub struct Session {
    install: Install,
    module: Option<ModuleName>,
    tlk: Option<Tlk>,
    recovery: Option<Recovery>,
    compiler: Option<ProjectCompiler<MemResolver>>,
    /// Why the recovery could not be built, once tried.
    no_recovery: Option<String>,
    cache: HashMap<ContentHash, Script>,
    voices: Option<HashMap<String, PathBuf>>,
    stats: Stats,
    warnings: Vec<Diagnostic>,
}

impl Session {
    /// Open the install at `game` (the folder holding `chitin.key`) and its
    /// `dialog.tlk`. Fails when the install does not open or the talk table
    /// does not read; an install without one shows strrefs unresolved.
    pub fn open(game: &Path, profile: Profile) -> Result<Self> {
        let install = Install::open(game, profile)?;
        let tlk = match find_ci(install.root(), "dialog.tlk") {
            Some(path) => Some(Tlk::open(&path).map_err(Error::Tlk)?),
            None => None,
        };
        Ok(Session {
            install,
            module: None,
            tlk,
            recovery: None,
            compiler: None,
            no_recovery: None,
            cache: HashMap::new(),
            voices: None,
            stats: Stats::default(),
            warnings: Vec::new(),
        })
    }

    pub fn install(&self) -> &Install {
        &self.install
    }

    /// Search this module's capsules too, for the dialogue and its scripts.
    pub fn set_module(&mut self, module: Option<ModuleName>) {
        self.module = module;
    }

    pub fn module(&self) -> Option<&ModuleName> {
        self.module.as_ref()
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Warnings from building the include libraries (RW1002).
    pub fn take_warnings(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.warnings)
    }

    fn relative(&self, l: &Location) -> String {
        let rel = |p: &Path| {
            p.strip_prefix(self.install.root())
                .unwrap_or(p)
                .display()
                .to_string()
        };
        match l {
            Location::File { path } => rel(path),
            Location::Capsule { path, member } => format!("{}:{member}", rel(path)),
            Location::Bif { path, index, .. } => format!("{}[{index}]", rel(path)),
        }
    }

    /// Load `what`: a DLG file when such a file exists, else `NAME.dlg` as
    /// the engine resolves it, through the session's module when one is
    /// set. With no module set and no global copy, the module that holds
    /// it is found and set; several are an error naming them.
    pub fn load(&mut self, what: &str) -> Result<Loaded> {
        let path = Path::new(what);
        if path.is_file() {
            let bytes = fs::read(path).map_err(|source| Error::Read {
                path: path.to_path_buf(),
                source,
            })?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| what.to_string());
            return Ok(Loaded {
                dialog: Dialog::parse(name, &bytes)?,
                location: path.display().to_string(),
                module: self.module.as_ref().map(ToString::to_string),
            });
        }
        let id = ResId::parse(what)
            .ok()
            .filter(|id| id.ty == ResType::Dlg)
            .ok_or_else(|| Error::NotDlg {
                name: what.to_string(),
            })?;
        self.load_id(id)
    }

    /// Load a dialogue by id, as [`Session::load`] does for a name.
    pub fn load_id(&mut self, id: ResId) -> Result<Loaded> {
        let mut found = self.install.read(id, self.module.as_ref())?;
        if found.is_none() && self.module.is_none() {
            let mut holders = Vec::new();
            for m in self.install.modules() {
                if self.install.module_ids(m)?.contains(&id) {
                    holders.push(m.clone());
                }
            }
            match holders.len() {
                0 => {}
                1 => {
                    self.module = holders.pop();
                    found = self.install.read(id, self.module.as_ref())?;
                }
                _ => {
                    return Err(Error::Modules {
                        id,
                        modules: holders.iter().map(ToString::to_string).collect(),
                    });
                }
            }
        }
        let Some((location, bytes)) = found else {
            let r = self.install.resolve(id, self.module.as_ref())?;
            return Err(r
                .not_found()
                .map(Error::Game)
                .unwrap_or_else(|| Error::NotDlg {
                    name: id.to_string(),
                }));
        };
        Ok(Loaded {
            dialog: Dialog::parse(id.to_string(), &bytes)?,
            location: self.relative(&location),
            module: self.module.as_ref().map(ToString::to_string),
        })
    }

    /// The transcript of a dialogue, with this session answering.
    pub fn transcript(&mut self, dialog: &Dialog, options: TranscriptOptions) -> Transcript {
        Transcript::build(dialog, self, options)
    }

    /// The node a key names, text keys compared through the talk table.
    pub fn resolve(&mut self, dialog: &Dialog, key: &NodeKey) -> Result<NodeRef> {
        Ok(key.resolve_with(dialog, &mut |s| self.text(s))?)
    }

    /// Every file under `streamwaves/` and `streamvoice/`, by lowercase
    /// stem, the first in path order winning.
    fn voices(&mut self) -> &HashMap<String, PathBuf> {
        self.voices.get_or_insert_with(|| {
            let mut found = Vec::new();
            for dir in ["streamwaves", "streamvoice"] {
                if let Some(d) = find_ci(self.install.root(), dir) {
                    walk(&d, &mut found);
                }
            }
            found.sort();
            let mut map = HashMap::new();
            for p in found {
                if let Some(stem) = p.file_stem() {
                    map.entry(stem.to_string_lossy().to_ascii_lowercase())
                        .or_insert(p);
                }
            }
            map
        })
    }

    fn recovery(&mut self) -> Option<(&Recovery, &mut ProjectCompiler<MemResolver>)> {
        if self.recovery.is_none() && self.no_recovery.is_none() {
            let start = Instant::now();
            match Recovery::new(&self.install, &mut self.warnings) {
                Ok(r) => {
                    self.compiler = Some(r.compiler());
                    self.recovery = Some(r);
                }
                Err(e) => self.no_recovery = Some(e.to_string()),
            }
            self.stats.setup_seconds = start.elapsed().as_secs_f64();
        }
        match (&self.recovery, &mut self.compiler) {
            (Some(r), Some(c)) => Some((r, c)),
            _ => None,
        }
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() => walk(&p, out),
            Ok(_) => out.push(p),
            Err(_) => {}
        }
    }
}

/// The recovered script as a transcript shows it.
fn script_of(name: &ResRef, location: String, r: &Recovered<'_>) -> Script {
    let d = &r.decompiled;
    let functions = d.functions.len();
    let verified = d.verified();
    let note = match &r.fold {
        FoldOutcome::None => None,
        FoldOutcome::Folded {
            functions,
            includes,
            ..
        } => Some(format!(
            "{functions} functions replaced by {}",
            includes
                .iter()
                .map(|i| format!("#include \"{i}\""))
                .collect::<Vec<_>>()
                .join(" and ")
        )),
        FoldOutcome::Refused { includes, why, .. } => Some(format!(
            "matched {} but the fold recompiled differently ({why}), so none replaced",
            includes.join(", ")
        )),
    };
    Script {
        name: name.to_string(),
        status: if verified == functions {
            ScriptStatus::Verified
        } else {
            ScriptStatus::Unverified
        },
        location: Some(location),
        functions,
        verified,
        source: Some(rakata_tlk::codepage::decode(&d.source())),
        note,
    }
}

impl Lookup for Session {
    fn text(&mut self, s: &LocalizedString) -> Option<String> {
        match &self.tlk {
            Some(t) => t.resolve(s, t.language, Gender::Male).map(str::to_owned),
            None => s
                .resolve_with(Language::English, Gender::Male, |_| None)
                .map(str::to_owned),
        }
    }

    fn voice(&mut self, vo: &ResRef) -> Option<Voice> {
        let path = self
            .voices()
            .get(&vo.to_string().to_ascii_lowercase())?
            .clone();
        let mut head = Vec::with_capacity(audio::HEAD);
        let len = fs::File::open(&path).ok().and_then(|f| {
            let len = f.metadata().ok()?.len();
            f.take(audio::HEAD as u64).read_to_end(&mut head).ok()?;
            Some(len)
        });
        Some(Voice {
            path: path
                .strip_prefix(self.install.root())
                .unwrap_or(&path)
                .display()
                .to_string(),
            audio: len.and_then(|len| audio::duration(&head, len)),
        })
    }

    fn script(&mut self, name: &ResRef, decompile: bool) -> Script {
        let id = ResId::new(*name, ResType::Ncs);
        let found = match self.install.read(id, self.module.as_ref()) {
            Ok(f) => f,
            Err(e) => {
                let mut s = Script::named(name, ScriptStatus::Unreadable);
                s.note = Some(e.message());
                return s;
            }
        };
        let Some((location, bytes)) = found else {
            self.stats.missing += 1;
            return Script::named(name, ScriptStatus::Missing);
        };
        let location = self.relative(&location);
        if !decompile {
            let mut s = Script::named(name, ScriptStatus::Found);
            s.location = Some(location);
            return s;
        }
        let hash = ContentHash::of(&bytes);
        if let Some(hit) = self.cache.get(&hash) {
            self.stats.cache_hits += 1;
            let mut s = hit.clone();
            s.name = name.to_string();
            s.location = Some(location);
            return s;
        }
        // The libraries are built on first use; their time is setup.
        self.recovery();
        let start = Instant::now();
        let display = format!("{location}:{name}.ncs");
        let script = match self.recovery() {
            None => {
                let mut s = Script::named(name, ScriptStatus::Unreadable);
                s.location = Some(location);
                s.note = self.no_recovery.clone();
                s
            }
            Some((recovery, compiler)) => {
                let mut map = SourceMap::new();
                match nwscript::ncs::decode_named(&display, &bytes, &mut map) {
                    Err(d) => unreadable(name, location, d.message),
                    Ok(program) => match recovery.recover(&display, &bytes, &program, compiler) {
                        Ok(r) => script_of(name, location, &r),
                        Err(d) => unreadable(name, location, d.message),
                    },
                }
            }
        };
        self.stats.decompile_seconds += start.elapsed().as_secs_f64();
        if self.recovery.is_some() {
            self.stats.decompiled += 1;
            if script.status == ScriptStatus::Verified {
                self.stats.verified += 1;
            }
        }
        self.cache.insert(hash, script.clone());
        script
    }
}

fn unreadable(name: &ResRef, location: String, why: String) -> Script {
    let mut s = Script::named(name, ScriptStatus::Unreadable);
    s.location = Some(location);
    s.note = Some(why);
    s
}
