//! Transcripts over the user's own installs: every vanilla dialogue
//! rendered with its conditions decompiled, timed; `kor37_freedonnad.dlg`'s
//! conditions with `k_inc_utility`'s constants by name; and `banter.dlg`
//! in the modded build showing the prone script and the get-up animation
//! on consecutive nodes.
//!
//! Reads `$RAKATA_GAME_K1` (a vanilla KotOR 1 install) and `$RAKATA_BUILD`
//! (a modded build), and skips, naming the variable, when one is unset. No
//! game bytes are committed.

use std::path::PathBuf;
use std::time::Instant;

use rakata::dialog::{Session, TranscriptOptions};
use rakata::game::{ModuleName, Profile};
use rakata_res::{ResId, ResType};

fn tree(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${var} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

#[test]
fn every_vanilla_dialogue_renders_with_its_conditions_decompiled() {
    let Some(root) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let start = Instant::now();
    let mut session = Session::open(&root, Profile::K1).unwrap();
    // Every dialogue the engine can load: the global layers' own, then each
    // module's, each with its module so its scripts resolve.
    let mut jobs: Vec<(Option<ModuleName>, ResId)> = Vec::new();
    let install = session.install();
    let mut globals: Vec<ResId> = install
        .key()
        .entries()
        .iter()
        .map(|e| e.id)
        .chain(install.override_ids())
        .filter(|id| id.ty == ResType::Dlg)
        .collect();
    globals.sort();
    globals.dedup();
    jobs.extend(globals.into_iter().map(|id| (None, id)));
    for m in install.modules().to_vec() {
        for id in install.module_ids(&m).unwrap() {
            if id.ty == ResType::Dlg {
                jobs.push((Some(m.clone()), id));
            }
        }
    }
    let opened = start.elapsed().as_secs_f64();

    let mut slowest = (0.0f64, String::new());
    let mut warnings = 0;
    let mut nodes = 0;
    let mut failed = Vec::new();
    for (module, id) in &jobs {
        let t = Instant::now();
        session.set_module(module.clone());
        let loaded = match session.load_id(*id) {
            Ok(l) => l,
            Err(e) => {
                failed.push(format!("{id}: {e}"));
                continue;
            }
        };
        let transcript = session.transcript(&loaded.dialog, TranscriptOptions::default());
        let text = transcript.text();
        assert!(!text.is_empty());
        nodes += transcript.nodes.len();
        warnings += transcript.warnings.len();
        let s = t.elapsed().as_secs_f64();
        if s > slowest.0 {
            slowest = (s, format!("{id} ({})", loaded.location));
        }
    }
    let total = start.elapsed().as_secs_f64();
    let stats = session.stats();
    println!(
        "{} dialogues ({} nodes) rendered in {total:.2} s, {:.1} ms each on average; install opened in {opened:.2} s; slowest {:.0} ms, {}",
        jobs.len(),
        nodes,
        (total - opened) * 1000.0 / jobs.len() as f64,
        slowest.0 * 1000.0,
        slowest.1
    );
    println!(
        "{} conditions decompiled, {} verified, {} from the cache by hash, {} missing; libraries {:.2} s, decompiling {:.2} s; {warnings} warnings",
        stats.decompiled,
        stats.verified,
        stats.cache_hits,
        stats.missing,
        stats.setup_seconds,
        stats.decompile_seconds
    );
    for f in failed.iter().take(20) {
        println!("  failed: {f}");
    }
    assert!(jobs.len() > 500, "an install has hundreds of dialogues");
    assert!(failed.is_empty(), "{} dialogues failed", failed.len());
    assert!(stats.decompiled > 0 && stats.cache_hits > 0);
}

#[test]
fn banter_shows_the_prone_script_then_the_get_up_animation() {
    let Some(root) = tree("RAKATA_BUILD") else {
        return;
    };
    let mut session = Session::open(&root, Profile::K1).unwrap();
    let loaded = session.load("banter.dlg").unwrap();
    let t = session.transcript(
        &loaded.dialog,
        TranscriptOptions {
            scripts: true,
            ..TranscriptOptions::default()
        },
    );
    // The node whose script knocks Mission down, and the next entry shown,
    // through its one reply, where she gets up.
    let at = t
        .nodes
        .iter()
        .position(|n| n.script.as_deref() == Some("k_swg_bantforce"))
        .expect("banter.dlg runs k_swg_bantforce");
    let script = t.script("k_swg_bantforce").unwrap();
    let source = script.source.as_deref().unwrap();
    assert!(source.contains("PlayAnimation"), "{source}");
    let reply = &t.nodes[at + 1];
    let entry = &t.nodes[at + 2];
    assert_eq!(reply.kind, "reply");
    assert_eq!(t.nodes[at].links[0].target, reply.id);
    assert_eq!(reply.links[0].target, entry.id);
    assert_eq!(entry.animations[0].animation, 10055);
    assert_eq!(entry.animations[0].participant, "Mission");
    // The starts' conditions, decompiled and verified, inline.
    let text = t.text();
    assert!(text.contains("if k_hbanter1 [verified"), "{text}");
    assert!(text.contains("int StartingConditional()"), "{text}");
    assert!(text.contains("animation 10055 on Mission"), "{text}");
    println!(
        "{} nodes; {} runs k_swg_bantforce, then {} with 10055 on Mission",
        t.nodes.len(),
        t.nodes[at].id,
        entry.id
    );
}

#[test]
fn freedon_nadd_s_conditions_name_k_inc_utility_s_constants() {
    let Some(root) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let mut session = Session::open(&root, Profile::K1).unwrap();
    session.set_module(Some(ModuleName::new("korr_m37aa")));
    let loaded = session.load("kor37_freedonnad.dlg").unwrap();
    let t = session.transcript(&loaded.dialog, TranscriptOptions::default());
    // The three starts' conditions are StartingConditionals that embed
    // k_inc_utility's functions; folded, its globals go with them.
    for name in ["k37_frd_wrongswd", "k37_frd_rightswo", "k_pkor_ajufirst"] {
        let s = t.script(name).unwrap();
        assert_eq!(s.verified, s.functions, "{name}");
        let source = s.source.as_deref().unwrap();
        assert!(source.contains("#include \"k_inc_utility\""), "{source}");
        assert!(
            source.contains("UT_GetTalkedToBooleanFlag(OBJECT_SELF)"),
            "{source}"
        );
        assert!(!source.contains("nGlobal1 "), "{source}");
        assert!(!source.contains("// verified"), "{source}");
    }
    let wrong = t.script("k37_frd_wrongswd").unwrap();
    assert!(
        wrong
            .source
            .as_deref()
            .unwrap()
            .contains("UT_GetPlotBooleanFlag(oLocal1, SW_PLOT_BOOLEAN_01)")
    );
    let text = t.text();
    // The starts list carries the source; the player's unvoiced replies
    // carry no voice line.
    let starts = text.split("\n\n").nth(1).unwrap();
    assert!(starts.contains("| int StartingConditional()"), "{starts}");
    assert!(!text.contains("voice _m37aafree04001_"), "{text}");
}
