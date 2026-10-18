//! Where the executor reads and writes: folders of the game named the way
//! `changes.ini` writes them (any case, `\` or `/`), loose files, and the
//! members of the capsules under `modules/`. Names compare without regard to
//! case, since the inis were written on Windows and the game runs on
//! case-sensitive file systems under Wine; a file that exists keeps its
//! spelling, and a new one takes the ini's.

use super::Failure;
use crate::issue::codes;
use rakata_archive::{Erf, ErfKind, Rim};
use rakata_game::Profile;
use rakata_res::ResId;
use std::fmt;
use std::path::{Path, PathBuf};

/// A file or a capsule member the executor writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A loose file.
    File(PathBuf),
    /// A member of a capsule (`.mod`, `.erf`, `.rim`, `.sav`).
    Member { capsule: PathBuf, id: ResId },
}

impl Target {
    /// The file on disk the target lives in.
    pub fn path(&self) -> &Path {
        match self {
            Target::File(p) => p,
            Target::Member { capsule, .. } => capsule,
        }
    }
}

/// A destination as `changes.ini` names it: a folder under the game or a
/// capsule in one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dest {
    Folder(PathBuf),
    Capsule(PathBuf),
}

/// The game folder and the installer's data folders.
#[derive(Debug, Clone)]
pub struct Files {
    pub root: PathBuf,
    /// Where the installer's files are looked for, in order: the option's
    /// folder, then `tslpatchdata` itself when they differ.
    pub data_dirs: Vec<PathBuf>,
    /// The game, which decides the capsules a new `.mod` is made from.
    pub profile: Profile,
}

/// Whether a file name is a capsule's, by its extension.
pub fn is_capsule_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".mod", ".erf", ".rim", ".sav", ".hak"]
        .iter()
        .any(|e| lower.ends_with(e))
}

/// The entry of `dir` named `name` ignoring ASCII case: the exact spelling
/// first, then the first other spelling in directory order.
pub fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// `rel`'s components under `root`, each matched without regard to case;
/// a component that does not exist keeps the spelling given.
pub fn resolve_under(root: &Path, rel: &str) -> PathBuf {
    let mut here = root.to_path_buf();
    for part in rel
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
    {
        here = find_ci(&here, part).unwrap_or_else(|| here.join(part));
    }
    here
}

impl Files {
    /// The path relative to the game, with `/`, for a report.
    pub fn shown_path(&self, p: &Path) -> String {
        let rel = p.strip_prefix(&self.root).unwrap_or(p);
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// How a report names a target: `Override/x.2da`, `modules/m.mod:x.utc`.
    pub fn shown(&self, t: &Target) -> String {
        match t {
            Target::File(p) => self.shown_path(p),
            Target::Member { capsule, id } => format!("{}:{id}", self.shown_path(capsule)),
        }
    }

    /// A file of the installer's, in the option's folder or in `tslpatchdata`.
    pub fn data_file(&self, name: &str) -> Option<PathBuf> {
        self.data_dirs
            .iter()
            .filter_map(|d| find_ci(d, name))
            .find(|p| p.is_file())
    }

    /// How a report names an installer's file: relative to `tslpatchdata`'s parent.
    pub fn shown_data(&self, p: &Path) -> String {
        let base = self
            .data_dirs
            .last()
            .and_then(|d| d.parent())
            .unwrap_or(Path::new(""));
        let rel = p.strip_prefix(base).unwrap_or(p);
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// The destination `text` names; an empty one is `override`.
    pub fn destination(&self, text: &str) -> Dest {
        let text = if text.trim().is_empty() {
            "override"
        } else {
            text.trim()
        };
        let last = text
            .rsplit(['\\', '/'])
            .find(|p| !p.is_empty())
            .unwrap_or(text);
        let path = resolve_under(&self.root, text);
        if is_capsule_name(last) {
            Dest::Capsule(path)
        } else {
            Dest::Folder(path)
        }
    }

    /// The target for `name` in `dest`.
    pub fn target(&self, dest: &Dest, name: &str) -> Result<Target, Failure> {
        match dest {
            Dest::Folder(dir) => Ok(Target::File(
                find_ci(dir, name).unwrap_or_else(|| dir.join(name)),
            )),
            Dest::Capsule(capsule) => {
                let id = ResId::parse(name).map_err(|e| {
                    Failure::new(
                        codes::CANNOT_APPLY,
                        format!(
                            "`{name}` cannot be a member of `{}`: {e}",
                            self.shown_path(capsule)
                        ),
                    )
                })?;
                Ok(Target::Member {
                    capsule: capsule.clone(),
                    id,
                })
            }
        }
    }

    /// The bytes the target holds now, if it exists. A member of a `.mod`
    /// not made yet is looked for in the RIMs the `.mod` would be made from.
    pub fn read(&self, t: &Target) -> Result<Option<Vec<u8>>, Failure> {
        match t {
            Target::File(p) => match std::fs::read(p) {
                Ok(b) => Ok(Some(b)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(Failure::new(
                    codes::WRITE_FAILED,
                    format!("cannot read `{}`: {e}", self.shown_path(p)),
                )),
            },
            Target::Member { capsule, id } => {
                let Some(bytes) = self.read_capsule_file(capsule)? else {
                    for (rim, bytes) in self.module_rims(capsule)? {
                        let c = Capsule::parse(&bytes).map_err(|e| self.unreadable(&rim, e))?;
                        if let Some(m) = c.get(*id) {
                            return Ok(Some(m.to_vec()));
                        }
                    }
                    return Ok(None);
                };
                let c = Capsule::parse(&bytes).map_err(|e| self.unreadable(capsule, e))?;
                Ok(c.get(*id).map(<[u8]>::to_vec))
            }
        }
    }

    fn read_capsule_file(&self, capsule: &Path) -> Result<Option<Vec<u8>>, Failure> {
        match std::fs::read(capsule) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Failure::new(
                codes::WRITE_FAILED,
                format!("cannot read `{}`: {e}", self.shown_path(capsule)),
            )),
        }
    }

    fn unreadable(&self, capsule: &Path, e: rakata_archive::Error) -> Failure {
        Failure::new(
            codes::TARGET_UNREADABLE,
            format!(
                "`{}` is not a capsule that reads: {e}",
                self.shown_path(capsule)
            ),
        )
    }

    /// Write `bytes` to the target. A member goes into its capsule with
    /// every other member's bytes kept; a `.mod` that does not exist yet is
    /// made from the module's RIMs first, as a module's `.mod` replaces
    /// them in the engine. Returns the capsule made, if one was.
    pub fn write(&self, t: &Target, bytes: &[u8]) -> Result<Option<String>, Failure> {
        let fail = |p: &Path, e: std::io::Error| {
            Failure::new(
                codes::WRITE_FAILED,
                format!("cannot write `{}`: {e}", self.shown_path(p)),
            )
        };
        match t {
            Target::File(p) => {
                if let Some(dir) = p.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| fail(dir, e))?;
                }
                std::fs::write(p, bytes).map_err(|e| fail(p, e))?;
                Ok(None)
            }
            Target::Member { capsule, id } => {
                let existing = self.read_capsule_file(capsule)?;
                let mut created = None;
                let rims;
                let mut c = match &existing {
                    Some(b) => Capsule::parse(b).map_err(|e| self.unreadable(capsule, e))?,
                    None => {
                        rims = self.module_rims(capsule)?;
                        created = Some(self.shown_path(capsule));
                        Capsule::new_mod(&rims).map_err(|e| self.unreadable(capsule, e))?
                    }
                };
                c.put(*id, bytes.to_vec())
                    .map_err(|e| self.unreadable(capsule, e))?;
                let out = c.to_bytes().map_err(|e| self.unreadable(capsule, e))?;
                if let Some(dir) = capsule.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| fail(dir, e))?;
                }
                std::fs::write(capsule, out).map_err(|e| fail(capsule, e))?;
                Ok(created)
            }
        }
    }

    /// The RIMs a new `STEM.mod` starts from, with their bytes: `STEM.rim`
    /// then `STEM_s.rim` (and KotOR 2's `STEM_dlg.erf`) beside it, those
    /// that exist. Empty for a capsule that is not a `.mod`.
    fn module_rims(&self, capsule: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        let name = capsule
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let Some(stem) = name
            .len()
            .checked_sub(4)
            .filter(|&n| name.to_ascii_lowercase().ends_with(".mod") && n > 0)
            .map(|n| name[..n].to_string())
        else {
            return Ok(Vec::new());
        };
        let dir = capsule.parent().unwrap_or(Path::new("."));
        let mut names = vec![format!("{stem}.rim"), format!("{stem}_s.rim")];
        if self.profile == Profile::K2 {
            names.push(format!("{stem}_dlg.erf"));
        }
        let mut out = Vec::new();
        for n in names {
            if let Some(p) = find_ci(dir, &n)
                && let Some(b) = self.read_capsule_file(&p)?
            {
                out.push((p, b));
            }
        }
        Ok(out)
    }
}

