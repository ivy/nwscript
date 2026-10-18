//! The script corpus: every script of an install extracted from its
//! archives ([`extract`]), checked against the committed manifest
//! ([`verify`]), and every script that ships only as bytecode recovered to
//! source with its origin, the copy the engine loads, folded includes and a
//! call index ([`decompile`]).
//!
//! The corpus is game data and lives under the user's cache, `$NWSCRIPT_CORPUS`
//! or `~/.cache/nwscript/corpus/k1` ([`default_out`]); the manifests
//! `conformance/k1.toml` and `conformance/k1-modules.toml` hold names and
//! hashes only. Both are written from scratch on every run so a rerun is
//! byte-identical and nothing stale survives.

mod decompile;

pub use decompile::{Options as DecompileOptions, Report as DecompileReport, Tally, decompile};

use rakata_archive::{Bif, Erf, Key, KeyEntry, Member, Rim, Source};
use rakata_res::ResType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Why the corpus could not be extracted, verified or decompiled. Each
/// variant has a registered code ([`Error::code`]) and a message.
#[derive(Debug)]
pub enum Error {
    /// A file of the install or the corpus could not be read.
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A file of the corpus could not be written.
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    /// An archive of the install did not parse.
    Archive {
        path: PathBuf,
        source: rakata_archive::Error,
    },
    /// The install could not be opened or a resource of it read.
    Game(rakata_game::Error),
    /// `--game` holds no `chitin.key`.
    NotAnInstall { dir: PathBuf },
    /// `chitin.key` names a BIF the install lacks.
    MissingBif { name: String },
    /// `chitin.key` lists no `nwscript.nss`.
    NoNwscript { dir: PathBuf },
    /// A manifest did not read or parse.
    Manifest { path: PathBuf, message: String },
}

