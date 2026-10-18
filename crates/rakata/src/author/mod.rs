//! The authoring API: the operations a mod performs on an install, as
//! library calls, each write checked before it lands.
//!
//! A [`Mod`] is an install opened for writing, with the module the mod
//! works in. It answers which copy of a resource the game loads
//! ([`Mod::loaded`], which sees the mod's own writes first), guards a
//! resource the mod replaces ([`Mod::guard_replaced`]), and writes to
//! `Override/` and `streamwaves/` ([`Mod::write_override`],
//! [`Mod::write_streamwaves`]). Every write is read back as its type first;
//! a dialogue is also checked with [`rakata_dialog::check()`] against the
//! install and the mod's earlier writes, and refused, with the file on disk
//! unchanged, when a node or link it adds or changes names a script, a
//! voice-over, a strref or a global the install lacks. A reference the
//! loaded copy already lacked is the game's or another mod's, and comes
//! back as a warning instead.
//!
//! The edits themselves work on owned values, read from
//! [`Mod::loaded`]'s bytes and written back as bytes: [`twoda`] adds rows,
//! [`globals`] declares globals in `globalcat.2da`, [`blueprint`] sets
//! fields by path, [`dialog`] adds entries, replies, links and starts by
//! stable key without moving an index, [`lip`] makes lip-sync files from
//! cues, and [`scripts`] compiles against the install's includes. [`voice`]
//! is the part of a voice pipeline that is not ffmpeg or Rhubarb: the
//! spans to cut, their cache key, and the cues back into a LIP.
//!
//! The order of a mod's writes matters for the checks: compile the scripts
//! a dialogue runs, write its voice-overs and declare its globals before
//! writing the dialogue.

pub mod blueprint;
pub mod dialog;
mod error;
pub mod globals;
pub mod lip;
mod refs;
pub mod scripts;
pub mod twoda;
pub mod voice;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rakata_dialog::{Dialog, Finding, NodeKey, NodeRef};
use rakata_game::{Install, Layer, Location, ModuleName};
use rakata_res::{ContentHash, ResId, ResRef, ResType};
use rakata_tlk::Tlk;

pub use error::{Error, Result, codes};

use crate::corpus::find_ci;

/// A copy of a resource as the game would load it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The bytes.
    pub bytes: Vec<u8>,
    /// Where they lie, relative to the install: `Override/x.2da`,
    /// `modules/danm14ac_s.rim:k_pdan_juhani31.ncs`, `data/2da.bif[12]`.
    pub location: String,
    /// Whether the copy is one this mod wrote.
    pub written: bool,
}

/// A file the mod wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The file.
    pub path: PathBuf,
    /// The SHA-256 of what was written.
    pub hash: ContentHash,
    /// Its length in bytes.
    pub len: usize,
    /// For a dialogue, the references its base copy already lacked: the
    /// game's or another mod's, reported and not refused.
    pub warnings: Vec<Finding>,
}

/// An install opened for a mod's writes, in one module.
pub struct Mod {
    install: Arc<Install>,
    module: Option<ModuleName>,
    /// What this mod wrote to `Override/`, which the install's listing,
    /// taken when it opened, does not show.
    written: HashMap<ResId, PathBuf>,
    /// The lowercase stems of voice-overs on disk under `streamwaves/` and
    /// `streamvoice/` and written by this mod; listed on first use.
    voices: Option<HashSet<String>>,
    tlk: Option<Option<Tlk>>,
    globals: Option<Option<HashSet<String>>>,
    routines: Option<Vec<String>>,
}

impl std::fmt::Debug for Mod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mod")
            .field("install", &self.install.root())
            .field("module", &self.module)
            .field("written", &self.written.len())
            .finish_non_exhaustive()
    }
}

