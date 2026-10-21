//! `Tpc::parse`, `Txi::parse` and `Tga::parse` on arbitrary bytes: each
//! returns a texture or an error, never panics, and what it returns
//! answers its accessors, and every level of a TPC read decodes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_texture::{Tga, Tpc, Txi};

fuzz_target!(|data: &[u8]| {
    if let Ok(t) = Tpc::parse(data) {
        let _ = (t.has_alpha(), t.image_size(), t.level_count());
        for image in &t.images {
            for level in &image.levels {
                assert_eq!(
                    &data[level.offset..level.offset + level.data.len()],
                    level.data
                );
            }
        }
        for level in 0..t.level_count() {
            let img = t.decode_level(level).expect("a level the TPC has decodes");
            assert_eq!(
                img.pixels.len(),
                img.width as usize * img.height as usize * 4
            );
        }
    }
    let txi = Txi::parse(data);
    for e in &txi.entries {
        let _ = (txi.value(&e.key), txi.int(&e.key));
    }
    if let Ok(t) = Tga::parse(data) {
        assert_eq!(
            t.rgba.len(),
            usize::from(t.width) * usize::from(t.height) * 4
        );
    }
});
