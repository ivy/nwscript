//! Each model `ModelFile::parse` accepts, written: the MDL and MDX read
//! back to the model written, writing that again gives the same bytes,
//! and `parse` reads them when it read the input.
//!
//! The input is an MDL with its MDX after it, split where the MDL's file
//! header says its model data ends.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_mdl::ModelFile;

fn split(data: &[u8]) -> (&[u8], &[u8]) {
    let size = data
        .get(4..8)
        .map_or(0, |w| u32::from_le_bytes(w.try_into().unwrap()) as usize);
    data.split_at(size.saturating_add(12).min(data.len()))
}

fuzz_target!(|data: &[u8]| {
    let (mdl, mdx) = split(data);
    if let Ok(m) = ModelFile::parse(mdl, mdx)
        && let Ok((wmdl, wmdx)) = m.to_bytes()
    {
        let back = ModelFile::parse(&wmdl, &wmdx).expect("a written model reads");
        // The padding after each MDX block is the writer's to choose.
        assert!(back.semantic_eq(&m), "{back:?}\n{m:?}");
        assert_eq!(
            back.to_bytes().expect("written again"),
            (wmdl.clone(), wmdx.clone())
        );
        if rakata_mdl::parse(mdl, mdx).is_ok() {
            rakata_mdl::parse(&wmdl, &wmdx).expect("a written model reads as a Model");
        }
    }
});
