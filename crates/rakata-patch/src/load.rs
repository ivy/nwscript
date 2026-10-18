//! Reading an installer's folder: `namespaces.ini` when it offers options,
//! then the selected option's changes file. The one place in the crate that
//! touches the file system.

use crate::changes::Changes;
use crate::ini::Ini;
use crate::issue::Issue;
use crate::namespaces::{Namespace, Namespaces};
use std::path::{Path, PathBuf};

/// A file the reader read, in the order it read them; `Issue::file` indexes this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

/// What `tslpatchdata` holds for one install.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub sources: Vec<Source>,
    pub namespaces: Option<Namespaces>,
    /// The option chosen, when the installer offers any.
    pub option: Option<Namespace>,
    pub changes: Changes,
    /// The folder the option's files are in: `tslpatchdata` or its `DataPath`.
    pub data_dir: PathBuf,
    pub issues: Vec<Issue>,
}

/// Why nothing could be loaded.
#[derive(Debug)]
pub enum LoadError {
    /// A file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// `namespaces.ini` offers options and none was chosen.
    OptionRequired { options: Vec<Namespace> },
    /// No option has that number.
    NoSuchOption {
        number: u32,
        options: Vec<Namespace>,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io { path, source } => {
                write!(f, "cannot read `{}`: {source}", path.display())
            }
            LoadError::OptionRequired { options } => {
                writeln!(f, "this installer offers options; pass --option N:")?;
                list_options(f, options)
            }
            LoadError::NoSuchOption { number, options } => {
                writeln!(f, "no option {number}; the installer offers:")?;
                list_options(f, options)
            }
        }
    }
}

fn list_options(f: &mut std::fmt::Formatter<'_>, options: &[Namespace]) -> std::fmt::Result {
    for o in options {
        writeln!(
            f,
            "  {}  {}",
            o.number,
            o.name.as_deref().unwrap_or(&o.section)
        )?;
    }
    Ok(())
}

impl std::error::Error for LoadError {}

/// Read `tslpatchdata`, choosing `option` when there is a `namespaces.ini`.
pub fn load(tslpatchdata: &Path, option: Option<u32>) -> Result<Loaded, LoadError> {
    let mut sources = Vec::new();
    let mut issues = Vec::new();
    let ns_path = find_file(tslpatchdata, "namespaces.ini");
    let (namespaces, chosen, data_dir, changes_path) = match ns_path {
        Some(ns_path) => {
            let bytes = read(&ns_path)?;
            let ini = Ini::parse(&bytes, sources.len(), &mut issues);
            let ns = Namespaces::parse(&ini, sources.len(), &mut issues);
            sources.push(Source {
                path: ns_path,
                bytes,
            });
            let chosen = match option {
                None => {
                    return Err(LoadError::OptionRequired {
                        options: ns.options,
                    });
                }
                Some(n) => match ns.option(n) {
                    Some(o) => o.clone(),
                    None => {
                        return Err(LoadError::NoSuchOption {
                            number: n,
                            options: ns.options,
                        });
                    }
                },
            };
            let data_dir = match &chosen.data_path {
                Some(p) => find_dir(tslpatchdata, p).unwrap_or_else(|| tslpatchdata.join(p)),
                None => tslpatchdata.to_path_buf(),
            };
            let changes_path = find_file(&data_dir, &chosen.ini_name)
                .unwrap_or_else(|| data_dir.join(&chosen.ini_name));
            (Some(ns), Some(chosen), data_dir, changes_path)
        }
        None => (
            None,
            None,
            tslpatchdata.to_path_buf(),
            find_file(tslpatchdata, "changes.ini")
                .unwrap_or_else(|| tslpatchdata.join("changes.ini")),
        ),
    };
    let bytes = read(&changes_path)?;
    let ini = Ini::parse(&bytes, sources.len(), &mut issues);
    let changes = Changes::parse(&ini, sources.len(), &mut issues);
    sources.push(Source {
        path: changes_path,
        bytes,
    });
    Ok(Loaded {
        sources,
        namespaces,
        option: chosen,
        changes,
        data_dir,
        issues,
    })
}

fn read(path: &Path) -> Result<Vec<u8>, LoadError> {
    std::fs::read(path).map_err(|source| LoadError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// `dir/name` by the spelling the file system has, since the inis were
/// written on Windows and the folder may have been unpacked anywhere.
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    find_entry(dir, name, false)
}

fn find_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    find_entry(dir, name, true)
}

fn find_entry(dir: &Path, name: &str, want_dir: bool) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_dir() == want_dir && exact.exists() {
        return Some(exact);
    }
    let entries = std::fs::read_dir(dir).ok()?;
    entries.flatten().map(|e| e.path()).find(|p| {
        p.is_dir() == want_dir
            && p.file_name()
                .is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case(name))
    })
}
