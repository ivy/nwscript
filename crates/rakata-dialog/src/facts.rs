//! What a dialogue contributes to an index: one [`NodeFact`] per entry and
//! reply with the fields the model names, one [`LinkFact`] per starting
//! link and per link onward, and the [`Ref`]s its fields make.
//!
//! The references are those of [`rakata_gff::facts::refs`] over the
//! dialogue's tree, so their sites are GFF paths
//! (`EntryList[3].RepliesList[0].Active`). The function is pure: it reads
//! the dialogue and nothing else; text a node takes from the talk table is
//! left to the caller, by strref.

use rakata_res::Ref;

use crate::model::{Animation, Dialog, Kind, NodeRef};

/// One node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeFact {
    pub at: NodeRef,
    pub speaker: Option<String>,
    pub listener: Option<String>,
    /// The text's strref, when it has one.
    pub strref: Option<u32>,
    /// The English text the node embeds, when it embeds one.
    pub text: Option<String>,
    pub voice: Option<String>,
    pub sound: Option<String>,
    pub script: Option<String>,
    pub delay: Option<u32>,
    pub camera_angle: Option<u32>,
    pub camera_id: Option<i32>,
    pub camera_animation: Option<u32>,
    pub quest: Option<String>,
    pub quest_entry: Option<u32>,
    pub comment: Option<String>,
    pub animations: Vec<Animation>,
}

/// One link: from the start (`from` is `None`) or from a node, to a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFact {
    pub from: Option<NodeRef>,
    /// Its position in the list it lies in.
    pub position: usize,
    pub to: NodeRef,
    /// The condition script, lowercase.
    pub condition: Option<String>,
    pub is_child: bool,
}

/// The facts of one dialogue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// Every entry, then every reply, each in index order.
    pub nodes: Vec<NodeFact>,
    /// The starting links in order, then each node's links in node order.
    pub links: Vec<LinkFact>,
    pub refs: Vec<Ref>,
}

fn name(r: rakata_res::ResRef) -> String {
    String::from_utf8_lossy(r.as_bytes()).to_ascii_lowercase()
}

/// The facts of `dialog`.
pub fn facts(dialog: &Dialog) -> Facts {
    let mut out = Facts {
        refs: rakata_gff::facts::refs(dialog.gff()),
        ..Facts::default()
    };
    for l in dialog.starts() {
        out.links.push(LinkFact {
            from: None,
            position: l.position,
            to: l.target,
            condition: l.condition().map(name),
            is_child: l.is_child(),
        });
    }
    for kind in [Kind::Entry, Kind::Reply] {
        for n in dialog.iter(kind) {
            out.nodes.push(NodeFact {
                at: n.at,
                speaker: n.speaker().filter(|s| !s.is_empty()),
                listener: n.listener().filter(|s| !s.is_empty()),
                strref: n.strref(),
                text: n.embedded_text().filter(|s| !s.is_empty()),
                voice: n.voice().map(name),
                sound: n.sound().map(name),
                script: n.script().map(name),
                delay: n.delay(),
                camera_angle: n.camera_angle(),
                camera_id: n.camera_id(),
                camera_animation: n.camera_animation(),
                quest: n.quest(),
                quest_entry: n.quest_entry(),
                comment: n.comment(),
                animations: n.animations(),
            });
            for l in n.links() {
                out.links.push(LinkFact {
                    from: Some(n.at),
                    position: l.position,
                    to: l.target,
                    condition: l.condition().map(name),
                    is_child: l.is_child(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rakata_gff::{Gff, Label, LocString, Struct, Value};
    use rakata_res::{RefKind, ResRef};

    fn label(s: &str) -> Label {
        Label::new(s).unwrap()
    }

    fn link(index: u32, active: &str) -> Struct {
        let mut s = Struct::new(0);
        s.push(label("Index"), Value::Dword(index))
            .push(label("Active"), Value::ResRef(ResRef::new(active).unwrap()));
        s
    }

    fn sample() -> Dialog {
        let mut gff = Gff::new(*b"DLG ");
        let mut entry = Struct::new(0);
        entry
            .push(label("Speaker"), Value::string("Juhani"))
            .push(label("Text"), Value::LocString(LocString::strref(42724)))
            .push(
                label("Script"),
                Value::ResRef(ResRef::new("K_PDAN_Juhani27").unwrap()),
            )
            .push(label("Comment"), Value::string("she draws"))
            .push(label("RepliesList"), Value::List(vec![link(0, "")]));
        let mut reply = Struct::new(1);
        let mut text = LocString::none();
        text.set(0, b"Wait.".to_vec());
        reply.push(label("Text"), Value::LocString(text));
        gff.root
            .push(label("EntryList"), Value::List(vec![entry]))
            .push(label("ReplyList"), Value::List(vec![reply]))
            .push(
                label("StartingList"),
                Value::List(vec![link(0, "k_con_talked")]),
            );
        Dialog::new("sample.dlg", gff).unwrap()
    }

    #[test]
    fn nodes_carry_their_fields_and_links_their_conditions() {
        let f = facts(&sample());
        assert_eq!(f.nodes.len(), 2);
        let e = &f.nodes[0];
        assert_eq!(e.at, NodeRef::entry(0));
        assert_eq!(e.speaker.as_deref(), Some("Juhani"));
        assert_eq!(e.strref, Some(42724));
        assert_eq!(e.text, None);
        assert_eq!(e.script.as_deref(), Some("k_pdan_juhani27"));
        assert_eq!(e.comment.as_deref(), Some("she draws"));
        let r = &f.nodes[1];
        assert_eq!(r.at, NodeRef::reply(0));
        assert_eq!(r.text.as_deref(), Some("Wait."));
        assert_eq!(r.strref, None);
        assert_eq!(f.links.len(), 2);
        assert_eq!(f.links[0].from, None);
        assert_eq!(f.links[0].condition.as_deref(), Some("k_con_talked"));
        assert_eq!(f.links[1].from, Some(NodeRef::entry(0)));
        assert_eq!(f.links[1].to, NodeRef::reply(0));
        assert_eq!(f.links[1].condition, None);
    }

    #[test]
    fn references_are_the_trees() {
        let f = facts(&sample());
        let got: Vec<(RefKind, &str, &str)> = f
            .refs
            .iter()
            .map(|r| (r.kind, r.value.as_str(), r.site.as_str()))
            .collect();
        assert!(got.contains(&(RefKind::Tag, "Juhani", "EntryList[0].Speaker")));
        assert!(got.contains(&(RefKind::StrRef, "42724", "EntryList[0].Text")));
        assert!(got.contains(&(RefKind::ResRef, "k_pdan_juhani27", "EntryList[0].Script")));
        assert!(got.contains(&(RefKind::ResRef, "k_con_talked", "StartingList[0].Active")));
    }
}
