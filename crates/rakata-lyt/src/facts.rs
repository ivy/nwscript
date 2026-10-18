//! What a layout contributes to an index: the [`Ref`]s it makes to models.
//!
//! Each room, swoop track piece and obstacle names a model, and each door
//! hook names the room (a model) it belongs to; the sites are `room[N]`,
//! `track[N]`, `obstacle[N]` and `doorhook[N].room`, by position in their
//! section. A hook's own name (`Door_01`) names a place in the area, not a
//! resource. An empty name and `****` name nothing: eleven vanilla cutscene
//! layouts (`stunt_endbridge`, `stunt_ebobridge` and others) hold 134
//! rooms named `****`, which no file can be named.

use rakata_res::Ref;

use crate::Lyt;

/// Whether a name in a layout names a model.
fn names(s: &str) -> bool {
    !s.is_empty() && s != "****"
}

/// Every reference `lyt` makes, in file order.
pub fn refs(lyt: &Lyt) -> Vec<Ref> {
    let mut out = Vec::new();
    for (section, placements) in [
        ("room", &lyt.rooms),
        ("track", &lyt.tracks),
        ("obstacle", &lyt.obstacles),
    ] {
        for (i, p) in placements.iter().enumerate() {
            if names(&p.model) {
                out.push(Ref::resref(&p.model, format!("{section}[{i}]")));
            }
        }
    }
    for (i, h) in lyt.door_hooks.iter().enumerate() {
        if names(&h.room) {
            out.push(Ref::resref(&h.room, format!("doorhook[{i}].room")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rooms_tracks_obstacles_and_hooks_name_models() {
        let lyt = Lyt::parse(
            b"#MAXLAYOUT ASCII\r\nfiledependancy M01aa.max\r\nbeginlayout\r\n   roomcount 2\r\n      M01aa_01a 0.0 0.0 0.0\r\n      **** 0.0 0.0 0.0\r\n   trackcount 0\r\n   obstaclecount 0\r\n   doorhookcount 1\r\n      M01aa_01a Door_01 0 1.0 2.0 3.0 1.0 0.0 0.0 0.0\r\ndonelayout\r\n",
        )
        .unwrap();
        let r = refs(&lyt);
        assert_eq!(
            r,
            [
                Ref::resref("m01aa_01a", "room[0]"),
                Ref::resref("m01aa_01a", "doorhook[0].room"),
            ]
        );
    }
}
