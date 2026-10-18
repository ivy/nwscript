//! The errors and warnings this crate reports, each with a registered
//! diagnostic code.

use std::fmt;
use std::path::PathBuf;

/// Diagnostic codes for the index: the `RE15xx` and `RW15xx` range of the
/// `rakata` registry.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// SQLite refused an operation on the database.
    pub const DATABASE: &str = "RE1501";
    /// A database written by another version of the schema.
    pub const SCHEMA: &str = "RE1502";
    /// A tree the database does not hold.
    pub const NO_SUCH_TREE: &str = "RE1503";
    /// A query SQLite rejects, or one that would change the database.
    pub const QUERY: &str = "RE1504";
    /// A file of changed paths that could not be read.
    pub const CHANGED_LIST: &str = "RE1505";
    /// A changed path the tree does not hold.
    pub const CHANGED_MISSING: &str = "RW1501";
    /// A file of the tree that could not be read.
    pub const UNREADABLE: &str = "RW1502";
    /// A tree without `nwscript.nss`: calls name routines by number.
    pub const NO_ENGINE_TABLE: &str = "RW1503";
    /// Files absent from the parent tree that the changed list does not
    /// name.
    pub const UNLISTED: &str = "RW1504";
}

/// An error of the index.
#[derive(Debug)]
pub enum Error {
    /// SQLite refused an operation.
    Database {
        what: String,
        source: rusqlite::Error,
    },
    /// The database's schema version is not this build's.
    Schema {
        path: PathBuf,
        found: String,
        expected: u32,
    },
    /// A tree label the database does not hold.
    NoSuchTree { label: String, known: Vec<String> },
    /// The database file or its folder could not be written.
    Io {
        what: String,
        source: std::io::Error,
    },
    /// A query SQLite rejects.
    Query {
        sql: String,
        source: rusqlite::Error,
    },
    /// The changed-paths file could not be read.
    Changed {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl Error {
    pub(crate) fn db(what: impl Into<String>) -> impl FnOnce(rusqlite::Error) -> Error {
        let what = what.into();
        move |source| Error::Database { what, source }
    }

    /// The registered diagnostic code.
    pub fn code(&self) -> &'static str {
        match self {
            Error::Database { .. } | Error::Io { .. } => codes::DATABASE,
            Error::Schema { .. } => codes::SCHEMA,
            Error::NoSuchTree { .. } => codes::NO_SUCH_TREE,
            Error::Query { .. } => codes::QUERY,
            Error::Changed { .. } => codes::CHANGED_LIST,
        }
    }

    /// The message without the code.
    pub fn message(&self) -> String {
        match self {
            Error::Database { what, source } => format!("{what}: {source}"),
            Error::Io { what, source } => format!("{what}: {source}"),
            Error::Schema {
                path,
                found,
                expected,
            } => format!(
                "`{}` holds an index of schema {found}; this build reads schema {expected}: build it again into a new file",
                path.display()
            ),
            Error::NoSuchTree { label, known } => {
                if known.is_empty() {
                    format!("the index holds no tree `{label}`: it holds none")
                } else {
                    format!(
                        "the index holds no tree `{label}`; it holds {}",
                        known
                            .iter()
                            .map(|k| format!("`{k}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            Error::Query { sql, source } => format!("the query `{sql}` failed: {source}"),
            Error::Changed { path, source } => {
                format!(
                    "cannot read the changed paths in {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for Error {}

/// The crate's result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A warning a build reports and goes on.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Warning {
    pub code: &'static str,
    pub message: String,
}
