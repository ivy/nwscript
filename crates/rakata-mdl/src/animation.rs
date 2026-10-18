//! A model's animations: each a header, its events, and a tree of nodes
//! whose controllers carry the keys.

use crate::error::Result;
use crate::model::{Data, GEOM_NAME, GEOM_ROOT, Node, read_tree};

// An animation header, from its start (after the 80-byte geometry part).
const ANIM_LENGTH: usize = 0x50;
const ANIM_TRANSITION: usize = 0x54;
const ANIM_ROOT: usize = 0x58;
const ANIM_EVENTS: usize = 0x78;
/// The animation header's length.
pub const ANIMATION_HEADER_LEN: usize = 0x88;
/// The length of one event record.
pub const EVENT_LEN: usize = 36;

/// One animation of a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation<'a> {
    /// Its name.
    pub name: &'a str,
    /// Its length in seconds; no key of a vanilla animation lies past it.
    pub length: f32,
    /// The transition time in seconds (0.25 in most).
    pub transition: f32,
    /// The animation root's name: the model's own name in most.
    pub root: &'a str,
    /// The events, in stored order.
    pub events: Vec<Event<'a>>,
    /// The animation's nodes, depth first in child order: `nodes[0]` is
    /// the root. Each is a dummy whose name (and number, in the model's
    /// name array) says which node of the model it animates; its
    /// controllers carry the keys.
    pub nodes: Vec<Node<'a>>,
}

/// A named moment of an animation, such as a footstep's sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event<'a> {
    /// The time in seconds from the animation's start.
    pub time: f32,
    /// The event's name (`snd_footstep`, `hit`, `detonate`, …).
    pub name: &'a str,
}

impl<'a> Animation<'a> {
    /// The first node named `name`, compared without case.
    pub fn node(&self, name: &str) -> Option<&Node<'a>> {
        self.nodes
            .iter()
            .find(|n| n.name.eq_ignore_ascii_case(name))
    }
}

pub(crate) fn read_animation<'a>(
    d: &Data<'a>,
    names: &[&'a str],
    at: usize,
) -> Result<Animation<'a>> {
    d.slice(at, ANIMATION_HEADER_LEN, "animation header")?;
    let (ev_off, ev_count) = d.array(at + ANIM_EVENTS, "event array")?;
    let records = d.slice(ev_off, ev_count * EVENT_LEN, "event array")?;
    let events = (0..records.len() / EVENT_LEN)
        .map(|i| {
            let e = ev_off + i * EVENT_LEN;
            Ok(Event {
                time: d.f32(e)?,
                name: d.fixed_str(e + 4, "event name")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Animation {
        name: d.fixed_str(at + GEOM_NAME, "animation name")?,
        length: d.f32(at + ANIM_LENGTH)?,
        transition: d.f32(at + ANIM_TRANSITION)?,
        root: d.fixed_str(at + ANIM_ROOT, "animation root")?,
        events,
        nodes: read_tree(
            d,
            &[],
            names,
            d.u32(at + GEOM_ROOT, "animation root node")? as usize,
        )?,
    })
}
