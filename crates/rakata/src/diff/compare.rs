//! Each pair compared by its format, and archives recursed to members.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use rakata_2da::TwoDa;
use rakata_archive::{Bif, Erf, Key, Rim};
use rakata_dialog::Dialog;
use rakata_gff::{Gff, LocString, Struct, Value};
use rakata_lip::Lip;
use rakata_res::ResId;
use rakata_tlk::{Tlk, codepage};

use super::{
    Cell, Change, Diff, Entry, FieldChange, Header, Kind, Options, Shown, Site, Walked, Warning,
    copies_elsewhere, extension, facts, matches,
};
use crate::conformance::Registry;
use crate::game::{Install, ModuleName, Profile};

/// One side's tree, opened as far as the comparison needs it.
struct Side {
    root: Option<PathBuf>,
    install: OnceCell<Option<Install>>,
    talk: OnceCell<Option<Tlk>>,
    modules: Option<HashMap<ResId, Vec<ModuleName>>>,
}

impl Side {
    fn new(root: Option<PathBuf>) -> Self {
        Side {
            root,
            install: OnceCell::new(),
            talk: OnceCell::new(),
            modules: None,
        }
    }

    fn talk(&self) -> Option<&Tlk> {
        self.talk
            .get_or_init(|| {
                let root = self.root.as_ref()?;
                let path = crate::inspect::child_ci(root, "dialog.tlk")?;
                Tlk::open(path).ok()
            })
            .as_ref()
    }
}

/// The state of one comparison.
pub(super) struct Cx {
    sides: [Side; 2],
    profile: Profile,
    registry: Registry,
    pub compared: usize,
    pub same_data: usize,
    entries: Vec<Entry>,
    warnings: Vec<Warning>,
}

/// What comparing one pair's contents found.
struct Found {
    format: Option<String>,
    compared: bool,
    sites: Vec<Site>,
}

