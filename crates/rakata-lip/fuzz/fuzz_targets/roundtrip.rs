//! Each track the reader accepts, written: the bytes read back to the track
//! written, floats bit for bit, and writing that again gives the same
//! bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_lip::Lip;

fuzz_target!(|data: &[u8]| {
    if let Ok(l) = Lip::parse(data)
        && let Ok(written) = l.to_bytes()
    {
        let back = Lip::parse(&written).expect("a written track reads");
        assert!(back.bits_eq(&l), "{back:?}\n{l:?}");
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
});
