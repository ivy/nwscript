//! `Source`: the bytes of an archive file, owned or mapped.

use std::ops::Deref;
use std::path::Path;

use crate::mmap::Mapped;
use crate::{Error, Result};

/// The bytes of an archive file. Readers borrow from it, so the members
/// they list are slices into these bytes and nothing is copied until asked.
///
/// [`Source::read`] loads the whole file; [`Source::map`] maps it, so that
/// listing a large BIF touches only its header and table. Both deref to
/// `[u8]`.
pub struct Source {
    inner: Inner,
}

enum Inner {
    Owned(Vec<u8>),
    Mapped(Mapped),
}

impl Source {
    /// The whole file, read into memory.
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self::from_vec(bytes))
    }

    /// The file, memory-mapped read-only. The file must not change while
    /// the source is alive; an archive the game or another tool is writing
    /// is read with [`Source::read`] instead.
    pub fn map(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mapped = Mapped::open(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            inner: Inner::Mapped(mapped),
        })
    }

    /// Bytes already in memory.
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        Self {
            inner: Inner::Owned(bytes),
        }
    }

    /// The bytes.
    pub fn bytes(&self) -> &[u8] {
        match &self.inner {
            Inner::Owned(v) => v,
            Inner::Mapped(m) => m.bytes(),
        }
    }

    /// Whether the bytes are a memory map rather than a buffer.
    pub fn is_mapped(&self) -> bool {
        matches!(self.inner, Inner::Mapped(_))
    }
}

impl Deref for Source {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.bytes()
    }
}

impl AsRef<[u8]> for Source {
    fn as_ref(&self) -> &[u8] {
        self.bytes()
    }
}

impl From<Vec<u8>> for Source {
    fn from(bytes: Vec<u8>) -> Self {
        Self::from_vec(bytes)
    }
}
