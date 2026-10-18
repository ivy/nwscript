//! A save whose `SAVEGAME.sav` is arbitrary bytes, opened and written
//! with its first global set and an empty party: the copy reads back with
//! that global changed and every other global as it was, and writing the
//! copy again with the same edits gives the same files.
//!
//! Each process works in its own directory under the system's temporary
//! directory.

#![no_main]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use rakata_save::{Assignment, Declarations, Edits, GlobalValue, Save};

fn dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        std::env::temp_dir().join(format!("rakata-save-fuzz-{}", std::process::id()))
    })
}

fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let (input, first, second) = (dir().join("in"), dir().join("first"), dir().join("second"));
    for d in [&input, &first, &second] {
        let _ = std::fs::remove_dir_all(d);
    }
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(input.join("SAVEGAME.sav"), data).unwrap();
    let Ok(save) = Save::open(&input) else {
        return;
    };
    let Ok(before) = save.globals() else {
        return;
    };
    let mut edits = Edits {
        declare: true,
        party: save.party().is_ok().then(Vec::new),
        ..Edits::default()
    };
    let target = before.vars.first().and_then(|g| {
        let (text, value) = match &g.value {
            GlobalValue::Boolean(b) => (format!("{}", !b), GlobalValue::Boolean(!b)),
            GlobalValue::Number(n) => {
                let v = n.wrapping_add(1);
                (format!("{v}"), GlobalValue::Number(v))
            }
            GlobalValue::String(_) => ("fuzz".to_owned(), GlobalValue::String("fuzz".into())),
            GlobalValue::Location(_) => return None,
        };
        // Built, not parsed: the name as the file spells it, spaces and all.
        let a = Assignment {
            name: g.name.clone(),
            kind: Some(g.value.global_kind()),
            text,
        };
        Some((a, g.name.clone(), value))
    });
    if let Some((a, _, _)) = &target {
        edits.globals.push(a.clone());
    }
    let declarations = Declarations::new();
    if save.write(&first, &edits, &declarations).is_err() {
        return;
    }
    let copy = Save::open(&first).expect("a written save opens");
    let after = copy.globals().expect("a written save's globals read");
    assert_eq!(after.vars.len(), before.vars.len());
    for (b, a) in before.vars.iter().zip(&after.vars) {
        assert_eq!(a.name, b.name);
        match &target {
            Some((_, name, value)) if *name == b.name && std::ptr::eq(b, &before.vars[0]) => {
                assert_eq!(&a.value, value)
            }
            // Through Debug, so a location's NaN equals itself.
            _ => assert_eq!(format!("{:?}", a.value), format!("{:?}", b.value)),
        }
    }
    copy.write(&second, &edits, &declarations)
        .expect("a written save writes again");
    assert_eq!(files(&first), files(&second));
});
