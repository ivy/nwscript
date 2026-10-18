//! The reference checks: what a dialogue names outside itself, asked of the
//! install. A node's script and a link's condition resolve to bytecode; a
//! voiced node's voice-over has its file; a strref lies inside the talk
//! table; a global a condition reads is declared in `globalcat.2da`.
//!
//! The crate does no I/O, so the install answers through [`References`];
//! `rakata`'s authoring API implements it over an install and what a mod
//! has written, and runs the checks before every dialogue it writes.

use std::fmt;

use rakata_res::{ResRef, ResType};

use crate::error::codes;
use crate::key::NodeKey;
use crate::model::{Dialog, Link, NodeRef};

/// What the checks ask of the install.
pub trait References {
    /// Whether the engine would find a resource of this name and type: a
    /// script as `ncs`, a voice-over as `wav`.
    fn resolves(&mut self, name: &ResRef, ty: ResType) -> bool;

    /// How many strings the talk table holds; `None` when there is none to
    /// ask, and the strref check is skipped.
    fn talk_table_len(&mut self) -> Option<u32>;

    /// The globals the condition script reads, by the names its bytecode
    /// passes to the engine's `GetGlobal*` routines; empty when the script
    /// cannot be read.
    fn globals_read(&mut self, condition: &ResRef) -> Vec<String>;

    /// Whether `globalcat.2da` declares the global, without regard to
    /// case; `None` when there is no table to ask, and the check is
    /// skipped.
    fn declares_global(&mut self, name: &str) -> Option<bool>;
}

/// Where in a dialogue a finding lies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Place {
    /// A node's own field: its script, voice-over or text.
    Node(NodeRef),
    /// The condition of the link from one node to another.
    Link { from: NodeRef, to: NodeRef },
    /// The condition of a start into an entry.
    Start { to: NodeRef },
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Place::Node(n) => write!(f, "{n}"),
            Place::Link { from, to } => write!(f, "the link {from} -> {to}"),
            Place::Start { to } => write!(f, "the start into {to}"),
        }
    }
}

/// One reference the install cannot satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Finding {
    /// The registered code: `RW1102`, `RW1104`, `RW1105` or `RW1106`.
    pub code: &'static str,
    /// Where it lies.
    pub place: Place,
    /// The stable key of the node it lies on (for a link, the node the
    /// link leaves; for a start, the entry it enters), as
    /// [`NodeKey::of`] chooses it.
    pub key: NodeKey,
    /// What is referred to: the script, the voice resref, the strref, or
    /// the global.
    pub subject: String,
    /// For an undeclared global, the condition that reads it.
    pub via: Option<String>,
}

impl Finding {
    /// The message without the code, naming the dialogue, the node key and
    /// the subject.
    pub fn message(&self, dialog: &str) -> String {
        let at = match &self.place {
            Place::Node(_) => format!("{dialog} {}", self.key),
            other => format!("{dialog} {other} ({})", self.key),
        };
        let what = match self.code {
            codes::MISSING_SCRIPT => format!(
                "names the script {}, which no layer of the install holds",
                self.subject
            ),
            codes::MISSING_VOICE => format!(
                "is voiced by {}, and no layer or streamwaves/ holds {}.wav",
                self.subject, self.subject
            ),
            codes::STRREF_PAST_TLK => format!(
                "shows strref {}, past the end of the talk table",
                self.subject
            ),
            _ => format!(
                "has the condition {}, which reads the global {}, and globalcat.2da does not declare it",
                self.via.as_deref().unwrap_or("?"),
                self.subject
            ),
        };
        format!("{at} {what}")
    }

    /// Whether `other` is the same finding: the same code, place and
    /// subject. A finding of an edited dialogue that its base also has is
    /// the base's, not the edit's.
    pub fn same_as(&self, other: &Finding) -> bool {
        self.code == other.code && self.place == other.place && self.subject == other.subject
    }
}

/// Every reference of `dialog` the install cannot satisfy, nodes first
/// (entries, then replies, each in index order: script, voice, strref),
/// then each node's links, then the starts.
pub fn check(dialog: &Dialog, refs: &mut dyn References) -> Vec<Finding> {
    let mut out = Vec::new();
    let tlk = refs.talk_table_len();
    for node in dialog.all() {
        let key = NodeKey::of(dialog, node.at);
        let find = |code, subject: String| Finding {
            code,
            place: Place::Node(node.at),
            key: key.clone(),
            subject,
            via: None,
        };
        if let Some(s) = node.script()
            && !refs.resolves(&s, ResType::Ncs)
        {
            out.push(find(codes::MISSING_SCRIPT, s.to_string()));
        }
        if let Some(vo) = node.voice()
            && !refs.resolves(&vo, ResType::Wav)
        {
            out.push(find(codes::MISSING_VOICE, vo.to_string()));
        }
        if let (Some(s), Some(len)) = (node.strref(), tlk)
            && s >= len
        {
            out.push(find(codes::STRREF_PAST_TLK, s.to_string()));
        }
    }
    for node in dialog.all() {
        for link in node.links() {
            let place = Place::Link {
                from: node.at,
                to: link.target,
            };
            condition(&link, place, NodeKey::of(dialog, node.at), refs, &mut out);
        }
    }
    for link in dialog.starts() {
        let place = Place::Start { to: link.target };
        let key = NodeKey::of(dialog, link.target);
        condition(&link, place, key, refs, &mut out);
    }
    out
}

fn condition(
    link: &Link<'_>,
    place: Place,
    key: NodeKey,
    refs: &mut dyn References,
    out: &mut Vec<Finding>,
) {
    let Some(c) = link.condition() else {
        return;
    };
    if !refs.resolves(&c, ResType::Ncs) {
        out.push(Finding {
            code: codes::MISSING_SCRIPT,
            place,
            key,
            subject: c.to_string(),
            via: None,
        });
        return;
    }
    for global in refs.globals_read(&c) {
        if refs.declares_global(&global) == Some(false) {
            out.push(Finding {
                code: codes::UNDECLARED_GLOBAL,
                place: place.clone(),
                key: key.clone(),
                subject: global,
                via: Some(c.to_string()),
            });
        }
    }
}
