//! Extract the script corpus from a game install, write its manifest, and
//! verify a corpus against a manifest. The corpus is game data and is never
//! committed; the manifest holds only names and hashes.

use crate::archive::{self, NCS, NSS};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str = "\
cargo xtask corpus: the script corpus, extracted from your own game

USAGE:
  cargo xtask corpus extract --game-dir DIR [--out DIR] [--manifest FILE --source TEXT]
  cargo xtask corpus verify [--out DIR] [--manifest FILE]

--out defaults to $NWSCRIPT_CORPUS, then ~/.cache/nwscript/corpus/k1.
--manifest defaults to conformance/k1.toml; module hashes go beside it in
k1-modules.toml. extract writes the manifest only when --manifest is given.

Layout written:
  nss/NAME.nss, ncs/NAME.ncs     every NSS and NCS chitin.key lists
  nwscript.nss                    as shipped
  oracle/                         nss/ with nwscript.nss's line 5145 typo fixed,
                                  the include directory for nwnnsscomp, which
                                  hangs on the shipped file
  modules/STEM/NAME.ncs           NCS from modules/*.rim|*.mod|*.erf
  rims/STEM/NAME.ncs              NCS from rims/*.rim

EXAMPLE:
  cargo xtask corpus extract --game-dir ~/games/swkotor
  NWSCRIPT_CORPUS=~/.cache/nwscript/corpus/k1 cargo xtask corpus verify
";

/// The manifest of the base scripts, committed as `conformance/k1.toml`.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Manifest {
    pub game: String,
    pub source: String,
    pub counts: Counts,
    /// Script name to the sha256 of its shipped NSS and NCS, either may be absent.
    pub scripts: BTreeMap<String, Hashes>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
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

struct Opts {
    game_dir: Option<PathBuf>,
    out: PathBuf,
    manifest: Option<PathBuf>,
    source: String,
}

fn parse(args: &[String]) -> Result<Opts> {
    let mut o = Opts {
        game_dir: None,
        out: default_out(),
        manifest: None,
        source: "unknown".into(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{a} needs a value"))
        };
        match a.as_str() {
            "--game-dir" => o.game_dir = Some(val()?.into()),
            "--out" => o.out = val()?.into(),
            "--manifest" => o.manifest = Some(val()?.into()),
            "--source" => o.source = val()?,
            other => return Err(format!("unknown argument {other}\n\n{USAGE}").into()),
        }
    }
    Ok(o)
}

pub fn default_out() -> PathBuf {
    if let Some(p) = std::env::var_os("NWSCRIPT_CORPUS") {
        return p.into();
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".cache/nwscript/corpus/k1")
}

pub fn main(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("extract") => extract(parse(&args[1..])?),
        Some("verify") => {
            let mut o = parse(&args[1..])?;
            o.manifest
                .get_or_insert_with(|| "conformance/k1.toml".into());
            verify(&o)
        }
        _ => {
            print!("{USAGE}");
            Ok(())
        }
    }
}

fn sha256(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Find a path case-insensitively below `root`, as the game itself does.
fn find_ci(root: &Path, rel: &str) -> Option<PathBuf> {
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

fn ext(restype: u16) -> Option<&'static str> {
    match restype {
        NSS => Some("nss"),
        NCS => Some("ncs"),
        _ => None,
    }
}

/// Files in `dir` whose extension is one of `exts`, sorted by name.
fn listed(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
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

/// A BIF read into memory with its resource table.
struct LoadedBif {
    bytes: Vec<u8>,
    table: Vec<(usize, usize)>,
}

fn write(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, data)?;
    Ok(())
}

fn extract(o: Opts) -> Result<()> {
    let game = o.game_dir.as_deref().ok_or("extract needs --game-dir")?;
    let key_path = find_ci(game, "chitin.key").ok_or("no chitin.key under --game-dir")?;
    let key = archive::read_key(&fs::read(&key_path)?)?;

    // Rewrite from scratch so a rerun is byte-identical and nothing stale survives.
    for sub in ["nss", "ncs", "oracle", "modules", "rims"] {
        let p = o.out.join(sub);
        if p.exists() {
            fs::remove_dir_all(&p)?;
        }
    }

    let mut scripts: BTreeMap<String, Hashes> = BTreeMap::new();
    let mut per_bif: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut bif_cache: BTreeMap<usize, LoadedBif> = BTreeMap::new();
    for e in &key.entries {
        let Some(x) = ext(e.restype) else { continue };
        if let std::collections::btree_map::Entry::Vacant(slot) = bif_cache.entry(e.bif) {
            let rel = key.bifs.get(e.bif).ok_or("key names a missing bif")?;
            let path = find_ci(game, rel).ok_or_else(|| format!("missing {rel}"))?;
            let bytes = fs::read(&path)?;
            let table = archive::read_bif(&bytes)?;
            slot.insert(LoadedBif { bytes, table });
        }
        let LoadedBif { bytes, table } = &bif_cache[&e.bif];
        let &(off, size) = table
            .get(e.index)
            .ok_or_else(|| format!("{}.{x}: index beyond bif table", e.name))?;
        let data = bytes
            .get(off..off + size)
            .ok_or_else(|| format!("{}.{x}: data beyond bif end", e.name))?;
        write(&o.out.join(x).join(format!("{}.{x}", e.name)), data)?;
        let h = scripts.entry(e.name.clone()).or_default();
        let c = per_bif.entry(key.bifs[e.bif].clone()).or_default();
        if e.restype == NSS {
            h.nss = Some(sha256(data));
            c.0 += 1;
        } else {
            h.ncs = Some(sha256(data));
            c.1 += 1;
        }
    }

    let shipped =
        fs::read(o.out.join("nss/nwscript.nss")).map_err(|_| "chitin.key lists no nwscript.nss")?;
    write(&o.out.join("nwscript.nss"), &shipped)?;
    for p in listed(&o.out.join("nss"), &["nss"]) {
        let name = p.file_name().unwrap();
        let data = fs::read(&p)?;
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
            let bytes = fs::read(&p)?;
            let is_rim = bytes.starts_with(b"RIM ");
            let entries = if is_rim {
                archive::read_rim(&bytes)?
            } else {
                archive::read_erf(&bytes)?
            };
            let file = p
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase();
            let stem = p
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase();
            for e in entries.iter().filter(|e| e.restype == NCS) {
                write(
                    &o.out.join(sub).join(&stem).join(format!("{}.ncs", e.name)),
                    e.data,
                )?;
                modules
                    .archives
                    .entry(format!("{sub}/{file}"))
                    .or_default()
                    .insert(e.name.clone(), sha256(e.data));
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
    println!("corpus written to {}", o.out.display());
    for (bif, (nss, ncs)) in &per_bif {
        println!("  {bif}: {nss} nss, {ncs} ncs");
    }
    println!(
        "  total: {} nss, {} ncs, {} pairs, {} module ncs in {} archives",
        counts.nss,
        counts.ncs,
        counts.pairs,
        counts.module_ncs,
        modules.archives.len()
    );

    if let Some(path) = &o.manifest {
        let m = Manifest {
            game: "k1".into(),
            source: o.source.clone(),
            counts,
            scripts,
        };
        write(path, toml::to_string(&m)?.as_bytes())?;
        write(&modules_path(path), toml::to_string(&modules)?.as_bytes())?;
        println!("manifest written to {}", path.display());
    }
    Ok(())
}

fn modules_path(manifest: &Path) -> PathBuf {
    let stem = manifest.file_stem().unwrap_or_default().to_string_lossy();
    manifest.with_file_name(format!("{stem}-modules.toml"))
}

fn verify(o: &Opts) -> Result<()> {
    let path = o.manifest.as_deref().unwrap();
    let m: Manifest = toml::from_str(&fs::read_to_string(path)?)?;
    let modules: ModuleManifest = toml::from_str(&fs::read_to_string(modules_path(path))?)?;
    let mut problems = Vec::new();
    let mut check = |rel: PathBuf, want: &str| match fs::read(o.out.join(&rel)) {
        Ok(b) if sha256(&b) == want => {}
        Ok(_) => problems.push(format!("{}: hash differs", rel.display())),
        Err(_) => problems.push(format!("{}: missing", rel.display())),
    };
    let mut n = 0;
    for (name, h) in &m.scripts {
        if let Some(want) = &h.nss {
            check(PathBuf::from(format!("nss/{name}.nss")), want);
            n += 1;
        }
        if let Some(want) = &h.ncs {
            check(PathBuf::from(format!("ncs/{name}.ncs")), want);
            n += 1;
        }
    }
    for (archive, files) in &modules.archives {
        let (sub, file) = archive.split_once('/').ok_or("bad archive key")?;
        let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
        for (name, want) in files {
            check(PathBuf::from(format!("{sub}/{stem}/{name}.ncs")), want);
            n += 1;
        }
    }
    if problems.is_empty() {
        println!("{n} files match {}", path.display());
        Ok(())
    } else {
        for p in problems.iter().take(20) {
            eprintln!("  {p}");
        }
        Err(format!("{} of {n} files do not match", problems.len()).into())
    }
}