impl Mod {
    /// Open the install for a mod working in `module` (`danm14ac`), or in
    /// the global layers alone. Fails when the install has no capsule of
    /// that module.
    pub fn open(install: impl Into<Arc<Install>>, module: Option<&str>) -> Result<Mod> {
        let mut m = Mod {
            install: install.into(),
            module: None,
            written: HashMap::new(),
            voices: None,
            tlk: None,
            globals: None,
            routines: None,
        };
        m.set_module(module)?;
        Ok(m)
    }

    /// The install.
    pub fn install(&self) -> &Install {
        &self.install
    }

    /// The module the mod works in.
    pub fn module(&self) -> Option<&ModuleName> {
        self.module.as_ref()
    }

    /// Work in `module` from now on, or in the global layers alone, keeping
    /// every write made so far: a mod whose scenes lie in several modules
    /// writes its scripts and globals once, then each module's dialogues
    /// with the checks run against that module. Fails, with the module
    /// unchanged, when the install has no capsule of it (`RE0403`).
    pub fn set_module(&mut self, module: Option<&str>) -> Result<()> {
        let module = match module {
            None => None,
            Some(m) => Some(self.module_named(m)?),
        };
        if module != self.module {
            self.module = module;
            // Read through `loaded`, which searches the module's capsules
            self.globals = None;
            self.routines = None;
        }
        Ok(())
    }

