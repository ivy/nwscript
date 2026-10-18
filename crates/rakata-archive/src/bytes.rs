//! Little-endian reads with bounds checks that report where the file ran
//! out, and the matching writes.

use crate::{Error, Result};

/// A cursor over the bytes of an archive. Every read names what it was
/// reading, so a truncated file says which table ended it.
#[derive(Clone, Copy)]
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    /// `len` bytes at `offset`.
    pub(crate) fn slice(&self, what: &'static str, offset: usize, len: usize) -> Result<&'a [u8]> {
        offset
            .checked_add(len)
            .and_then(|end| self.bytes.get(offset..end))
            .ok_or(Error::Truncated {
                what,
                offset,
                len,
                available: self.bytes.len(),
            })
    }

    pub(crate) fn u8(&self, what: &'static str, offset: usize) -> Result<u8> {
        Ok(self.slice(what, offset, 1)?[0])
    }

    pub(crate) fn u16(&self, what: &'static str, offset: usize) -> Result<u16> {
        let b = self.slice(what, offset, 2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub(crate) fn u32(&self, what: &'static str, offset: usize) -> Result<u32> {
        let b = self.slice(what, offset, 4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// The eight signature bytes must be `expected`.
    pub(crate) fn signature(&self, expected: &'static str) -> Result<()> {
        let found = self.slice("the signature", 0, 8)?;
        if found == expected.as_bytes() {
            Ok(())
        } else {
            Err(Error::NotThisArchive {
                expected,
                found: String::from_utf8_lossy(found).into_owned(),
            })
        }
    }
}

/// The bytes of an archive being written.
#[derive(Default)]
pub(crate) struct Writer {
    out: Vec<u8>,
}

impl Writer {
    pub(crate) fn with_capacity(n: usize) -> Self {
        Self {
            out: Vec::with_capacity(n),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.out.len()
    }

    pub(crate) fn bytes(&mut self, b: &[u8]) {
        self.out.extend_from_slice(b);
    }

    pub(crate) fn u8(&mut self, v: u8) {
        self.out.push(v);
    }

    pub(crate) fn u16(&mut self, v: u16) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    /// `n` zero bytes.
    pub(crate) fn zeros(&mut self, n: usize) {
        self.out.resize(self.out.len() + n, 0);
    }

    /// Zero bytes up to the next multiple of `align`.
    pub(crate) fn align(&mut self, align: usize) {
        let rem = self.out.len() % align;
        if rem != 0 {
            self.zeros(align - rem);
        }
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.out
    }
}

/// A size or offset as the 32-bit field that holds it.
pub(crate) fn u32_field(n: usize) -> Result<u32> {
    u32::try_from(n).map_err(|_| Error::TooLarge { size: n as u64 })
}
