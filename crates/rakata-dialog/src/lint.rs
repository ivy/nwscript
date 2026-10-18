//! The dialogue lints: the reference checks of [`check`], and the rules that
//! read the whole dialogue. A link to a node the dialogue lacks; a node no
//! start reaches; a voiced node without its lip-sync file; a condition that
//! computes the name of the global it reads (a name built on a literal
//! prefix is checked as a family: `globalcat.2da` must declare some global
//! with that prefix); a `SoundExists` that disagrees
//! with the node's `Sound` and `VO_ResRef`.
//!
//! Reachability is structural: from every start, along every link, whatever
//! the conditions answer.

use rakata_res::ResType;

use crate::check::{Finding, Place, References, check};
use crate::error::codes;
use crate::key::NodeKey;
use crate::model::{Dialog, Kind, Link, Node, NodeRef};

/// Every finding of `dialog`, ordered by where it lies (entries, then
/// replies, each in index order, a node's own findings before those of its
/// links, the starts last) and then by code.
pub fn lint(dialog: &Dialog, refs: &mut dyn References) -> Vec<Finding> {
    let mut out = check(dialog, refs);
    dangling(dialog, &mut out);
    unreached(dialog, &mut out);
    for node in dialog.all() {
        let find = |code, subject: String| Finding {
            code,
            place: Place::Node(node.at),
            key: Some(NodeKey::of(dialog, node.at)),
            subject,
            via: None,
        };
        if let Some(vo) = node.voiced()
            && !refs.resolves(&vo, ResType::Lip)
        {
            out.push(find(codes::MISSING_LIP, vo.to_string()));
        }
        if let Some(what) = sound_exists(&node, refs) {
            out.push(find(codes::SOUND_EXISTS, what));
        }
    }
    for node in dialog.all() {
        for link in node.links() {
            let place = Place::Link {
                from: node.at,
                to: link.target,
            };
            computed(&link, place, NodeKey::of(dialog, node.at), refs, &mut out);
        }
    }
    for link in dialog.starts() {
        let key = NodeKey::of(dialog, link.target);
        computed(&link, Place::Start { to: link.target }, key, refs, &mut out);
    }
    out.sort_by(|a, b| {
        order(&a.place)
            .cmp(&order(&b.place))
            .then(a.code.cmp(b.code))
    });
    out
}

/// A place's position in the order findings are listed: a node, then the
/// links leaving it, then the starts after every node, then the end
/// scripts.
fn order(p: &Place) -> (u8, NodeRef, u8, Option<NodeRef>) {
    match p {
        Place::Node(n) => (0, *n, 0, None),
        Place::Link { from, to } => (0, *from, 1, Some(*to)),
        Place::Start { to } => (1, *to, 0, None),
        Place::End { aborted } => (2, NodeRef::entry(0), u8::from(*aborted), None),
    }
}

fn dangling(dialog: &Dialog, out: &mut Vec<Finding>) {
    let mut push = |place, key, target: NodeRef| {
        out.push(Finding {
            code: codes::DANGLING_LINK,
            place,
            key: Some(key),
            subject: target.to_string(),
            via: None,
        })
    };
    for node in dialog.all() {
        for link in node.links() {
            if dialog.node(link.target).is_none() {
                let place = Place::Link {
                    from: node.at,
                    to: link.target,
                };
                push(place, NodeKey::of(dialog, node.at), link.target);
            }
        }
    }
    for link in dialog.starts() {
        if dialog.node(link.target).is_none() {
            let key = NodeKey::Index(link.target.kind, link.target.index);
            push(Place::Start { to: link.target }, key, link.target);
        }
    }
}

/// The nodes no start reaches through any chain of links.
fn unreached(dialog: &Dialog, out: &mut Vec<Finding>) {
    let mut seen = [
        vec![false; dialog.len(Kind::Entry)],
        vec![false; dialog.len(Kind::Reply)],
    ];
    let slot = |n: NodeRef| (usize::from(n.kind == Kind::Reply), n.index);
    let mut todo: Vec<NodeRef> = dialog.starts().iter().map(|l| l.target).collect();
    while let Some(at) = todo.pop() {
        let (k, i) = slot(at);
        let Some(node) = dialog.node(at) else {
            continue;
        };
        if std::mem::replace(&mut seen[k][i], true) {
            continue;
        }
        todo.extend(node.links().iter().map(|l| l.target));
    }
    for node in dialog.all() {
        let (k, i) = slot(node.at);
        if !seen[k][i] {
            out.push(Finding {
                code: codes::UNREACHED_NODE,
                place: Place::Node(node.at),
                key: Some(NodeKey::of(dialog, node.at)),
                subject: String::new(),
                via: None,
            });
        }
    }
}

/// How `SoundExists` disagrees with the node, if it does: set where the
/// node names no audio at all, or `0` where the voice-over it names is
/// present.
fn sound_exists(node: &Node<'_>, refs: &mut dyn References) -> Option<String> {
    let flag = node.sound_exists()?;
    match (flag, node.voice(), node.sound()) {
        (0, Some(vo), _) if refs.resolves(&vo, ResType::Wav) => Some(format!(
            "SoundExists 0, and its voice-over {vo}.wav is present"
        )),
        (f, None, None) if f != 0 => Some(format!(
            "SoundExists {f}, and it names neither a Sound nor a VO_ResRef"
        )),
        _ => None,
    }
}

fn computed(
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
        return;
    }
    let reads = refs.global_reads(&c);
    for prefix in reads.prefixes {
        let (code, subject, via) = match refs.declares_prefix(&prefix) {
            Some(true) => continue,
            Some(false) => (
                codes::UNDECLARED_GLOBAL,
                format!("{prefix}*"),
                c.to_string(),
            ),
            None => (
                codes::UNDECIDABLE_GLOBAL,
                c.to_string(),
                format!("{prefix}*"),
            ),
        };
        out.push(Finding {
            code,
            place: place.clone(),
            key: Some(key.clone()),
            subject,
            via: Some(via),
        });
    }
    for routine in reads.computed {
        out.push(Finding {
            code: codes::UNDECIDABLE_GLOBAL,
            place: place.clone(),
            key: Some(key.clone()),
            subject: c.to_string(),
            via: Some(routine),
        });
    }
}
