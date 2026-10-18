//! The conversation over its GFF: entries (what the owner and the other
//! speakers say), replies (what the player says), the starting links into
//! the entries, and each node's links onward. The model reads the fields it
//! names from the tree and keeps the tree itself, so every field it does
//! not name (`SoundExists` written six times over, KotOR 2's `Active2`)
//! writes back as it was read.

use std::fmt;

use rakata_gff::{Gff, LocString, Struct, Value};
use rakata_res::{Gender, Language, LocalizedString, ResRef, StrRef, Substring, from_string_id};
use rakata_tlk::codepage;

use crate::error::{Error, Result};

/// Which list a node lies in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// `EntryList`: lines spoken by the owner or another speaker.
    Entry,
    /// `ReplyList`: lines the player chooses.
    Reply,
}

impl Kind {
    /// `entry` or `reply`.
    pub const fn name(self) -> &'static str {
        match self {
            Kind::Entry => "entry",
            Kind::Reply => "reply",
        }
    }

    /// The root list holding nodes of this kind.
    pub const fn list(self) -> &'static str {
        match self {
            Kind::Entry => "EntryList",
            Kind::Reply => "ReplyList",
        }
    }

    /// The list, inside a node of this kind, of links to the other kind.
    pub const fn links(self) -> &'static str {
        match self {
            Kind::Entry => "RepliesList",
            Kind::Reply => "EntriesList",
        }
    }

    /// The kind a node of this kind links to.
    pub const fn other(self) -> Kind {
        match self {
            Kind::Entry => Kind::Reply,
            Kind::Reply => Kind::Entry,
        }
    }

    const fn letter(self) -> char {
        match self {
            Kind::Entry => 'E',
            Kind::Reply => 'R',
        }
    }
}

/// A node by its list and index: `E3` is `EntryList[3]`, `R0` is
/// `ReplyList[0]`. Indices move when nodes are inserted, so a reference
/// meant to survive edits is a [`crate::NodeKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeRef {
    pub kind: Kind,
    pub index: usize,
}

impl NodeRef {
    pub const fn entry(index: usize) -> Self {
        NodeRef {
            kind: Kind::Entry,
            index,
        }
    }

    pub const fn reply(index: usize) -> Self {
        NodeRef {
            kind: Kind::Reply,
            index,
        }
    }
}

impl fmt::Display for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.kind.letter(), self.index)
    }
}

/// The text a `CExoString` holds, decoded from the game's code page.
pub(crate) fn text(bytes: &[u8]) -> String {
    codepage::decode(bytes)
}

/// A GFF `CExoLocString` as `rakata-res`'s [`LocalizedString`], the
/// substrings decoded from the game's code page.
pub fn localized(l: &LocString) -> LocalizedString {
    LocalizedString {
        strref: StrRef::new(l.strref),
        substrings: l
            .substrings
            .iter()
            .map(|s| {
                let (language, gender) = from_string_id(s.id);
                Substring {
                    language,
                    gender,
                    text: codepage::decode(&s.text),
                }
            })
            .collect(),
    }
}

/// An integer field of any width as `i64`.
fn number(s: &Struct, label: &str) -> Option<i64> {
    Some(match s.get(label)? {
        Value::Byte(v) => i64::from(*v),
        Value::Char(v) => i64::from(*v),
        Value::Word(v) => i64::from(*v),
        Value::Short(v) => i64::from(*v),
        Value::Dword(v) => i64::from(*v),
        Value::Int(v) => i64::from(*v),
        Value::Dword64(v) => *v as i64,
        Value::Int64(v) => *v,
        _ => return None,
    })
}

fn string(s: &Struct, label: &str) -> Option<String> {
    match s.get(label)? {
        Value::String(b) => Some(text(b)),
        _ => None,
    }
}

/// A resref field, `None` when absent or empty.
fn resref(s: &Struct, label: &str) -> Option<ResRef> {
    match s.get(label)? {
        Value::ResRef(r) if !r.is_empty() => Some(*r),
        _ => None,
    }
}

fn list<'a>(s: &'a Struct, label: &str) -> &'a [Struct] {
    s.get(label).and_then(Value::as_list).unwrap_or(&[])
}

/// A link: from the start, or from a node to a node of the other kind.
#[derive(Debug, Clone, Copy)]
pub struct Link<'a> {
    /// The node it leads to. The index may lie past the end of the list
    /// in a damaged file; [`Dialog::node`] then answers `None`.
    pub target: NodeRef,
    /// Its position in the list it lies in.
    pub position: usize,
    raw: &'a Struct,
}

impl<'a> Link<'a> {
    /// The condition script (`Active`): the link is offered when it returns
    /// true, or always when there is none.
    pub fn condition(&self) -> Option<ResRef> {
        resref(self.raw, "Active")
    }

