//! The transcript of a conversation: the starts in order, then the tree
//! from each start, every node as its speaker and listener, its text, its
//! voice with the duration, its script, animations, camera, delay and
//! quest, and every condition's source. A link to a node already shown
//! prints a reference instead of the node again, and nodes no start reaches
//! follow the tree, so every node of the dialogue appears once.
//!
//! What the transcript needs from outside the dialogue (the talk table, the
//! voice files, the scripts and their decompiled source) comes through a
//! [`Lookup`]; [`Embedded`] answers from the dialogue alone.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use rakata_res::{Gender, Language, LocalizedString, ResRef};
use serde::Serialize;

use crate::audio::Audio;
use crate::error::codes;
use crate::key::NodeKey;
use crate::model::{Dialog, Info, Kind, Link, Node, NodeRef};

/// A voice file found for a node.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Voice {
    /// Where the file lies, relative to the install.
    pub path: String,
    /// What its head says; `None` when the head does not read.
    pub audio: Option<Audio>,
}

/// What is known of a script a dialogue names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptStatus {
    /// Not looked up: the lookup has no install.
    Unresolved,
    /// No layer of the install holds it.
    Missing,
    /// Found, and not decompiled (an action script, unless asked).
    Found,
    /// Found, and the decompiler could not read the bytecode.
    Unreadable,
    /// Decompiled, and every function recompiles to the original bytes.
    Verified,
    /// Decompiled, and some function recompiles differently.
    Unverified,
}

impl ScriptStatus {
    /// The status as the transcript prints it.
    pub fn text(self) -> &'static str {
        match self {
            ScriptStatus::Unresolved => "not looked up",
            ScriptStatus::Missing => "missing",
            ScriptStatus::Found => "found",
            ScriptStatus::Unreadable => "not decompiled",
            ScriptStatus::Verified => "verified",
            ScriptStatus::Unverified => "unverified",
        }
    }
}

/// A script a dialogue names, as the [`Lookup`] found it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Script {
    /// The resref, as the dialogue spells it.
    pub name: String,
    pub status: ScriptStatus,
    /// Where the copy the engine loads lies.
    pub location: Option<String>,
    /// Functions recovered, and how many of them verified.
    pub functions: usize,
    pub verified: usize,
    /// The recovered source.
    pub source: Option<String>,
    /// Why it is unreadable, or what folding did.
    pub note: Option<String>,
}

impl Script {
    /// A script with only a name and a status.
    pub fn named(name: &ResRef, status: ScriptStatus) -> Self {
        Script {
            name: name.to_string(),
            status,
            location: None,
            functions: 0,
            verified: 0,
            source: None,
            note: None,
        }
    }
}

/// What the transcript asks of the world outside the dialogue.
pub trait Lookup {
    /// The text a localized string shows: the embedded substring or the
    /// talk table's line.
    fn text(&mut self, s: &LocalizedString) -> Option<String>;
    /// The voice file for a node's `VO_ResRef`, when there is one.
    fn voice(&mut self, vo: &ResRef) -> Option<Voice>;
    /// A script the dialogue names, decompiled when `decompile` is set.
    fn script(&mut self, name: &ResRef, decompile: bool) -> Script;
}

/// A [`Lookup`] with nothing but the dialogue: embedded text only, no voice
/// files, no scripts.
#[derive(Debug, Clone, Copy, Default)]
pub struct Embedded;

impl Lookup for Embedded {
    fn text(&mut self, s: &LocalizedString) -> Option<String> {
        s.resolve_with(Language::English, Gender::Male, |_| None)
            .map(str::to_owned)
    }

    fn voice(&mut self, _: &ResRef) -> Option<Voice> {
        None
    }

    fn script(&mut self, name: &ResRef, _: bool) -> Script {
        Script::named(name, ScriptStatus::Unresolved)
    }
}

