//! Two dialogues compared as transcripts: nodes added, removed and changed,
//! each named by a stable key rather than by its index.
//!
//! A node's key is its voice resref, else its strref, else its embedded
//! text, as [`rakata_dialog::NodeKey`] spells them; a node with none of
//! these is keyed by its index, which moves when nodes are inserted, and
//! the site says so (`fragile`). Nodes pair by kind and key, the n-th node
//! of a repeated key with the n-th. A paired node is compared field by
//! field, with each link's `Index` replaced by its target's key, so a node
//! that only moved within its list is not a change; the root's fields and
//! the starting list are compared the same way.

use std::collections::HashMap;

use rakata_dialog::{Dialog, Kind, NodeKey, NodeRef};
use rakata_gff::{Gff, Struct, Value};
use rakata_res::{Gender, Language};
use rakata_tlk::Tlk;
use serde::Serialize;

use super::{Change, FieldChange, Site};

/// A node added, removed or changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeChange {
    pub change: Change,
    /// `entry` or `reply`.
    pub kind: &'static str,
    /// The stable key: `voice:RESREF`, `strref:N`, `text:"..."`, or
    /// `index:E3 (fragile)`; a key repeated in the dialogue carries its
    /// occurrence (`strref:N #2`).
    pub key: String,
    /// Whether the key is the node's index.
    pub fragile: bool,
    /// The node's index on each side, as `E3`.
    pub a: Option<String>,
    pub b: Option<String>,
    /// The text the node shows: the talk table's line when a tree's
    /// `dialog.tlk` is at hand, else the embedded text.
    pub text: Option<String>,
    /// For a node added or removed: the keys of the nodes it links to.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
    /// For a node changed: every field that differs, links by target key.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldChange>,
}

/// One side's nodes keyed.
struct Keyed {
    /// Each node's key and whether it is fragile.
    of: HashMap<NodeRef, (String, bool)>,
    /// The nodes by kind and key.
    by: HashMap<(Kind, String), NodeRef>,
    /// The nodes in order: entries, then replies.
    order: Vec<NodeRef>,
}

fn keyed(d: &Dialog) -> Keyed {
    let mut of = HashMap::new();
    let mut by = HashMap::new();
    let mut order = Vec::new();
    let mut seen: HashMap<(Kind, String), usize> = HashMap::new();
    for n in d.all() {
        let base = if let Some(vo) = n.voice() {
            NodeKey::Voice(vo)
        } else if let Some(s) = n.strref() {
            NodeKey::StrRef(s)
        } else if let Some(t) = n.embedded_text().filter(|t| !t.trim().is_empty()) {
            NodeKey::Text(t.trim().to_owned())
        } else {
            NodeKey::Index(n.at.kind, n.at.index)
        };
        let fragile = base.is_fragile();
        let text = base.to_string();
        let count = seen.entry((n.at.kind, text.clone())).or_insert(0);
        *count += 1;
        let key = if *count == 1 {
            text
        } else {
            format!("{text} #{count}")
        };
        by.insert((n.at.kind, key.clone()), n.at);
        of.insert(n.at, (key, fragile));
        order.push(n.at);
    }
    Keyed { of, by, order }
}

/// What a link's `Index` is replaced with: its target's key.
fn target_key(k: &Keyed, kind: Kind, index: &Value) -> Value {
    let i = match index {
        Value::Dword(i) => *i as usize,
        Value::Int(i) => *i as usize,
        Value::Word(i) => usize::from(*i),
        _ => return index.clone(),
    };
    let at = NodeRef { kind, index: i };
    let text = match k.of.get(&at) {
        Some((key, _)) => key.clone(),
        None => format!("missing {at}"),
    };
    Value::String(text.into_bytes())
}

/// A list of links with each `Index` replaced by its target key and each
/// struct id, which follows the link's position, cleared.
fn links_keyed(k: &Keyed, kind: Kind, links: &mut [Struct]) {
    for l in links {
        l.id = 0;
        if let Some(v) = l.get_mut("Index") {
            *v = target_key(k, kind, v);
        }
    }
}

