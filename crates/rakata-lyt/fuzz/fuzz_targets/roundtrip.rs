//! Each layout and visibility file the readers accept, written: the bytes
//! read back to the file written, and writing that again gives the same
//! bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_lyt::{Lyt, Vis};

fuzz_target!(|data: &[u8]| {
    if let Ok(lyt) = Lyt::parse(data)
        && let Ok(written) = lyt.to_bytes()
    {
        let back = Lyt::parse(&written).expect("a written layout reads");
        assert!(back.semantic_eq(&lyt), "{back:?}\n{lyt:?}");
        assert_eq!(back.trailer(), lyt.trailer());
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
    if let Ok(vis) = Vis::parse(data)
        && let Ok(written) = vis.to_bytes()
    {
        let back = Vis::parse(&written).expect("a written visibility file reads");
        assert!(back.semantic_eq(&vis), "{back:?}\n{vis:?}");
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
});
