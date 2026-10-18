//! Edits that keep every index: new nodes go at the ends of their lists,
//! links are added or cleared inside a node, and starts are added to the
//! starting list, so a node another dialogue edit, a script or a mod's
//! change refers to by index stays where it is. Reordering a list moves
//! indices, so it is its own operation, [`Dialog::reorder`], which rewrites
//! every link into the list to follow.
//!
//! A node is written with the fields BioWare's conversations carry, in the
//! order the vanilla files hold them where the model has a choice.

use rakata_gff::{Label, LocString, Struct, Value};
use rakata_res::ResRef;
use rakata_tlk::codepage;

use crate::error::{Error, Result};
use crate::model::{Dialog, Kind, NodeRef};

/// `Delay` for a node that lasts as long as its voice line, or until the
/// player goes on: the value vanilla files hold on most voiced nodes.
pub const NO_DELAY: u32 = 0xFFFF_FFFF;

/// `CameraAngle` for the module's placed camera named by `CameraID` (the
/// area's `CameraList`).
pub const CAMERA_STATIC: u32 = 6;

/// What a new node says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Text {
    /// No text: for an entry, a beat with no line; for a reply, the
    /// conversation goes on without the player choosing.
    #[default]
    None,
    /// The game's own line, by its `dialog.tlk` number.
    StrRef(u32),
    /// A line of the mod's own, embedded in English.
    English(String),
}

/// A new entry or reply. Fields a reply does not carry (`speaker`,
/// `animations`, `quest`, `camera`) are ignored for one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewNode {
    pub text: Text,
    /// The speaker's tag; empty for the dialogue's owner.
    pub speaker: String,
    /// The tag of who is spoken to; empty when unset.
    pub listener: String,
    /// `VO_ResRef`: the voice-over under `streamwaves/`.
    pub voice: Option<ResRef>,
    /// `Script`: run when the node is shown.
    pub script: Option<ResRef>,
    /// Participant tag and the engine's animation number, in order.
    pub animations: Vec<(String, u16)>,
    /// The journal category and entry the node updates.
    pub quest: Option<(String, u32)>,
    /// A placed camera: `CameraAngle` becomes [`CAMERA_STATIC`] and
    /// `CameraID` this.
    pub camera: Option<i32>,
    /// `Comment`, the author's note.
    pub comment: String,
}

fn label(s: &str) -> Label {
    Label::new(s).expect("the model's labels are at most sixteen bytes")
}

fn resref(r: Option<ResRef>) -> Value {
    Value::ResRef(r.unwrap_or_else(|| ResRef::from_bytes(b"").expect("the empty resref")))
}

fn string(text: &str, field: &'static str) -> Result<Value> {
    codepage::encode(text)
        .map(Value::String)
        .map_err(|c| Error::Unencodable {
            field,
            character: c,
        })
}

fn locstring(text: &Text) -> Result<LocString> {
    Ok(match text {
        Text::None => LocString::none(),
        Text::StrRef(s) => LocString::strref(*s),
        Text::English(t) if t.is_empty() => LocString::none(),
        Text::English(t) => {
            let mut l = LocString::none();
            let bytes = codepage::encode(t).map_err(|c| Error::Unencodable {
                field: "Text",
                character: c,
            })?;
            l.set(0, bytes);
            l
        }
    })
}

fn link_struct(id: usize, index: usize, condition: Option<ResRef>, child: Option<bool>) -> Struct {
    let mut s = Struct::new(id as u32);
    s.push(label("Index"), Value::Dword(index as u32))
        .push(label("Active"), resref(condition));
    if let Some(child) = child {
        s.push(label("IsChild"), Value::Byte(u8::from(child)));
    }
    s
}

fn list_mut<'a>(s: &'a mut Struct, name: &str) -> &'a mut Vec<Struct> {
    if !matches!(s.get(name), Some(Value::List(_))) {
        s.set(label(name), Value::List(Vec::new()));
    }
    match s.get_mut(name) {
        Some(Value::List(l)) => l,
        _ => unreachable!("set to a list above"),
    }
}

