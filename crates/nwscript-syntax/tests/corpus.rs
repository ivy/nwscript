//! Parse every NSS file in the corpus: no diagnostics, and the tree prints
//! back to the file's exact bytes.

use std::path::PathBuf;

use nwscript_diag::{FileId, SourceMap, render};
use nwscript_syntax::{ast, parse};

fn corpus() -> Option<PathBuf> {
    let dir = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache/nwscript/corpus/k1"),
    };
    dir.join("nss").is_dir().then_some(dir)
}

#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn parse_corpus() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let dir = root.join("nss");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("nss")))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no .nss files in {}", dir.display());

    let mut map = SourceMap::new();
    let mut failures = Vec::new();
    for path in &files {
        let bytes = std::fs::read(path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let file = map.add(name.clone(), bytes.clone());
        let p = parse(file, &bytes);
        if p.to_bytes() != bytes {
            failures.push(format!("{name}: does not print back byte-identical"));
        }
        for d in p.diagnostics() {
            failures.push(render::render(d, &map, render::Color::Never));
        }
    }
    eprintln!("parsed {} files, {} problems", files.len(), failures.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `nwscript.nss` as the engine function table will read it: the five
/// `#define` lines, 772 prototypes in order (the typo line among them), and
/// the constants.
#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn nwscript_nss_corpus() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let bytes = std::fs::read(root.join("nwscript.nss")).unwrap();
    let p = parse(FileId(0), &bytes);
    assert!(p.diagnostics().is_empty());
    let file = p.tree();

    let structs: Vec<(u32, String)> = file
        .defines()
        .filter_map(|d| d.engine_structure())
        .collect();
    let names: Vec<&str> = structs.iter().map(|(_, n)| n.as_str()).collect();
    assert_eq!(names, ["effect", "event", "location", "talent"]);
    assert_eq!(
        file.defines().find_map(|d| d.engine_num_structures()),
        Some(4)
    );

    let protos: Vec<ast::FunctionDecl> = file.functions().collect();
    assert_eq!(protos.len(), 772);
    assert!(protos.iter().all(|f| f.is_prototype()));
    let typo = &protos[767];
    assert_eq!(typo.name().unwrap().text(), "SetAvailableNPCId");
    let types: Vec<String> = typo.params().map(|p| p.ty().unwrap().text()).collect();
    assert_eq!(types, ["INT", "OBJECT_ID"]);

    assert_eq!(file.globals().count(), 1490);
}
