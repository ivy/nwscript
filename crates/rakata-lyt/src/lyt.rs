//! The room layout, `.lyt`.

use crate::text::{self, Line, Style, Writer};
use crate::{Error, Result};

/// A model placed in the area: a room, a swoop track piece or an obstacle.
/// The position is the model's origin in the area, in the game's
/// coordinates (metres, Z up), as the file stores it.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// The model's resref, as the file spells it (case varies; the engine
    /// compares without regard to case).
    pub model: String,
    /// `[x, y, z]`.
    pub position: [f32; 3],
}

impl Placement {
    pub fn new(model: impl Into<String>, position: [f32; 3]) -> Self {
        Placement {
            model: model.into(),
            position,
        }
    }

    /// Whether the two hold the same name and the same bits, which `==`
    /// does not ask of a float (`-0.0 == 0.0`).
    pub fn bits_eq(&self, other: &Placement) -> bool {
        self.model == other.model && bits3(self.position) == bits3(other.position)
    }
}

/// A rotation as a unit quaternion, `w` the scalar part. The file stores it
/// `w x y z` (`docs/formats/lyt.md` gives the evidence): a hook turned by
/// an angle `a` about the vertical axis is `w = cos(a/2)`, `z = sin(a/2)`,
/// `x = y = 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quaternion {
    /// No rotation.
    pub const IDENTITY: Quaternion = Quaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// The four components in file order, `[w, x, y, z]`.
    pub fn to_array(self) -> [f32; 4] {
        [self.w, self.x, self.y, self.z]
    }

    /// From the four components in file order, `[w, x, y, z]`.
    pub fn from_array([w, x, y, z]: [f32; 4]) -> Self {
        Quaternion { w, x, y, z }
    }

    pub fn bits_eq(&self, other: &Quaternion) -> bool {
        self.to_array().map(f32::to_bits) == other.to_array().map(f32::to_bits)
    }
}

/// Where a door stands: a named hook in a room's model, with the hook's
/// position in the area and its orientation.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorHook {
    /// The room (a [`Placement::model`] of [`Lyt::rooms`]) the hook belongs
    /// to.
    pub room: String,
    /// The hook's name, `Door_01` and the like.
    pub name: String,
    /// The integer between the name and the position: 0 in every vanilla
    /// file; what it means is not established.
    pub unknown: i32,
    /// `[x, y, z]` in the area, metres, Z up.
    pub position: [f32; 3],
    pub orientation: Quaternion,
}

impl DoorHook {
    pub fn bits_eq(&self, other: &DoorHook) -> bool {
        self.room == other.room
            && self.name == other.name
            && self.unknown == other.unknown
            && bits3(self.position) == bits3(other.position)
            && self.orientation.bits_eq(&other.orientation)
    }
}

fn bits3(v: [f32; 3]) -> [u32; 3] {
    v.map(f32::to_bits)
}

/// A line before `beginlayout`, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Preamble {
    /// A line beginning `#`, kept as read: `#MAXLAYOUT ASCII`.
    Comment(String),
    /// Where `filedependancy` stands.
    Dependency,
}

/// An area's room layout: the rooms and where they stand, the swoop track
/// pieces and obstacles of a minigame area, and the door hooks.
#[derive(Debug, Clone, PartialEq)]
pub struct Lyt {
    /// The `filedependancy` line's value (the format's spelling): the 3ds
    /// Max scene the layout was exported from, `M01aa.max`.
    pub file_dependency: Option<String>,
    pub rooms: Vec<Placement>,
    /// A swoop race's track pieces; empty outside the minigame areas.
    pub tracks: Vec<Placement>,
    /// A swoop race's obstacles; empty outside the minigame areas.
    pub obstacles: Vec<Placement>,
    pub door_hooks: Vec<DoorHook>,
    preamble: Vec<Preamble>,
    /// The bytes after the `donelayout` line, as read.
    trailer: String,
    style: Style,
}