impl Dialog {
    fn nodes_mut(&mut self, kind: Kind) -> &mut Vec<Struct> {
        list_mut(&mut self.gff.root, kind.list())
    }

    fn require(&self, at: NodeRef) -> Result<()> {
        match self.node(at) {
            Some(_) => Ok(()),
            None => Err(Error::NoSuchNode {
                key: crate::NodeKey::Index(at.kind, at.index).to_string(),
                dialog: self.name.clone(),
            }),
        }
    }

    /// Append an entry: a line of the owner's or another speaker's. Returns
    /// where it lies; nothing else moves.
    pub fn push_entry(&mut self, node: &NewNode) -> Result<NodeRef> {
        self.push_node(Kind::Entry, node)
    }

    /// Append a reply: a line of the player's. Returns where it lies;
    /// nothing else moves.
    pub fn push_reply(&mut self, node: &NewNode) -> Result<NodeRef> {
        self.push_node(Kind::Reply, node)
    }

    fn push_node(&mut self, kind: Kind, n: &NewNode) -> Result<NodeRef> {
        let index = self.len(kind);
        let mut s = Struct::new(index as u32);
        let mut anims = Vec::new();
        if kind == Kind::Entry {
            for (i, (participant, animation)) in n.animations.iter().enumerate() {
                let mut a = Struct::new(i as u32);
                a.push(label("Participant"), string(participant, "Participant")?)
                    .push(label("Animation"), Value::Word(*animation));
                anims.push(a);
            }
        }
        s.push(label("AnimList"), Value::List(anims))
            .push(label("Text"), Value::LocString(locstring(&n.text)?))
            .push(label("VO_ResRef"), resref(n.voice))
            .push(label("Script"), resref(n.script))
            .push(label("Delay"), Value::Dword(NO_DELAY))
            .push(label("Comment"), string(&n.comment, "Comment")?)
            .push(label("Sound"), resref(None))
            .push(label("Quest"), Value::string(""))
            .push(label("PlotIndex"), Value::Int(-1))
            .push(label("PlotXPPercentage"), Value::Float(1.0))
            .push(label("Listener"), string(&n.listener, "Listener")?)
            .push(label("WaitFlags"), Value::Dword(0))
            .push(label("CameraAngle"), Value::Dword(0))
            .push(label("FadeType"), Value::Byte(0))
            .push(label(kind.links()), Value::List(Vec::new()));
        if kind == Kind::Entry {
            s.push(label("Speaker"), string(&n.speaker, "Speaker")?)
                .push(
                    label("SoundExists"),
                    Value::Byte(u8::from(n.voice.is_some())),
                );
            if let Some((quest, entry)) = &n.quest {
                s.set(label("Quest"), string(quest, "Quest")?);
                s.push(label("QuestEntry"), Value::Dword(*entry));
            }
            if let Some(camera) = n.camera {
                s.set(label("CameraAngle"), Value::Dword(CAMERA_STATIC));
                s.push(label("CameraID"), Value::Int(camera))
                    .push(label("CamVidEffect"), Value::Int(-1));
            }
        }
        self.nodes_mut(kind).push(s);
        Ok(NodeRef { kind, index })
    }

    /// Add a link from `from` to `to`, a node of the other kind, offered
    /// when `condition` returns true (always without one), at `at` among
    /// `from`'s links or after the last. Fails when either node is missing,
    /// when both are of one kind, and when `at` lies past the end.
    pub fn push_link(
        &mut self,
        from: NodeRef,
        to: NodeRef,
        condition: Option<ResRef>,
        at: Option<usize>,
    ) -> Result<usize> {
        self.require(from)?;
        self.require(to)?;
        if from.kind == to.kind {
            return Err(Error::BadLink {
                dialog: self.name.clone(),
                reason: format!(
                    "{from} and {to} are both {}s; a link joins an entry and a reply",
                    from.kind.name()
                ),
            });
        }
        let name = self.name.clone();
        let node = &mut self.nodes_mut(from.kind)[from.index];
        let links = list_mut(node, from.kind.links());
        let at = position(at, links.len(), &name, &format!("{from}'s links"))?;
        links.insert(
            at,
            link_struct(links.len(), to.index, condition, Some(false)),
        );
        Ok(at)
    }

