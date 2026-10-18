//! `Key::parse`, `Bif::parse`, `Erf::parse` and `Rim::parse` on arbitrary
//! bytes: each returns an archive or an error, never panics, and finds
//! every member it lists.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_archive::{Bif, Erf, Key, Rim};

fuzz_target!(|data: &[u8]| {
    if let Ok(k) = Key::parse(data) {
        for b in k.bifs() {
            let _ = (b.name_str(), b.components().count());
        }
        for e in k.entries() {
            let _ = e.resource_id();
            assert!(k.get(e.id).is_some());
        }
    }
    if let Ok(b) = Bif::parse(data) {
        for i in 0..b.len() {
            assert!(b.get(i).is_some());
        }
    }
    if let Ok(e) = Erf::parse(data) {
        for m in e.members() {
            assert!(e.contains(m.id) && e.get(m.id).is_some());
        }
    }
    if let Ok(r) = Rim::parse(data) {
        let _ = r.alignment();
        for m in r.members() {
            assert!(r.contains(m.id) && r.get(m.id).is_some());
        }
    }
});
