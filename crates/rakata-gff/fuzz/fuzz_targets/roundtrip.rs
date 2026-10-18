//! Each GFF the reader accepts, written: the bytes read back to the file
//! written, and writing that again gives the same bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_gff::Gff;

fuzz_target!(|data: &[u8]| {
    if let Ok(g) = Gff::parse(data)
        && let Ok(written) = g.to_bytes()
    {
        let back = Gff::parse(&written).expect("a written GFF reads");
        assert_eq!(back, g);
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
});
