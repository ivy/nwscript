//! Each texture the readers accept, written: the bytes read back to the
//! texture written, and writing that again gives the same bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_texture::{Tga, Tpc, Txi};

fuzz_target!(|data: &[u8]| {
    if let Ok(t) = Tpc::parse(data)
        && let Ok(written) = t.to_bytes()
    {
        let back = Tpc::parse(&written).expect("a written TPC reads");
        // The float by its bits, so a NaN equals itself.
        assert_eq!(back.alpha_mean.to_bits(), t.alpha_mean.to_bits());
        assert_eq!(back.to_bytes().expect("written again"), written);
        let (mut back, mut t) = (back, t);
        (back.alpha_mean, t.alpha_mean) = (0.0, 0.0);
        assert_eq!(back, t);
    }

    let t = Txi::parse(data);
    if let Ok(written) = t.to_bytes() {
        let back = Txi::parse(&written);
        assert_eq!(back, t);
        assert_eq!(back.to_bytes().expect("written again"), written);
    }

    if let Ok(t) = Tga::parse(data)
        && let Ok(written) = t.to_bytes()
    {
        let back = Tga::parse(&written).expect("a written TGA reads");
        assert_eq!(back, t);
        assert_eq!(back.layout, t.layout);
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
});
