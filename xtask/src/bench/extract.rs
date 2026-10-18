//! `cargo xtask bench extract`: every vanilla file of each benchmarked
//! format, once per distinct content, as a loose file the comparators can
//! open, and one list per format naming them.
//!
//! The tree's own files (`chitin.key`, the BIFs, the capsules, `Override/`,
//! `dialog.tlk`) are listed where they lie; the members of a BIF, an ERF or
//! a RIM are written under the output directory, named by the KEY or by the
//! capsule. A model's MDL is written beside its MDX under one stem, the
//! pair counted as one file. Two files with the same bytes are listed once.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rakata_archive::{Bif, Erf, Key, Rim, Source};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::formats::{self, BenchFormat};

const USAGE: &str = "\
usage: cargo xtask bench extract --game DIR [--out target/bench/files]

Write every vanilla file of each benchmarked format under OUT/FORMAT/ once
per distinct content, and OUT/FORMAT.list naming them (the tree's own files
by their path in the tree). OUT/manifest.json records the counts.";

/// What an extraction wrote: per format, the files and their bytes.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Manifest {
    /// The tree as given.
    pub tree: String,
    pub formats: BTreeMap<String, Count>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Count {
    /// Distinct files listed.
    pub files: usize,
    /// Their bytes, an MDL's with its MDX.
    pub bytes: u64,
    /// Copies met in the tree that had the same bytes as one listed.
    pub duplicates: usize,
}

pub fn main(args: &[String]) -> Result<(), String> {
    let mut game: Option<PathBuf> = None;
    let mut out = PathBuf::from("target/bench/files");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--game" => game = Some(it.next().ok_or("--game needs DIR")?.into()),
            "--out" => out = it.next().ok_or("--out needs DIR")?.into(),
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        }
    }
    let game = game.ok_or_else(|| format!("--game DIR is required\n{USAGE}"))?;
    let manifest = extract(&game, &out)?;
    for (name, c) in &manifest.formats {
        println!(
            "{name:>9}: {:>6} files, {:>7.1} MB, {} duplicates",
            c.files,
            c.bytes as f64 / 1e6,
            c.duplicates
        );
    }
    println!("lists in {}", out.display());
    Ok(())
}

struct Sink {
    out: PathBuf,
    seen: HashSet<[u8; 32]>,
    lists: BTreeMap<&'static str, Vec<PathBuf>>,
    counts: BTreeMap<&'static str, Count>,
}

impl Sink {
    fn digest(parts: &[&[u8]]) -> [u8; 32] {
        let mut h = Sha256::new();
        for p in parts {
            h.update((p.len() as u64).to_le_bytes());
            h.update(p);
        }
        h.finalize().into()
    }

    /// Record a file of the tree, listed where it lies.
    fn tree_file(&mut self, format: &BenchFormat, path: &Path, bytes: &[u8], mdx: Option<&[u8]>) {
        let d = Self::digest(&[bytes, mdx.unwrap_or_default()]);
        let c = self.counts.entry(format.name).or_default();
        if !self.seen.insert(d) {
            c.duplicates += 1;
            return;
        }
        c.files += 1;
        c.bytes += (bytes.len() + mdx.map_or(0, <[u8]>::len)) as u64;
        self.lists
            .entry(format.name)
            .or_default()
            .push(path.to_path_buf());
    }

    /// Record a member of an archive, written out under its name.
    fn member(
        &mut self,
        format: &BenchFormat,
        name: &str,
        bytes: &[u8],
        mdx: Option<&[u8]>,
    ) -> Result<(), String> {
        let d = Self::digest(&[bytes, mdx.unwrap_or_default()]);
        let c = self.counts.entry(format.name).or_default();
        if !self.seen.insert(d) {
            c.duplicates += 1;
            return Ok(());
        }
        c.files += 1;
        c.bytes += (bytes.len() + mdx.map_or(0, <[u8]>::len)) as u64;
        let dir = self.out.join(format.name);
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let hex: String = d[..6].iter().map(|b| format!("{b:02x}")).collect();
        let safe: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = dir.join(format!("{hex}-{safe}"));
        fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(mdx) = mdx {
            let p = path.with_extension("mdx");
            fs::write(&p, mdx).map_err(|e| format!("{}: {e}", p.display()))?;
        }
        self.lists.entry(format.name).or_default().push(path);
        Ok(())
    }

