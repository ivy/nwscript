//! The reference checks: what a dialogue names outside itself, asked of the
//! install. A node's script and a link's condition resolve to bytecode; a
//! voiced node's voice-over has its file (a node is voiced, as
//! [`crate::Node::voiced`] says, when it names a voice-over, no `Sound`,
//! and a `SoundExists` other than `0`); a strref lies inside the talk
//! table; a global a condition reads is declared in `globalcat.2da`; the
//! dialogue's own `EndConversation` and `EndConverAbort` scripts resolve to
//! bytecode.
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

    /// The globals the condition script reads, as [`References::globals_read`]
    /// names them, with the `GetGlobal*` calls whose name is computed
    /// rather than a literal, which no check can follow. By default, the
    /// names alone.
    fn global_reads(&mut self, condition: &ResRef) -> GlobalReads {
        GlobalReads {
            names: self.globals_read(condition),
            ..GlobalReads::default()
        }
    }

    /// Whether `globalcat.2da` declares any global whose name starts with
    /// `prefix`, without regard to case; `None` when there is no table to
    /// ask, or the implementation cannot say.
    fn declares_prefix(&mut self, prefix: &str) -> Option<bool> {
        let _ = prefix;
        None
    }

    /// Whether `globalcat.2da` declares the global, without regard to
    /// case; `None` when there is no table to ask, and the check is
    /// skipped.
    fn declares_global(&mut self, name: &str) -> Option<bool>;
}

/// What a condition reads of the globals.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobalReads {
    /// The literal names, each once, in the order the bytecode passes them.
    pub names: Vec<String>,
    /// The literal prefixes of names built by appending a computed part
    /// (`"OAJ_DOWN_" + IntToString(n)`), each once.
    pub prefixes: Vec<String>,
    /// The routines (`GetGlobalNumber`, ...) called with a name computed
    /// with no literal prefix, each once.
    pub computed: Vec<String>,
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
    /// The dialogue's own script run when the conversation ends:
    /// `EndConversation`, or with `aborted`, `EndConverAbort`.
    End { aborted: bool },
}

impl Place {
    /// For a place on the dialogue itself, the field it names.
    fn end_field(&self) -> Option<&'static str> {
        match self {
            Place::End { aborted: false } => Some("EndConversation"),
            Place::End { aborted: true } => Some("EndConverAbort"),
            _ => None,
        }
    }
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Place::Node(n) => write!(f, "{n}"),
            Place::Link { from, to } => write!(f, "the link {from} -> {to}"),
            Place::Start { to } => write!(f, "the start into {to}"),
            Place::End { .. } => f.write_str(self.end_field().expect("an end place")),
        }
    }
}

/// One reference the install cannot satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Finding {
    /// The registered code: `RW1102`, `RW1104`, `RW1105` or `RW1106` from
    /// [`check`], and from [`crate::lint`] also `RW1101` and `RW1107` to
    /// `RW1110`.
    pub code: &'static str,
    /// Where it lies.
    pub place: Place,
    /// The stable key of the node it lies on (for a link, the node the
    /// link leaves; for a start, the entry it enters), as
    /// [`NodeKey::of`] chooses it; `None` for a place on the dialogue
    /// itself, an end script, which the dialogue's name alone names.
    pub key: Option<NodeKey>,
    /// What is referred to: the script, the voice resref, the strref, the
    /// global, the condition (for a computed global), the missing node
    /// (for a dangling link), or the disagreement (for `SoundExists`);
    /// empty for an unreached node.
    pub subject: String,
    /// For an undeclared global, the condition that reads it; for a
    /// computed one, the routine it calls.
    pub via: Option<String>,
}

impl Finding {
    /// The message without the code, naming the dialogue, the node key and
    /// the subject.
    pub fn message(&self, dialog: &str) -> String {
        let at = match (&self.place, &self.key) {
            (p, None) => format!("{dialog} {p}"),
            (Place::Node(_), Some(k)) => format!("{dialog} {k}"),
            (other, Some(k)) => format!("{dialog} {other} ({k})"),
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
            codes::DANGLING_LINK => format!(
                "links to {}, which the dialogue does not have",
                self.subject
            ),
            codes::UNREACHED_NODE => {
                "is reached by no start, through any chain of links".to_string()
            }
            codes::MISSING_LIP => format!(
                "is voiced by {}, and no layer holds {}.lip",
                self.subject, self.subject
            ),
            codes::UNDECIDABLE_GLOBAL => match self.via.as_deref() {
                Some(v) if v.ends_with('*') => format!(
                    "has the condition {}, which reads a global named {v} with the rest computed, and the install cannot say whether globalcat.2da declares one",
                    self.subject
                ),
                v => format!(
                    "has the condition {}, which passes a computed name to {}, so the global it reads cannot be checked",
                    self.subject,
                    v.unwrap_or("?")
                ),
            },
            codes::UNDECLARED_GLOBAL if self.subject.ends_with('*') => format!(
                "has the condition {}, which reads a global named {} with the rest computed, and globalcat.2da declares none with that prefix",
                self.via.as_deref().unwrap_or("?"),
                self.subject
            ),
            codes::SOUND_EXISTS => format!("has {}", self.subject),
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
/// then each node's links, then the starts, then the end scripts
/// (`EndConversation`, then `EndConverAbort`).
pub fn check(dialog: &Dialog, refs: &mut dyn References) -> Vec<Finding> {
    let mut out = Vec::new();
    let tlk = refs.talk_table_len();
    for node in dialog.all() {
        let key = NodeKey::of(dialog, node.at);
        let find = |code, subject: String| Finding {
            code,
            place: Place::Node(node.at),
            key: Some(key.clone()),
            subject,
            via: None,
        };
        if let Some(s) = node.script()
            && !refs.resolves(&s, ResType::Ncs)
        {
            out.push(find(codes::MISSING_SCRIPT, s.to_string()));
        }
        if let Some(vo) = node.voiced()
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
    for aborted in [false, true] {
        let place = Place::End { aborted };
        if let Some(s) = dialog.end_script(aborted)
            && !refs.resolves(&s, ResType::Ncs)
        {
            out.push(Finding {
                code: codes::MISSING_SCRIPT,
                place,
                key: None,
                subject: s.to_string(),
                via: None,
            });
        }
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
            key: Some(key),
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
                key: Some(key.clone()),
                subject: global,
                via: Some(c.to_string()),
            });
        }
    }
}
