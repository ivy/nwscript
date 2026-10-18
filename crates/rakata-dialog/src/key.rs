//! Stable keys: a node addressed by what it says rather than where it lies,
//! so an edit made against one build of a dialogue lands on the same line in
//! the next.

use std::fmt;
use std::str::FromStr;

use rakata_res::{LocalizedString, ResRef};

use crate::error::{Error, Result};
use crate::model::{Dialog, Kind, NodeRef};

/// How to find one node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeKey {
    /// The node voiced by this resref (`VO_ResRef`), without regard to case.
    Voice(ResRef),
    /// The node whose text is this strref.
    StrRef(u32),
    /// The node whose text, resolved through the talk table and trimmed of
    /// whitespace, is this text, trimmed.
    Text(String),
    /// The node at this index. Indices move when a list is reordered, so
    /// this key is fragile and says so wherever it is shown.
    Index(Kind, usize),
}

impl NodeKey {
    /// Whether the key names a position rather than content.
    pub fn is_fragile(&self) -> bool {
        matches!(self, NodeKey::Index(..))
    }

    /// The stable key that names the node at `at` and no other: its voice
    /// resref, else its strref, else its embedded text, whichever is first
    /// to match it alone; an index key (fragile) when none does.
    pub fn of(dialog: &Dialog, at: NodeRef) -> NodeKey {
        let index = NodeKey::Index(at.kind, at.index);
        let Some(node) = dialog.node(at) else {
            return index;
        };
        let candidates = [
            node.voice().map(NodeKey::Voice),
            node.strref().map(NodeKey::StrRef),
            node.embedded_text()
                .filter(|t| !t.trim().is_empty())
                .map(NodeKey::Text),
        ];
        candidates
            .into_iter()
            .flatten()
            .find(|k| k.resolve(dialog).ok() == Some(at))
            .unwrap_or(index)
    }

    /// The one node of `dialog` the key names. Text keys compare with the
    /// text the node embeds, as when no talk table is at hand; a caller
    /// holding one uses [`NodeKey::resolve_with`].
    pub fn resolve(&self, dialog: &Dialog) -> Result<NodeRef> {
        self.resolve_with(dialog, &mut |_| None)
    }

    /// The one node of `dialog` the key names, with `talk` answering a
    /// localized string's text for text keys (the talk table's, or the
    /// embedded substring's; `None` when it has none). Fails naming the key
    /// and the dialogue when no node matches, and naming every match when
    /// more than one does.
    pub fn resolve_with(
        &self,
        dialog: &Dialog,
        talk: &mut dyn FnMut(&LocalizedString) -> Option<String>,
    ) -> Result<NodeRef> {
        let matches: Vec<NodeRef> = match self {
            NodeKey::Index(kind, index) => {
                let at = NodeRef {
                    kind: *kind,
                    index: *index,
                };
                dialog.node(at).map(|n| n.at).into_iter().collect()
            }
            NodeKey::Voice(vo) => dialog
                .all()
                .filter(|n| n.voice().as_ref() == Some(vo))
                .map(|n| n.at)
                .collect(),
            NodeKey::StrRef(s) => dialog
                .all()
                .filter(|n| n.strref() == Some(*s))
                .map(|n| n.at)
                .collect(),
            NodeKey::Text(t) => {
                let want = t.trim();
                dialog
                    .all()
                    .filter(|n| {
                        let text = n.text();
                        let shown = talk(&text).or_else(|| n.embedded_text());
                        shown.is_some_and(|s| s.trim() == want)
                    })
                    .map(|n| n.at)
                    .collect()
            }
        };
        match matches.as_slice() {
            [one] => Ok(*one),
            [] => Err(Error::NoSuchNode {
                key: self.to_string(),
                dialog: dialog.name().to_string(),
            }),
            _ => Err(Error::AmbiguousKey {
                key: self.to_string(),
                dialog: dialog.name().to_string(),
                nodes: matches,
            }),
        }
    }
}

impl From<NodeRef> for NodeKey {
    /// The index key of a node, as an edit that just made it returns.
    fn from(at: NodeRef) -> Self {
        NodeKey::Index(at.kind, at.index)
    }
}

impl fmt::Display for NodeKey {
    /// `voice:nm14acjuha07024_`, `strref:38576`, `text:"Hello."`, and
    /// `index:E3 (fragile)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeKey::Voice(vo) => write!(f, "voice:{vo}"),
            NodeKey::StrRef(s) => write!(f, "strref:{s}"),
            NodeKey::Text(t) => write!(f, "text:{t:?}"),
            NodeKey::Index(kind, index) => write!(
                f,
                "index:{} (fragile)",
                NodeRef {
                    kind: *kind,
                    index: *index
                }
            ),
        }
    }
}

impl FromStr for NodeKey {
    type Err = Error;

    /// `voice:RESREF`, `strref:N`, `text:TEXT`, or an index as `E3`, `R0`,
    /// `entry:3` or `reply:0`.
    fn from_str(s: &str) -> Result<Self> {
        let bad = |reason| Error::BadKey {
            text: s.to_string(),
            reason,
        };
        let index = |kind, n: &str| {
            n.parse::<usize>()
                .map(|i| NodeKey::Index(kind, i))
                .map_err(|_| bad("an index is a number"))
        };
        if let Some((prefix, rest)) = s.split_once(':') {
            return match prefix.to_ascii_lowercase().as_str() {
                "voice" => ResRef::new(rest)
                    .map(NodeKey::Voice)
                    .map_err(|_| bad("a voice resref is at most sixteen bytes")),
                "strref" => rest
                    .parse()
                    .map(NodeKey::StrRef)
                    .map_err(|_| bad("a strref is a number")),
                "text" => Ok(NodeKey::Text(rest.to_string())),
                "entry" => index(Kind::Entry, rest),
                "reply" => index(Kind::Reply, rest),
                _ => Err(bad(
                    "a key is voice:, strref:, text:, entry: or reply:, or an index as E3 or R0",
                )),
            };
        }
        match s.as_bytes().first() {
            Some(b'E' | b'e') => index(Kind::Entry, &s[1..]),
            Some(b'R' | b'r') => index(Kind::Reply, &s[1..]),
            _ => Err(bad(
                "a key is voice:, strref:, text:, entry: or reply:, or an index as E3 or R0",
            )),
        }
    }
}