impl Cx {
    pub fn new(a: Option<PathBuf>, b: Option<PathBuf>, profile: Profile) -> Self {
        Cx {
            sides: [Side::new(a), Side::new(b)],
            profile,
            registry: Registry::builtin(),
            compared: 0,
            same_data: 0,
            entries: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn finish(mut self, a: String, b: String, kind: &'static str, options: &Options) -> Diff {
        if !options.only.is_empty() {
            self.entries
                .retain(|e| options.only.iter().any(|p| matches(p, &e.path)));
        }
        self.entries.sort_by(|x, y| {
            x.path
                .to_ascii_lowercase()
                .cmp(&y.path.to_ascii_lowercase())
                .then_with(|| x.against.cmp(&y.against))
        });
        Diff {
            a,
            b,
            kind,
            compared: self.compared,
            same_data: self.same_data,
            entries: self.entries,
            warnings: self.warnings,
        }
    }

    fn warn(&mut self, code: &'static str, message: String) {
        self.warnings.push(Warning { code, message });
    }

    fn read(&mut self, path: &std::path::Path) -> Option<Vec<u8>> {
        match fs::read(path) {
            Ok(b) => Some(b),
            Err(e) => {
                self.warn(
                    "RW1602",
                    format!("cannot read `{}`: {e}; it is left out", path.display()),
                );
                None
            }
        }
    }

    /// Two files of the trees, paired.
    pub fn files(&mut self, a: &Walked, b: &Walked) {
        let renamed = a.rel != b.rel;
        let (change, from) = if renamed {
            (Change::Renamed, Some(a.rel.clone()))
        } else {
            (Change::Changed, None)
        };
        match (&a.what, &b.what) {
            (
                Kind::File {
                    path: pa, id: ia, ..
                },
                Kind::File {
                    path: pb, id: ib, ..
                },
            ) => {
                if ia == ib && ia != &(0, 0) {
                    if renamed {
                        self.rename_only(a, b);
                    }
                    return;
                }
                let ext = extension(&b.rel);
                if self.format_of(ext.as_deref()).format.is_none() {
                    // A format without a crate is told apart by hash, read
                    // in pieces: a movie is not held in memory.
                    match super::same_bytes(pa, pb) {
                        Ok(true) => {
                            if renamed {
                                self.rename_only(a, b);
                            }
                        }
                        Ok(false) => {
                            let fa = self.stream_facts(&a.rel, pa);
                            let fb = self.stream_facts(&b.rel, pb);
                            if fa.is_some() && fb.is_some() {
                                self.entries.push(Entry {
                                    path: b.rel.clone(),
                                    change,
                                    renamed_from: from,
                                    format: None,
                                    compared: false,
                                    a: fa,
                                    b: fb,
                                    against: Vec::new(),
                                    sites: Vec::new(),
                                });
                            }
                        }
                        Err(e) => self.warn(
                            "RW1602",
                            format!(
                                "cannot read `{}` or `{}`: {e}; it is left out",
                                pa.display(),
                                pb.display()
                            ),
                        ),
                    }
                    return;
                }
                let Some(xa) = self.read(pa) else { return };
                let Some(xb) = self.read(pb) else { return };
                if xa == xb {
                    if renamed {
                        self.rename_only(a, b);
                    }
                    return;
                }
                self.pair(&b.rel, ext.as_deref(), &xa, &xb, change, from);
            }
            (Kind::Link { target: ta }, Kind::Link { target: tb }) => {
                if ta != tb || renamed {
                    let sites = if ta == tb {
                        Vec::new()
                    } else {
                        vec![link_site(Some(ta), Some(tb))]
                    };
                    self.entries.push(Entry {
                        path: b.rel.clone(),
                        change,
                        renamed_from: from,
                        format: None,
                        compared: false,
                        a: None,
                        b: None,
                        against: Vec::new(),
                        sites,
                    });
                }
            }
            (wa, wb) => {
                let (fa, ta) = self.side_facts(a, wa);
                let (fb, tb) = self.side_facts(b, wb);
                self.entries.push(Entry {
                    path: b.rel.clone(),
                    change,
                    renamed_from: from,
                    format: None,
                    compared: false,
                    a: fa,
                    b: fb,
                    against: Vec::new(),
                    sites: vec![link_site(ta.as_ref(), tb.as_ref())],
                });
            }
        }
    }

    fn side_facts(&mut self, w: &Walked, k: &Kind) -> (Option<super::Facts>, Option<PathBuf>) {
        match k {
            Kind::File { path, .. } => (self.read(path).map(|b| facts(&w.rel, &b)), None),
            Kind::Link { target } => (None, Some(target.clone())),
        }
    }

    fn stream_facts(&mut self, rel: &str, path: &std::path::Path) -> Option<super::Facts> {
        match super::file_facts(rel, path) {
            Ok(f) => Some(f),
            Err(e) => {
                self.warn(
                    "RW1602",
                    format!("cannot read `{}`: {e}; it is left out", path.display()),
                );
                None
            }
        }
    }

    fn rename_only(&mut self, a: &Walked, b: &Walked) {
        self.entries.push(Entry {
            path: b.rel.clone(),
            change: Change::Renamed,
            renamed_from: Some(a.rel.clone()),
            format: None,
            compared: false,
            a: None,
            b: None,
            against: Vec::new(),
            sites: Vec::new(),
        });
    }

    /// A file only one tree has. An Override file is compared with the
    /// copies the other tree's install loads in its place.
    pub fn one_side(&mut self, w: &Walked, change: Change) {
        let here = usize::from(change == Change::Added);
        let other = 1 - here;
        let file = w.rel.rsplit('/').next().unwrap_or(&w.rel);
        let in_override = w
            .rel
            .split('/')
            .next()
            .is_some_and(|d| d.eq_ignore_ascii_case("override"));
        let ext = extension(&w.rel);
        let id = ResId::parse(file).ok().filter(|_| in_override);
        let bytes = match &w.what {
            Kind::File { path, .. } => {
                if id.is_none() && self.format_of(ext.as_deref()).format.is_none() {
                    let Some(f) = self.stream_facts(&w.rel, path) else {
                        return;
                    };
                    self.entries.push(Entry {
                        path: w.rel.clone(),
                        change,
                        renamed_from: None,
                        format: None,
                        compared: false,
                        a: (change == Change::Removed).then(|| f.clone()),
                        b: (change == Change::Added).then_some(f),
                        against: Vec::new(),
                        sites: Vec::new(),
                    });
                    return;
                }
                match self.read(path) {
                    Some(b) => b,
                    None => return,
                }
            }
            Kind::Link { target } => {
                let t = Some(target);
                let site = if change == Change::Added {
                    link_site(None, t)
                } else {
                    link_site(t, None)
                };
                self.entries.push(Entry {
                    path: w.rel.clone(),
                    change,
                    renamed_from: None,
                    format: None,
                    compared: false,
                    a: None,
                    b: None,
                    against: Vec::new(),
                    sites: vec![site],
                });
                return;
            }
        };
        let copies = match id {
            Some(id) => self.copies(other, id),
            None => Vec::new(),
        };
        if copies.is_empty() {
            let f = facts(&w.rel, &bytes);
            self.entries.push(Entry {
                path: w.rel.clone(),
                change,
                renamed_from: None,
                format: self.format_of(ext.as_deref()).format,
                compared: false,
                a: (change == Change::Removed).then(|| f.clone()),
                b: (change == Change::Added).then_some(f),
                against: Vec::new(),
                sites: Vec::new(),
            });
            return;
        }
        for (location, theirs) in copies {
            let (xa, xb) = if change == Change::Added {
                (&theirs, &bytes)
            } else {
                (&bytes, &theirs)
            };
            let found = if xa == xb {
                Found {
                    format: self.format_of(ext.as_deref()).format,
                    compared: true,
                    sites: Vec::new(),
                }
            } else {
                self.contents(&w.rel, ext.as_deref(), xa, xb)
            };
            self.entries.push(Entry {
                path: w.rel.clone(),
                change,
                renamed_from: None,
                format: found.format,
                compared: found.compared,
                a: Some(facts(&w.rel, xa)),
                b: Some(facts(&w.rel, xb)),
                against: vec![location],
                sites: found.sites,
            });
        }
    }

    /// The copies of `id` the side's install loads outside Override.
    fn copies(&mut self, side: usize, id: ResId) -> Vec<(String, Vec<u8>)> {
        let profile = self.profile;
        let s = &mut self.sides[side];
        let Some(root) = s.root.clone() else {
            return Vec::new();
        };
        s.install.get_or_init(|| Install::open(&root, profile).ok());
        let Some(install) = s.install.get().and_then(Option::as_ref) else {
            return Vec::new();
        };
        copies_elsewhere(install, &mut s.modules, id)
    }

    /// Two byte strings that differ, under `path`: an entry when their
    /// data differs, members of an archive as entries of their own.
    pub fn pair(
        &mut self,
        path: &str,
        ext: Option<&str>,
        a: &[u8],
        b: &[u8],
        change: Change,
        renamed_from: Option<String>,
    ) {
        let before = self.entries.len();
        let found = self.contents(path, ext, a, b);
        if found.compared && found.sites.is_empty() && renamed_from.is_none() {
            // An archive whose members differ is listed by its members.
            if self.entries.len() == before {
                self.same_data += 1;
            }
            return;
        }
        self.entries.push(Entry {
            path: path.to_owned(),
            change,
            renamed_from,
            format: found.format,
            compared: found.compared,
            a: Some(facts(path, a)),
            b: Some(facts(path, b)),
            against: Vec::new(),
            sites: found.sites,
        });
    }

    /// The name of the format reading `ext`, without reading.
    fn format_of(&self, ext: Option<&str>) -> Found {
        let format = match ext {
            Some("dlg") => Some("dlg".to_owned()),
            Some(e) => self.registry.by_extension(e).map(|d| d.name().to_owned()),
            None => None,
        };
        Found {
            format,
            compared: false,
            sites: Vec::new(),
        }
    }

    /// The sites at which two byte strings differ, by format.
    fn contents(&mut self, path: &str, ext: Option<&str>, a: &[u8], b: &[u8]) -> Found {
        let format = self.format_of(ext).format;
        let result = match format.as_deref() {
            Some("dlg") => match (Dialog::parse(path, a), Dialog::parse(path, b)) {
                (Ok(da), Ok(db)) => {
                    let ta = self.sides[0].talk().cloned();
                    let tb = self.sides[1].talk().cloned();
                    Ok(super::dialog::diff(&da, &db, ta.as_ref(), tb.as_ref()))
                }
                (Err(e), _) | (_, Err(e)) => Err(e.to_string()),
            },
            Some("gff") => both(a, b, Gff::parse).map(|(x, y)| gff(&x, &y)),
            Some("2da") => both(a, b, TwoDa::parse).map(|(x, y)| twoda(&x, &y)),
            Some("tlk") => both(a, b, Tlk::parse).map(|(x, y)| tlk(&x, &y)),
            Some("lip") => both(a, b, Lip::parse).map(|(x, y)| lip(&x, &y)),
            Some("archive") => self.archive(path, a, b),
            _ => {
                return Found {
                    format,
                    compared: false,
                    sites: Vec::new(),
                };
            }
        };
        match result {
            Ok(sites) => Found {
                format,
                compared: true,
                sites,
            },
            Err(message) => {
                self.warn(
                    "RW1601",
                    format!(
                        "`{path}` does not read as {}: {message}; compared by hash",
                        format.as_deref().unwrap_or("its format")
                    ),
                );
                Found {
                    format,
                    compared: false,
                    sites: Vec::new(),
                }
            }
        }
    }

    /// An archive's header sites; its members become entries of their
    /// own under `path!member`.
    fn archive(&mut self, path: &str, a: &[u8], b: &[u8]) -> Result<Vec<Site>, String> {
        let kind = |x: &[u8]| x.get(..4).map(<[u8]>::to_vec).unwrap_or_default();
        if kind(a) != kind(b) {
            return Ok(vec![Site::Header {
                field: "kind".into(),
                before: Some(String::from_utf8_lossy(&kind(a)).trim().to_owned()),
                after: Some(String::from_utf8_lossy(&kind(b)).trim().to_owned()),
            }]);
        }
        let mut sites = Vec::new();
        let (ma, mb): (Vec<Member>, Vec<Member>) = match &kind(a)[..] {
            b"KEY " => {
                let (ka, kb) = both(a, b, Key::parse)?;
                return Ok(key(&ka, &kb));
            }
            b"BIFF" => {
                let (ba, bb) = both(a, b, Bif::parse)?;
                let m = |x: &Bif<'_>| -> Vec<Member> {
                    x.members()
                        .iter()
                        .enumerate()
                        .map(|(i, m)| Member {
                            name: format!("#{i}.{}", m.ty),
                            ext: m.ty.extension().map(str::to_owned),
                            data: m.data().to_vec(),
                        })
                        .collect()
                };
                (m(&ba), m(&bb))
            }
            b"RIM " => {
                let (ra, rb) = both(a, b, Rim::parse)?;
                header(&mut sites, "flag", ra.flag, rb.flag);
                let m = |x: &Rim<'_>| -> Vec<Member> {
                    x.members()
                        .iter()
                        .map(|m| Member {
                            name: m.id.to_string(),
                            ext: m.id.ty.extension().map(str::to_owned),
                            data: m.data().to_vec(),
                        })
                        .collect()
                };
                (m(&ra), m(&rb))
            }
            _ => {
                let (ea, eb) = both(a, b, Erf::parse)?;
                header(
                    &mut sites,
                    "kind",
                    format!("{:?}", ea.kind),
                    format!("{:?}", eb.kind),
                );
                header(&mut sites, "build_year", ea.build_year, eb.build_year);
                header(&mut sites, "build_day", ea.build_day, eb.build_day);
                header(
                    &mut sites,
                    "description_strref",
                    ea.description_strref as i32,
                    eb.description_strref as i32,
                );
                let desc = |e: &Erf<'_>| {
                    e.descriptions
                        .iter()
                        .map(|d| format!("{}={:?}", d.language, codepage::decode(&d.text)))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                header(&mut sites, "descriptions", desc(&ea), desc(&eb));
                let m = |x: &Erf<'_>| -> Vec<Member> {
                    x.members()
                        .iter()
                        .map(|m| Member {
                            name: m.id.to_string(),
                            ext: m.id.ty.extension().map(str::to_owned),
                            data: m.data().to_vec(),
                        })
                        .collect()
                };
                (m(&ea), m(&eb))
            }
        };
        self.members(path, &ma, &mb);
        Ok(sites)
    }

    /// Members paired by name, in any case; the n-th of a repeated name
    /// with the n-th.
    fn members(&mut self, path: &str, a: &[Member], b: &[Member]) {
        let mut theirs: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, m) in b.iter().enumerate() {
            theirs
                .entry(m.name.to_ascii_lowercase())
                .or_default()
                .push(i);
        }
        let mut used = vec![false; b.len()];
        for ma in a {
            let partner = theirs
                .get(&ma.name.to_ascii_lowercase())
                .and_then(|v| v.iter().copied().find(|&j| !used[j]));
            let here = format!("{path}!{}", ma.name);
            match partner {
                None => self.member_alone(&here, ma, Change::Removed),
                Some(j) => {
                    used[j] = true;
                    let mb = &b[j];
                    self.compared += 1;
                    let renamed = ma.name != mb.name;
                    if ma.data == mb.data && !renamed {
                        continue;
                    }
                    let there = format!("{path}!{}", mb.name);
                    let (change, from) = if renamed {
                        (Change::Renamed, Some(here))
                    } else {
                        (Change::Changed, None)
                    };
                    if ma.data == mb.data {
                        self.entries.push(Entry {
                            path: there,
                            change,
                            renamed_from: from,
                            format: None,
                            compared: false,
                            a: None,
                            b: None,
                            against: Vec::new(),
                            sites: Vec::new(),
                        });
                    } else {
                        let ext = mb.ext.clone();
                        self.pair(&there, ext.as_deref(), &ma.data, &mb.data, change, from);
                    }
                }
            }
        }
        for (j, mb) in b.iter().enumerate() {
            if !used[j] {
                self.member_alone(&format!("{path}!{}", mb.name), mb, Change::Added);
            }
        }
    }

    fn member_alone(&mut self, path: &str, m: &Member, change: Change) {
        let f = facts(path, &m.data);
        let format = self.format_of(m.ext.as_deref()).format;
        self.entries.push(Entry {
            path: path.to_owned(),
            change,
            renamed_from: None,
            format,
            compared: false,
            a: (change == Change::Removed).then(|| f.clone()),
            b: (change == Change::Added).then_some(f),
            against: Vec::new(),
            sites: Vec::new(),
        });
    }
}

