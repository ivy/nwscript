//! Every vanilla script compiled and compared with BioWare's bytecode.

use nwscript_codegen::Profile;
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

/// Where two programs first differ, ignoring jump distances: the index of
/// the instruction, or `None` when one is a prefix of the other.
fn first_difference(a: &nwscript_ncs::Program, b: &nwscript_ncs::Program) -> usize {
    use nwscript_ncs::Operands;
    let same = |x: &nwscript_ncs::Instruction, y: &nwscript_ncs::Instruction| {
        x.opcode == y.opcode
            && x.ty == y.ty
            && (x.operands == y.operands
                || matches!(
                    (&x.operands, &y.operands),
                    (Operands::Jump(_), Operands::Jump(_))
                ))
    };
    a.instructions()
        .iter()
        .zip(b.instructions())
        .position(|(x, y)| !same(x, y))
        .unwrap_or(a.len().min(b.len()))
}

struct Run {
    table: EngineTable,
    name: &'static str,
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
    let shipped = EngineTable::from_nwscript(nsf, &ns).0;
    let mut bioware = shipped.clone();
    let o = include_str!("../../nwscript/src/overlays/k1-bioware.nss");
    let of = map.add("k1-bioware.nss", o.as_bytes().to_vec());
    assert!(bioware.overlay(of, o.as_bytes()).is_empty());
    let runs = [
        Run {
            table: bioware,
            name: "bioware",
        },
        Run {
            table: shipped,
            name: "shipped",
        },
    ];
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
    let out_dir = std::env::var_os("DIFF_DIR").map(PathBuf::from);
    // Each script's outcome under each set of declarations.
    let mut outcome: BTreeMap<String, Vec<Outcome>> = BTreeMap::new();
    for path in &paths {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let Ok(want) = std::fs::read(root.join("ncs").join(format!("{stem}.ncs"))) else {
            continue;
        };
        let theirs = nwscript_ncs::decode(FileId(u32::MAX), &want).unwrap();
        let bytes = std::fs::read(path).unwrap();
        let shown = format!("{stem}.nss");
        let file = dir.map.add(shown.clone(), bytes.clone());
        let main = parse(file, &bytes);
        let mut results = Vec::new();
        for run in &runs {
            let a = analyze(&shown, main.clone(), &run.table, &mut dir);
            if a.program.entry.is_none() || a.has_errors() {
                results.push(Outcome::Failed);
                continue;
            }
            let (got, layout) =
                nwscript_codegen::generate_with_layout(&a.program, Profile::K1).unwrap();
            if nwscript_ncs::encode(&got) == want {
                results.push(Outcome::Identical);
                continue;
            }
            let i = first_difference(&got, &theirs);
            let at = got.offsets().get(i).copied().unwrap_or(u32::MAX);
            let file_name = |f: FileId| -> String {
                a.program
                    .files
                    .iter()
                    .find(|u| u.parse.file() == f)
                    .map_or("?".into(), |u| u.name.clone())
            };
            let place = match layout.iter().rev().find(|(_, o)| *o <= at) {
                Some((f, _)) => {
                    let f = a.program.function(*f);
                    (file_name(f.file), format!("`{}`", f.name))
                }
                None => {
                    // Each global's initialisation starts with one RSADD per
                    // four bytes of it.
                    let reserved = got.instructions()[..i]
                        .iter()
                        .filter(|x| x.opcode == nwscript_ncs::Opcode::RsAdd)
                        .count() as u32;
                    let mut slots = 0;
                    let g = a.program.globals.iter().find(|g| {
                        slots += a.program.size_of(&g.ty) / 4;
                        slots >= reserved
                    });
                    match g {
                        Some(g) => (file_name(g.span.file), format!("global `{}`", g.name)),
                        None => (shown.clone(), "the globals routine".into()),
                    }
                }
            };
            let delta = nwscript_ncs::encode(&got).len() as i64 - want.len() as i64;
            if let Some(d) = &out_dir {
                std::fs::create_dir_all(d).unwrap();
                std::fs::write(
                    d.join(format!("{stem}.{}.ours", run.name)),
                    listing(&got).join("\n"),
                )
                .unwrap();
                std::fs::write(
                    d.join(format!("{stem}.theirs")),
                    listing(&theirs).join("\n"),
                )
                .unwrap();
            }
            results.push(Outcome::Differs {
                delta,
                file: place.0,
                what: place.1,
            });
        }
        outcome.insert(stem, results);
    }

    // What each script's outcomes say its cause is.
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut identical = 0;
    for (s, rs) in &outcome {
        let (bio, ship) = (&rs[0], &rs[1]);
        let cause = match (bio, ship) {
            (Outcome::Identical, Outcome::Identical) => {
                identical += 1;
                continue;
            }
            (Outcome::Identical, _) => "conversation-eleven".to_string(),
            (_, Outcome::Identical) => "conversation-twelve".to_string(),
            (Outcome::Failed, _) => "failed".to_string(),
            (Outcome::Differs { delta, file, what }, _) => {
                println!("{s}: {delta:+} bytes, first difference at {what} of {file}");
                if file.starts_with("k_inc_") {
                    "stale-include"
                } else {
                    "stale-bytecode"
                }
                .to_string()
            }
        };
        if cause.starts_with("conversation") {
            identical += 1;
        }
        found.entry(cause).or_default().push(s.clone());
    }
    println!("identical with one set of declarations or the other: {identical}");

    let known: toml::Value = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/known.toml"),
    )
    .unwrap()
    .parse()
    .unwrap();
    let listed = |cause: &str| -> Vec<String> {
        let mut v: Vec<String> = known["cause"][cause]["scripts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        v.sort();
        v
    };
    for cause in [
        "conversation-eleven",
        "conversation-twelve",
        "stale-include",
        "stale-bytecode",
    ] {
        assert_eq!(
            found.remove(cause).unwrap_or_default(),
            listed(cause),
            "the scripts of cause {cause}"
        );
    }
    let mut failed = [
        listed("stale-prototype"),
        listed("undeclared"),
        listed("not-a-script"),
    ]
    .concat();
    failed.sort();
    assert_eq!(found.remove("failed").unwrap_or_default(), failed);
    assert!(found.is_empty(), "{found:?}");
    // 1,754 pairs, less the 23 that do not compile and the 81 whose
    // bytecode is older than their sources.
    assert_eq!(identical, 1650);
}

enum Outcome {
    Identical,
    Failed,
    Differs {
        delta: i64,
        file: String,
        what: String,
    },
}
