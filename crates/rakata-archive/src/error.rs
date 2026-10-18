//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;
use std::path::PathBuf;

use rakata_res::ResId;

/// Diagnostic codes for archives: the `RE02xx` range of the `rakata`
/// registry, plus `RE0001` for a file that could not be read. Each is a
/// stable string a caller may match on and a user may look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// A file could not be read.
    pub const IO: &str = "RE0001";
    /// The bytes do not begin with the signature and version of the archive
    /// kind asked for.
    pub const NOT_THIS_ARCHIVE: &str = "RE0201";
    /// A header, table or string block lies beyond the end of the bytes.
    pub const TRUNCATED: &str = "RE0202";
    /// A member's offset and length lie beyond the end of the bytes.
    pub const MEMBER_BEYOND_END: &str = "RE0203";
    /// A resource type id wider than the sixteen bits a type holds.
    pub const TYPE_TOO_WIDE: &str = "RE0204";
    /// An archive too large for its 32-bit offsets and lengths.
    pub const TOO_LARGE: &str = "RE0205";
    /// A member added under a name and type the archive already holds.
    pub const ALREADY_PRESENT: &str = "RE0206";
    /// A member replaced that the archive does not hold.
    pub const NOT_PRESENT: &str = "RE0207";
    /// A BIF whose header counts fixed resources.
    pub const FIXED_RESOURCES: &str = "RE0208";
    /// A BIF member index past the end of its table.
    pub const NO_SUCH_INDEX: &str = "RE0209";
}

/// An error in reading or writing an archive.
#[derive(Debug)]
pub enum Error {
    /// A file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// The bytes do not begin with the signature of the archive kind asked
    /// for.
    NotThisArchive {
        /// The kind asked for, as its signature reads: `RIM V1.0`.
        expected: &'static str,
        /// The first eight bytes found, lossily as text.
        found: String,
    },
    /// A header, table or string block lies beyond the end of the bytes.
    Truncated {
        /// What was being read.
        what: &'static str,
        /// The offset at which it begins.
        offset: usize,
        /// Its length.
        len: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// A member's offset and length lie beyond the end of the bytes.
    MemberBeyondEnd {
        /// The member.
        id: ResId,
        /// Its offset.
        offset: usize,
        /// Its length.
        len: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// A BIF member's offset and length lie beyond the end of the bytes.
    /// BIF members have no names, so the table position stands in.
    BifMemberBeyondEnd {
        /// The member's position in the table.
        index: usize,
        /// Its offset.
        offset: usize,
        /// Its length.
        len: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// A resource type id wider than the sixteen bits a type holds.
    TypeTooWide {
        /// The id as stored.
        type_id: u32,
        /// The member's position in the table.
        index: usize,
    },
    /// The archive is too large for its 32-bit offsets and lengths.
    TooLarge {
        /// The size it would have been.
        size: u64,
    },
    /// A member added under a name and type the archive already holds.
    AlreadyPresent {
        /// The member.
        id: ResId,
    },
    /// A member replaced that the archive does not hold.
    NotPresent {
        /// The member.
        id: ResId,
    },
    /// A BIF whose header counts fixed resources: a table no KotOR file has
    /// and this crate does not read.
    FixedResources {
        /// The count in the header.
        count: u32,
    },
    /// A BIF member index past the end of its table.
    NoSuchIndex {
        /// The index asked for.
        index: usize,
        /// How many members the BIF holds.
        len: usize,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::Io { .. } => codes::IO,
            Error::NotThisArchive { .. } => codes::NOT_THIS_ARCHIVE,
            Error::Truncated { .. } => codes::TRUNCATED,
            Error::MemberBeyondEnd { .. } | Error::BifMemberBeyondEnd { .. } => {
                codes::MEMBER_BEYOND_END
            }
            Error::TypeTooWide { .. } => codes::TYPE_TOO_WIDE,
            Error::TooLarge { .. } => codes::TOO_LARGE,
            Error::AlreadyPresent { .. } => codes::ALREADY_PRESENT,
            Error::NotPresent { .. } => codes::NOT_PRESENT,
            Error::FixedResources { .. } => codes::FIXED_RESOURCES,
            Error::NoSuchIndex { .. } => codes::NO_SUCH_INDEX,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Error::NotThisArchive { expected, found } => {
                write!(f, "not a `{expected}` archive: the file begins `{found}`")
            }
            Error::Truncated {
                what,
                offset,
                len,
                available,
            } => write!(
                f,
                "{what} at offset {offset} for {len} bytes lies beyond the end of the {available}-byte file"
            ),
            Error::MemberBeyondEnd {
                id,
                offset,
                len,
                available,
            } => write!(
                f,
                "member {id} at offset {offset} for {len} bytes lies beyond the end of the {available}-byte file"
            ),
            Error::BifMemberBeyondEnd {
                index,
                offset,
                len,
                available,
            } => write!(
                f,
                "member {index} at offset {offset} for {len} bytes lies beyond the end of the {available}-byte file"
            ),
            Error::TypeTooWide { type_id, index } => write!(
                f,
                "entry {index} has resource type id {type_id}, wider than the sixteen bits a type holds"
            ),
            Error::TooLarge { size } => {
                write!(
                    f,
                    "an archive of {size} bytes exceeds the format's 32-bit offsets"
                )
            }
            Error::AlreadyPresent { id } => {
                write!(f, "the archive already holds {id}; replace it instead")
            }
            Error::NotPresent { id } => write!(f, "the archive holds no {id}; add it instead"),
            Error::FixedResources { count } => write!(
                f,
                "the BIF counts {count} fixed resources, a table this crate does not read"
            ),
            Error::NoSuchIndex { index, len } => {
                write!(f, "the BIF has no member {index}; it holds {len}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