    /// Add a starting link to the entry `to`, at `at` in the starting list.
    /// The engine starts the conversation at the first start whose
    /// condition passes, so a start meant to be tried before the
    /// dialogue's own goes at 0. Fails when `to` is not an entry of the
    /// dialogue or `at` lies past the end.
    pub fn push_start(
        &mut self,
        to: NodeRef,
        condition: Option<ResRef>,
        at: usize,
    ) -> Result<usize> {
        self.require(to)?;
        if to.kind != Kind::Entry {
            return Err(Error::BadLink {
                dialog: self.name.clone(),
                reason: format!("{to} is a reply; a conversation starts at an entry"),
            });
        }
        let name = self.name.clone();
        let starts = list_mut(&mut self.gff.root, "StartingList");
        let at = position(Some(at), starts.len(), &name, "the starting list")?;
        starts.insert(at, link_struct(starts.len(), to.index, condition, None));
        Ok(at)
    }

    /// Remove every link out of `from`, leaving the node where it is.
    pub fn clear_links(&mut self, from: NodeRef) -> Result<()> {
        self.require(from)?;
        let node = &mut self.nodes_mut(from.kind)[from.index];
        list_mut(node, from.kind.links()).clear();
        Ok(())
    }

    /// Reorder a list: `order[i]` is the index, before, of the node that
    /// lies at `i` after. Every link into the list (the other kind's links,
    /// and the starts for entries) is rewritten to follow its node. Fails
    /// unless `order` holds each index of the list once.
    pub fn reorder(&mut self, kind: Kind, order: &[usize]) -> Result<()> {
        let len = self.len(kind);
        let mut new_of = vec![usize::MAX; len];
        for (new, &old) in order.iter().enumerate() {
            if old >= len || new_of[old] != usize::MAX {
                break;
            }
            new_of[old] = new;
        }
        if order.len() != len || new_of.contains(&usize::MAX) {
            return Err(Error::BadOrder {
                dialog: self.name.clone(),
                list: kind.list(),
                len,
            });
        }
        let renumber = |links: &mut Vec<Struct>| {
            for l in links {
                if let Some(Value::Dword(i)) = l.get_mut("Index")
                    && let Some(&n) = new_of.get(*i as usize)
                {
                    *i = n as u32;
                }
            }
        };
        for node in self.nodes_mut(kind.other()) {
            renumber(list_mut(node, kind.other().links()));
        }
        if kind == Kind::Entry {
            renumber(list_mut(&mut self.gff.root, "StartingList"));
        }
        let nodes = std::mem::take(self.nodes_mut(kind));
        let mut slots: Vec<Option<Struct>> = nodes.into_iter().map(Some).collect();
        let reordered = order
            .iter()
            .map(|&old| slots[old].take().expect("a permutation"))
            .collect();
        *self.nodes_mut(kind) = reordered;
        Ok(())
    }

    /// A dialogue with this one's conversation-wide settings (type,
    /// skippable, camera model, the voice-over id) and no lines: no
    /// entries, replies or starts, and no end scripts. Named `name`. It is
    /// a new file, laid out as one written in one pass, so none of this
    /// one's layout comes with it.
    pub fn emptied(&self, name: impl Into<String>) -> Dialog {
        let mut gff = rakata_gff::Gff::new(self.gff.file_type);
        gff.version = self.gff.version;
        gff.root = self.gff.root.clone();
        let mut d = Dialog {
            name: name.into(),
            gff,
        };
        for list in ["EntryList", "ReplyList", "StartingList"] {
            d.gff.root.set(label(list), Value::List(Vec::new()));
        }
        for script in ["EndConversation", "EndConverAbort"] {
            d.gff.root.set(label(script), resref(None));
        }
        d
    }
}

fn position(at: Option<usize>, len: usize, dialog: &str, what: &str) -> Result<usize> {
    match at {
        None => Ok(len),
        Some(at) if at <= len => Ok(at),
        Some(at) => Err(Error::BadLink {
            dialog: dialog.to_string(),
            reason: format!("position {at} lies past the end of {what}, which holds {len}"),
        }),
    }
}