/// What to put in a transcript.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Decompile the nodes' action scripts as well as the conditions.
    pub scripts: bool,
    /// Each node's speaker and text, and the names of its conditions and
    /// script, nothing else: no script is decompiled, and
    /// [`Options::scripts`] is ignored.
    pub brief: bool,
}

/// A link as the transcript shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkView {
    /// The node it leads to, as `E3` or `R0`.
    pub target: String,
    pub condition: Option<String>,
    pub is_child: bool,
}

/// An animation as the transcript shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnimationView {
    pub participant: String,
    pub animation: u32,
}

/// A voice-over as the transcript shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VoiceView {
    pub resref: String,
    /// The file, when one was found.
    pub file: Option<Voice>,
    /// Whether the line is voiced, so a file is expected: an entry whose
    /// `SoundExists` is not `0` and that names no `Sound` ([`Node::voiced`]).
    /// A player reply never is; KotOR's protagonist is unvoiced.
    pub expected: bool,
}

/// One node as the transcript shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeView {
    /// `E3` or `R0`.
    pub id: String,
    pub kind: &'static str,
    pub index: usize,
    /// The node's stable key, as [`NodeKey::of`] chooses it and
    /// [`NodeKey`] displays it: `voice:RESREF`, `strref:N`, `text:"…"`, or
    /// `index:E3 (fragile)`.
    pub key: String,
    /// Whether the key is the node's index, which moves when its list is
    /// reordered.
    pub fragile: bool,
    /// The `Speaker` tag as stored; replies have none.
    pub speaker: Option<String>,
    /// `owner` for an entry with an empty speaker, `tag` for an entry with
    /// one, `player` for a reply.
    pub speaker_role: &'static str,
    pub listener: Option<String>,
    /// The text shown, through the talk table.
    pub text: Option<String>,
    pub strref: Option<u32>,
    pub voice: Option<VoiceView>,
    pub sound: Option<String>,
    pub script: Option<String>,
    pub delay: Option<u32>,
    pub animations: Vec<AnimationView>,
    pub camera_angle: Option<u32>,
    pub camera_id: Option<i32>,
    pub camera_animation: Option<u32>,
    pub quest: Option<String>,
    pub quest_entry: Option<u32>,
    pub comment: Option<String>,
    pub links: Vec<LinkView>,
}

/// One line of the tree: a node reached through a link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// The indentation level: a node's links are one level in when it has
    /// more than one, at its own level when it has one.
    pub depth: usize,
    /// The node, as `E3` or `R0`.
    pub node: String,
    /// The starting link's position, for a row that begins a start.
    pub start: Option<usize>,
    pub condition: Option<String>,
    pub is_child: bool,
    /// The node was shown above, so this row is a reference to it.
    pub repeat: bool,
    /// The link leads past the end of its list.
    pub missing: bool,
    /// The node is reached from no start; such rows follow the tree.
    pub unreached: bool,
}

/// Something the transcript found wrong, with its registered code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub code: &'static str,
    pub message: String,
}

/// A conversation as a transcript.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Transcript {
    pub dialog: String,
    pub info: Info,
    pub entries: usize,
    pub replies: usize,
    pub starts: Vec<LinkView>,
    /// Every node, in the order the tree first shows it.
    pub nodes: Vec<NodeView>,
    /// Every script the dialogue names, in the order first named.
    pub scripts: Vec<Script>,
    pub tree: Vec<Row>,
    pub warnings: Vec<Warning>,
    /// Built with [`Options::brief`]: the text leaves out all but the
    /// speakers, the lines and the scripts' names.
    #[serde(skip)]
    pub brief: bool,
}

struct Builder<'l> {
    lookup: &'l mut dyn Lookup,
    options: Options,
    scripts: Vec<Script>,
    by_name: HashMap<ResRef, usize>,
    warned: HashSet<ResRef>,
    warnings: Vec<Warning>,
    dialog: String,
}

