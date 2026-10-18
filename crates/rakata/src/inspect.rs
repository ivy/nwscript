//! One resource named on a command line, read, and one field of a GFF
//! assigned from text: what `rakata cat`, `gff`, `2da`, `tlk` and `lip`
//! share.
//!
//! A name takes one of three forms ([`load`]):
//!
//! - `ARCHIVE!MEMBER`, the round-trip report's syntax: the member of an
//!   ERF-family file or a RIM by `name.ext`, or of a BIF by position as
//!   `#N` or `#N.ext`. Members nest (`a.mod!b.erf!c.utc`). The archive is a
//!   path; when no file is there and an install is named, it is a path
//!   within the install, matched without regard to case.
//! - a path: any text holding a `/`, and any text at all when no install is
//!   named, is read from disk.
//! - `NAME.EXT`: resolved through the install as `rakata which` resolves
//!   it, the module's capsules searched when one is named, and the copy the
//!   engine loads read.

use std::fs;
use std::path::{Path, PathBuf};

use rakata_archive::{Bif, Erf, Rim};
use rakata_gff::{FieldType, Gff, LocString, Segment, Value};
use rakata_res::ResId;

use crate::codes;
use crate::diag::Diagnostic;
use crate::game::{Install, Location, ModuleName, Profile};

/// Where [`load`] looks a `NAME.EXT` up.
#[derive(Clone, Copy, Debug)]
pub struct Lookup<'a> {
    /// The install; without one every name is a path.
    pub game: Option<&'a Path>,
    /// The module whose capsules to search.
    pub module: Option<&'a str>,
    /// The game the install belongs to.
    pub profile: Profile,
}

/// A resource read: where it was found, as a name [`load`] takes back, and
/// its bytes.
#[derive(Clone, Debug)]
pub struct Loaded {
    /// The path, the member path (`modules/x.mod!y.utc`, `data/2da.bif!#74.2da`)
    /// relative to the install when it lies in one.
    pub source: String,
    /// The resource's own name, whose extension says its type.
    pub name: String,
    pub bytes: Vec<u8>,
}

impl Loaded {
    /// The extension of the resource's name, lowercase.
    pub fn extension(&self) -> Option<String> {
        self.name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .filter(|e| !e.is_empty())
    }
}

/// The registered code for a code another crate reports.
fn code(id: &str) -> &'static codes::Code {
    codes::lookup(id).unwrap_or_else(|| panic!("diagnostic code {id} is not registered"))
}

/// A diagnostic from another crate's error, which renders as `CODE: message`.
pub fn diagnostic(id: &str, error: &impl std::fmt::Display) -> Diagnostic {
    let text = error.to_string();
    let message = text
        .strip_prefix(id)
        .and_then(|t| t.strip_prefix(": "))
        .unwrap_or(&text)
        .to_owned();
    Diagnostic::new(code(id), message)
}

fn game_error(e: &crate::game::Error) -> Diagnostic {
    Diagnostic::new(code(e.code()), e.message())
}

fn unreadable(path: &Path, e: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        &codes::RE0001,
        format!("cannot read `{}`: {e}", path.display()),
    )
}

