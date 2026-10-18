//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;

use crate::NodeRef;

/// Diagnostic codes for dialogues: the `RE11xx` and `RW11xx` range of the
/// `rakata` registry. Each is a stable string a caller may match on and a
/// user may look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// A GFF whose file type is not `DLG `.
    pub const NOT_DIALOG: &str = "RE1101";
    /// A node key that matches no node of the dialogue.
    pub const NO_SUCH_NODE: &str = "RE1102";
    /// A node key that matches more than one node of the dialogue.
    pub const AMBIGUOUS_KEY: &str = "RE1103";
    /// A node key that does not parse.
    pub const BAD_KEY: &str = "RE1104";
    /// A link or start that cannot be made: between two nodes of one kind,
    /// a start to a reply, or at a position past the end of its list.
    pub const BAD_LINK: &str = "RE1106";
    /// A reordering that is not a permutation of the list.
    pub const BAD_ORDER: &str = "RE1107";
    /// Text for a new node holding a character Windows-1252 cannot encode.
    pub const UNENCODABLE: &str = "RE1108";
    /// A link to a node the dialogue does not have.
    pub const DANGLING_LINK: &str = "RW1101";
    /// A script or condition no layer of the install holds.
    pub const MISSING_SCRIPT: &str = "RW1102";
    /// A script whose bytecode the decompiler could not read back.
    pub const UNREADABLE_SCRIPT: &str = "RW1103";
    /// A voiced node whose voice-over no layer or `streamwaves/` holds.
    pub const MISSING_VOICE: &str = "RW1104";
    /// A node whose strref lies past the end of the talk table.
    pub const STRREF_PAST_TLK: &str = "RW1105";
    /// A condition reading a global `globalcat.2da` does not declare.
    pub const UNDECLARED_GLOBAL: &str = "RW1106";
    /// A node no start reaches, by any chain of links.
    pub const UNREACHED_NODE: &str = "RW1107";
    /// A voiced node whose voice-over has no lip-sync file.
    pub const MISSING_LIP: &str = "RW1108";
    /// A condition passing a computed name to a `GetGlobal*` routine, so
    /// the global it reads cannot be checked.
    pub const UNDECIDABLE_GLOBAL: &str = "RW1109";
    /// A `SoundExists` that disagrees with the node's `Sound` and
    /// `VO_ResRef`.
    pub const SOUND_EXISTS: &str = "RW1110";
}

/// An error in reading a dialogue or addressing one of its nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The GFF could not be read; the code is `rakata-gff`'s.
    Gff(rakata_gff::Error),
    /// A GFF whose file type is not `DLG `.
    NotDialog {
        /// The dialogue's name.
        dialog: String,
        /// The file type found, trimmed.
        found: String,
    },
    /// A key that matches no node.
    NoSuchNode {
        /// The key, as [`crate::NodeKey`] displays it.
        key: String,
        /// The dialogue's name.
        dialog: String,
    },
    /// A key that matches more than one node.
    AmbiguousKey {
        /// The key, as [`crate::NodeKey`] displays it.
        key: String,
        /// The dialogue's name.
        dialog: String,
        /// Every node it matches, entries first, each list in index order.
        nodes: Vec<NodeRef>,
    },
    /// A key that does not parse.
    BadKey {
        /// The text.
        text: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A link or start that cannot be made.
    BadLink {
        /// The dialogue's name.
        dialog: String,
        /// Why, naming the nodes or the position.
        reason: String,
    },
    /// A reordering that is not a permutation of the list.
    BadOrder {
        /// The dialogue's name.
        dialog: String,
        /// The list, `EntryList` or `ReplyList`.
        list: &'static str,
        /// How many nodes it holds.
        len: usize,
    },
    /// Text for a new node that the game's code page cannot hold.
    Unencodable {
        /// The field.
        field: &'static str,
        /// The first character it cannot encode.
        character: char,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::Gff(e) => e.code(),
            Error::NotDialog { .. } => codes::NOT_DIALOG,
            Error::NoSuchNode { .. } => codes::NO_SUCH_NODE,
            Error::AmbiguousKey { .. } => codes::AMBIGUOUS_KEY,
            Error::BadKey { .. } => codes::BAD_KEY,
            Error::BadLink { .. } => codes::BAD_LINK,
            Error::BadOrder { .. } => codes::BAD_ORDER,
            Error::Unencodable { .. } => codes::UNENCODABLE,
        }
    }

    /// The message without the code.
    pub fn message(&self) -> String {
        match self {
            Error::Gff(e) => {
                let text = e.to_string();
                text.strip_prefix(e.code())
                    .and_then(|t| t.strip_prefix(": "))
                    .unwrap_or(&text)
                    .to_string()
            }
            Error::NotDialog { dialog, found } => {
                format!("{dialog} is a `{found}` GFF, not a dialogue (`DLG `)")
            }
            Error::NoSuchNode { key, dialog } => {
                format!("{dialog} has no node matching {key}")
            }
            Error::AmbiguousKey { key, dialog, nodes } => format!(
                "{key} matches {} nodes of {dialog}: {}",
                nodes.len(),
                nodes
                    .iter()
                    .map(NodeRef::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Error::BadKey { text, reason } => {
                format!("cannot parse the node key `{text}`: {reason}")
            }
            Error::BadLink { dialog, reason } => format!("{dialog}: {reason}"),
            Error::BadOrder { dialog, list, len } => format!(
                "{dialog}: a new order for {list} names each of its {len} indices once, from 0"
            ),
            Error::Unencodable { field, character } => format!(
                "`{character}` in a node's {field} has no Windows-1252 byte, the game's code page"
            ),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for Error {}

impl From<rakata_gff::Error> for Error {
    fn from(e: rakata_gff::Error) -> Self {
        Error::Gff(e)
    }
}