    fn module_named(&self, m: &str) -> Result<ModuleName> {
        let name = ModuleName::new(m);
        if self.install.modules().contains(&name) {
            Ok(name)
        } else {
            Err(Error::Game(rakata_game::Error::NoSuchModule {
                module: m.to_string(),
                dir: find_ci(self.install.root(), "modules"),
            }))
        }
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

    /// The copy of `id` the game loads: this mod's own write when it made
    /// one, else the install's in the engine's order (`Override/`, the
    /// module's capsules, `patch.erf`, the BIFs). Fails with `RE0404` when
    /// no layer holds it.
    pub fn loaded(&self, id: ResId) -> Result<Loaded> {
        if let Some(path) = self.written.get(&id) {
            let bytes = fs::read(path).map_err(|source| {
                Error::Game(rakata_game::Error::Io {
                    path: path.clone(),
                    source,
                })
            })?;
            return Ok(Loaded {
                bytes,
                location: self.relative(&Location::File { path: path.clone() }),
                written: true,
            });
        }
        match self.install.read(id, self.module.as_ref())? {
            Some((location, bytes)) => Ok(Loaded {
                bytes,
                location: self.relative(&location),
                written: false,
            }),
            None => Err(Error::Game(rakata_game::Error::NotFound {
                id,
                module: self.module.as_ref().map(ToString::to_string),
            })),
        }
    }

    /// Guard a resource the mod replaces: every copy in the module's
    /// capsules must be the one the mod was written against, by SHA-256,
    /// and `Override/` must hold none, or another mod's change to it would
    /// be dropped without a word. Fails naming the capsule whose copy
    /// differs (`RE1401`), the `Override/` file (`RE1402`), or the module
    /// when it holds no copy at all (`RE1403`).
    pub fn guard_replaced(&self, id: ResId, expected: ContentHash) -> Result<()> {
        if let Some(path) = self.written.get(&id) {
            return Err(Error::OverrideHolds {
                id,
                path: path.clone(),
            });
        }
        let module = self.module.as_ref();
        let r = self.install.resolve(id, module)?;
        if let Some(c) = r.candidates.iter().find(|c| c.layer == Layer::Override) {
            return Err(Error::OverrideHolds {
                id,
                path: c.location.path().to_path_buf(),
            });
        }
        let copies: Vec<_> = r
            .candidates
            .iter()
            .filter(|c| c.layer == Layer::Module)
            .collect();
        if copies.is_empty() {
            return Err(Error::NotInModule {
                id,
                module: module.map(ToString::to_string),
            });
        }
        for c in copies {
            if c.hash != expected {
                return Err(Error::ReplacedDiffers {
                    id,
                    capsule: c
                        .location
                        .path()
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    expected,
                    found: c.hash,
                });
            }
        }
        Ok(())
    }

    /// Guard a resource the mod writes to `Override/` from the copy the
    /// module loads: `Override/` outranks every module's capsules, so the
    /// write replaces the resource in every module, and a module holding
    /// its own copy that differs (two cantinas each with their own copy of
    /// one conversation) would lose it. Passes when `Override/` already
    /// holds a copy, or the mod wrote one, since every module loads that
    /// already. Fails naming the first other module whose own copy differs
    /// from this module's (`RE1415`).
    pub fn guard_other_modules(&self, id: ResId) -> Result<()> {
        if self.written.contains_key(&id) {
            return Ok(());
        }
        let here = self.install.resolve(id, self.module.as_ref())?;
        if here.candidates.iter().any(|c| c.layer == Layer::Override) {
            return Ok(());
        }
        let edited = here.candidates.first().map(|c| c.hash);
        for module in self.install.modules() {
            if Some(module) == self.module.as_ref() {
                continue;
            }
            let there = self.install.resolve(id, Some(module))?;
            let own = there
                .candidates
                .iter()
                .find(|c| c.layer == Layer::Module && Some(c.hash) != edited);
            if let Some(c) = own {
                return Err(Error::OtherModuleDiffers {
                    id,
                    here: self.module.as_ref().map(ToString::to_string),
                    module: module.to_string(),
                    capsule: c
                        .location
                        .path()
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                });
            }
        }
        Ok(())
    }

    /// The node of `dialog` a key names, text keys compared through the
    /// install's talk table as well as the text a node embeds.
    pub fn find(&mut self, dialog: &Dialog, key: &NodeKey) -> Result<NodeRef> {
        let tlk = self.tlk();
        Ok(key.resolve_with(dialog, &mut |s| {
            tlk.and_then(|t| t.resolve(s, t.language, rakata_res::Gender::Male))
                .map(str::to_owned)
        })?)
    }

    /// The reference checks over `dialog`, against the install and this
    /// mod's writes: what `write_override` runs before writing one.
    pub fn check_dialog(&mut self, dialog: &Dialog) -> Vec<Finding> {
        rakata_dialog::check(dialog, self)
    }

    /// Write `bytes` to `Override/` as `name` (`appearance.2da`,
    /// `oaj_juhani1.utc`, `k_oaj_fight.ncs`), onto the spelling of a file
    /// already there when one differs only in case. The bytes are read
    /// back as their type first (a GFF, a 2DA, a LIP, NCS); a dialogue is
    /// checked and refused (`RE1407`) when a node or link it adds or
    /// changes against the copy the game loads now names what the install
    /// lacks. A refused write leaves the disk as it was.
    pub fn write_override(&mut self, name: &str, bytes: &[u8]) -> Result<Written> {
        let id = ResId::parse(name)?;
        let warnings = self.validate(id, name, bytes)?;
        let dir = match find_ci(self.install.root(), "Override") {
            Some(d) => d,
            None => self.install.root().join("Override"),
        };
        let path = self.written.get(&id).cloned().unwrap_or_else(|| {
            self.install
                .resolve(id, None)
                .ok()
                .and_then(|r| {
                    r.candidates
                        .into_iter()
                        .find(|c| c.layer == Layer::Override)
                        .map(|c| c.location.path().to_path_buf())
                })
                .unwrap_or_else(|| dir.join(name))
        });
        write_file(&path, bytes)?;
        self.written.insert(id, path.clone());
        match id.ty {
            ResType::TwoDa if id.name == res("globalcat") => self.globals = None,
            ResType::Nss if id.name == res("nwscript") => self.routines = None,
            _ => {}
        }
        Ok(Written {
            path,
            hash: ContentHash::of(bytes),
            len: bytes.len(),
            warnings,
        })
    }

    /// Write a voice-over to `streamwaves/` as `resref.wav`, onto the
    /// spelling of a file already there when one differs only in case.
    /// Refuses bytes that do not begin with a RIFF WAVE header (`RE1413`),
    /// which every voice file the game ships carries.
    pub fn write_streamwaves(&mut self, resref: &str, bytes: &[u8]) -> Result<Written> {
        let r = ResRef::new(resref)?;
        let name = format!("{resref}.wav");
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(Error::NotAWave { name });
        }
        let dir = match find_ci(self.install.root(), "streamwaves") {
            Some(d) => d,
            None => self.install.root().join("streamwaves"),
        };
        let path = find_ci(&dir, &name).unwrap_or_else(|| dir.join(&name));
        write_file(&path, bytes)?;
        let stem = r.to_string().to_ascii_lowercase();
        self.voices().insert(stem);
        Ok(Written {
            path,
            hash: ContentHash::of(bytes),
            len: bytes.len(),
            warnings: Vec::new(),
        })
    }