/// The resource `name` names, read; see the module documentation for the
/// forms a name takes.
pub fn load(name: &str, lookup: &Lookup<'_>) -> Result<Loaded, Diagnostic> {
    if name.contains('!') {
        let mut parts = name.split('!');
        let archive = parts.next().unwrap_or_default();
        let path = archive_path(archive, lookup.game);
        let mut bytes = fs::read(&path).map_err(|e| unreadable(&path, &e))?;
        let mut source = shown(&path, lookup.game);
        let mut last = archive.to_owned();
        for member in parts {
            bytes = member_of(&bytes, member, &source)?;
            source = format!("{source}!{member}");
            last = member.to_owned();
        }
        return Ok(Loaded {
            source,
            name: last,
            bytes,
        });
    }
    let game = match lookup.game {
        Some(g) if !name.contains('/') && !name.contains('\\') => g,
        _ => {
            let path = Path::new(name);
            let bytes = fs::read(path).map_err(|e| {
                let d = unreadable(path, &e);
                if lookup.game.is_none() && !name.contains('/') {
                    d.help("to read the copy an install loads, name the install with --game DIR")
                } else {
                    d
                }
            })?;
            let file = path
                .file_name()
                .map_or_else(|| name.to_owned(), |f| f.to_string_lossy().into_owned());
            return Ok(Loaded {
                source: name.to_owned(),
                name: file,
                bytes,
            });
        }
    };
    let id = ResId::parse(name).map_err(|e| diagnostic(e.code(), &e))?;
    let install = Install::open(game, lookup.profile).map_err(|e| game_error(&e))?;
    let module = lookup.module.map(ModuleName::new);
    match install.read(id, module.as_ref()) {
        Ok(Some((location, bytes))) => Ok(Loaded {
            source: location_name(&location, game),
            name: id.to_string(),
            bytes,
        }),
        Ok(None) => Err(game_error(&crate::game::Error::NotFound {
            id,
            module: lookup.module.map(str::to_owned),
        })),
        Err(e) => Err(game_error(&e)),
    }
}

/// An install's location as a name [`load`] takes back.
fn location_name(l: &Location, game: &Path) -> String {
    let path = shown(l.path(), Some(game));
    match l {
        Location::File { .. } => path,
        Location::Capsule { member, .. } => format!("{path}!{member}"),
        Location::Bif {
            index, path: bif, ..
        } => {
            let ty = fs::read(bif)
                .ok()
                .and_then(|b| {
                    Bif::parse(&b)
                        .ok()
                        .and_then(|bif| bif.members().get(*index as usize).map(|m| m.ty))
                })
                .map(|t| format!(".{t}"))
                .unwrap_or_default();
            format!("{path}!#{index}{ty}")
        }
    }
}

