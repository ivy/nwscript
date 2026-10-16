//! `decode` on arbitrary bytes, with and without a valid header and size
//! record: it returns a program or a diagnostic, never panics, and whatever
//! it accepts encodes back to the same bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use nwscript_diag::FileId;
use nwscript_ncs::{decode, encode};

fuzz_target!(|data: &[u8]| {
    check(data);
    // Most random inputs fail the header; also try the bytes as a program body.
    let mut file = b"NCS V1.0\x42".to_vec();
    file.extend_from_slice(&((13 + data.len()) as u32).to_be_bytes());
    file.extend_from_slice(data);
    check(&file);
});

fn check(bytes: &[u8]) {
    match decode(FileId(0), bytes) {
        Ok(program) => assert_eq!(encode(&program), bytes),
        Err(d) => assert!(d.primary_span().is_some()),
    }
}
