//! Each archive the readers accept, written: the bytes read back to the
//! archive written, and writing that again gives the same bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_archive::{Bif, Erf, Key, Rim};

macro_rules! round_trip {
    ($ty:ident, $data:expr) => {
        if let Ok(a) = $ty::parse($data)
            && let Ok(written) = a.to_bytes()
        {
            let back =
                $ty::parse(&written).expect(concat!("a written ", stringify!($ty), " reads"));
            assert_eq!(back, a);
            assert_eq!(back.to_bytes().expect("written again"), written);
        }
    };
}

fuzz_target!(|data: &[u8]| {
    round_trip!(Key, data);
    round_trip!(Bif, data);
    round_trip!(Erf, data);
    round_trip!(Rim, data);
});