impl Builder<'_> {
    /// Look a script up once per name; a condition is always decompiled,
    /// unless the transcript is brief.
    fn script(&mut self, name: &ResRef, decompile: bool) -> String {
        let decompile = (decompile || self.options.scripts) && !self.options.brief;
        let redo = match self.by_name.get(name) {
            Some(&i) => decompile && self.scripts[i].status == ScriptStatus::Found,
            None => true,
        };
        if redo {
            let s = self.lookup.script(name, decompile);
            if !self.warned.contains(name) {
                match s.status {
                    ScriptStatus::Missing => self.warnings.push(Warning {
                        code: codes::MISSING_SCRIPT,
                        message: format!(
                            "{} names the script {name}, which no layer of the install holds",
                            self.dialog
                        ),
                    }),
                    ScriptStatus::Unreadable => self.warnings.push(Warning {
                        code: codes::UNREADABLE_SCRIPT,
                        message: format!(
                            "{}: the decompiler could not read {name}: {}",
                            self.dialog,
                            s.note.as_deref().unwrap_or("no reason given")
                        ),
                    }),
                    _ => {}
                }
                self.warned.insert(*name);
            }
            match self.by_name.get(name) {
                Some(&i) => self.scripts[i] = s,
                None => {
                    self.by_name.insert(*name, self.scripts.len());
                    self.scripts.push(s);
                }
            }
        }
        name.to_string()
    }

    fn link(&mut self, l: &Link<'_>) -> LinkView {
        LinkView {
            target: l.target.to_string(),
            condition: l.condition().map(|c| self.script(&c, true)),
            is_child: l.is_child(),
        }
    }

    fn node(&mut self, dialog: &Dialog, n: &Node<'_>) -> NodeView {
        let key = NodeKey::of(dialog, n.at);
        let text = n.text();
        let shown = self.lookup.text(&text);
        let voiced = n.at.kind == Kind::Entry && n.voiced().is_some();
        let voice = n.voice().map(|vo| VoiceView {
            resref: vo.to_string(),
            file: self.lookup.voice(&vo),
            expected: voiced,
        });
        let speaker = n.speaker();
        let speaker_role = match (n.at.kind, speaker.as_deref()) {
            (Kind::Reply, _) => "player",
            (Kind::Entry, None | Some("")) => "owner",
            (Kind::Entry, Some(_)) => "tag",
        };
        let script = n.script().map(|s| self.script(&s, false));
        let links = n.links().iter().map(|l| self.link(l)).collect();
        NodeView {
            id: n.at.to_string(),
            kind: n.at.kind.name(),
            index: n.at.index,
            key: key.to_string(),
            fragile: key.is_fragile(),
            speaker,
            speaker_role,
            listener: n.listener(),
            text: shown,
            strref: n.strref(),
            voice,
            sound: n.sound().map(|s| s.to_string()),
            script,
            delay: n.delay(),
            animations: n
                .animations()
                .into_iter()
                .map(|a| AnimationView {
                    participant: a.participant,
                    animation: a.animation,
                })
                .collect(),
            camera_angle: n.camera_angle(),
            camera_id: n.camera_id(),
            camera_animation: n.camera_animation(),
            quest: n.quest(),
            quest_entry: n.quest_entry(),
            comment: n.comment(),
            links,
        }
    }
}

/// A pending row of the walk.
struct Step {
    depth: usize,
    target: NodeRef,
    start: Option<usize>,
    condition: Option<String>,
    is_child: bool,
}