/// An archive member, its bytes copied out.
struct Member {
    name: String,
    ext: Option<String>,
    data: Vec<u8>,
}

fn link_site(a: Option<&PathBuf>, b: Option<&PathBuf>) -> Site {
    let show = |t: Option<&PathBuf>| t.map(|t| format!("symbolic link to {}", t.display()));
    Site::Header {
        field: "link".into(),
        before: show(a).or_else(|| a.is_none().then(|| "file".to_owned())),
        after: show(b).or_else(|| b.is_none().then(|| "file".to_owned())),
    }
}

fn both<'a, T, E: std::fmt::Display>(
    a: &'a [u8],
    b: &'a [u8],
    parse: impl Fn(&'a [u8]) -> Result<T, E>,
) -> Result<(T, T), String> {
    let x = parse(a).map_err(|e| format!("A: {e}"))?;
    let y = parse(b).map_err(|e| format!("B: {e}"))?;
    Ok((x, y))
}

fn header<T: PartialEq + ToString>(sites: &mut Vec<Site>, field: &str, a: T, b: T) {
    if a != b {
        sites.push(Site::Header {
            field: field.to_owned(),
            before: Some(a.to_string()),
            after: Some(b.to_string()),
        });
    }
}

/// Two GFF trees by field path.
pub(super) fn gff(a: &Gff, b: &Gff) -> Vec<Site> {
    let mut sites = Vec::new();
    header(
        &mut sites,
        "file_type",
        a.file_type_str(),
        b.file_type_str(),
    );
    header(
        &mut sites,
        "version",
        String::from_utf8_lossy(&a.version).into_owned(),
        String::from_utf8_lossy(&b.version).into_owned(),
    );
    sites.extend(fields(a, b).into_iter().map(Site::Field));
    sites
}

