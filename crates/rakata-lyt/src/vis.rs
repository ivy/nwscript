//! Room visibility, `.vis`.

use crate::text::{self, Style, Writer};
use crate::{Error, Result};

/// One room and the rooms seen from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisRoom {
    /// The room's model, as the file spells it (the case need not match
    /// the layout's; the engine compares without regard to case).
    pub room: String,
    /// The rooms drawn while the camera is in [`room`](Self::room), in file
    /// order. The room itself is not listed in the vanilla files, with
    /// exceptions `docs/formats/lyt.md` names.
    pub visible: Vec<String>,
}

/// An area's room visibility: for each room, the rooms seen from it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vis {
    pub rooms: Vec<VisRoom>,
    style: Style,
}

const CHILD_INDENT: &str = "  ";

impl Vis {
    /// A visibility file of these rooms, written in the vanilla form.
    pub fn new(rooms: Vec<VisRoom>) -> Self {
        Vis {
            rooms,
            style: Style::default(),
        }
    }

    /// Parse a visibility file: a room line, `ROOM COUNT` at the start of a
    /// line, then the rooms it sees, one to an indented line. The rooms a
    /// room sees are the indented lines that follow it, whatever its count
    /// says (in three room lines of two vanilla files the count is one off;
    /// in one a room line has no count), and the count is kept only to write the file
    /// back. Blank lines are skipped. Refuses an indented line before any
    /// room line, a line in neither form, a count that is not a number,
    /// and a byte outside ASCII, each with its code.
    pub fn parse(bytes: &[u8]) -> Result<Vis> {
        let (lines, newline) = text::lines(bytes)?;
        let mut style = Style::new(newline);
        let mut rooms: Vec<VisRoom> = Vec::new();
        // The open room line: its index among the lines written and its
        // bytes, recorded once its rooms are known.
        let mut open: Option<(usize, &str)> = None;
        let close = |style: &mut Style, open: Option<(usize, &str)>, rooms: &[VisRoom]| {
            if let (Some((at, raw)), Some(r)) = (open, rooms.last()) {
                style.record(at, &format!("{} {}", r.room, r.visible.len()), raw);
            }
        };
        for (index, l) in lines.iter().filter(|l| !l.is_blank()).enumerate() {
            let tokens = l.tokens();
            if l.is_indented() {
                let [name] = tokens[..] else {
                    return Err(bad(l));
                };
                let Some(room) = rooms.last_mut() else {
                    return Err(Error::OrphanVisible {
                        line: l.number,
                        text: name.to_owned(),
                    });
                };
                room.visible.push(name.to_owned());
                style.record(index, &format!("{CHILD_INDENT}{name}"), l.raw);
            } else {
                let room = match tokens[..] {
                    [room] => room,
                    [room, count] => {
                        text::parse_count(count, l.number)?;
                        room
                    }
                    _ => return Err(bad(l)),
                };
                close(&mut style, open.take(), &rooms);
                rooms.push(VisRoom {
                    room: room.to_owned(),
                    visible: Vec::new(),
                });
                open = Some((index, l.raw));
            }
        }
        close(&mut style, open, &rooms);
        Ok(Vis { rooms, style })
    }

    /// The file's bytes. A file read writes back as the bytes it came
    /// from; one built or edited is written in the vanilla files' form:
    /// CRLF endings, `ROOM COUNT` with the count of rooms listed, and each
    /// room it sees indented two spaces. Refuses a name that is empty or
    /// holds whitespace.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut w = Writer::new(&self.style);
        for r in &self.rooms {
            let room = text::name(&r.room, || format!("visibility room `{}`", r.room))?;
            w.line(&format!("{room} {}", r.visible.len()));
            for v in &r.visible {
                let v = text::name(v, || format!("a room `{}` sees", r.room))?;
                w.line(&format!("{CHILD_INDENT}{v}"));
            }
        }
        Ok(w.finish().into_bytes())
    }

    /// Whether the two list the same rooms seeing the same rooms, in the
    /// same order and spelling; how the file spelled the lines aside.
    pub fn semantic_eq(&self, other: &Vis) -> bool {
        self.rooms == other.rooms
    }

    /// The rooms seen from `room`, compared without regard to case: the
    /// first entry for it, if the file has one.
    pub fn visible(&self, room: &str) -> Option<&[String]> {
        self.rooms
            .iter()
            .find(|r| r.room.eq_ignore_ascii_case(room))
            .map(|r| &r.visible[..])
    }

    /// How many lines of the file read depart from the form the writer
    /// produces (a count that disagrees with its rooms or is missing,
    /// trailing spaces, a missing final line ending): the writer keeps them
    /// as read.
    pub fn departures(&self) -> usize {
        self.style.departures()
    }
}

fn bad(l: &text::Line<'_>) -> Error {
    Error::BadVisLine {
        line: l.number,
        text: l.text.trim().to_owned(),
    }
}
