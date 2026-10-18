//! A save whose `SAVEGAME.sav` is arbitrary bytes, opened and read by
//! name: every reader returns its value or an error and none panics. The
//! bytes are also read as a `GLOBALVARS.res` on their own.
//!
//! Each process writes its input to its own directory under the system's
//! temporary directory.

#![no_main]

use std::path::PathBuf;
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use rakata_gff::Gff;
use rakata_save::{Globals, Save};

fn dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let d = std::env::temp_dir().join(format!("rakata-save-fuzz-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    })
}

fuzz_target!(|data: &[u8]| {
    if let Ok(g) = Gff::parse(data)
        && let Ok(globals) = Globals::from_gff(&g, "GLOBALVARS.res")
    {
        for v in &globals.vars {
            assert!(globals.get(&v.name).is_some());
        }
        let _ = globals.matching("a").count();
    }

    std::fs::write(dir().join("SAVEGAME.sav"), data).unwrap();
    let Ok(save) = Save::open(dir()) else {
        return;
    };
    let _ = (save.info(), save.globals(), save.party(), save.party_gffs());
    let _ = (save.creatures(None), save.find_creatures("a", None));
    if let Ok(modules) = save.modules() {
        for m in &modules {
            let _ = (save.module(m), save.events(m), save.creatures(Some(m)));
        }
    }
    let _ = save.gff_members();
});
