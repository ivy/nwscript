//! The overlays of `conformance/overlays/`, declarations inferred from
//! BioWare's bytecode: with each, the scripts `conformance/known.toml`
//! lists for it compile to BioWare's bytes.

#[path = "decompile/oracle.rs"]
mod common;

use nwscript::diag::SourceMap;
use nwscript::modes::{Mode, finish};
use nwscript::{Checker, Declarations, FsResolver, Game};
use std::path::Path;

#[test]
#[ignore = "needs the corpus; run with --ignored"]
fn inferred_overlays_reproduce_their_scripts() {
    let Some(corpus) = common::corpus() else {
        return;
    };
    let conformance = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    let known: toml::Value = std::fs::read_to_string(conformance.join("known.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let overlays = known["overlay"].as_table().unwrap();
    assert!(!overlays.is_empty());
    let mut compared = 0;
    for (file, entry) in overlays {
        let text = std::fs::read(conformance.join(file)).unwrap();
        let mut checker = Checker::with_declarations(Declarations::BioWare, Game::K1)
            .with_overlay(file.clone(), text);
        for script in entry["scripts"].as_array().unwrap() {
            let script = script.as_str().unwrap();
            let nss = corpus.join("nss").join(format!("{script}.nss"));
            let source = std::fs::read(&nss).unwrap();
            let mut resolver = FsResolver::new(vec![corpus.join("nss")], None);
            let mut map = SourceMap::new();
            let out = checker.check(&mut resolver, &mut map, script, &source);
            let diags = finish(
                out.analysis.as_ref(),
                out.diagnostics,
                &map,
                &Mode::Bioware.levels(),
            );
            assert!(
                !nwscript::diag::has_errors(&diags),
                "{script} with {file}: {:?}",
                diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            );
            let program = nwscript::compile::compile(&out.analysis.unwrap().program, Game::K1)
                .unwrap_or_else(|d| panic!("{script} with {file}: {}", d.message));
            let ours = nwscript::ncs::encode(&program);
            let bioware = std::fs::read(corpus.join("ncs").join(format!("{script}.ncs"))).unwrap();
            assert!(
                ours == bioware,
                "{script} with {file} differs from BioWare's bytecode"
            );
            compared += 1;
        }
    }
    println!("{compared} scripts identical under inferred overlays");
}
