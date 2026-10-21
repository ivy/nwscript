//! The census over the game's own textures: every TPC, TXI and TGA in a
//! KotOR 1 install (the BIFs the KEY names, `TexturePacks/*.erf`,
//! `patch.erf`, the capsules under `modules/` and `rims/`, and loose files
//! in `Override/`) is parsed, and what the layout leaves open is counted:
//! formats, layouts, mip counts, TXI keywords, TGA variants. Each is also
//! written again and must be the bytes it was read from. Run with
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
use rakata_texture::{Layout, Levels, PixelFormat, Tga, Tpc, Txi};

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
    rewritten: Vec<String>,
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

    /// Note a file the writer does not give back byte for byte.
    fn rewrite(&mut self, name: &str, written: rakata_texture::Result<Vec<u8>>, data: &[u8]) {
        match written {
            Ok(w) if w == data => {}
            Ok(w) => self.rewritten.push(format!(
                "{name}: {} bytes written for {}",
                w.len(),
                data.len()
            )),
            Err(e) => self.rewritten.push(format!("{name}: {e}")),
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
                self.rewrite(name, t.to_bytes(), data);
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
                let t = Txi::parse(data);
                self.rewrite(name, t.to_bytes(), data);
                self.txi(name, &t);
            }
            _ => {
                self.tga += 1;
                match Tga::parse(data) {
                    Ok(t) => {
                        self.rewrite(name, t.to_bytes(), data);
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
    show("written back other than read", &c.rewritten);

    assert!(c.tpc > 0 && c.txi > 0 && c.tga > 0, "no textures found");
    assert!(c.failed.is_empty(), "files refused: {:#?}", c.failed);
    assert!(
        c.rewritten.is_empty(),
        "not written back: {:#?}",
        c.rewritten
    );
    assert!(c.cube_without_txi_cube.is_empty());
    assert!(c.data_size_rule_broken.is_empty());
}

/// The size a PNG's header states.
fn png_size(bytes: &[u8]) -> (u32, u32) {
    let reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let info = reader.info();
    (info.width, info.height)
}

/// Every TPC and TGA exports as a PNG of the size its header states (a
/// cube's faces stacked, a sheet's frames laid out), and every lower level
/// of a TPC decodes. A DXT5 file's header float is compared with the mean
/// of its decoded level-0 alpha over 255, as `docs/formats/tpc.md` finds
/// it for uncompressed RGBA. Ignored by default, since it decodes and
/// encodes every texture of the install: run it in release with
/// `RAKATA_GAME_K1=… cargo test --release -p rakata-texture --test vanilla
/// -- --ignored --nocapture` to see the counts.
#[test]
#[ignore = "decodes every texture of $RAKATA_GAME_K1; run in release"]
fn every_vanilla_texture_exports_as_png() {
    let Some(root) = game() else { return };
    let (mut tpc, mut packs, mut tga) = (0usize, 0usize, 0usize);
    // How far the header float lies from the decoded mean alpha.
    const BUCKETS: [f64; 4] = [1e-3, 1e-2, 5e-2, 2e-1];
    let mut dxt5 = 0usize;
    let mut dxt5_off = [0usize; BUCKETS.len() + 1];
    let mut failed = Vec::new();
    each_texture(&root, |name, ty, data| match ty {
        ResType::Tpc => {
            tpc += 1;
            if name.to_ascii_lowercase().starts_with("swpc_tex_") {
                packs += 1;
            }
            let t = Tpc::parse(data).unwrap();
            let png = match t.to_pngs(Levels::Largest, None) {
                Ok(p) => p,
                Err(e) => return failed.push(format!("{name}: {e}")),
            };
            if t.format == PixelFormat::Dxt5 && t.layout == Layout::Single {
                dxt5 += 1;
                let px = t.decode_level(0).unwrap().pixels;
                let sum: u64 = px.chunks(4).map(|p| u64::from(p[3])).sum();
                let mean = sum as f64 / (px.len() / 4) as f64 / 255.0;
                let off = (mean - f64::from(t.alpha_mean)).abs();
                let bucket = BUCKETS
                    .iter()
                    .position(|&b| off < b)
                    .unwrap_or(BUCKETS.len());
                dxt5_off[bucket] += 1;
            }
            let want = (u32::from(t.width), u32::from(t.height));
            let got = png_size(&png[0].bytes);
            if got != want {
                failed.push(format!("{name}: {got:?} for {want:?}"));
            }
            for level in 1..t.level_count() {
                if let Err(e) = t.decode_level(level) {
                    failed.push(format!("{name} level {level}: {e}"));
                }
            }
        }
        ResType::Tga => {
            tga += 1;
            let t = Tga::parse(data).unwrap();
            let want = (u32::from(t.width), u32::from(t.height));
            match t.to_png() {
                Ok(p) if png_size(&p) == want => {}
                Ok(p) => failed.push(format!("{name}: {:?} for {want:?}", png_size(&p))),
                Err(e) => failed.push(format!("{name}: {e}")),
            }
        }
        _ => {}
    });
    println!("exported {tpc} TPC ({packs} in the texture packs) and {tga} TGA");
    println!(
        "single-image DXT5, {dxt5}, by how far the header float lies from level 0's decoded mean alpha:"
    );
    for (i, n) in dxt5_off.iter().enumerate() {
        match BUCKETS.get(i) {
            Some(b) => println!("    under {b}: {n}"),
            None => println!("    more: {n}"),
        }
    }
    show("refused", &failed);
    assert!(packs > 0, "no texture packs found");
    assert!(failed.is_empty(), "refused: {failed:#?}");
    assert_eq!(
        dxt5_off[2..].iter().sum::<usize>(),
        0,
        "a DXT5 header float 0.01 or more from its decoded mean alpha"
    );
}
