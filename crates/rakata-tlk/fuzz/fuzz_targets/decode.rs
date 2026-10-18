//! `Tlk::parse` on arbitrary bytes: it returns a talk table or an error,
//! never panics, and a table read answers every strref and its facts.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_res::StrRef;
use rakata_tlk::{Tlk, facts};

fuzz_target!(|data: &[u8]| {
    if let Ok(t) = Tlk::parse(data) {
        for i in 0..=t.len() as u32 {
            let _ = (t.get(StrRef::new(i)), t.text(StrRef::new(i)));
        }
        for e in &t.entries {
            let _ = e.shown_text();
        }
        let _ = facts::facts(&t);
    }
});