/// A capsule of either family, read or new.
enum Capsule<'a> {
    Erf(Erf<'a>),
    Rim(Rim<'a>),
}

impl<'a> Capsule<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, rakata_archive::Error> {
        if bytes.starts_with(b"RIM ") {
            Rim::parse(bytes).map(Capsule::Rim)
        } else {
            Erf::parse(bytes).map(Capsule::Erf)
        }
    }

    /// A `.mod` holding every member of `rims`, the first copy of a name
    /// winning, as the engine's search over them would.
    fn new_mod(rims: &'a [(PathBuf, Vec<u8>)]) -> Result<Self, rakata_archive::Error> {
        let mut m = Erf::new(ErfKind::Mod);
        for (_, bytes) in rims {
            let members: Vec<(ResId, Vec<u8>)> = match Capsule::parse(bytes)? {
                Capsule::Rim(r) => r
                    .members()
                    .iter()
                    .map(|x| (x.id, x.data().to_vec()))
                    .collect(),
                Capsule::Erf(e) => e
                    .members()
                    .iter()
                    .map(|x| (x.id, x.data().to_vec()))
                    .collect(),
            };
            for (id, data) in members {
                if !m.contains(id) {
                    m.add(id, data)?;
                }
            }
        }
        Ok(Capsule::Erf(m))
    }

    fn get(&self, id: ResId) -> Option<&[u8]> {
        match self {
            Capsule::Erf(e) => e.get(id),
            Capsule::Rim(r) => r.get(id),
        }
    }

    /// Replace the member, or add it when the capsule lacks it.
    fn put(&mut self, id: ResId, data: Vec<u8>) -> Result<(), rakata_archive::Error> {
        match self {
            Capsule::Erf(e) if e.contains(id) => e.replace(id, data),
            Capsule::Erf(e) => e.add(id, data),
            Capsule::Rim(r) if r.contains(id) => r.replace(id, data),
            Capsule::Rim(r) => r.add(id, data),
        }
    }

    fn to_bytes(&self) -> Result<Vec<u8>, rakata_archive::Error> {
        match self {
            Capsule::Erf(e) => e.to_bytes(),
            Capsule::Rim(r) => r.to_bytes(),
        }
    }
}

impl fmt::Display for Dest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dest::Folder(p) | Dest::Capsule(p) => write!(f, "{}", p.display()),
        }
    }
}
