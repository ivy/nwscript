//! Each walkmesh the reader accepts, written: the bytes read back to the
//! walkmesh written, and writing that again gives the same bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(w) = rakata_walkmesh::parse(data)
        && let Ok(written) = w.to_bytes()
    {
        let back = rakata_walkmesh::parse(&written).expect("a written walkmesh reads");
        assert!(back.semantic_eq(&w), "{back:?}\n{w:?}");
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
});
