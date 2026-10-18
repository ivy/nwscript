//! Additions to a dialogue the game already has, nodes addressed by stable
//! key: entries and replies at the ends of their lists, links added inside
//! a node, starts added to the starting list, so every index the dialogue,
//! its scripts or another mod's change uses stays where it is. Reordering a
//! list is [`reorder`], apart, because it moves indices.
//!
//! A key that matches no node or several fails with `rakata-dialog`'s
//! diagnostic, unchanged (`RE1102`, `RE1103`). Text keys here compare with
//! the text a node embeds; [`super::Mod::find`] compares through the talk
//! table too, and its [`NodeRef`] becomes a key with `NodeKey::from`.
//! What a dialogue refers to is checked when it is written
//! ([`super::Mod::write_override`]), not when it is edited, so a scene can
//! be built before its scripts and voice-overs exist.

use rakata_dialog::{Dialog, Kind, NewNode, NodeKey, NodeRef};
use rakata_res::ResRef;

pub use rakata_dialog::{CAMERA_STATIC, NO_DELAY, Text};

use super::{Error, Result};

fn condition(c: Option<&str>) -> Result<Option<ResRef>> {
    Ok(match c {
        None | Some("") => None,
        Some(c) => Some(ResRef::new(c)?),
    })
}

/// Append an entry, a line of the owner's or of the speaker `node.speaker`
/// names, and return where it lies.
pub fn add_entry(dialog: &mut Dialog, node: &NewNode) -> Result<NodeRef> {
    Ok(dialog.push_entry(node)?)
}

/// Append a reply, a line of the player's (with [`Text::None`], the
/// conversation goes on without a choice), and return where it lies.
pub fn add_reply(dialog: &mut Dialog, node: &NewNode) -> Result<NodeRef> {
    Ok(dialog.push_reply(node)?)
}

/// Link the node `from` to the node `to`, of the other kind, offered when
/// the script `condition` returns true (always without one), at `at` among
/// `from`'s links or after the last. Returns the link's position. Fails on
/// a key that names no node or several, on two nodes of one kind and on a
/// position past the end (`RE1106`).
pub fn link(
    dialog: &mut Dialog,
    from: &NodeKey,
    to: &NodeKey,
    condition: Option<&str>,
    at: Option<usize>,
) -> Result<usize> {
    let f = from.resolve(dialog)?;
    let t = to.resolve(dialog)?;
    Ok(dialog.push_link(f, t, self::condition(condition)?, at)?)
}

/// Change the script the node `at` runs when it is shown from `old` to
/// `new` (`None` for no script), as a mod changes what one of the game's
/// lines does: a companion's join reply running the mod's join script.
/// Names compare without case. Fails, with the dialogue unchanged, on a key
/// that names no node or several, and when the node does not run `old`
/// (`RE1416`): another mod changed it, or the key names another node.
pub fn replace_script(
    dialog: &mut Dialog,
    at: &NodeKey,
    old: Option<&str>,
    new: Option<&str>,
) -> Result<()> {
    let n = at.resolve(dialog)?;
    let expected = condition(old)?;
    let new = condition(new)?;
    let found = dialog.node(n).and_then(|node| node.script());
    let same = match (&found, &expected) {
        (None, None) => true,
        (Some(f), Some(e)) => f.to_string().eq_ignore_ascii_case(&e.to_string()),
        _ => false,
    };
    if !same {
        return Err(Error::ScriptDiffers {
            dialog: dialog.name().to_string(),
            key: at.to_string(),
            expected: expected.map(|r| r.to_string()),
            found: found.map(|r| r.to_string()),
        });
    }
    dialog.set_script(n, new)?;
    Ok(())
}

/// Remove every link out of the node `from`. Nodes keep their indices; the
/// ones only `from` led to are left unreached until linked again.
pub fn unlink_all(dialog: &mut Dialog, from: &NodeKey) -> Result<()> {
    let f = from.resolve(dialog)?;
    Ok(dialog.clear_links(f)?)
}

/// A way into the conversation at the entry `to`, under `condition`, at
/// `at` in the starting list, or first: the engine starts at the first
/// start whose condition passes, and a dialogue's own last start usually
/// has none, so a start placed after it is never taken. Returns its
/// position.
pub fn start(
    dialog: &mut Dialog,
    to: &NodeKey,
    condition: Option<&str>,
    at: Option<usize>,
) -> Result<usize> {
    let t = to.resolve(dialog)?;
    Ok(dialog.push_start(t, self::condition(condition)?, at.unwrap_or(0))?)
}

/// Reorder a list: `order[i]` is the old index of the node that goes to
/// `i`. Every link and start into the list follows its node; any index held
/// outside the dialogue (a script's, another mod's change) does not, which
/// is why this is not one of the additions. Fails unless `order` is a
/// permutation of the list (`RE1107`).
pub fn reorder(dialog: &mut Dialog, kind: Kind, order: &[usize]) -> Result<()> {
    Ok(dialog.reorder(kind, order)?)
}

/// A new dialogue named `name` with the settings of `like` (the
/// conversation type, skippable, the camera model, the voice-over id) and
/// no lines, as a mod's party banter starts from the game's.
pub fn empty_like(like: &Dialog, name: &str) -> Dialog {
    like.emptied(name)
}
