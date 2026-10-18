//! The archives an install holds, opened on first use and kept open: a
//! memory map and a table of where each member lies in it.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rakata_archive::{Bif, Erf, Rim, Source};
use rakata_res::ResId;

use crate::{Error, Result};

/// Where `slice` lies within `base`, which it must be a part of.
fn range_in(base: &[u8], slice: &[u8]) -> Range<usize> {
    let start = slice.as_ptr() as usize - base.as_ptr() as usize;
    start..start + slice.len()
}

fn map(path: &Path) -> Result<Source> {
    Source::map(path).map_err(|source| match source {
        rakata_archive::Error::Io { path, source } => Error::Io { path, source },
        other => Error::Archive {
            path: path.to_path_buf(),
            source: other,
        },
    })
}

/// A capsule: an ERF-family file or a RIM, members by name and type. The
/// first member wins when a capsule repeats a name, as the archive crate's
/// lookups answer.
pub struct Capsule {
    path: PathBuf,
    cell: OnceLock<Result<CapsuleIndex>>,
}

struct CapsuleIndex {
    source: Source,
    members: Vec<(ResId, Range<usize>)>,
    by_id: HashMap<ResId, usize>,
}

impl std::fmt::Debug for Capsule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capsule")
            .field("path", &self.path)
            .field("open", &self.is_open())
            .finish()
    }
}

impl Capsule {
    /// A capsule at `path`, opened on first use.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cell: OnceLock::new(),
        }
    }

    /// The capsule's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the capsule has been opened.
    pub fn is_open(&self) -> bool {
        self.cell.get().is_some()
    }

    fn open(&self) -> Result<CapsuleIndex> {
        let source = map(&self.path)?;
        let is_rim = self
            .path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rim"));
        let archive = |source: rakata_archive::Error| Error::Archive {
            path: self.path.clone(),
            source,
        };
        let members: Vec<(ResId, Range<usize>)> = if is_rim {
            let rim = Rim::parse(&source).map_err(archive)?;
            rim.members()
                .iter()
                .map(|m| (m.id, range_in(&source, m.data())))
                .collect()
        } else {
            let erf = Erf::parse(&source).map_err(archive)?;
            erf.members()
                .iter()
                .map(|m| (m.id, range_in(&source, m.data())))
                .collect()
        };
        let mut by_id = HashMap::with_capacity(members.len());
        for (i, (id, _)) in members.iter().enumerate() {
            by_id.entry(*id).or_insert(i);
        }
        Ok(CapsuleIndex {
            source,
            members,
            by_id,
        })
    }

    fn index(&self) -> Result<&CapsuleIndex> {
        match self.cell.get_or_init(|| self.open()) {
            Ok(index) => Ok(index),
            Err(e) => Err(e.remembered()),
        }
    }

    /// The bytes of the member named `id`, if the capsule holds one.
    pub fn get(&self, id: ResId) -> Result<Option<&[u8]>> {
        let index = self.index()?;
        Ok(index
            .by_id
            .get(&id)
            .map(|&i| &index.source[index.members[i].1.clone()]))
    }

    /// Every member's name and type, in table order.
    pub fn ids(&self) -> Result<Vec<ResId>> {
        Ok(self.index()?.members.iter().map(|(id, _)| *id).collect())
    }
}

/// A BIF: members by their position in its table, as the KEY indexes them.
pub struct BifFile {
    path: PathBuf,
    cell: OnceLock<Result<BifIndex>>,
}

impl std::fmt::Debug for BifFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BifFile")
            .field("path", &self.path)
            .field("open", &self.cell.get().is_some())
            .finish()
    }
}

struct BifIndex {
    source: Source,
    members: Vec<Range<usize>>,
}

impl BifFile {
    /// A BIF at `path`, opened on first use.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cell: OnceLock::new(),
        }
    }

    /// The BIF's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn open(&self) -> Result<BifIndex> {
        let source = map(&self.path)?;
        let bif = Bif::parse(&source).map_err(|source| Error::Archive {
            path: self.path.clone(),
            source,
        })?;
        let members = bif
            .members()
            .iter()
            .map(|m| range_in(&source, m.data()))
            .collect();
        Ok(BifIndex { source, members })
    }

    /// The bytes of the member at `index`, if the table reaches it.
    pub fn get(&self, index: usize) -> Result<Option<&[u8]>> {
        let bif = match self.cell.get_or_init(|| self.open()) {
            Ok(bif) => bif,
            Err(e) => return Err(e.remembered()),
        };
        Ok(bif.members.get(index).map(|r| &bif.source[r.clone()]))
    }
}