    /// The members of one container, each by its `name.ext`; an MDL is
    /// paired with the MDX of the same name.
    fn members(&mut self, members: &[(String, &[u8])]) -> Result<(), String> {
        let mdx: HashMap<String, &[u8]> = members
            .iter()
            .filter_map(|(n, b)| {
                let (stem, ext) = n.rsplit_once('.')?;
                ext.eq_ignore_ascii_case("mdx")
                    .then(|| (stem.to_ascii_lowercase(), *b))
            })
            .collect();
        for (name, bytes) in members {
            let Some((stem, ext)) = name.rsplit_once('.') else {
                continue;
            };
            let Some(format) = formats::by_extension(ext) else {
                continue;
            };
            if format.name == "mdl" {
                match mdx.get(&stem.to_ascii_lowercase()) {
                    Some(x) => self.member(format, name, bytes, Some(x))?,
                    None => continue,
                }
            } else {
                self.member(format, name, bytes, None)?;
            }
        }
        Ok(())
    }
}

fn read(path: &Path) -> Result<Source, String> {
    let size = fs::metadata(path).map_or(0, |m| m.len());
    if size >= 1 << 20 {
        Source::map(path)
    } else {
        Source::read(path)
    }
    .map_err(|e| format!("{}: {e}", path.display()))
}

fn extension(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn extract(game: &Path, out: &Path) -> Result<Manifest, String> {
    let files = rakata::conformance::walk(game).map_err(|d| d.message)?;
    if out.exists() {
        fs::remove_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut sink = Sink {
        out: out.to_path_buf(),
        seen: HashSet::new(),
        lists: BTreeMap::new(),
        counts: BTreeMap::new(),
    };

    // The KEY names each BIF's members.
    let key_path = &files[0];
    let key = Key::parse(&read(key_path)?).map_err(|e| e.to_string())?;
    let mut bif_index: HashMap<String, u32> = HashMap::new();
    for (i, b) in key.bifs().iter().enumerate() {
        let name = b.name_str().replace('\\', "/");
        let base = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .to_ascii_lowercase();
        bif_index.insert(base, i as u32);
    }

    for path in &files {
        let ext = extension(path);
        let Some(format) = formats::by_extension(&ext) else {
            continue;
        };
        let bytes = read(path)?;
        match format.name {
            "bif" => {
                sink.tree_file(format, path, &bytes, None);
                let base = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                let Ok(bif) = Bif::parse(&bytes) else {
                    continue;
                };
                let Some(&index) = bif_index.get(&base) else {
                    continue;
                };
                let members: Vec<(String, &[u8])> = key
                    .entries()
                    .iter()
                    .filter(|e| e.bif == index)
                    .filter_map(|e| Some((e.id.to_string(), bif.get(e.index as usize)?)))
                    .collect();
                sink.members(&members)?;
            }
            "erf" => {
                sink.tree_file(format, path, &bytes, None);
                if let Ok(erf) = Erf::parse(&bytes) {
                    let members: Vec<(String, &[u8])> = erf
                        .members()
                        .iter()
                        .map(|m| (m.id.to_string(), m.data()))
                        .collect();
                    sink.members(&members)?;
                }
            }
            "rim" => {
                sink.tree_file(format, path, &bytes, None);
                if let Ok(rim) = Rim::parse(&bytes) {
                    let members: Vec<(String, &[u8])> = rim
                        .members()
                        .iter()
                        .map(|m| (m.id.to_string(), m.data()))
                        .collect();
                    sink.members(&members)?;
                }
            }
            "mdl" => {
                let mdx_path = path.with_extension("mdx");
                let alt = path.with_extension("MDX");
                let mdx = fs::read(&mdx_path).or_else(|_| fs::read(&alt));
                if let Ok(mdx) = mdx {
                    sink.tree_file(format, path, &bytes, Some(&mdx));
                }
            }
            _ => sink.tree_file(format, path, &bytes, None),
        }
    }

    let mut manifest = Manifest {
        tree: game.display().to_string(),
        formats: BTreeMap::new(),
    };
    for f in formats::ALL {
        let list = sink.lists.remove(f.name).unwrap_or_default();
        let text: String = list.iter().map(|p| format!("{}\n", p.display())).collect();
        let lp = out.join(format!("{}.list", f.name));
        fs::write(&lp, text).map_err(|e| format!("{}: {e}", lp.display()))?;
        manifest.formats.insert(
            f.name.to_string(),
            sink.counts.remove(f.name).unwrap_or_default(),
        );
    }
    let mp = out.join("manifest.json");
    fs::write(
        &mp,
        serde_json::to_string_pretty(&manifest).expect("manifest serializes") + "\n",
    )
    .map_err(|e| format!("{}: {e}", mp.display()))?;
    Ok(manifest)
}
