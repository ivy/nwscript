//! `Dialog::parse` on arbitrary bytes: it returns a dialogue or an error,
//! never panics, and a dialogue read answers every accessor, builds its
//! transcript, takes the checks and the lints, and gives each node a key
//! that resolves.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_dialog::{
    Dialog, Embedded, NodeKey, Options, References, Transcript, check, facts, lint,
};
use rakata_res::{ResRef, ResType};

/// A module where every name resolves and nothing else is known.
struct Anything;

impl References for Anything {
    fn resolves(&mut self, _: &ResRef, _: ResType) -> bool {
        true
    }
    fn talk_table_len(&mut self) -> Option<u32> {
        None
    }
    fn globals_read(&mut self, condition: &ResRef) -> Vec<String> {
        vec![condition.to_string()]
    }
    fn declares_global(&mut self, _: &str) -> Option<bool> {
        Some(false)
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(d) = Dialog::parse("fuzz.dlg", data) else {
        return;
    };
    for n in d.all() {
        let _ = (
            n.speaker(),
            n.listener(),
            n.text(),
            n.strref(),
            n.embedded_text(),
        );
        let _ = (
            n.voice(),
            n.sound_exists(),
            n.voiced(),
            n.sound(),
            n.script(),
        );
        let _ = (n.delay(), n.animations(), n.camera_angle(), n.camera_id());
        let _ = (
            n.camera_animation(),
            n.quest(),
            n.quest_entry(),
            n.comment(),
        );
        for l in n.links() {
            let _ = (l.condition(), l.is_child(), d.node(l.target).is_some());
        }
        let key = NodeKey::of(&d, n.at);
        let _ = key.is_fragile();
        if let Ok(at) = key.resolve(&d) {
            assert!(d.node(at).is_some());
        }
    }
    for l in d.starts() {
        let _ = (l.condition(), d.node(l.target).is_some());
    }
    let _ = (
        d.end_script(false),
        d.end_script(true),
        d.info(),
        facts::facts(&d),
    );
    for options in [
        Options::default(),
        Options {
            scripts: true,
            brief: false,
        },
        Options {
            scripts: false,
            brief: true,
        },
    ] {
        let t = Transcript::build(&d, &mut Embedded, options);
        let _ = t.text();
    }
    let _ = check(&d, &mut Anything);
    let _ = lint(&d, &mut Anything);
});