impl Transcript {
    /// The transcript of `dialog`, with `lookup` answering for text, voice
    /// files and scripts.
    pub fn build(dialog: &Dialog, lookup: &mut dyn Lookup, options: Options) -> Transcript {
        let mut b = Builder {
            lookup,
            options,
            scripts: Vec::new(),
            by_name: HashMap::new(),
            warned: HashSet::new(),
            warnings: Vec::new(),
            dialog: dialog.name().to_string(),
        };
        let starts: Vec<LinkView> = dialog.starts().iter().map(|l| b.link(l)).collect();
        let mut shown: HashMap<NodeRef, usize> = HashMap::new();
        let mut nodes: Vec<NodeView> = Vec::new();
        let mut tree: Vec<Row> = Vec::new();

        let mut stack: Vec<Step> = dialog
            .starts()
            .iter()
            .rev()
            .map(|l| Step {
                depth: 0,
                target: l.target,
                start: Some(l.position),
                condition: l.condition().map(|c| c.to_string()),
                is_child: false,
            })
            .collect();
        let mut unreached = false;
        loop {
            let Some(step) = stack.pop() else {
                // What no start reaches, in index order, entries first.
                let next = dialog.all().find(|n| !shown.contains_key(&n.at));
                match next {
                    Some(n) => {
                        unreached = true;
                        stack.push(Step {
                            depth: 0,
                            target: n.at,
                            start: None,
                            condition: None,
                            is_child: false,
                        });
                        continue;
                    }
                    None => break,
                }
            };
            let mut row = Row {
                depth: step.depth,
                node: step.target.to_string(),
                start: step.start,
                condition: step.condition,
                is_child: step.is_child,
                repeat: false,
                missing: false,
                unreached,
            };
            let Some(node) = dialog.node(step.target) else {
                row.missing = true;
                b.warnings.push(Warning {
                    code: codes::DANGLING_LINK,
                    message: format!(
                        "{} links to {}, past the end of its {} entries and {} replies",
                        dialog.name(),
                        step.target,
                        dialog.len(Kind::Entry),
                        dialog.len(Kind::Reply)
                    ),
                });
                tree.push(row);
                continue;
            };
            if shown.contains_key(&step.target) {
                row.repeat = true;
                tree.push(row);
                continue;
            }
            shown.insert(step.target, nodes.len());
            let view = b.node(dialog, &node);
            let links = node.links();
            let depth = if links.len() > 1 {
                step.depth + 1
            } else {
                step.depth
            };
            for l in links.iter().rev() {
                stack.push(Step {
                    depth,
                    target: l.target,
                    start: None,
                    condition: l.condition().map(|c| c.to_string()),
                    is_child: l.is_child(),
                });
            }
            nodes.push(view);
            tree.push(row);
        }

        Transcript {
            dialog: dialog.name().to_string(),
            info: dialog.info(),
            entries: dialog.len(Kind::Entry),
            replies: dialog.len(Kind::Reply),
            starts,
            nodes,
            scripts: b.scripts,
            tree,
            warnings: b.warnings,
            brief: options.brief,
        }
    }

    /// The node shown as `id` (`E3`).
    pub fn node(&self, id: &str) -> Option<&NodeView> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// The nodes shown as `ids` (`E3`), each with its links: every link's
    /// condition, then the node it leads to by id and key. An id the
    /// transcript does not hold is left out.
    pub fn nodes_text(&self, ids: &[&str]) -> String {
        let mut text = Text {
            t: self,
            printed: HashSet::new(),
            out: String::new(),
        };
        for (i, id) in ids.iter().enumerate() {
            let Some(n) = self.node(id) else {
                continue;
            };
            if i > 0 {
                text.out.push('\n');
            }
            text.body(n, "", false);
            if n.links.is_empty() {
                let _ = writeln!(text.out, "  (no links)");
            }
            for l in &n.links {
                if let Some(c) = &l.condition {
                    text.script_block("  ", "if ", c);
                }
                let _ = match self.node(&l.target) {
                    Some(to) if l.is_child => {
                        writeln!(text.out, "  -> {} [{}] (a link)", to.id, to.key)
                    }
                    Some(to) => writeln!(text.out, "  -> {} [{}]", to.id, to.key),
                    None => writeln!(text.out, "  -> {} (no such node)", l.target),
                };
            }
        }
        text.out
    }

    /// The script named `name`, without regard to case.
    pub fn script(&self, name: &str) -> Option<&Script> {
        self.scripts
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// The transcript as text.
    pub fn text(&self) -> String {
        Text {
            t: self,
            printed: HashSet::new(),
            out: String::new(),
        }
        .render()
    }
}

struct Text<'t> {
    t: &'t Transcript,
    /// Scripts whose source is already printed.
    printed: HashSet<String>,
    out: String,
}