    /// `IsChild`: whether the link points at a node another link owns, as
    /// BioWare's editor draws a link rather than a child. Starting links
    /// have no such field and are never children.
    pub fn is_child(&self) -> bool {
        number(self.raw, "IsChild").is_some_and(|v| v != 0)
    }

    /// The link's struct, with every field the model does not name.
    pub fn raw(&self) -> &'a Struct {
        self.raw
    }
}

/// One animation a node plays: the participant (a tag, `PLAYER`, or empty
/// for the speaker) and the animation number. Numbers are the engine's;
/// they are not named here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    pub participant: String,
    pub animation: u32,
}

/// One node of a dialogue, read from its struct.
#[derive(Debug, Clone, Copy)]
pub struct Node<'a> {
    pub at: NodeRef,
    raw: &'a Struct,
}

impl<'a> Node<'a> {
    /// The node's struct, with every field the model does not name.
    pub fn raw(&self) -> &'a Struct {
        self.raw
    }

    /// `Speaker`: the tag of who speaks an entry; empty means the
    /// dialogue's owner. Replies carry no speaker: the player speaks them.
    pub fn speaker(&self) -> Option<String> {
        string(self.raw, "Speaker")
    }

    /// `Listener`: the tag of who is spoken to; empty when unset.
    pub fn listener(&self) -> Option<String> {
        string(self.raw, "Listener")
    }

    /// `Text`, as a localized string; empty when the field is absent.
    pub fn text(&self) -> LocalizedString {
        match self.raw.get("Text") {
            Some(Value::LocString(l)) => localized(l),
            _ => LocalizedString::EMPTY,
        }
    }

    /// The text's strref, when it has one.
    pub fn strref(&self) -> Option<u32> {
        match self.raw.get("Text") {
            Some(Value::LocString(l)) if l.has_strref() => Some(l.strref),
            _ => None,
        }
    }

    /// The text the node embeds, in English, when it embeds one: what a
    /// dialogue shows without a talk table.
    pub fn embedded_text(&self) -> Option<String> {
        self.text()
            .resolve_with(Language::English, Gender::Male, |_| None)
            .map(str::to_owned)
    }

    /// `VO_ResRef`: the voice-over, a WAV under `streamwaves/`.
    pub fn voice(&self) -> Option<ResRef> {
        resref(self.raw, "VO_ResRef")
    }

    /// `SoundExists`, the first of its copies: nonzero where BioWare's
    /// tool found the line's audio, `0` where it did not, `None` when the
    /// node has no such field.
    pub fn sound_exists(&self) -> Option<i64> {
        number(self.raw, "SoundExists")
    }

    /// The voice-over the node plays: `VO_ResRef`, unless `SoundExists` is
    /// `0`, which BioWare's tool writes on a line it has no recording for
    /// (the vanilla player replies name placeholders such as
    /// `_m13aabast01001_`), or the node names a `Sound`, which the engine
    /// plays instead (a Wookiee's speech, or a line recorded under another
    /// name).
    pub fn voiced(&self) -> Option<ResRef> {
        self.voice()
            .filter(|_| self.sound_exists() != Some(0) && self.sound().is_none())
    }

    /// `Sound`: a sound played with the line.
    pub fn sound(&self) -> Option<ResRef> {
        resref(self.raw, "Sound")
    }

    /// `Script`: run when the node is shown.
    pub fn script(&self) -> Option<ResRef> {
        resref(self.raw, "Script")
    }

    /// `Delay`, as stored; vanilla files hold `0xFFFFFFFF` on most voiced
    /// nodes.
    pub fn delay(&self) -> Option<u32> {
        number(self.raw, "Delay").map(|v| v as u32)
    }

    /// `AnimList`: participant and animation number, in order.
    pub fn animations(&self) -> Vec<Animation> {
        list(self.raw, "AnimList")
            .iter()
            .map(|a| Animation {
                participant: string(a, "Participant").unwrap_or_default(),
                animation: number(a, "Animation").unwrap_or(0) as u32,
            })
            .collect()
    }

    /// `CameraAngle`, the engine's number.
    pub fn camera_angle(&self) -> Option<u32> {
        number(self.raw, "CameraAngle").map(|v| v as u32)
    }

    /// `CameraID`: the placed camera, `-1` for none.
    pub fn camera_id(&self) -> Option<i32> {
        number(self.raw, "CameraID").map(|v| v as i32)
    }

    /// `CameraAnimation`, the engine's number.
    pub fn camera_animation(&self) -> Option<u32> {
        number(self.raw, "CameraAnimation").map(|v| v as u32)
    }

    /// `Quest`: the journal category the node updates, when set.
    pub fn quest(&self) -> Option<String> {
        string(self.raw, "Quest").filter(|q| !q.is_empty())
    }

    /// `QuestEntry`: the journal entry's id.
    pub fn quest_entry(&self) -> Option<u32> {
        number(self.raw, "QuestEntry").map(|v| v as u32)
    }

    /// `Comment`: the author's note, when set.
    pub fn comment(&self) -> Option<String> {
        string(self.raw, "Comment").filter(|c| !c.is_empty())
    }

    /// The links onward, to replies from an entry and to entries from a
    /// reply, in list order.
    pub fn links(&self) -> Vec<Link<'a>> {
        links(list(self.raw, self.at.kind.links()), self.at.kind.other())
    }
}