impl Default for Lyt {
    /// No rooms, `#MAXLAYOUT ASCII` and no dependency: what the writer
    /// produces for a layout built from nothing.
    fn default() -> Self {
        Lyt {
            file_dependency: None,
            rooms: Vec::new(),
            tracks: Vec::new(),
            obstacles: Vec::new(),
            door_hooks: Vec::new(),
            preamble: vec![
                Preamble::Comment("#MAXLAYOUT ASCII".to_owned()),
                Preamble::Dependency,
            ],
            trailer: String::new(),
            style: Style::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Rooms,
    Tracks,
    Obstacles,
    DoorHooks,
}

impl Section {
    const ALL: [Section; 4] = [
        Section::Rooms,
        Section::Tracks,
        Section::Obstacles,
        Section::DoorHooks,
    ];

    fn keyword(self) -> &'static str {
        match self {
            Section::Rooms => "roomcount",
            Section::Tracks => "trackcount",
            Section::Obstacles => "obstaclecount",
            Section::DoorHooks => "doorhookcount",
        }
    }

    fn from_keyword(word: &str) -> Option<Section> {
        Self::ALL
            .into_iter()
            .find(|s| s.keyword().eq_ignore_ascii_case(word))
    }
}

const BEGIN: &str = "beginlayout";
const DONE: &str = "donelayout";
const DEPENDENCY: &str = "filedependancy";
const COUNT_INDENT: &str = "   ";
const ROW_INDENT: &str = "      ";

fn is_keyword(word: &str) -> bool {
    Section::from_keyword(word).is_some() || word.eq_ignore_ascii_case(DONE)
}

fn placement_line(p: &Placement) -> Result<String> {
    let what = || format!("`{}`", p.model);
    let [x, y, z] = p.position;
    Ok(format!(
        "{ROW_INDENT}{} {} {} {}",
        text::name(&p.model, || "a model name".to_owned())?,
        text::float(x, what)?,
        text::float(y, what)?,
        text::float(z, what)?
    ))
}

fn hook_line(h: &DoorHook) -> Result<String> {
    let what = || format!("door hook `{}` of `{}`", h.name, h.room);
    let [x, y, z] = h.position;
    let q = h.orientation;
    let mut s = format!(
        "{ROW_INDENT}{} {} {}",
        text::name(&h.room, what)?,
        text::name(&h.name, what)?,
        h.unknown
    );
    for v in [x, y, z, q.w, q.x, q.y, q.z] {
        s.push(' ');
        s.push_str(&text::float(v, what)?);
    }
    Ok(s)
}

impl Lyt {
    /// A layout with no rooms, written in the vanilla form with this
    /// dependency.
    pub fn new(file_dependency: Option<String>) -> Self {
        Lyt {
            file_dependency,
            ..Lyt::default()
        }
    }

