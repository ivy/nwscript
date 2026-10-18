//! Each SSF the installer's reader accepts, written: the bytes read back to
//! the soundset written, and writing that again gives the same bytes.
//! (`changes.ini` has no writer; `decode` covers its reader.)

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_patch::ssf::Ssf;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = Ssf::parse(data) {
        let written = s.to_bytes();
        let back = Ssf::parse(&written).expect("a written soundset reads");
        assert_eq!(back, s);
        assert_eq!(back.to_bytes(), written);
    }
});