    /// Read `bytes` back as `id`'s type; for a dialogue, run the checks
    /// against the copy loaded now and refuse what the edit introduced.
    fn validate(&mut self, id: ResId, name: &str, bytes: &[u8]) -> Result<Vec<Finding>> {
        match id.ty {
            ResType::Dlg => {
                let dialog = Dialog::parse(name, bytes)?;
                let base = self
                    .loaded(id)
                    .ok()
                    .and_then(|l| Dialog::parse(name, &l.bytes).ok());
                let before = match &base {
                    Some(b) => self.check_dialog(b),
                    None => Vec::new(),
                };
                let (old, new): (Vec<Finding>, Vec<Finding>) = self
                    .check_dialog(&dialog)
                    .into_iter()
                    .partition(|f| before.iter().any(|b| b.same_as(f)));
                if !new.is_empty() {
                    return Err(Error::DialogRefused {
                        dialog: name.to_string(),
                        findings: new,
                    });
                }
                Ok(old)
            }
            ResType::TwoDa => {
                rakata_2da::TwoDa::parse(bytes)?;
                Ok(Vec::new())
            }
            ResType::Lip => {
                rakata_lip::Lip::parse(bytes)?;
                Ok(Vec::new())
            }
            ResType::Ncs => {
                let mut map = nwscript::diag::SourceMap::new();
                nwscript::ncs::decode_named(name, bytes, &mut map).map_err(|d| Error::Ncs {
                    name: name.to_string(),
                    message: d.message,
                })?;
                Ok(Vec::new())
            }
            _ if bytes.get(4..8) == Some(b"V3.2") => {
                rakata_gff::Gff::parse(bytes)?;
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    fn tlk(&mut self) -> Option<&Tlk> {
        let root = self.install.root().to_path_buf();
        self.tlk
            .get_or_insert_with(|| find_ci(&root, "dialog.tlk").and_then(|p| Tlk::open(p).ok()))
            .as_ref()
    }

    fn voices(&mut self) -> &mut HashSet<String> {
        let root = self.install.root().to_path_buf();
        self.voices.get_or_insert_with(|| {
            let mut found = HashSet::new();
            for dir in ["streamwaves", "streamvoice"] {
                if let Some(d) = find_ci(&root, dir) {
                    stems(&d, &mut found);
                }
            }
            found
        })
    }
}

fn res(name: &str) -> ResRef {
    ResRef::new(name).expect("a short name")
}

/// Every file's lowercase stem below `dir`.
fn stems(dir: &Path, out: &mut HashSet<String>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() => stems(&p, out),
            Ok(_) => {
                if let Some(s) = p.file_stem() {
                    out.insert(s.to_string_lossy().to_ascii_lowercase());
                }
            }
            Err(_) => {}
        }
    }
}

/// Write the file whole or not at all: to a temporary name beside it, then
/// renamed over it.
fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let fail = |source| Error::Write {
        path: path.to_path_buf(),
        source,
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(fail)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".rakata-tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes).map_err(fail)?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        fail(e)
    })
}