/// The field changes between two GFF trees. A field repeating the label
/// and the value of an earlier field of its struct is not compared: the
/// copy adds no data (BioWare's dialogues write `SoundExists` six times
/// over in a node, and other writers write it once).
pub(super) fn fields(a: &Gff, b: &Gff) -> Vec<FieldChange> {
    let (mut a, mut b) = (a.clone(), b.clone());
    without_repeats(&mut a.root);
    without_repeats(&mut b.root);
    rakata_gff::diff(&a, &b)
        .into_iter()
        .map(|c| FieldChange {
            path: if c.path.is_root() {
                "(root)".to_owned()
            } else {
                c.path.to_string()
            },
            before: c.before.as_ref().map(shown),
            after: c.after.as_ref().map(shown),
        })
        .collect()
}

/// The struct without the fields that repeat an earlier field's label and
/// value, at any depth.
fn without_repeats(s: &mut Struct) {
    let mut keep = vec![true; s.fields.len()];
    for i in 0..s.fields.len() {
        let f = &s.fields[i];
        keep[i] = !s.fields[..i]
            .iter()
            .zip(&keep)
            .any(|(g, k)| *k && g.label == f.label && g.value == f.value);
    }
    let mut it = keep.iter();
    s.fields.retain(|_| *it.next().unwrap_or(&true));
    for f in &mut s.fields {
        match &mut f.value {
            Value::Struct(x) => without_repeats(x),
            Value::List(l) => l.iter_mut().for_each(without_repeats),
            _ => {}
        }
    }
}

