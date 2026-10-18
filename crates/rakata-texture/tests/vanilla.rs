//! The census over the game's own textures: every TPC, TXI and TGA in a
//! KotOR 1 install (the BIFs the KEY names, `TexturePacks/*.erf`,
//! `patch.erf`, the capsules under `modules/` and `rims/`, and loose files
//! in `Override/`) is parsed, and what the layout leaves open is counted:
//! formats, layouts, mip counts, TXI keywords, TGA variants. Run with
//! `--nocapture` to see the counts.
//!
//! Reads the user's own install from `$RAKATA_GAME_K1` (the folder holding
//! `chitin.key`) and skips, naming the variable, when it is unset. No game
//! bytes are committed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rakata_archive::{Bif, Erf, Key, Rim};
use rakata_res::ResType;
use rakata_texture::{Layout, Tga, Tpc, Txi};

const GAME_VAR: &str = "RAKATA_GAME_K1";

fn game() -> Option<PathBuf> {
    match std::env::var(GAME_VAR) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${GAME_VAR} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

/// A child of `dir` named `name` in any case, as the game finds it.
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// Files in `dir` whose extension is one of `exts`, sorted by name.
fn files_with(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
        })
        .collect();
    files.sort();
    files
}

fn file_name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

/// Every TPC, TXI and TGA in the install, by a name saying where it lives.
fn each_texture(root: &Path, mut f: impl FnMut(&str, ResType, &[u8])) {
    let wanted = |ty: ResType| matches!(ty, ResType::Tpc | ResType::Txi | ResType::Tga);
    // The BIFs, through the KEY's names.
    let key_path = child_ci(root, "chitin.key").expect("chitin.key");
    let key = Key::parse(&fs::read(&key_path).unwrap()).unwrap();
    for (b, bif_ref) in key.bifs().iter().enumerate() {
        let mut path = root.to_path_buf();
        for part in bif_ref.components() {
            path = child_ci(&path, &part).unwrap_or_else(|| panic!("{}", bif_ref.name_str()));
        }
        let bytes = fs::read(&path).unwrap();
        let bif = Bif::parse(&bytes).unwrap();
        let bif_name = file_name(&path);
        for e in key.entries().iter().filter(|e| e.bif as usize == b) {
            if wanted(e.id.ty) {
                let data = bif.get(e.index as usize).expect("member the KEY names");
                f(&format!("{bif_name}/{}", e.id), e.id.ty, data);
            }
        }
    }
    // ERF-family capsules and RIMs.
    let mut capsules: Vec<PathBuf> = Vec::new();
    if let Some(dir) = child_ci(root, "TexturePacks") {
        capsules.extend(files_with(&dir, &["erf"]));
    }
    if let Some(p) = child_ci(root, "patch.erf") {
        capsules.push(p);
    }
    for dir in ["modules", "rims"] {
        if let Some(dir) = child_ci(root, dir) {
            capsules.extend(files_with(&dir, &["erf", "mod", "rim"]));
        }
    }
    for path in capsules {
        let bytes = fs::read(&path).unwrap();
        let name = file_name(&path);
        if bytes.starts_with(b"RIM ") {
            for m in Rim::parse(&bytes).unwrap().members() {
                if wanted(m.id.ty) {
                    f(&format!("{name}/{}", m.id), m.id.ty, m.data());
                }
            }
        } else {
            for m in Erf::parse(&bytes).unwrap().members() {
                if wanted(m.id.ty) {
                    f(&format!("{name}/{}", m.id), m.id.ty, m.data());
                }
            }
        }
    }
    // Loose files.
    if let Some(dir) = child_ci(root, "Override") {
        for (ext, ty) in [
            ("tpc", ResType::Tpc),
            ("txi", ResType::Txi),
            ("tga", ResType::Tga),
        ] {
            for path in files_with(&dir, &[ext]) {
                f(
                    &format!("Override/{}", file_name(&path)),
                    ty,
                    &fs::read(&path).unwrap(),
                );
            }
        }
    }
}

#[derive(Default)]
struct Census {
    tpc: usize,
    txi: usize,
    tga: usize,
    failed: Vec<String>,
    by_where: BTreeMap<(String, &'static str), usize>,
    formats: BTreeMap<String, usize>,
    mips: BTreeMap<usize, usize>,
    tpc_with_txi: usize,
    txi_keys: BTreeMap<String, usize>,
    txi_repeated_key: Vec<String>,
    txi_preamble: Vec<String>,
    cube_without_txi_cube: Vec<String>,
    data_size_rule_broken: Vec<String>,
    alpha_float: BTreeMap<String, usize>,
    tga_variants: BTreeMap<String, usize>,
}

impl Census {
    fn txi(&mut self, name: &str, txi: &Txi) {
        let mut seen = std::collections::BTreeSet::new();
        for e in &txi.entries {
            let k = e.key.to_ascii_lowercase();
            *self.txi_keys.entry(k.clone()).or_default() += 1;
            if !seen.insert(k.clone()) {
                self.txi_repeated_key.push(format!("{name}: {k}"));
            }
        }
        if !txi.preamble.is_empty() {
            self.txi_preamble
                .push(format!("{name}: {:?}", txi.preamble.first().unwrap()));
        }
    }

