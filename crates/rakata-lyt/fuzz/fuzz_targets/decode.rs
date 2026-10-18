//! `Lyt::parse` and `Vis::parse` on arbitrary bytes: each returns a file
//! or an error, never panics, and what it returns answers its accessors.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_lyt::{Lyt, Vis};

fuzz_target!(|data: &[u8]| {
    if let Ok(lyt) = Lyt::parse(data) {
        let _ = (lyt.trailer(), lyt.departures());
        for r in &lyt.rooms {
            assert!(lyt.room(&r.model).is_some());
        }
    }
    if let Ok(vis) = Vis::parse(data) {
        let _ = vis.departures();
        for r in &vis.rooms {
            assert!(vis.visible(&r.room).is_some());
        }
    }
});