/// A value with its type, as the diff shows it.
pub(super) fn shown(v: &Value) -> Shown {
    Shown {
        ty: v.field_type().to_string(),
        value: value_text(v),
    }
}

/// A value as text: strings quoted, a struct as its fields in braces.
pub(super) fn value_text(v: &Value) -> String {
    let floats = |f: &[f32]| f.iter().map(f32::to_string).collect::<Vec<_>>().join(",");
    match v {
        Value::Byte(n) => n.to_string(),
        Value::Char(n) => n.to_string(),
        Value::Word(n) => n.to_string(),
        Value::Short(n) => n.to_string(),
        Value::Dword(n) => n.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Dword64(n) => n.to_string(),
        Value::Int64(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::Double(n) => n.to_string(),
        Value::String(b) => format!("{:?}", codepage::decode(b)),
        Value::ResRef(r) => codepage::decode(r.as_bytes()),
        Value::LocString(l) => locstring(l),
        Value::Void(b) => b.iter().map(|x| format!("{x:02x}")).collect(),
        Value::Struct(s) => struct_text(s),
        Value::List(l) => format!("[{} structs]", l.len()),
        Value::Orientation(f) => floats(f),
        Value::Vector(f) => floats(f),
        Value::StrRef(n) if *n == LocString::NO_STRREF => "-1".to_owned(),
        Value::StrRef(n) => n.to_string(),
    }
}

fn locstring(l: &LocString) -> String {
    let mut out = if l.has_strref() {
        format!("strref {}", l.strref)
    } else {
        "strref -1".to_owned()
    };
    for s in &l.substrings {
        out.push_str(&format!(" {}={:?}", s.id, codepage::decode(&s.text)));
    }
    out
}

fn struct_text(s: &Struct) -> String {
    let inner = s
        .fields
        .iter()
        .map(|f| format!("{}={}", f.label, value_text(&f.value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{inner}}}")
}

/// Two tables by row label and column.
fn twoda(a: &TwoDa, b: &TwoDa) -> Vec<Site> {
    use rakata_2da::Change as C;
    let cells = |t: &TwoDa, label: &str, nth: usize| -> Vec<Cell> {
        let Some(row) = t.rows.iter().filter(|r| r.label == label).nth(nth) else {
            return Vec::new();
        };
        t.columns
            .iter()
            .zip(&row.cells)
            .filter(|(_, v)| !v.is_empty())
            .map(|(c, v)| Cell {
                column: c.clone(),
                value: v.clone(),
            })
            .collect()
    };
    // The n-th row added under a label is the n-th of B's rows with that
    // label not paired with A's: rows pair by occurrence.
    let mut added: HashMap<String, usize> = HashMap::new();
    let mut removed: HashMap<String, usize> = HashMap::new();
    let count = |t: &TwoDa, l: &str| t.rows.iter().filter(|r| r.label == l).count();
    a.diff(b)
        .into_iter()
        .map(|c| match c {
            C::ColumnAdded { name } => Site::Column {
                change: Change::Added,
                column: name,
            },
            C::ColumnRemoved { name } => Site::Column {
                change: Change::Removed,
                column: name,
            },
            C::RowAdded { label } => {
                let n = added.entry(label.clone()).or_insert(0);
                let nth = count(a, &label) + *n;
                *n += 1;
                Site::Row {
                    change: Change::Added,
                    cells: cells(b, &label, nth),
                    row: label,
                }
            }
            C::RowRemoved { label } => {
                let n = removed.entry(label.clone()).or_insert(0);
                let nth = count(b, &label) + *n;
                *n += 1;
                Site::Row {
                    change: Change::Removed,
                    cells: cells(a, &label, nth),
                    row: label,
                }
            }
            C::Cell {
                label,
                column,
                before,
                after,
            } => Site::Cell {
                row: label,
                column,
                before,
                after,
            },
        })
        .collect()
}

/// Two talk tables by strref.
fn tlk(a: &Tlk, b: &Tlk) -> Vec<Site> {
    let mut sites = Vec::new();
    header(
        &mut sites,
        "language",
        format!("{:?}", a.language),
        format!("{:?}", b.language),
    );
    let fields = |e: &rakata_tlk::Entry| -> [(&'static str, String); 6] {
        [
            ("text", e.text.clone()),
            ("sound", e.sound.map(|s| s.to_string()).unwrap_or_default()),
            ("sound_length", e.sound_length.to_string()),
            ("flags", format!("{:#x}", e.flags)),
            ("volume_variance", e.volume_variance.to_string()),
            ("pitch_variance", e.pitch_variance.to_string()),
        ]
    };
    let n = a.entries.len().max(b.entries.len());
    for i in 0..n {
        let strref = i as u32;
        match (a.entries.get(i), b.entries.get(i)) {
            (Some(x), Some(y)) => {
                if x == y && x.sound_length.to_bits() == y.sound_length.to_bits() {
                    continue;
                }
                for ((field, before), (_, after)) in fields(x).into_iter().zip(fields(y)) {
                    if before != after
                        || (field == "sound_length"
                            && x.sound_length.to_bits() != y.sound_length.to_bits())
                    {
                        sites.push(Site::Strref {
                            change: Change::Changed,
                            strref,
                            field: field.to_owned(),
                            before: Some(before),
                            after: Some(after),
                        });
                    }
                }
            }
            (None, Some(y)) => sites.push(Site::Strref {
                change: Change::Added,
                strref,
                field: "text".to_owned(),
                before: None,
                after: Some(y.text.clone()),
            }),
            (Some(x), None) => sites.push(Site::Strref {
                change: Change::Removed,
                strref,
                field: "text".to_owned(),
                before: Some(x.text.clone()),
                after: None,
            }),
            (None, None) => {}
        }
    }
    sites
}

/// Two lip-sync tracks: the length and each keyframe by position.
fn lip(a: &Lip, b: &Lip) -> Vec<Site> {
    let mut sites = Vec::new();
    if a.length.to_bits() != b.length.to_bits() {
        sites.push(Site::Header {
            field: "length".into(),
            before: Some(a.length.to_string()),
            after: Some(b.length.to_string()),
        });
    }
    header(&mut sites, "keyframes", a.keys.len(), b.keys.len());
    let key = |k: &rakata_lip::Key| format!("{}s {}", k.time, k.shape.label());
    for i in 0..a.keys.len().max(b.keys.len()) {
        let (x, y) = (a.keys.get(i), b.keys.get(i));
        let same = matches!((x, y), (Some(x), Some(y)) if x.bits_eq(y));
        if !same {
            sites.push(Site::Header {
                field: format!("keyframe[{i}]"),
                before: x.map(key),
                after: y.map(key),
            });
        }
    }
    sites
}

/// Two KEYs by resource: where each names its bytes.
fn key(a: &Key, b: &Key) -> Vec<Site> {
    let mut sites = Vec::new();
    header(&mut sites, "build_year", a.build_year, b.build_year);
    header(&mut sites, "build_day", a.build_day, b.build_day);
    let bifs = |k: &Key| -> Vec<String> { k.bifs().iter().map(|b| b.name_str()).collect() };
    let (ba, bb) = (bifs(a), bifs(b));
    for name in &ba {
        if !bb.contains(name) {
            sites.push(Site::Header {
                field: "bif".into(),
                before: Some(name.clone()),
                after: None,
            });
        }
    }
    for name in &bb {
        if !ba.contains(name) {
            sites.push(Site::Header {
                field: "bif".into(),
                before: None,
                after: Some(name.clone()),
            });
        }
    }
    let at = |k: &Key, names: &[String]| -> HashMap<ResId, String> {
        k.entries()
            .iter()
            .map(|e| {
                let bif = names
                    .get(e.bif as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("BIF #{}", e.bif));
                (e.id, format!("{bif}!#{}", e.index))
            })
            .collect()
    };
    let (ma, mb) = (at(a, &ba), at(b, &bb));
    let mut ids: Vec<&ResId> = ma.keys().chain(mb.keys()).collect();
    ids.sort();
    ids.dedup();
    for id in ids {
        let (x, y) = (ma.get(id), mb.get(id));
        if x == y {
            continue;
        }
        let change = match (x, y) {
            (None, Some(_)) => Change::Added,
            (Some(_), None) => Change::Removed,
            _ => Change::Changed,
        };
        sites.push(Site::Resource {
            change,
            resource: id.to_string(),
            before: x.cloned(),
            after: y.cloned(),
        });
    }
    sites
}

/// The header fields known for a format without a crate.
pub(super) fn header_fields(ext: Option<&str>, b: &[u8], len: u64) -> Vec<Header> {
    let h = |field: &str, value: String| Header {
        field: field.to_owned(),
        value,
    };
    let u16_at = |at: usize| b.get(at..at + 2).map(|x| u16::from_le_bytes([x[0], x[1]]));
    let u32_at = |at: usize| {
        b.get(at..at + 4)
            .map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]))
    };
    match ext {
        // The TPC header: data size (0 when uncompressed), alpha test,
        // width, height, encoding, mipmap count, per xoreos's document.
        Some("tpc") if b.len() >= 128 => {
            let mut out = Vec::new();
            if let (Some(size), Some(w), Some(hh)) = (u32_at(0), u16_at(8), u16_at(10)) {
                out.push(h("width", w.to_string()));
                out.push(h("height", hh.to_string()));
                out.push(h("encoding", b[12].to_string()));
                out.push(h("mipmaps", b[13].to_string()));
                out.push(h("data_size", size.to_string()));
            }
            out
        }
        Some("tga") if b.len() >= 18 => vec![
            h("width", u16_at(12).unwrap_or(0).to_string()),
            h("height", u16_at(14).unwrap_or(0).to_string()),
            h("bits", b[16].to_string()),
        ],
        Some("wav") => rakata_dialog::audio::duration(b, len)
            .map(|a| {
                vec![
                    h("seconds", format!("{:.3}", a.seconds)),
                    h("encoding", a.encoding.to_owned()),
                ]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