fn links(items: &[Struct], kind: Kind) -> Vec<Link<'_>> {
    items
        .iter()
        .enumerate()
        .map(|(position, raw)| Link {
            target: NodeRef {
                kind,
                index: number(raw, "Index").unwrap_or(-1) as u32 as usize,
            },
            position,
            raw,
        })
        .collect()
}

/// The conversation-wide fields of the root struct the model names.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Info {
    pub delay_entry: Option<u32>,
    pub delay_reply: Option<u32>,
    pub skippable: Option<bool>,
    pub conversation_type: Option<i32>,
    pub computer_type: Option<u32>,
    pub camera_model: Option<String>,
    pub end_conversation: Option<String>,
    pub end_conversation_abort: Option<String>,
    pub ambient_track: Option<String>,
    pub vo_id: Option<String>,
    pub animated_cut: Option<bool>,
}

/// A dialogue: a DLG's GFF, kept whole, read through the model.
#[derive(Debug, Clone)]
pub struct Dialog {
    pub(crate) name: String,
    pub(crate) gff: Gff,
}

impl Dialog {
    /// The dialogue in `gff`, named `name` in messages (`banter.dlg`).
    /// Fails when the file type is not `DLG `.
    pub fn new(name: impl Into<String>, gff: Gff) -> Result<Self> {
        let name = name.into();
        if &gff.file_type != b"DLG " {
            return Err(Error::NotDialog {
                dialog: name,
                found: gff.file_type_str(),
            });
        }
        Ok(Dialog { name, gff })
    }

    /// Read a DLG's bytes.
    pub fn parse(name: impl Into<String>, bytes: &[u8]) -> Result<Self> {
        Dialog::new(name, Gff::parse(bytes)?)
    }

    /// The bytes: for a dialogue read and not changed, the bytes it was
    /// read from.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.gff.to_bytes()?)
    }

    /// The name messages use.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The GFF beneath.
    pub fn gff(&self) -> &Gff {
        &self.gff
    }

    pub fn into_gff(self) -> Gff {
        self.gff
    }

    fn nodes(&self, kind: Kind) -> &[Struct] {
        list(&self.gff.root, kind.list())
    }

    /// How many nodes of a kind there are.
    pub fn len(&self, kind: Kind) -> usize {
        self.nodes(kind).len()
    }

    /// The node at `at`, or `None` past the end of its list.
    pub fn node(&self, at: NodeRef) -> Option<Node<'_>> {
        self.nodes(at.kind)
            .get(at.index)
            .map(|raw| Node { at, raw })
    }

    /// Every node of a kind, in index order.
    pub fn iter(&self, kind: Kind) -> impl Iterator<Item = Node<'_>> {
        self.nodes(kind)
            .iter()
            .enumerate()
            .map(move |(index, raw)| Node {
                at: NodeRef { kind, index },
                raw,
            })
    }

    /// Every entry, then every reply.
    pub fn all(&self) -> impl Iterator<Item = Node<'_>> {
        self.iter(Kind::Entry).chain(self.iter(Kind::Reply))
    }

    /// `StartingList`: the links into the entries, tried in order; the
    /// first whose condition passes starts the conversation.
    pub fn starts(&self) -> Vec<Link<'_>> {
        links(list(&self.gff.root, "StartingList"), Kind::Entry)
    }

    /// The conversation-wide fields.
    pub fn info(&self) -> Info {
        let r = &self.gff.root;
        let resref_text = |label: &str| resref(r, label).map(|x| x.to_string());
        Info {
            delay_entry: number(r, "DelayEntry").map(|v| v as u32),
            delay_reply: number(r, "DelayReply").map(|v| v as u32),
            skippable: number(r, "Skippable").map(|v| v != 0),
            conversation_type: number(r, "ConversationType").map(|v| v as i32),
            computer_type: number(r, "ComputerType").map(|v| v as u32),
            camera_model: resref_text("CameraModel"),
            end_conversation: resref_text("EndConversation"),
            end_conversation_abort: resref_text("EndConverAbort"),
            ambient_track: resref_text("AmbientTrack"),
            vo_id: string(r, "VO_ID").filter(|v| !v.is_empty()),
            animated_cut: number(r, "AnimatedCut").map(|v| v != 0),
        }
    }
}