    fn add(&mut self, name: &str, ty: ResType, data: &[u8]) {
        let place = name.split('/').next().unwrap().to_string();
        let kind = match ty {
            ResType::Tpc => "tpc",
            ResType::Txi => "txi",
            _ => "tga",
        };
        *self.by_where.entry((place, kind)).or_default() += 1;
        match ty {
            ResType::Tpc => {
                self.tpc += 1;
                let t = match Tpc::parse(data) {
                    Ok(t) => t,
                    Err(e) => return self.failed.push(format!("{name}: {e}")),
                };
                let layout = match t.layout {
                    Layout::Single => "single".to_string(),
                    Layout::Cube => "cube".to_string(),
                    Layout::Flipbook { numx, numy } => format!("flipbook {numx}x{numy}"),
                };
                let layout = layout.split(' ').next().unwrap().to_string();
                *self
                    .formats
                    .entry(format!("{:?} {layout}", t.format))
                    .or_default() += 1;
                *self.mips.entry(t.level_count()).or_default() += 1;
                // Level 0's length, at least 16, unless a flipbook.
                if t.format.is_compressed() && !matches!(t.layout, Layout::Flipbook { .. }) {
                    let l0 = t.images[0].levels[0].data.len();
                    if t.data_size as usize != l0.max(16) {
                        self.data_size_rule_broken
                            .push(format!("{name}: {} for level 0 of {l0}", t.data_size));
                    }
                }
                let af = if !t.has_alpha() {
                    if t.alpha_mean == 1.0 {
                        "no alpha: 1.0"
                    } else {
                        "no alpha: other"
                    }
                } else if (0.0..=1.0).contains(&t.alpha_mean) {
                    "alpha: in 0..=1"
                } else {
                    "alpha: outside 0..=1"
                };
                *self.alpha_float.entry(af.to_string()).or_default() += 1;
                if let Some(txi) = &t.txi {
                    self.tpc_with_txi += 1;
                    self.txi(name, txi);
                }
                let says_cube = t.txi.as_ref().and_then(|x| x.int("cube")) == Some(1);
                if says_cube != (t.layout == Layout::Cube) {
                    self.cube_without_txi_cube.push(name.to_string());
                }
            }
            ResType::Txi => {
                self.txi += 1;
                self.txi(name, &Txi::parse(data));
            }
            _ => {
                self.tga += 1;
                match Tga::parse(data) {
                    Ok(t) => {
                        *self
                            .tga_variants
                            .entry(format!(
                                "{:?} {}-bit, {} alpha bits, {:?}",
                                t.image_type, t.bits, t.alpha_bits, t.origin
                            ))
                            .or_default() += 1;
                    }
                    Err(e) => self.failed.push(format!("{name}: {e}")),
                }
            }
        }
    }
}

fn show(label: &str, v: &[String]) {
    println!("{label}: {}", v.len());
    for s in v.iter().take(8) {
        println!("    {s}");
    }
    if v.len() > 8 {
        println!("    ... and {} more", v.len() - 8);
    }
}

#[test]
fn every_vanilla_texture_parses() {
    let Some(root) = game() else { return };
    let mut c = Census::default();
    each_texture(&root, |name, ty, data| c.add(name, ty, data));

    println!("{} TPC, {} TXI, {} TGA", c.tpc, c.txi, c.tga);
    println!("by location:");
    for ((place, kind), n) in &c.by_where {
        println!("    {kind} {place}: {n}");
    }
    println!("TPC format and layout:");
    for (k, n) in &c.formats {
        println!("    {k}: {n}");
    }
    println!("TPC levels per image:");
    for (k, n) in &c.mips {
        println!("    {k}: {n}");
    }
    println!("TPC with TXI text: {}", c.tpc_with_txi);
    println!("TPC header float:");
    for (k, n) in &c.alpha_float {
        println!("    {k}: {n}");
    }
    println!("TGA variants:");
    for (k, n) in &c.tga_variants {
        println!("    {k}: {n}");
    }
    println!("TXI keywords:");
    for (k, n) in &c.txi_keys {
        println!("    {k}: {n}");
    }
    show("TXI with a keyword repeated", &c.txi_repeated_key);
    show("TXI with numeric lines before any keyword", &c.txi_preamble);
    show(
        "TPC whose cube layout and TXI `cube 1` disagree",
        &c.cube_without_txi_cube,
    );
    show(
        "compressed TPC whose data size is not max(level 0, 16)",
        &c.data_size_rule_broken,
    );
    show("failed", &c.failed);

    assert!(c.tpc > 0 && c.txi > 0 && c.tga > 0, "no textures found");
    assert!(c.failed.is_empty(), "files refused: {:#?}", c.failed);
    assert!(c.cube_without_txi_cube.is_empty());
    assert!(c.data_size_rule_broken.is_empty());
}
