//! The memory-mapped file behind [`Source::map`](crate::Source::map): the
//! one place in the workspace that uses `unsafe`.
//!
//! The call is unsafe because a mapping reflects the file as it is on disk,
//! so a writer changing the file while the map is alive would change the
//! bytes under a `&[u8]`. This crate opens the file read-only, never writes
//! to a file it has mapped, and documents on `Source::map` that an archive
//! another process is writing must be read with `Source::read`. The measure
//! that earned the exception is in `docs/formats/keybif.md`, "Reading".
#![allow(unsafe_code)]

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

pub(crate) struct Mapped {
    map: Mmap,
}

impl Mapped {
    pub(crate) fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        // SAFETY: the file is opened read-only and the map is never written
        // through; the hazard that remains, another process truncating or
        // rewriting the file while the map lives, is the caller's contract
        // on `Source::map`, which names `Source::read` for that case.
        let map = unsafe { Mmap::map(&file)? };
        Ok(Self { map })
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.map
    }
}
