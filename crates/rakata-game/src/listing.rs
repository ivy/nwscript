//! `Listing`: one directory read once, its names compared without case.
//!
//! The game runs under Wine on a case-sensitive file system, and mods write
//! names in every case, so every lookup folds ASCII case. Where two entries
//! fold to one name, both are kept and reported: which the engine opens is
//! not established.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One entry of a directory, as read.
#[derive(Debug, Clone)]
struct Entry {
    name: OsString,
    is_file: bool,
    is_dir: bool,
}

/// A directory's entries, indexed by their names folded to lowercase.
#[derive(Debug, Clone)]
pub struct Listing {
    path: PathBuf,
    by_fold: HashMap<String, Vec<Entry>>,
}

impl Listing {
    /// Read `path` once. Entries whose names are not UTF-8 are kept under
    /// their lossy spelling.
    pub fn read(path: &Path) -> io::Result<Self> {
        let mut by_fold: HashMap<String, Vec<Entry>> = HashMap::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name();
            // Follows symlinks, as the engine does: `Modules -> modules`
            // is a directory.
            let meta = fs::metadata(entry.path()).ok();
            let fold = name.to_string_lossy().to_ascii_lowercase();
            by_fold.entry(fold).or_default().push(Entry {
                name,
                is_file: meta.as_ref().is_some_and(|m| m.is_file()),
                is_dir: meta.as_ref().is_some_and(|m| m.is_dir()),
            });
        }
        for entries in by_fold.values_mut() {
            entries.sort_by(|a, b| a.name.cmp(&b.name));
        }
        Ok(Self {
            path: path.to_path_buf(),
            by_fold,
        })
    }

    /// The directory listed.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every file whose name is `name` in any case, the exact spelling
    /// first and the rest in byte order. More than one means the directory
    /// holds case variants.
    pub fn files(&self, name: &str) -> Vec<PathBuf> {
        let mut hits: Vec<&Entry> = self
            .by_fold
            .get(&name.to_ascii_lowercase())
            .map(|v| v.iter().filter(|e| e.is_file).collect())
            .unwrap_or_default();
        hits.sort_by_key(|e| e.name != name);
        hits.iter().map(|e| self.path.join(&e.name)).collect()
    }

    /// The subdirectory named `name` in any case, the exact spelling
    /// preferred.
    pub fn dir(&self, name: &str) -> Option<PathBuf> {
        let entries = self.by_fold.get(&name.to_ascii_lowercase())?;
        entries
            .iter()
            .filter(|e| e.is_dir)
            .min_by_key(|e| e.name != name)
            .map(|e| self.path.join(&e.name))
    }

    /// Every file in the directory, in no particular order.
    pub fn file_names(&self) -> impl Iterator<Item = &OsString> + '_ {
        self.by_fold
            .values()
            .flatten()
            .filter(|e| e.is_file)
            .map(|e| &e.name)
    }

    /// The files whose names differ only in case, each group sorted by
    /// name and the groups sorted by their folded name.
    pub fn case_variants(&self) -> Vec<Vec<PathBuf>> {
        let mut groups: Vec<(&String, Vec<PathBuf>)> = self
            .by_fold
            .iter()
            .filter_map(|(fold, entries)| {
                let files: Vec<PathBuf> = entries
                    .iter()
                    .filter(|e| e.is_file)
                    .map(|e| self.path.join(&e.name))
                    .collect();
                (files.len() > 1).then_some((fold, files))
            })
            .collect();
        groups.sort_by(|a, b| a.0.cmp(b.0));
        groups.into_iter().map(|(_, files)| files).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rakata-game-listing-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_files_in_any_case_with_the_exact_spelling_first() {
        let dir = scratch("files");
        fs::write(dir.join("PLC_Invis.utp"), b"A").unwrap();
        fs::write(dir.join("plc_invis.utp"), b"B").unwrap();
        fs::write(dir.join("other.utp"), b"C").unwrap();
        fs::create_dir(dir.join("Sub")).unwrap();
        let l = Listing::read(&dir).unwrap();
        assert_eq!(
            l.files("plc_invis.utp"),
            [dir.join("plc_invis.utp"), dir.join("PLC_Invis.utp")]
        );
        assert_eq!(l.files("PLC_INVIS.UTP").len(), 2);
        assert_eq!(l.files("missing.utp"), Vec::<PathBuf>::new());
        assert_eq!(l.dir("sub"), Some(dir.join("Sub")));
        assert_eq!(l.dir("other.utp"), None);
        assert_eq!(
            l.case_variants(),
            vec![vec![dir.join("PLC_Invis.utp"), dir.join("plc_invis.utp")]]
        );
        assert_eq!(l.file_names().count(), 3);
        fs::remove_dir_all(dir).unwrap();
    }
}
