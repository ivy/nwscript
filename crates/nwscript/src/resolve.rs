//! How a compilation finds its inputs. The library asks a [`Resolver`] for
//! includes and for `nwscript.nss` by name; the CLI answers from directories,
//! a test answers from strings, a game installation could answer from its
//! archives. Every answer is recorded as an [`InputRecord`].

use crate::{InputRecord, sha256_hex};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A file a resolver returned: a name to show the user and the bytes.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub path: String,
    pub bytes: Arc<[u8]>,
}

pub trait Resolver {
    /// An include by the name written in `#include "NAME"`, without the
    /// `.nss` extension. `Ok(None)` when no such file exists.
    fn include(&mut self, name: &str) -> io::Result<Option<Resolved>>;

    /// The engine's `nwscript.nss`.
    fn nwscript(&mut self) -> io::Result<Option<Resolved>>;

    /// Every file returned so far, in order, with role and hash.
    fn inputs(&self) -> &[InputRecord];
}

/// Resolves from directories, searched in order, matching names
/// case-insensitively as the games do.
#[derive(Debug, Default)]
pub struct FsResolver {
    pub include_dirs: Vec<PathBuf>,
    /// An explicit `nwscript.nss`; otherwise it is searched like an include.
    pub nwscript: Option<PathBuf>,
    inputs: Vec<InputRecord>,
}

impl FsResolver {
    pub fn new(include_dirs: Vec<PathBuf>, nwscript: Option<PathBuf>) -> Self {
        FsResolver {
            include_dirs,
            nwscript,
            inputs: Vec::new(),
        }
    }

    fn find(&self, file: &str) -> Option<PathBuf> {
        self.include_dirs.iter().find_map(|d| find_ci(d, file))
    }

    fn load(&mut self, path: &Path, role: &str) -> io::Result<Resolved> {
        let bytes: Arc<[u8]> = std::fs::read(path)?.into();
        let shown = path.display().to_string();
        self.inputs.push(InputRecord {
            role: role.into(),
            path: shown.clone(),
            sha256: sha256_hex(&bytes),
        });
        Ok(Resolved { path: shown, bytes })
    }
}

/// The entry of `dir` named `file`, ignoring ASCII case.
fn find_ci(dir: &Path, file: &str) -> Option<PathBuf> {
    let exact = dir.join(file);
    if exact.is_file() {
        return Some(exact);
    }
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(file))
        .map(|e| e.path())
        .filter(|p| p.is_file())
}

impl Resolver for FsResolver {
    fn include(&mut self, name: &str) -> io::Result<Option<Resolved>> {
        match self.find(&format!("{name}.nss")) {
            Some(p) => self.load(&p, "include").map(Some),
            None => Ok(None),
        }
    }

    fn nwscript(&mut self) -> io::Result<Option<Resolved>> {
        let path = match &self.nwscript {
            Some(p) => Some(p.clone()),
            None => self.find("nwscript.nss"),
        };
        match path {
            Some(p) => self.load(&p, "nwscript").map(Some),
            None => Ok(None),
        }
    }

    fn inputs(&self) -> &[InputRecord] {
        &self.inputs
    }
}

/// Resolves from an in-memory map of name to contents, for tests and for
/// embedders that hold sources themselves. Names are matched ignoring case.
#[derive(Debug, Default)]
pub struct MemResolver {
    files: HashMap<String, Arc<[u8]>>,
    inputs: Vec<InputRecord>,
}

impl MemResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file by its name with extension, such as `k_inc_generic.nss` or `nwscript.nss`.
    pub fn with(mut self, name: &str, contents: impl Into<Vec<u8>>) -> Self {
        self.files
            .insert(name.to_ascii_lowercase(), contents.into().into());
        self
    }

    fn get(&mut self, file: &str, role: &str) -> Option<Resolved> {
        let bytes = self.files.get(&file.to_ascii_lowercase())?.clone();
        self.inputs.push(InputRecord {
            role: role.into(),
            path: file.into(),
            sha256: sha256_hex(&bytes),
        });
        Some(Resolved {
            path: file.into(),
            bytes,
        })
    }
}

impl Resolver for MemResolver {
    fn include(&mut self, name: &str) -> io::Result<Option<Resolved>> {
        Ok(self.get(&format!("{name}.nss"), "include"))
    }

    fn nwscript(&mut self) -> io::Result<Option<Resolved>> {
        Ok(self.get("nwscript.nss", "nwscript"))
    }

    fn inputs(&self) -> &[InputRecord] {
        &self.inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_resolver_records_inputs_case_insensitively() {
        let mut r = MemResolver::new()
            .with("K_INC_Utility.nss", "int x;")
            .with("nwscript.nss", "void main();");
        assert_eq!(
            &*r.include("k_inc_utility").unwrap().unwrap().bytes,
            b"int x;"
        );
        assert!(r.include("missing").unwrap().is_none());
        assert!(r.nwscript().unwrap().is_some());
        let roles: Vec<&str> = r.inputs().iter().map(|i| i.role.as_str()).collect();
        assert_eq!(roles, ["include", "nwscript"]);
        assert_eq!(r.inputs()[0].sha256.len(), 64);
    }

    #[test]
    fn fs_resolver_searches_dirs_in_order_ignoring_case() {
        let base = std::env::temp_dir().join(format!("nws-resolve-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("K_INC_X.NSS"), "from b").unwrap();
        std::fs::write(b.join("nwscript.nss"), "ns").unwrap();
        let mut r = FsResolver::new(vec![a, b], None);
        assert_eq!(&*r.include("k_inc_x").unwrap().unwrap().bytes, b"from b");
        assert!(r.nwscript().unwrap().is_some());
        assert_eq!(r.inputs().len(), 2);
        std::fs::remove_dir_all(base).unwrap();
    }
}