/// A node's struct as compared, without its list of links, and the links
/// keyed.
fn normalized(k: &Keyed, d: &Dialog, at: NodeRef) -> (Struct, Vec<Struct>) {
    let mut s = d
        .node(at)
        .map_or_else(|| Struct::new(0), |n| n.raw().clone());
    // A node's struct id follows its place in the list, which the key
    // replaces.
    s.id = 0;
    let mut links = match s.remove(at.kind.links()).map(|f| f.value) {
        Some(Value::List(l)) => l,
        _ => Vec::new(),
    };
    links_keyed(k, at.kind.other(), &mut links);
    (s, links)
}

/// The key a link pairs by: its target's.
fn link_target(l: &Struct) -> String {
    match l.get("Index") {
        Some(Value::String(b)) => String::from_utf8_lossy(b).into_owned(),
        Some(v) => super::compare::value_text(v),
        None => String::new(),
    }
}

/// A keyed link as text: `to KEY if CONDITION (child)`, then any field
/// the model does not name.
fn link_text(l: &Struct) -> String {
    let mut out = format!("to {}", link_target(l));
    for f in &l.fields {
        match (f.label.as_str(), &f.value) {
            (Some("Index"), _) => {}
            (Some("Active"), Value::ResRef(r)) if r.is_empty() => {}
            (Some("Active"), v) => out.push_str(&format!(" if {}", super::compare::value_text(v))),
            (Some("IsChild"), Value::Byte(0)) => {}
            (Some("IsChild"), Value::Byte(1)) => out.push_str(" (child)"),
            (_, v) => out.push_str(&format!(" {}={}", f.label, super::compare::value_text(v))),
        }
    }
    out
}

/// Two lists of keyed links compared by target: a link added, removed or
/// changed (its condition, `IsChild` or any other field), and the order of
/// the links both have when it differs. A link inserted before others is
/// one change, not a shift of every link after it.
fn link_changes(label: &str, a: &[Struct], b: &[Struct]) -> Vec<FieldChange> {
    let link = |l: &Struct| super::Shown {
        ty: "link".to_owned(),
        value: link_text(l),
    };
    let mut theirs: HashMap<String, Vec<usize>> = HashMap::new();
    for (j, l) in b.iter().enumerate() {
        theirs.entry(link_target(l)).or_default().push(j);
    }
    let mut used = vec![false; b.len()];
    let mut out = Vec::new();
    let mut order_a = Vec::new();
    for (i, la) in a.iter().enumerate() {
        let partner = theirs
            .get(&link_target(la))
            .and_then(|v| v.iter().copied().find(|&j| !used[j]));
        match partner {
            None => out.push(FieldChange {
                path: format!("{label}[{i}]"),
                before: Some(link(la)),
                after: None,
            }),
            Some(j) => {
                used[j] = true;
                order_a.push(j);
                if !la.semantic_eq(&b[j]) {
                    out.push(FieldChange {
                        path: format!("{label}[{j}]"),
                        before: Some(link(la)),
                        after: Some(link(&b[j])),
                    });
                }
            }
        }
    }
    for (j, lb) in b.iter().enumerate() {
        if !used[j] {
            out.push(FieldChange {
                path: format!("{label}[{j}]"),
                before: None,
                after: Some(link(lb)),
            });
        }
    }
    if order_a.windows(2).any(|w| w[0] > w[1]) {
        let mut order_b = order_a.clone();
        order_b.sort_unstable();
        let keys = |js: &[usize]| super::Shown {
            ty: "order".to_owned(),
            value: js
                .iter()
                .map(|&j| link_target(&b[j]))
                .collect::<Vec<_>>()
                .join(", "),
        };
        out.push(FieldChange {
            path: format!("{label} (order)"),
            before: Some(keys(&order_a)),
            after: Some(keys(&order_b)),
        });
    }
    out
}

