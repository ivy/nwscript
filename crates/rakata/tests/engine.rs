//! What a game engine reads through the public API to draw and voice a
//! scene: an emitter's header, an animation's events, a soundset entry's
//! sound, a saber blade's triangles, and a controller other than position
//! and orientation on an animation node. Each is read here from the user's
//! own install, `$RAKATA_GAME_K1`, through the façade alone; the test skips,
//! naming the variable, when it is unset.

use std::path::PathBuf;

use rakata::game::{Install, Profile};
use rakata::mdl::{Model, NodeKind, controller_types};
use rakata::patch::ssf::Ssf;
use rakata::res::{ResId, ResRef, ResType, StrRef};
use rakata::tlk::Tlk;

fn install() -> Option<Install> {
    match std::env::var("RAKATA_GAME_K1") {
        Ok(root) => Some(Install::open(PathBuf::from(root), Profile::K1).unwrap()),
        Err(_) => {
            eprintln!("skipped: $RAKATA_GAME_K1 is unset; set it to a KotOR 1 install");
            None
        }
    }
}

fn read(install: &Install, name: &str, ty: ResType) -> Vec<u8> {
    let id = ResId::new(ResRef::new(name).unwrap(), ty);
    install.read(id, None).unwrap().unwrap().1
}

#[test]
fn an_engine_reads_effects_events_sounds_and_sabers_through_the_facade() {
    let Some(install) = install() else { return };

    // The emitters of a muzzle flash, and the `detonate` events of its
    // `impact` animation that fire them.
    let (mdl, mdx) = (
        read(&install, "v_muzflash_01", ResType::Mdl),
        read(&install, "v_muzflash_01", ResType::Mdx),
    );
    let m = Model::parse(&mdl, &mdx).unwrap();
    let emitters: Vec<_> = m.nodes.iter().filter_map(|n| n.emitter.as_ref()).collect();
    assert_eq!(emitters.len(), 5);
    assert!(emitters.iter().all(|e| e.update == "Explosion"));
    let impact = m.animation("impact").unwrap();
    let detonations = impact
        .events
        .iter()
        .filter(|e| e.name == "detonate")
        .count();
    assert_eq!(detonations, 5);

    // A creature's soundset: each slot a strref, and the talk table's
    // entry of it the sound to play.
    let ssf = Ssf::parse(&read(&install, "c_bantha", ResType::Ssf)).unwrap();
    let tlk = Tlk::open(install.root().join("dialog.tlk")).unwrap();
    let sounds: Vec<ResRef> = ssf
        .strrefs
        .iter()
        .filter(|&&s| s != rakata::patch::ssf::NONE)
        .filter_map(|&s| tlk.get(StrRef::new(s)).and_then(|e| e.sound))
        .collect();
    assert!(!sounds.is_empty());
    assert!(sounds.iter().all(|r| !r.is_empty()));

    // A saber blade: its vertices from the saber header, and its surface,
    // both strips of it, over the whole of the texture's width.
    let (mdl, mdx) = (
        read(&install, "w_lghtsbr_006", ResType::Mdl),
        read(&install, "w_lghtsbr_006", ResType::Mdx),
    );
    let m = Model::parse(&mdl, &mdx).unwrap();
    let blade = m.nodes.iter().find(|n| n.saber.is_some()).unwrap();
    assert_eq!(blade.kind, NodeKind::Saber);
    let saber = blade.saber.as_ref().unwrap();
    let triangles = saber.blade_triangles();
    assert_eq!(triangles.len(), 12);
    let used: std::collections::BTreeSet<u16> = triangles.iter().flatten().copied().collect();
    let us: Vec<f32> = used
        .iter()
        .map(|&v| saber.uv.get(v as usize).unwrap()[0])
        .collect();
    assert!(us.iter().any(|&u| u < 0.01) && us.iter().any(|&u| u > 0.99));
    // Every column of the blade that is not the centre line is drawn.
    let centre = saber.positions.get(0).unwrap();
    for v in 0..saber.positions.len() {
        let p = saber.positions.get(v).unwrap();
        if v % 4 == 0 && (p[0] - centre[0]).abs() > 1e-4 {
            assert!(used.contains(&(v as u16)), "vertex {v}");
        }
    }

    // A droid's eye dimming as it dies: self-illumination keyed on an
    // animation node that keys neither position nor orientation.
    let (mdl, mdx) = (
        read(&install, "c_drdspyder", ResType::Mdl),
        read(&install, "c_drdspyder", ResType::Mdx),
    );
    let m = Model::parse(&mdl, &mdx).unwrap();
    let node = m.animation("cdie").unwrap().node("Object01").unwrap();
    let glow = node
        .controller(controller_types::mesh::SELF_ILLUM_COLOR)
        .unwrap();
    assert!(glow.len() > 1);
    assert!(node.controller(controller_types::POSITION).is_none());
}