impl Error {
    /// The registered diagnostic code.
    pub fn code(&self) -> &'static str {
        match self {
            Error::Read { .. } => "RE0001",
            Error::Write { .. } => "RE1001",
            Error::Archive { source, .. } => source.code(),
            Error::Game(e) => e.code(),
            Error::NotAnInstall { .. } => "RE0401",
            Error::MissingBif { .. } => "RE0402",
            Error::NoNwscript { .. } => "RE1002",
            Error::Manifest { .. } => "RE1003",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Read { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Error::Write { path, source } => {
                write!(f, "cannot write {}: {source}", path.display())
            }
            Error::Archive { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Game(e) => write!(f, "{}", e.message()),
            Error::NotAnInstall { dir } => write!(
                f,
                "`{}` is not a game install: it holds no chitin.key",
                dir.display()
            ),
            Error::MissingBif { name } => {
                write!(f, "chitin.key names `{name}`, which the install lacks")
            }
            Error::NoNwscript { dir } => write!(
                f,
                "`{}` has no nwscript.nss to read the engine's declarations from",
                dir.display()
            ),
            Error::Manifest { path, message } => write!(f, "{}: {message}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

impl From<rakata_game::Error> for Error {
    fn from(e: rakata_game::Error) -> Self {
        Error::Game(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// The manifest of the base scripts, committed as `conformance/k1.toml`.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Manifest {
    pub game: String,
    pub source: String,
    pub counts: Counts,
    /// Script name to the sha256 of its shipped NSS and NCS, either may be absent.
    pub scripts: BTreeMap<String, Hashes>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Counts {
    pub nss: usize,
    pub ncs: usize,
    pub pairs: usize,
    pub module_ncs: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct Hashes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncs: Option<String>,
}

/// Module NCS hashes, `conformance/k1-modules.toml`: archive path to name to sha256.
#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct ModuleManifest {
    pub archives: BTreeMap<String, BTreeMap<String, String>>,
}

/// What [`extract`] takes.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// The install: the folder holding `chitin.key`.
    pub game: PathBuf,
    /// Where the corpus is written; see [`default_out`].
    pub out: PathBuf,
    /// Write the manifests here (`FILE` and `FILE-modules.toml`); none by default.
    pub manifest: Option<PathBuf>,
    /// The manifest's `source` field: where the game came from.
    pub source: String,
}

/// What [`extract`] wrote.
#[derive(Debug, Clone, Serialize)]
pub struct Extracted {
    pub out: PathBuf,
    /// Each BIF's NSS and NCS counts, by its name in the KEY.
    pub per_bif: BTreeMap<String, (usize, usize)>,
    pub counts: Counts,
    /// How many module and global capsules held NCS.
    pub archives: usize,
    /// The manifest written, if one was asked for.
    pub manifest: Option<PathBuf>,
}

/// The corpus directory: `$NWSCRIPT_CORPUS`, else `~/.cache/nwscript/corpus/k1`.
pub fn default_out() -> PathBuf {
    if let Some(p) = std::env::var_os("NWSCRIPT_CORPUS") {
        return p.into();
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".cache/nwscript/corpus/k1")
}

fn sha256(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Find a path case-insensitively below `root`, as the game itself does.
pub(crate) fn find_ci(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let hit = fs::read_dir(&cur)
            .ok()?
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = hit.path();
    }
    Some(cur)
}

/// The extension of the two types the corpus keeps; `None` for the rest.
fn ext(ty: ResType) -> Option<&'static str> {
    match ty {
        ResType::Nss => Some("nss"),
        ResType::Ncs => Some("ncs"),
        _ => None,
    }
}

/// A resource name as the corpus files it: lowercase, as the engine compares
/// it, whatever the archive's spelling.
fn lower(m: &rakata_res::ResRef) -> String {
    m.to_string().to_ascii_lowercase()
}

/// Files in `dir` whose extension is one of `exts`, sorted by name.
pub(crate) fn listed(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Correct BioWare's typo at line 5145 of K1's nwscript.nss, which makes
/// nwnnsscomp hang: `INT` and `OBJECT_ID` where `int` and `object` belong.
fn fix_nwscript(src: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(src);
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if i + 1 == 5145 && line.contains("SetAvailableNPCId") {
            out.push_str(
                &line
                    .replace("INT ", "int ")
                    .replace("OBJECT_ID ", "object "),
            );
        } else {
            out.push_str(line);
        }
    }
    out.into_bytes()
}

pub(crate) fn write(path: &Path, data: &[u8]) -> Result<()> {
    let fail = |source| Error::Write {
        path: path.to_path_buf(),
        source,
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(fail)?;
    }
    fs::write(path, data).map_err(fail)
}

fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn map(path: &Path) -> Result<Source> {
    Source::map(path).map_err(|source| Error::Archive {
        path: path.to_path_buf(),
        source,
    })
}

/// Remove `dir` and everything under it, if it exists.
pub(crate) fn clear(dir: &Path) -> Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|source| Error::Write {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

/// Write the scripts of the install at `o.game` to `o.out`: `nss/` and
/// `ncs/` (every script `chitin.key` lists), `nwscript.nss` as shipped,
/// `oracle/` (`nss/` with the line-5145 typo corrected, the include
/// directory for nwnnsscomp), and `modules/STEM/` and `rims/STEM/` (each
/// capsule's NCS). With `o.manifest`, the manifests too.
pub fn extract(o: &ExtractOptions) -> Result<Extracted> {
    let game = o.game.as_path();
    let key_path = find_ci(game, "chitin.key").ok_or_else(|| Error::NotAnInstall {
        dir: game.to_path_buf(),
    })?;
    let key = Key::parse(&read(&key_path)?).map_err(|source| Error::Archive {
        path: key_path.clone(),
        source,
    })?;

    for sub in ["nss", "ncs", "oracle", "modules", "rims"] {
        clear(&o.out.join(sub))?;
    }

    let mut scripts: BTreeMap<String, Hashes> = BTreeMap::new();
    let mut per_bif: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    // One pass per BIF: each is mapped once and its scripts copied out.
    let mut by_bif: BTreeMap<u32, Vec<&KeyEntry>> = BTreeMap::new();
    for e in key.entries().iter().filter(|e| ext(e.id.ty).is_some()) {
        by_bif.entry(e.bif).or_default().push(e);
    }
    for (bif, entries) in &by_bif {
        let rel = key
            .bifs()
            .get(*bif as usize)
            .map(|b| b.name_str())
            .unwrap_or_else(|| format!("BIF #{bif}"));
        let path = find_ci(game, &rel).ok_or_else(|| Error::MissingBif { name: rel.clone() })?;
        let source = map(&path)?;
        let archive = Bif::parse(&source).map_err(|source| Error::Archive {
            path: path.clone(),
            source,
        })?;
        for e in entries {
            let x = ext(e.id.ty).expect("filtered to nss and ncs");
            let name = lower(&e.id.name);
            let data = archive
                .get(e.index as usize)
                .ok_or_else(|| Error::Archive {
                    path: path.clone(),
                    source: rakata_archive::Error::NoSuchIndex {
                        index: e.index as usize,
                        len: archive.members().len(),
                    },
                })?;
            write(&o.out.join(x).join(format!("{name}.{x}")), data)?;
            let h = scripts.entry(name).or_default();
            let c = per_bif.entry(rel.clone()).or_default();
            if e.id.ty == ResType::Nss {
                h.nss = Some(sha256(data));
                c.0 += 1;
            } else {
                h.ncs = Some(sha256(data));
                c.1 += 1;
            }
        }
    }

    let shipped = o.out.join("nss/nwscript.nss");
    let shipped = fs::read(&shipped).map_err(|_| Error::NoNwscript {
        dir: game.to_path_buf(),
    })?;
    write(&o.out.join("nwscript.nss"), &shipped)?;
    for p in listed(&o.out.join("nss"), &["nss"]) {
        let name = p.file_name().unwrap_or_default();
        let data = read(&p)?;
        let data = if name == "nwscript.nss" {
            fix_nwscript(&data)
        } else {
            data
        };
        write(&o.out.join("oracle").join(name), &data)?;
    }

    let mut modules = ModuleManifest::default();
    for (dir, sub) in [("modules", "modules"), ("rims", "rims")] {
        let Some(d) = find_ci(game, dir) else {
            continue;
        };
        for p in listed(&d, &["rim", "mod", "erf"]) {
            let source = map(&p)?;
            let rim;
            let erf;
            let archive_err = |source| Error::Archive {
                path: p.clone(),
                source,
            };
            let entries: &[Member] = if source.starts_with(b"RIM ") {
                rim = Rim::parse(&source).map_err(archive_err)?;
                rim.members()
            } else {
                erf = Erf::parse(&source).map_err(archive_err)?;
                erf.members()
            };
            let file = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase();
            let stem = p
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase();
            for e in entries.iter().filter(|e| e.id.ty == ResType::Ncs) {
                let name = lower(&e.id.name);
                write(
                    &o.out.join(sub).join(&stem).join(format!("{name}.ncs")),
                    e.data(),
                )?;
                modules
                    .archives
                    .entry(format!("{sub}/{file}"))
                    .or_default()
                    .insert(name, sha256(e.data()));
            }
        }
    }

    let counts = Counts {
        nss: scripts.values().filter(|h| h.nss.is_some()).count(),
        ncs: scripts.values().filter(|h| h.ncs.is_some()).count(),
        pairs: scripts
            .values()
            .filter(|h| h.nss.is_some() && h.ncs.is_some())
            .count(),
        module_ncs: modules.archives.values().map(BTreeMap::len).sum(),
    };
    let archives = modules.archives.len();

    if let Some(path) = &o.manifest {
        let m = Manifest {
            game: "k1".into(),
            source: o.source.clone(),
            counts: counts.clone(),
            scripts,
        };
        let text = toml::to_string(&m).map_err(|e| Error::Manifest {
            path: path.clone(),
            message: e.to_string(),
        })?;
        write(path, text.as_bytes())?;
        let text = toml::to_string(&modules).map_err(|e| Error::Manifest {
            path: path.clone(),
            message: e.to_string(),
        })?;
        write(&modules_path(path), text.as_bytes())?;
    }
    Ok(Extracted {
        out: o.out.clone(),
        per_bif,
        counts,
        archives,
        manifest: o.manifest.clone(),
    })
}

/// `FILE-modules.toml` beside the manifest `FILE.toml`.
pub fn modules_path(manifest: &Path) -> PathBuf {
    let stem = manifest.file_stem().unwrap_or_default().to_string_lossy();
    manifest.with_file_name(format!("{stem}-modules.toml"))
}

/// What [`verify`] found: how many files the manifests name, and each one
/// that is missing or differs.
#[derive(Debug, Clone, Serialize)]
pub struct Verified {
    pub files: usize,
    pub problems: Vec<String>,
}

impl Verified {
    pub fn is_ok(&self) -> bool {
        self.problems.is_empty()
    }
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).map_err(|e| Error::Manifest {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    toml::from_str(&text).map_err(|e| Error::Manifest {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

/// Check the corpus at `out` against the manifest and its modules file.
pub fn verify(out: &Path, manifest: &Path) -> Result<Verified> {
    let m: Manifest = read_toml(manifest)?;
    let modules: ModuleManifest = read_toml(&modules_path(manifest))?;
    let mut problems = Vec::new();
    let mut n = 0;
    let mut check = |rel: String, want: &str| {
        n += 1;
        match fs::read(out.join(&rel)) {
            Ok(b) if sha256(&b) == want => {}
            Ok(_) => problems.push(format!("{rel}: hash differs")),
            Err(_) => problems.push(format!("{rel}: missing")),
        }
    };
    for (name, h) in &m.scripts {
        if let Some(want) = &h.nss {
            check(format!("nss/{name}.nss"), want);
        }
        if let Some(want) = &h.ncs {
            check(format!("ncs/{name}.ncs"), want);
        }
    }
    for (archive, files) in &modules.archives {
        let Some((sub, file)) = archive.split_once('/') else {
            return Err(Error::Manifest {
                path: modules_path(manifest),
                message: format!("archive key `{archive}` is not `dir/file`"),
            });
        };
        let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
        for (name, want) in files {
            check(format!("{sub}/{stem}/{name}.ncs"), want);
        }
    }
    Ok(Verified { files: n, problems })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_typo_fix_touches_only_line_5145() {
        let mut lines: Vec<String> = (1..=5146).map(|i| format!("// {i}\n")).collect();
        lines[5144] = "void SetAvailableNPCId(INT nNPC, OBJECT_ID oidNPC);\n".into();
        let fixed = fix_nwscript(lines.concat().as_bytes());
        let text = String::from_utf8(fixed).unwrap();
        assert!(text.contains("void SetAvailableNPCId(int nNPC, object oidNPC);"));
        assert_eq!(text.lines().count(), 5146);
    }

    #[test]
    fn modules_manifest_sits_beside_the_manifest() {
        assert_eq!(
            modules_path(Path::new("conformance/k1.toml")),
            PathBuf::from("conformance/k1-modules.toml")
        );
    }

    #[test]
    fn verify_reports_missing_and_differing_files() {
        let dir = std::env::temp_dir().join(format!("rakata-corpus-verify-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("nss")).unwrap();
        fs::write(dir.join("nss/a.nss"), b"a").unwrap();
        fs::write(dir.join("nss/b.nss"), b"not b").unwrap();
        let manifest = dir.join("m.toml");
        let mut scripts = BTreeMap::new();
        for (name, bytes) in [("a", &b"a"[..]), ("b", &b"b"[..]), ("c", &b"c"[..])] {
            scripts.insert(
                name.to_string(),
                Hashes {
                    nss: Some(sha256(bytes)),
                    ncs: None,
                },
            );
        }
        let m = Manifest {
            game: "k1".into(),
            source: "test".into(),
            counts: Counts {
                nss: 3,
                ncs: 0,
                pairs: 0,
                module_ncs: 0,
            },
            scripts,
        };
        fs::write(&manifest, toml::to_string(&m).unwrap()).unwrap();
        fs::write(
            modules_path(&manifest),
            toml::to_string(&ModuleManifest::default()).unwrap(),
        )
        .unwrap();
        let v = verify(&dir, &manifest).unwrap();
        assert_eq!(v.files, 3);
        assert_eq!(
            v.problems,
            ["nss/b.nss: hash differs", "nss/c.nss: missing"]
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
