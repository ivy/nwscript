//! `Lip::parse` on arbitrary bytes: it returns a track or an error, never
//! panics, and a track read answers its accessors.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_lip::{Lip, Shape};

fuzz_target!(|data: &[u8]| {
    if let Ok(l) = Lip::parse(data) {
        let _ = l.is_sorted();
        for k in &l.keys {
            assert_eq!(Shape::from_label(k.shape.label()), Some(k.shape));
        }
        let _ = Lip::from_cues(l.keys.iter().map(|k| (k.time, k.shape)), l.length);
    }
});
