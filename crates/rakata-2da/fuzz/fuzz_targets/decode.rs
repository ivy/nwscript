//! `TwoDa::parse` and `inspect` on arbitrary bytes: each returns a table
//! or an error, never panics, and a table read answers its lookups, its
//! facts and a diff with itself.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_2da::{TwoDa, facts, inspect};

fuzz_target!(|data: &[u8]| {
    if let Ok(l) = inspect(data) {
        let _ = l.to_string();
    }
    if let Ok(t) = TwoDa::parse(data) {
        for (i, row) in t.rows.iter().enumerate() {
            assert!(t.row(i).is_ok());
            assert!(t.row_index(row.label.as_str()).is_some());
            for c in &t.columns {
                assert!(t.column_index(c).is_some());
                let _ = t.get(i, c);
            }
        }
        assert!(t.diff(&t).is_empty());
        let _ = facts::facts(&t, "table");
    }
});