    /// Parse a layout. Lines before `beginlayout` are `#` comments and the
    /// `filedependancy` line; then each section is a count line and that
    /// many rows, and `donelayout` ends the layout. Keywords match without
    /// regard to case and blank lines are skipped. Refuses a file without
    /// `beginlayout` or `donelayout`, a line in no form the layout takes, a
    /// number that does not read, a count its rows disagree with, and a
    /// byte outside ASCII, each with its code.
    pub fn parse(bytes: &[u8]) -> Result<Lyt> {
        let (lines, newline) = text::lines(bytes)?;
        let mut style = Style::new(newline);
        let begin = lines
            .iter()
            .position(|l| matches!(l.tokens()[..], [w] if w.eq_ignore_ascii_case(BEGIN)))
            .ok_or(Error::NotALyt)?;
        let mut lyt = Lyt {
            preamble: Vec::new(),
            ..Lyt::default()
        };
        let mut index = 0;
        for l in lines[..begin].iter().filter(|l| !l.is_blank()) {
            if l.text.trim_start().starts_with('#') {
                lyt.preamble.push(Preamble::Comment(l.text.to_owned()));
                style.record(index, l.text, l.raw);
            } else if let [w, dep] = l.tokens()[..]
                && w.eq_ignore_ascii_case(DEPENDENCY)
                && lyt.file_dependency.is_none()
            {
                lyt.file_dependency = Some(dep.to_owned());
                lyt.preamble.push(Preamble::Dependency);
                style.record(index, &format!("{DEPENDENCY} {dep}"), l.raw);
            } else {
                return Err(unexpected(
                    l,
                    "a `#` comment or `filedependancy NAME` before `beginlayout`",
                ));
            }
            index += 1;
        }
        style.record(index, BEGIN, lines[begin].raw);
        index += 1;

        let mut rest = lines[begin + 1..]
            .iter()
            .filter(|l| !l.is_blank())
            .peekable();
        let mut done = None;
        // The section lines as read, each with the line the writer would
        // produce for it and its row, if it is one. They are placed where
        // the writer puts them once every section's rows are known, since
        // the writer orders the sections and writes each once.
        let mut read: Vec<(Section, Option<usize>, String, &str)> = Vec::new();
        while let Some(l) = rest.next() {
            let tokens = l.tokens();
            if let [w] = tokens[..]
                && w.eq_ignore_ascii_case(DONE)
            {
                done = Some((l.number, l.raw));
                break;
            }
            let Some(section) = tokens.first().and_then(|w| Section::from_keyword(w)) else {
                return Err(unexpected(l, "a section's count line or `donelayout`"));
            };
            let [_, count] = tokens[..] else {
                return Err(unexpected(l, "`KEYWORD COUNT`"));
            };
            let declared = text::parse_count(count, l.number)?;
            read.push((
                section,
                None,
                format!("{COUNT_INDENT}{} {declared}", section.keyword()),
                l.raw,
            ));
            let mut found = 0;
            while let Some(row) =
                rest.next_if(|r| !r.tokens().first().is_some_and(|w| is_keyword(w)))
            {
                let canonical = match section {
                    Section::DoorHooks => {
                        let h = parse_hook(row)?;
                        let c = hook_line(&h)?;
                        lyt.door_hooks.push(h);
                        c
                    }
                    _ => {
                        let p = parse_placement(row)?;
                        let c = placement_line(&p)?;
                        match section {
                            Section::Rooms => lyt.rooms.push(p),
                            Section::Tracks => lyt.tracks.push(p),
                            _ => lyt.obstacles.push(p),
                        }
                        c
                    }
                };
                read.push((section, Some(lyt.rows(section) - 1), canonical, row.raw));
                found += 1;
            }
            if found != declared {
                return Err(Error::CountMismatch {
                    line: l.number,
                    section: section.keyword(),
                    declared,
                    found,
                });
            }
        }
        let (done, done_raw) = done.ok_or(Error::Unterminated)?;
        let mut start = [0; 4];
        for (k, section) in Section::ALL.into_iter().enumerate() {
            start[k] = index;
            index += 1 + lyt.rows(section);
        }
        let mut counted = [false; 4];
        for (section, row, canonical, raw) in &read {
            let k = *section as usize;
            match row {
                Some(i) => style.record(start[k] + 1 + i, canonical, raw),
                // A section given twice is written once, counting both.
                None if !counted[k] => {
                    counted[k] = true;
                    style.record(start[k], canonical, raw);
                }
                None => {}
            }
        }
        style.record(index, DONE, done_raw);
        // Blank lines fold into the line before them, so a line's number
        // is not its place in `lines`.
        let after = lines.iter().position(|l| l.number == done).expect("read") + 1;
        lyt.trailer = lines[after..].iter().map(|l| l.raw).collect();
        lyt.style = style;
        Ok(lyt)
    }

    fn rows(&self, section: Section) -> usize {
        match section {
            Section::Rooms => self.rooms.len(),
            Section::Tracks => self.tracks.len(),
            Section::Obstacles => self.obstacles.len(),
            Section::DoorHooks => self.door_hooks.len(),
        }
    }