fn yes(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

impl Text<'_> {
    fn render(mut self) -> String {
        let t = self.t;
        let _ = writeln!(
            self.out,
            "{}: {} entries, {} replies, {} starts",
            t.dialog,
            t.entries,
            t.replies,
            t.starts.len()
        );
        let i = &t.info;
        let mut facts = Vec::new();
        if let Some(s) = i.skippable {
            facts.push(format!("skippable {}", yes(s)));
        }
        if let Some(c) = i.conversation_type {
            facts.push(format!("conversation type {c}"));
        }
        if let Some(c) = i.computer_type {
            facts.push(format!("computer type {c}"));
        }
        if let (Some(e), Some(r)) = (i.delay_entry, i.delay_reply) {
            facts.push(format!("delay entry {e}, reply {r}"));
        }
        for (label, v) in [
            ("camera model", &i.camera_model),
            ("VO_ID", &i.vo_id),
            ("ambient track", &i.ambient_track),
            ("on end", &i.end_conversation),
            ("on abort", &i.end_conversation_abort),
        ] {
            if let Some(v) = v {
                facts.push(format!("{label} {v}"));
            }
        }
        if i.animated_cut == Some(true) {
            facts.push("animated cut".into());
        }
        if !facts.is_empty() {
            let _ = writeln!(self.out, "{}", facts.join("; "));
        }

        let _ = writeln!(self.out, "\nStarts, tried in order:");
        if t.starts.is_empty() {
            let _ = writeln!(self.out, "  (none)");
        }
        for (n, s) in t.starts.iter().enumerate() {
            match &s.condition {
                Some(c) => self.script_block("  ", &format!("{}. {} if ", n + 1, s.target), c),
                None => {
                    let _ = writeln!(self.out, "  {}. {} always", n + 1, s.target);
                }
            }
        }

        let mut unreached = false;
        for row in &t.tree {
            if row.unreached && !unreached {
                unreached = true;
                let _ = writeln!(self.out, "\nReached from no start:");
            }
            if row.start.is_some() || (row.unreached && row.depth == 0) {
                self.out.push('\n');
            }
            self.row(row);
        }
        self.out
    }

    fn status(&self, name: &str) -> String {
        match self.t.script(name) {
            Some(s) if s.functions > 0 => {
                format!(
                    "{}, {} of {} functions",
                    s.status.text(),
                    s.verified,
                    s.functions
                )
            }
            Some(s) => s.status.text().to_string(),
            None => ScriptStatus::Unresolved.text().into(),
        }
    }

    /// A script's name and status, then its source the first time; in a
    /// brief transcript, its name alone.
    fn script_block(&mut self, pad: &str, lead: &str, name: &str) {
        if self.t.brief {
            let _ = writeln!(self.out, "{pad}{lead}{name}");
            return;
        }
        let status = self.status(name);
        let s = self.t.script(name);
        let source = s.and_then(|s| s.source.as_deref());
        let first = source.is_some() && self.printed.insert(name.to_ascii_lowercase());
        let after = if source.is_some() && !first {
            " (source above)"
        } else {
            ""
        };
        let _ = writeln!(self.out, "{pad}{lead}{name} [{status}]{after}");
        if let (true, Some(src)) = (first, source) {
            for line in src.trim_end().lines() {
                let _ = writeln!(self.out, "{}", format!("{pad}  | {line}").trim_end());
            }
            if let Some(note) = s.and_then(|s| s.note.as_deref()) {
                let _ = writeln!(self.out, "{pad}  ({note})");
            }
        }
    }

    fn row(&mut self, row: &Row) {
        let pad = "  ".repeat(row.depth);
        if let Some(c) = &row.condition {
            self.script_block(&pad, "if ", c);
        }
        let child = if row.is_child { ", a link" } else { "" };
        if row.missing {
            let _ = writeln!(self.out, "{pad}-> {} (no such node{child})", row.node);
            return;
        }
        if row.repeat {
            let _ = writeln!(self.out, "{pad}-> {} (shown above{child})", row.node);
            return;
        }
        let Some(n) = self.t.node(&row.node) else {
            return;
        };
        self.body(n, &pad, row.is_child);
    }

    /// A node's line, `E3 [KEY] SPEAKER to LISTENER: "TEXT"`, and its
    /// fields beneath it.
    fn body(&mut self, n: &NodeView, pad: &str, is_child: bool) {
        let who = match n.speaker_role {
            "player" => "(player)".to_string(),
            "owner" => "(owner)".to_string(),
            _ => n.speaker.clone().unwrap_or_default(),
        };
        let to = match n.listener.as_deref() {
            Some(l) if !l.is_empty() => format!(" to {l}"),
            _ => String::new(),
        };
        let said = match (&n.text, n.strref) {
            (Some(t), _) => format!("{t:?}"),
            (None, Some(s)) => format!("(strref {s}, not in the talk table)"),
            (None, None) => "(no text)".into(),
        };
        let child = if is_child { " (a link)" } else { "" };
        let _ = writeln!(
            self.out,
            "{pad}{} [{}] {who}{to}: {said}{child}",
            n.id, n.key
        );
        let pad = format!("{pad}     ");
        if self.t.brief {
            if let Some(s) = &n.script {
                self.script_block(&pad, "script ", s);
            }
            return;
        }
        if let Some(s) = n.strref {
            let _ = writeln!(self.out, "{pad}strref {s}");
        }
        // A line nobody voiced names a placeholder; say nothing of it.
        if let Some(v) = n.voice.as_ref().filter(|v| v.file.is_some() || v.expected) {
            let file = match &v.file {
                Some(Voice {
                    audio: Some(a),
                    path,
                }) => format!(", {:.2} s ({path})", a.seconds),
                Some(Voice { audio: None, path }) => format!(", duration unread ({path})"),
                None => ", no file under streamwaves/".into(),
            };
            let _ = writeln!(self.out, "{pad}voice {}{file}", v.resref);
        }
        if let Some(s) = &n.sound {
            let _ = writeln!(self.out, "{pad}sound {s}");
        }
        if let Some(s) = &n.script {
            self.script_block(&pad, "script ", s);
        }
        for a in &n.animations {
            if a.participant.is_empty() {
                let _ = writeln!(self.out, "{pad}animation {}", a.animation);
            } else {
                let _ = writeln!(
                    self.out,
                    "{pad}animation {} on {}",
                    a.animation, a.participant
                );
            }
        }
        let angle = n.camera_angle.filter(|&a| a != 0);
        let id = n.camera_id.filter(|&i| i != -1);
        if angle.is_some() || id.is_some() || n.camera_animation.is_some_and(|a| a != 0) {
            let mut cam = vec![format!("camera angle {}", n.camera_angle.unwrap_or(0))];
            if let Some(i) = n.camera_id {
                cam.push(format!("id {i}"));
            }
            if let Some(a) = n.camera_animation.filter(|&a| a != 0) {
                cam.push(format!("animation {a}"));
            }
            let _ = writeln!(self.out, "{pad}{}", cam.join(", "));
        }
        if let Some(d) = n.delay.filter(|&d| d != u32::MAX) {
            let _ = writeln!(self.out, "{pad}delay {d}");
        }
        if let Some(q) = &n.quest {
            match n.quest_entry {
                Some(e) => {
                    let _ = writeln!(self.out, "{pad}quest {q}, entry {e}");
                }
                None => {
                    let _ = writeln!(self.out, "{pad}quest {q}");
                }
            }
        }
        if let Some(c) = &n.comment {
            let c = c.replace(['\r', '\n'], " ");
            let _ = writeln!(self.out, "{pad}comment: {}", c.trim());
        }
    }
}
