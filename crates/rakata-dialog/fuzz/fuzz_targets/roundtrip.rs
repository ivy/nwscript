//! Each dialogue the reader accepts, written: the bytes read back to the
//! dialogue written, and writing that again gives the same bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_dialog::Dialog;

fuzz_target!(|data: &[u8]| {
    if let Ok(d) = Dialog::parse("fuzz.dlg", data)
        && let Ok(written) = d.to_bytes()
    {
        let back = Dialog::parse("fuzz.dlg", &written).expect("a written dialogue reads");
        assert_eq!(back.gff(), d.gff());
        assert_eq!(back.to_bytes().expect("written again"), written);
    }
});