    /// The file's bytes. A layout read writes back as the bytes it came
    /// from (when a section is missing, out of order or given twice, the
    /// sections are written in order, each once, their lines as read); a
    /// layout built or edited is written in the vanilla files' form:
    /// CRLF endings, count lines indented three spaces and rows six, the
    /// sections in the order rooms, tracks, obstacles, door hooks, every
    /// count written even when zero, and numbers spelled as
    /// [`format_float`](crate::format_float) spells them. Refuses a value
    /// that is not finite and a name that is empty or holds whitespace.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut w = Writer::new(&self.style);
        let dependency = match &self.file_dependency {
            Some(d) => Some(format!(
                "{DEPENDENCY} {}",
                text::name(d, || "the file dependency".to_owned())?
            )),
            None => None,
        };
        for p in &self.preamble {
            match p {
                Preamble::Comment(c) => w.line(c),
                Preamble::Dependency => {
                    if let Some(d) = &dependency {
                        w.line(d);
                    }
                }
            }
        }
        if let Some(d) = &dependency
            && !self.preamble.contains(&Preamble::Dependency)
        {
            w.line(d);
        }
        w.line(BEGIN);
        for section in Section::ALL {
            let rows: Vec<String> = match section {
                Section::Rooms => self
                    .rooms
                    .iter()
                    .map(placement_line)
                    .collect::<Result<_>>()?,
                Section::Tracks => self
                    .tracks
                    .iter()
                    .map(placement_line)
                    .collect::<Result<_>>()?,
                Section::Obstacles => self
                    .obstacles
                    .iter()
                    .map(placement_line)
                    .collect::<Result<_>>()?,
                Section::DoorHooks => self
                    .door_hooks
                    .iter()
                    .map(hook_line)
                    .collect::<Result<_>>()?,
            };
            w.line(&format!(
                "{COUNT_INDENT}{} {}",
                section.keyword(),
                rows.len()
            ));
            for r in &rows {
                w.line(r);
            }
        }
        w.line(DONE);
        let mut out = w.finish();
        out.push_str(&self.trailer);
        Ok(out.into_bytes())
    }

    /// Whether the two hold the same dependency, rooms, tracks, obstacles
    /// and door hooks, floats by their bits; how the file spelled them
    /// aside.
    pub fn semantic_eq(&self, other: &Lyt) -> bool {
        fn all<T>(a: &[T], b: &[T], eq: impl Fn(&T, &T) -> bool) -> bool {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| eq(x, y))
        }
        self.file_dependency == other.file_dependency
            && all(&self.rooms, &other.rooms, Placement::bits_eq)
            && all(&self.tracks, &other.tracks, Placement::bits_eq)
            && all(&self.obstacles, &other.obstacles, Placement::bits_eq)
            && all(&self.door_hooks, &other.door_hooks, DoorHook::bits_eq)
    }

    /// The room whose model is `model`, compared without regard to case.
    pub fn room(&self, model: &str) -> Option<&Placement> {
        self.rooms
            .iter()
            .find(|r| r.model.eq_ignore_ascii_case(model))
    }

    /// The bytes after `donelayout`, as read: empty in every vanilla file
    /// but one.
    pub fn trailer(&self) -> &str {
        &self.trailer
    }

    /// How many lines of the file read depart from the form the writer
    /// produces (another indentation, spacing, case or line ending): the
    /// writer keeps them as read.
    pub fn departures(&self) -> usize {
        self.style.departures()
    }
}

fn unexpected(l: &Line<'_>, expected: &'static str) -> Error {
    Error::UnexpectedLine {
        line: l.number,
        text: l.text.trim().to_owned(),
        expected,
    }
}

fn parse_placement(l: &Line<'_>) -> Result<Placement> {
    let [model, x, y, z] = l.tokens()[..] else {
        return Err(unexpected(l, "a row `MODEL X Y Z`"));
    };
    Ok(Placement {
        model: model.to_owned(),
        position: [
            text::parse_float(x, l.number)?,
            text::parse_float(y, l.number)?,
            text::parse_float(z, l.number)?,
        ],
    })
}

fn parse_hook(l: &Line<'_>) -> Result<DoorHook> {
    let [room, name, unknown, x, y, z, w, qx, qy, qz] = l.tokens()[..] else {
        return Err(unexpected(l, "a door hook `ROOM NAME 0 X Y Z W X Y Z`"));
    };
    let f = |t| text::parse_float(t, l.number);
    Ok(DoorHook {
        room: room.to_owned(),
        name: name.to_owned(),
        unknown: unknown.parse().map_err(|_| Error::BadNumber {
            line: l.number,
            text: unknown.to_owned(),
        })?,
        position: [f(x)?, f(y)?, f(z)?],
        orientation: Quaternion {
            w: f(w)?,
            x: f(qx)?,
            y: f(qy)?,
            z: f(qz)?,
        },
    })
}
