//! Every vanilla script compiled and compared with BioWare's bytecode.

use nwscript_codegen::{Profile, generate};
use nwscript_diag::{FileId, SourceMap};
use nwscript_ncs::asm::{PrintOptions, print};
use nwscript_sema::{EngineTable, Includes, analyze};
use nwscript_syntax::{Parse, parse};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

fn corpus() -> Option<PathBuf> {
    let dir = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache/nwscript/corpus/k1"),
    };
    dir.join("nwscript.nss").is_file().then_some(dir)
}

struct Dir<'a> {
    root: PathBuf,
    map: &'a mut SourceMap,
    cache: HashMap<String, Parse>,
}

impl Includes for Dir<'_> {
    fn include(&mut self, name: &str) -> Result<Option<(String, Parse)>, String> {
        let key = name.to_ascii_lowercase();
        let shown = format!("{key}.nss");
        if let Some(p) = self.cache.get(&key) {
            return Ok(Some((shown, p.clone())));
        }
        let Ok(bytes) = std::fs::read(self.root.join(&shown)) else {
            return Ok(None);
        };
        let file = self.map.add(shown.clone(), bytes.clone());
        let p = parse(file, &bytes);
        self.cache.insert(key, p.clone());
        Ok(Some((shown, p)))
    }
}

fn listing(p: &nwscript_ncs::Program) -> Vec<String> {
    print(
        p,
        &PrintOptions {
            raw: true,
            routine_name: None,
        },
    )
    .lines()
    .map(str::to_string)
    .collect()
}

#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn k1_identical() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let mut map = SourceMap::new();
    let ns = std::fs::read(root.join("nwscript.nss")).unwrap();
    let nsf = map.add("nwscript.nss", ns.clone());
    let (table, _) = EngineTable::from_nwscript(nsf, &ns);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root.join("nss"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap() != "nwscript.nss")
        .collect();
    paths.sort();
    let mut dir = Dir {
        root: root.join("nss"),
        map: &mut map,
        cache: HashMap::new(),
    };
    let mut classes: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut deltas: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    let show = std::env::var("SHOW").ok();
    let out_dir = std::env::var_os("DIFF_DIR").map(PathBuf::from);
    for path in &paths {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let Ok(want) = std::fs::read(root.join("ncs").join(format!("{stem}.ncs"))) else {
            continue;
        };
        let bytes = std::fs::read(path).unwrap();
        let shown = format!("{stem}.nss");
        let file = dir.map.add(shown.clone(), bytes.clone());
        let a = analyze(&shown, parse(file, &bytes), &table, &mut dir);
        if a.program.entry.is_none() || a.has_errors() {
            classes.entry("failed").or_default().push(stem);
            continue;
        }
        let got = match generate(&a.program, Profile::K1) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{stem}: {}", e.message);
                classes.entry("failed").or_default().push(stem);
                continue;
            }
        };
        let got_bytes = nwscript_ncs::encode(&got);
        if got_bytes == want {
            classes.entry("identical").or_default().push(stem);
            continue;
        }
        let class = if got_bytes.len() == want.len() {
            "samelen"
        } else {
            "difflen"
        };
        deltas
            .entry(got_bytes.len() as i64 - want.len() as i64)
            .or_default()
            .push(stem.clone());
        classes.entry(class).or_default().push(stem.clone());
        let theirs = listing(&nwscript_ncs::decode(FileId(9999), &want).unwrap());
        let ours = listing(&got);
        if let Some(d) = &out_dir {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join(format!("{stem}.ours")), ours.join("\n")).unwrap();
            std::fs::write(d.join(format!("{stem}.theirs")), theirs.join("\n")).unwrap();
        }
        if show.as_deref() == Some(&stem) {
            let i = ours
                .iter()
                .zip(&theirs)
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            println!("{stem}: first difference at line {i}");
            for j in i.saturating_sub(8)..(i + 12).min(ours.len().max(theirs.len())) {
                println!(
                    "{:50} | {}",
                    ours.get(j).map_or("", |s| s),
                    theirs.get(j).map_or("", |s| s)
                );
            }
        }
    }
    for (c, v) in &classes {
        println!("{c}: {}", v.len());
    }
    let mut ds: Vec<(&i64, &Vec<String>)> = deltas.iter().collect();
    ds.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
    for (d, v) in ds.iter().take(25) {
        println!(
            "{d:+}: {} e.g. {}",
            v.len(),
            v.iter().take(4).cloned().collect::<Vec<_>>().join(" ")
        );
    }
    if let Some(v) = classes.get("samelen") {
        println!(
            "samelen: {}",
            v.iter().take(20).cloned().collect::<Vec<_>>().join(" ")
        );
    }
    let identical = classes.get("identical").map_or(0, Vec::len);
    assert!(
        identical >= 1599,
        "{identical} identical, fewer than before"
    );
}
