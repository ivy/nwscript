//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;
use std::path::PathBuf;

use rakata_res::ResId;

/// Diagnostic codes for the install: the `RE04xx` and `RW04xx` ranges of
/// the `rakata` registry. Each is a stable string a caller may match on and
/// a user may look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; the façade
// depends on this crate, not the reverse, so this crate carries the strings.
pub mod codes {
    /// A directory that is not a game install: no `chitin.key` in it.
    pub const NOT_AN_INSTALL: &str = "RE0401";
    /// A BIF the KEY names is missing from the install.
    pub const MISSING_BIF: &str = "RE0402";
    /// A module name with no capsule under `modules/`.
    pub const NO_SUCH_MODULE: &str = "RE0403";
    /// A resource no layer of the install holds.
    pub const NOT_FOUND: &str = "RE0404";
    /// Two Override files that differ only in case: the engine's choice
    /// between them is not established.
    pub const CASE_VARIANTS: &str = "RW0401";
    /// A texture pack named by something other than `tpa`, `tpb` or `tpc`.
    pub const UNKNOWN_TEXTURE_PACK: &str = "RE0405";
    /// The selected quality pack is not under `TexturePacks/`: the texture
    /// layer holds only the GUI pack.
    pub const MISSING_TEXTURE_PACK: &str = "RW0402";
}

/// An error in opening or reading an install.
#[derive(Debug)]
pub enum Error {
    /// The directory holds no `chitin.key`.
    NotAnInstall {
        /// The directory named.
        dir: PathBuf,
    },
    /// A BIF the KEY names is not in the install.
    MissingBif {
        /// The BIF's path as the KEY writes it.
        name: String,
        /// The KEY that names it.
        key: PathBuf,
    },
    /// No capsule under `modules/` carries the module's name.
    NoSuchModule {
        /// The module name as given.
        module: String,
        /// The modules directory searched, when the install has one.
        dir: Option<PathBuf>,
    },
    /// No layer of the install holds the resource.
    NotFound {
        /// The resource asked for.
        id: ResId,
        /// The module it was asked in, if any.
        module: Option<String>,
    },
    /// A texture pack named by something other than `tpa`, `tpb` or `tpc`.
    UnknownTexturePack {
        /// The name as given.
        name: String,
    },
    /// A file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// An archive in the install did not parse.
    Archive {
        /// The archive.
        path: PathBuf,
        /// What the reader said.
        source: rakata_archive::Error,
    },
    /// An archive that failed to open earlier in this install's life; the
    /// first failure is kept as its code and message, since the install
    /// opens each archive once.
    Unreadable {
        /// The archive.
        path: PathBuf,
        /// The code of the first failure.
        code: &'static str,
        /// Its message.
        message: String,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error. I/O and archive
    /// errors carry the codes of the crates that report them.
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotAnInstall { .. } => codes::NOT_AN_INSTALL,
            Error::MissingBif { .. } => codes::MISSING_BIF,
            Error::NoSuchModule { .. } => codes::NO_SUCH_MODULE,
            Error::NotFound { .. } => codes::NOT_FOUND,
            Error::UnknownTexturePack { .. } => codes::UNKNOWN_TEXTURE_PACK,
            Error::Io { .. } => rakata_archive::codes::IO,
            Error::Archive { source, .. } => source.code(),
            Error::Unreadable { code, .. } => code,
        }
    }

    /// The message without its code prefix, as a diagnostic carries it.
    pub fn message(&self) -> String {
        let text = self.to_string();
        match text.split_once(": ") {
            Some((code, rest)) if code == self.code() => rest.to_owned(),
            _ => text,
        }
    }

    /// The same failure, free of the unclonable source, for keeping.
    pub(crate) fn remembered(&self) -> Error {
        let path = match self {
            Error::Io { path, .. }
            | Error::Archive { path, .. }
            | Error::Unreadable { path, .. } => path.clone(),
            _ => PathBuf::new(),
        };
        Error::Unreadable {
            path,
            code: self.code(),
            message: self.message(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        match self {
            Error::NotAnInstall { dir } => write!(
                f,
                "{code}: `{}` is not a game install: it holds no chitin.key",
                dir.display()
            ),
            Error::MissingBif { name, key } => write!(
                f,
                "{code}: `{}` names `{name}`, which the install does not hold",
                key.display()
            ),
            Error::NoSuchModule {
                module,
                dir: Some(dir),
            } => write!(
                f,
                "{code}: no capsule named `{module}` under `{}`",
                dir.display()
            ),
            Error::NoSuchModule { module, dir: None } => {
                write!(f, "{code}: no modules directory to find `{module}` in")
            }
            Error::NotFound {
                id,
                module: Some(m),
            } => write!(
                f,
                "{code}: no `{id}` in Override, module `{m}`, the texture packs or the game's own files"
            ),
            Error::NotFound { id, module: None } => write!(
                f,
                "{code}: no `{id}` in Override, the texture packs or the game's own files; a module's copy needs its module named"
            ),
            Error::UnknownTexturePack { name } => {
                write!(f, "{code}: `{name}` is not a texture pack: tpa, tpb or tpc")
            }
            Error::Io { path, source } => {
                write!(f, "{code}: cannot read `{}`: {source}", path.display())
            }
            Error::Archive { path, source } => {
                let text = source.to_string();
                let rest = text.strip_prefix(code).and_then(|t| t.strip_prefix(": "));
                write!(f, "{code}: `{}`: {}", path.display(), rest.unwrap_or(&text))
            }
            Error::Unreadable { path, message, .. } => {
                write!(f, "{code}: `{}`: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Archive { source, .. } => Some(source),
            _ => None,
        }
    }
}
