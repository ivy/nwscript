//! Each talk table the reader accepts, written: the bytes read back to the
//! table written, and writing that again gives the same bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_tlk::Tlk;

/// The sound lengths by their bits, so a NaN equals itself, and the table
/// with them set aside.
fn split(t: &Tlk) -> (Vec<u32>, Tlk) {
    let mut t = t.clone();
    let lengths = t
        .entries
        .iter_mut()
        .map(|e| std::mem::take(&mut e.sound_length).to_bits())
        .collect();
    (lengths, t)
}

fuzz_target!(|data: &[u8]| {
    if let Ok(t) = Tlk::parse(data)
        && let Ok(written) = t.to_bytes()
    {
        let back = Tlk::parse(&written).expect("a written talk table reads");
        assert_eq!(split(&back), split(&t));
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
});