fn wrap(root: Struct) -> Gff {
    let mut g = Gff::new(*b"DLG ");
    g.root = root;
    g
}

fn shown_text(d: &Dialog, at: NodeRef, talk: Option<&Tlk>) -> Option<String> {
    let n = d.node(at)?;
    let text = n.text();
    talk.and_then(|t| t.resolve(&text, Language::English, Gender::Male))
        .map(str::to_owned)
        .or_else(|| n.embedded_text())
        .filter(|t| !t.is_empty())
}

fn link_keys(k: &Keyed, d: &Dialog, at: NodeRef) -> Vec<String> {
    d.node(at)
        .map(|n| {
            n.links()
                .iter()
                .map(|l| match k.of.get(&l.target) {
                    Some((key, _)) => key.clone(),
                    None => format!("missing {}", l.target),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The sites at which two dialogues differ: the root's fields and the
/// starting list as fields, then each node by key.
pub fn diff(a: &Dialog, b: &Dialog, talk_a: Option<&Tlk>, talk_b: Option<&Tlk>) -> Vec<Site> {
    let (ka, kb) = (keyed(a), keyed(b));
    let mut sites = Vec::new();

    let root = |k: &Keyed, d: &Dialog| {
        let mut r = d.gff().root.clone();
        r.remove("EntryList");
        r.remove("ReplyList");
        let mut starts = match r.remove("StartingList").map(|f| f.value) {
            Some(Value::List(l)) => l,
            _ => Vec::new(),
        };
        links_keyed(k, Kind::Entry, &mut starts);
        (wrap(r), starts)
    };
    let ((ra, sa), (rb, sb)) = (root(&ka, a), root(&kb, b));
    sites.extend(
        super::compare::fields(&ra, &rb)
            .into_iter()
            .chain(link_changes("StartingList", &sa, &sb))
            .map(Site::Field),
    );

    let mut paired: HashMap<NodeRef, NodeRef> = HashMap::new();
    for &at in &ka.order {
        let (key, _) = &ka.of[&at];
        if let Some(&bt) = kb.by.get(&(at.kind, key.clone())) {
            paired.insert(bt, at);
        }
    }
    let unpaired_a: Vec<NodeRef> = {
        let back: std::collections::HashSet<&NodeRef> = paired.values().collect();
        ka.order
            .iter()
            .copied()
            .filter(|at| !back.contains(at))
            .collect()
    };
    for &bt in &kb.order {
        let (key, fragile) = kb.of[&bt].clone();
        match paired.get(&bt) {
            Some(&at) => {
                let ((na, la), (nb, lb)) = (normalized(&ka, a, at), normalized(&kb, b, bt));
                let mut fields = super::compare::fields(&wrap(na), &wrap(nb));
                fields.extend(link_changes(bt.kind.links(), &la, &lb));
                if !fields.is_empty() {
                    sites.push(Site::Node(NodeChange {
                        change: Change::Changed,
                        kind: bt.kind.name(),
                        key,
                        fragile,
                        a: Some(at.to_string()),
                        b: Some(bt.to_string()),
                        text: shown_text(b, bt, talk_b),
                        links: Vec::new(),
                        fields,
                    }));
                }
            }
            None => sites.push(Site::Node(NodeChange {
                change: Change::Added,
                kind: bt.kind.name(),
                key,
                fragile,
                a: None,
                b: Some(bt.to_string()),
                text: shown_text(b, bt, talk_b),
                links: link_keys(&kb, b, bt),
                fields: Vec::new(),
            })),
        }
    }
    for at in unpaired_a {
        let (key, fragile) = ka.of[&at].clone();
        sites.push(Site::Node(NodeChange {
            change: Change::Removed,
            kind: at.kind.name(),
            key,
            fragile,
            a: Some(at.to_string()),
            b: None,
            text: shown_text(a, at, talk_a),
            links: link_keys(&ka, a, at),
            fields: Vec::new(),
        }));
    }
    sites
}