/// A path relative to the install when it lies in one.
fn shown(path: &Path, game: Option<&Path>) -> String {
    game.and_then(|g| path.strip_prefix(g).ok())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// The archive's path: as given when a file is there, else within the
/// install, each component matched without regard to case.
fn archive_path(text: &str, game: Option<&Path>) -> PathBuf {
    let given = PathBuf::from(text);
    if given.is_file() {
        return given;
    }
    let Some(game) = game else { return given };
    let mut p = game.to_path_buf();
    for part in text.split(['/', '\\']).filter(|s| !s.is_empty()) {
        p = child_ci(&p, part).unwrap_or_else(|| p.join(part));
    }
    p
}

/// The entry of `dir` named `name` in any case; the exact spelling first.
pub fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// The member of an archive held in `bytes`, whose kind its first four
/// bytes name.
fn member_of(bytes: &[u8], member: &str, archive: &str) -> Result<Vec<u8>, Diagnostic> {
    let missing = || {
        Diagnostic::new(
            &codes::RE1301,
            format!("`{archive}` holds no member `{member}`"),
        )
    };
    let archive_error = |e: rakata_archive::Error| diagnostic(e.code(), &e);
    match bytes.get(..4).unwrap_or(bytes) {
        b"BIFF" => {
            let bif = Bif::parse(bytes).map_err(archive_error)?;
            let spec = member.strip_prefix('#').ok_or_else(|| {
                missing()
                    .help("a BIF holds no names: name a member by position, as `#N` or `#N.ext`")
            })?;
            let (index, ext) = match spec.split_once('.') {
                Some((i, e)) => (i, Some(e)),
                None => (spec, None),
            };
            let m = index
                .parse::<usize>()
                .ok()
                .and_then(|i| bif.members().get(i))
                .ok_or_else(missing)?;
            if ext.is_some_and(|e| !e.eq_ignore_ascii_case(&m.ty.to_string())) {
                return Err(missing().note(format!("member {index} is a .{}", m.ty)));
            }
            Ok(m.data().to_vec())
        }
        b"KEY " => Err(missing().help("a KEY indexes the BIFs and holds no bytes: name the BIF")),
        kind => {
            let id = ResId::parse(member).map_err(|e| diagnostic(e.code(), &e))?;
            let data = if kind == b"RIM " {
                Rim::parse(bytes)
                    .map_err(archive_error)?
                    .get(id)
                    .map(<[u8]>::to_vec)
            } else {
                Erf::parse(bytes)
                    .map_err(archive_error)?
                    .get(id)
                    .map(<[u8]>::to_vec)
            };
            data.ok_or_else(missing)
        }
    }
}

/// The install's `dialog.tlk`, in any case.
pub fn dialog_tlk(game: &Path) -> Result<PathBuf, Diagnostic> {
    child_ci(game, "dialog.tlk").ok_or_else(|| {
        Diagnostic::new(
            &codes::RE1305,
            format!("`{}` holds no dialog.tlk", game.display()),
        )
        .help("--game names the folder holding chitin.key and dialog.tlk")
    })
}

/// What [`assign`] did: the field's path, its value before (`None` for a
/// field it created) and after.
#[derive(Clone, Debug)]
pub struct Assigned {
    pub path: rakata_gff::Path,
    pub before: Option<Value>,
    pub after: Value,
}

/// The type names `PATH=TYPE:VALUE` takes, in any case, with the type each
/// names.
pub const TYPE_NAMES: &[(&str, FieldType)] = &[
    ("byte", FieldType::Byte),
    ("char", FieldType::Char),
    ("word", FieldType::Word),
    ("short", FieldType::Short),
    ("dword", FieldType::Dword),
    ("int", FieldType::Int),
    ("dword64", FieldType::Dword64),
    ("int64", FieldType::Int64),
    ("float", FieldType::Float),
    ("double", FieldType::Double),
    ("string", FieldType::String),
    ("cexostring", FieldType::String),
    ("resref", FieldType::ResRef),
    ("cresref", FieldType::ResRef),
    ("locstring", FieldType::LocString),
    ("cexolocstring", FieldType::LocString),
    ("void", FieldType::Void),
    ("strref", FieldType::StrRef),
    ("vector", FieldType::Vector),
    ("orientation", FieldType::Orientation),
];

/// Assign one field from `PATH=VALUE` or `PATH=TYPE:VALUE`. Without a type
/// the value is read as the existing field's type; with one the field may
/// change type, or be created in an existing struct. Text is encoded in
/// the game's code page.
pub fn assign(gff: &mut Gff, text: &str) -> Result<Assigned, Diagnostic> {
    let bad = |msg: String| Diagnostic::new(&codes::RE1302, msg);
    let (lhs, rhs) = text
        .split_once('=')
        .ok_or_else(|| bad(format!("`{text}` is not PATH=VALUE or PATH=TYPE:VALUE")))?;
    let path = rakata_gff::Path::parse(lhs).map_err(|e| diagnostic(e.code(), &e))?;
    let (ty, value) = match rhs.split_once(':') {
        Some((t, v)) => match TYPE_NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)) {
            Some((_, ty)) => (Some(*ty), v),
            None => (None, rhs),
        },
        None => (None, rhs),
    };
    match gff.get(&path) {
        Ok(old) => {
            let ty = ty.unwrap_or_else(|| old.field_type());
            let new = parse_value(ty, value, Some(old)).map_err(bad)?;
            let before = gff
                .set(&path, new.clone())
                .map_err(|e| diagnostic(e.code(), &e))?;
            Ok(Assigned {
                path,
                before: Some(before),
                after: new,
            })
        }
        Err(e) => {
            // A field is created only where its struct exists and its type
            // is named.
            let (Some(parent), Some(Segment::Field(label))) =
                (path.parent(), path.segments().last())
            else {
                return Err(diagnostic(e.code(), &e));
            };
            let Ok(st) = gff.struct_at_mut(&parent) else {
                return Err(diagnostic(e.code(), &e));
            };
            if st.field(&label.to_string()).is_some() {
                return Err(diagnostic(e.code(), &e));
            }
            let Some(ty) = ty else {
                return Err(Diagnostic::new(
                    &codes::RE1303,
                    format!("`{path}` names no field, and without a type none is created"),
                )
                .help(format!("to create it, name its type: `{lhs}=TYPE:{value}`")));
            };
            let new = parse_value(ty, value, None).map_err(bad)?;
            st.push(*label, new.clone());
            Ok(Assigned {
                path,
                before: None,
                after: new,
            })
        }
    }
}

