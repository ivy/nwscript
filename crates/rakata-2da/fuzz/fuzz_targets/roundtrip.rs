//! Each table the reader accepts, written: the bytes read back to the
//! table written, and writing that again gives the same bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_2da::TwoDa;

fuzz_target!(|data: &[u8]| {
    if let Ok(t) = TwoDa::parse(data)
        && let Ok(written) = t.to_bytes()
    {
        let back = TwoDa::parse(&written).expect("a written table reads");
        assert_eq!(back, t);
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
});