/// The value of type `ty` that `text` writes; `old` is the field's value,
/// whose substrings a strref assignment to a `CExoLocString` keeps.
fn parse_value(ty: FieldType, text: &str, old: Option<&Value>) -> Result<Value, String> {
    fn num<T: std::str::FromStr>(text: &str, ty: FieldType) -> Result<T, String> {
        text.trim()
            .parse()
            .map_err(|_| format!("`{text}` is not a {ty} value"))
    }
    fn floats<const N: usize>(text: &str, ty: FieldType) -> Result<[f32; N], String> {
        let parts: Vec<&str> = text.split(',').collect();
        if parts.len() != N {
            return Err(format!(
                "a {ty} is {N} comma-separated floats, not `{text}`"
            ));
        }
        let mut out = [0.0; N];
        for (o, p) in out.iter_mut().zip(parts) {
            *o = num(p, ty)?;
        }
        Ok(out)
    }
    let encode = |t: &str| {
        rakata_tlk::codepage::encode(t).map_err(|c| {
            format!(
                "`{c}` (U+{:04X}) is outside the game's code page",
                u32::from(c)
            )
        })
    };
    let strref = |t: &str| -> Result<u32, String> {
        if t.trim() == "-1" {
            Ok(LocString::NO_STRREF)
        } else {
            num(t, ty)
        }
    };
    Ok(match ty {
        FieldType::Byte => Value::Byte(num(text, ty)?),
        FieldType::Char => Value::Char(num(text, ty)?),
        FieldType::Word => Value::Word(num(text, ty)?),
        FieldType::Short => Value::Short(num(text, ty)?),
        FieldType::Dword => Value::Dword(num(text, ty)?),
        FieldType::Int => Value::Int(num(text, ty)?),
        FieldType::Dword64 => Value::Dword64(num(text, ty)?),
        FieldType::Int64 => Value::Int64(num(text, ty)?),
        FieldType::Float => Value::Float(num(text, ty)?),
        FieldType::Double => Value::Double(num(text, ty)?),
        FieldType::String => Value::String(encode(text)?),
        FieldType::ResRef => Value::resref(text).map_err(|e| {
            let m = e.to_string();
            m.strip_prefix(e.code())
                .and_then(|t| t.strip_prefix(": "))
                .unwrap_or(&m)
                .to_owned()
        })?,
        FieldType::StrRef => Value::StrRef(strref(text)?),
        FieldType::Vector => Value::Vector(floats::<3>(text, ty)?),
        FieldType::Orientation => Value::Orientation(floats::<4>(text, ty)?),
        FieldType::Void => Value::Void(hex(text)?),
        FieldType::LocString => {
            let mut ls = match old {
                Some(Value::LocString(l)) => l.clone(),
                _ => LocString::none(),
            };
            match text.split_once('=') {
                Some((id, t)) => {
                    let id: u32 = id
                        .trim()
                        .parse()
                        .map_err(|_| format!("`{id}` is not a substring id"))?;
                    ls.set(id, encode(t)?);
                }
                None => ls.strref = strref(text)?,
            }
            Value::LocString(ls)
        }
        FieldType::Struct | FieldType::List => {
            return Err(format!(
                "a {ty} is not set from the command line; assign its fields by path"
            ));
        }
    })
}

/// Bytes from hexadecimal digits.
fn hex(text: &str) -> Result<Vec<u8>, String> {
    let t = text.trim();
    if !t.len().is_multiple_of(2) || !t.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "`{text}` is not an even number of hexadecimal digits"
        ));
    }
    (0..t.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&t[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
