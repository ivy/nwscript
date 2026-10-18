//! `rakata.twoda`, `rakata.gff`, `rakata.lip` and `rakata.tlk`: each
//! format read from bytes, edited as an owned value, written back to bytes.

use mlua::{Lua, Table, UserData, UserDataFields, UserDataMethods, UserDataRef, Value};
use rakata::gff::{FieldType, Gff, LocString, Path as GffPath, Struct, Value as GValue};
use rakata::lip::{Lip, Shape};
use rakata::tlk::Tlk;
use rakata::twoda::{RowRef, TwoDa};

use crate::error::{Lift, bad};
use crate::values::bytes;

// ---- 2DA ----

/// A table.
pub struct LTwoDa(pub TwoDa);

/// A row named from Lua: an integer is an index from 0, a string a label.
pub enum LRow {
    Index(usize),
    Label(String),
}

impl LRow {
    pub fn from_lua(v: Value) -> mlua::Result<LRow> {
        match v {
            Value::Integer(i) if i >= 0 => Ok(LRow::Index(i as usize)),
            Value::Number(n) if n >= 0.0 && n.fract() == 0.0 => Ok(LRow::Index(n as usize)),
            Value::String(s) => Ok(LRow::Label(s.to_str()?.to_string())),
            other => Err(bad(format!(
                "a row is an index from 0 or a label, not {}",
                describe(&other)
            ))),
        }
    }

    pub fn as_ref(&self) -> RowRef<'_> {
        match self {
            LRow::Index(i) => RowRef::Index(*i),
            LRow::Label(l) => RowRef::Label(l),
        }
    }
}

fn describe(v: &Value) -> String {
    match v {
        Value::Integer(i) => format!("the integer {i}"),
        Value::Number(n) => format!("the number {n}"),
        other => format!("a {}", other.type_name()),
    }
}

/// A cell's text: strings as they are, numbers as Lua prints them.
fn cell_text(v: &Value) -> mlua::Result<String> {
    match v {
        Value::String(s) => Ok(s.to_str()?.to_string()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        other => Err(bad(format!(
            "a 2DA cell is a string or a number, not {}",
            describe(other)
        ))),
    }
}

/// `{column = value, ...}` as (column, value) pairs, by column name.
pub fn cells(t: Option<Table>) -> mlua::Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    if let Some(t) = t {
        for pair in t.pairs::<Value, Value>() {
            let (k, v) = pair?;
            let Value::String(k) = k else {
                return Err(bad("2DA cells are given as {column = value}"));
            };
            out.push((k.to_str()?.to_string(), cell_text(&v)?));
        }
    }
    out.sort();
    Ok(out)
}

fn pairs(c: &[(String, String)]) -> Vec<(&str, &str)> {
    c.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
}

impl UserData for LTwoDa {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_method("columns", |lua, this, ()| {
            lua.create_sequence_from(this.0.columns.iter().cloned())
        });
        m.add_method("len", |_, this, ()| Ok(this.0.rows.len()));
        m.add_meta_method("__len", |_, this, ()| Ok(this.0.rows.len()));
        m.add_method("label", |_, this, row: Value| {
            let r = LRow::from_lua(row)?;
            Ok(this.0.row(r.as_ref()).lift()?.label.clone())
        });
        m.add_method("row", |lua, this, row: Value| {
            let r = LRow::from_lua(row)?;
            let i = this.0.row_index(r.as_ref());
            let row = this.0.row(r.as_ref()).lift()?;
            let t = lua.create_table()?;
            t.set("index", i)?;
            t.set("label", row.label.as_str())?;
            let cells = lua.create_table()?;
            for (c, name) in this.0.columns.iter().enumerate() {
                cells.set(name.as_str(), row.cells.get(c).map_or("", String::as_str))?;
            }
            t.set("cells", cells)?;
            Ok(t)
        });
        m.add_method("get", |_, this, (row, column): (Value, String)| {
            let r = LRow::from_lua(row)?;
            Ok(this.0.get(r.as_ref(), &column).lift()?.to_string())
        });
        m.add_method_mut(
            "set",
            |_, this, (row, column, value): (Value, String, Value)| {
                let r = LRow::from_lua(row)?;
                this.0.set(r.as_ref(), &column, cell_text(&value)?).lift()
            },
        );
        m.add_method("row_index", |_, this, row: Value| {
            let r = LRow::from_lua(row)?;
            Ok(this.0.row_index(r.as_ref()))
        });
        m.add_method("column_index", |_, this, name: String| {
            Ok(this.0.column_index(&name))
        });
        m.add_method_mut("add_row", |_, this, (label, c): (String, Option<Table>)| {
            let c = cells(c)?;
            this.0.add_row(label, &pairs(&c)).lift()
        });
        m.add_method_mut(
            "add_row_copying",
            |_, this, (from, label, c): (Value, String, Option<Table>)| {
                let r = LRow::from_lua(from)?;
                let c = cells(c)?;
                this.0.add_row_copying(r.as_ref(), label, &pairs(&c)).lift()
            },
        );
        m.add_method_mut(
            "add_column",
            |_, this, (name, fill): (String, Option<String>)| {
                this.0
                    .add_column(name, fill.as_deref().unwrap_or(""))
                    .lift()
            },
        );
        m.add_method("semantic_eq", |_, this, other: UserDataRef<LTwoDa>| {
            Ok(this.0.semantic_eq(&other.0))
        });
        m.add_method("diff", |lua, this, other: UserDataRef<LTwoDa>| {
            lua.create_sequence_from(this.0.diff(&other.0).iter().map(ToString::to_string))
        });
        m.add_method("clone", |_, this, ()| Ok(LTwoDa(this.0.clone())));
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!(
                "TwoDa({} columns, {} rows)",
                this.0.columns.len(),
                this.0.rows.len()
            ))
        });
    }
}

// ---- GFF ----

/// A GFF file.
pub struct LGff(pub Gff);

/// A GFF value with its type, as `rakata.gff.byte(5)` makes one.
#[derive(Clone)]
pub struct LValue(pub GValue);

impl UserData for LValue {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("type", |_, this| Ok(this.0.field_type().name()));
        f.add_field_method_get("value", |lua, this| plain(lua, &this.0));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method("__tostring", |lua, this, ()| {
            let v = plain(lua, &this.0)?;
            let shown = match &v {
                Value::String(s) => format!("{:?}", s.to_string_lossy()),
                Value::Table(_) => "{...}".to_string(),
                other => other.to_string()?,
            };
            Ok(format!("{}({shown})", this.0.field_type().name()))
        });
        m.add_meta_method("__eq", |_, this, other: UserDataRef<LValue>| {
            Ok(this.0 == other.0)
        });
    }
}

/// A locstring as `{strref = N, strings = {[id] = text}}`, `strref` nil
/// when the string has none; `id` is the language times two plus one for
/// the feminine form.
fn locstring_table(lua: &Lua, l: &LocString) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    if l.has_strref() {
        t.set("strref", l.strref)?;
    }
    let strings = lua.create_table()?;
    for s in &l.substrings {
        strings.raw_set(s.id, lua.create_string(&s.text)?)?;
    }
    t.set("strings", strings)?;
    Ok(t)
}

fn struct_table(lua: &Lua, s: &Struct) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for f in &s.fields {
        t.raw_set(
            lua.create_string(f.label.as_bytes())?,
            plain(lua, &f.value)?,
        )?;
    }
    Ok(t)
}

/// A value as plain Lua: integers, numbers, strings (the bytes as the file
/// holds them), a locstring table, a struct as `{Label = value}`, a list as
/// a sequence of those, a vector or orientation as a sequence of numbers.
pub fn plain(lua: &Lua, v: &GValue) -> mlua::Result<Value> {
    Ok(match v {
        GValue::Byte(n) => Value::Integer(i64::from(*n)),
        GValue::Char(n) => Value::Integer(i64::from(*n)),
        GValue::Word(n) => Value::Integer(i64::from(*n)),
        GValue::Short(n) => Value::Integer(i64::from(*n)),
        GValue::Dword(n) => Value::Integer(i64::from(*n)),
        GValue::Int(n) => Value::Integer(i64::from(*n)),
        GValue::Dword64(n) => Value::Integer(*n as i64),
        GValue::Int64(n) => Value::Integer(*n),
        GValue::Float(n) => Value::Number(f64::from(*n)),
        GValue::Double(n) => Value::Number(*n),
        GValue::String(b) | GValue::Void(b) => Value::String(lua.create_string(b)?),
        GValue::ResRef(r) => Value::String(lua.create_string(r.as_bytes())?),
        GValue::LocString(l) => Value::Table(locstring_table(lua, l)?),
        GValue::Struct(s) => Value::Table(struct_table(lua, s)?),
        GValue::List(items) => {
            let t = lua.create_table()?;
            for (i, s) in items.iter().enumerate() {
                t.raw_set(i + 1, struct_table(lua, s)?)?;
            }
            Value::Table(t)
        }
        GValue::Orientation(q) => Value::Table(lua.create_sequence_from(q.iter().copied())?),
        GValue::Vector(q) => Value::Table(lua.create_sequence_from(q.iter().copied())?),
        GValue::StrRef(n) => Value::Integer(i64::from(*n)),
    })
}

fn integer(v: &Value, ty: FieldType) -> mlua::Result<i128> {
    match v {
        Value::Integer(i) => Ok(i128::from(*i)),
        Value::Number(n) if n.fract() == 0.0 => Ok(*n as i128),
        other => Err(bad(format!(
            "a {} field takes an integer, not {}",
            ty.name(),
            describe(other)
        ))),
    }
}

fn number(v: &Value, ty: FieldType) -> mlua::Result<f64> {
    match v {
        Value::Integer(i) => Ok(*i as f64),
        Value::Number(n) => Ok(*n),
        other => Err(bad(format!(
            "a {} field takes a number, not {}",
            ty.name(),
            describe(other)
        ))),
    }
}

fn byte_string(v: &Value, ty: FieldType) -> mlua::Result<Vec<u8>> {
    match v {
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        other => Err(bad(format!(
            "a {} field takes a string, not {}",
            ty.name(),
            describe(other)
        ))),
    }
}

fn floats<const N: usize>(v: &Value, ty: FieldType) -> mlua::Result<[f32; N]> {
    let Value::Table(t) = v else {
        return Err(bad(format!(
            "a {} field takes a sequence of {N} numbers, not {}",
            ty.name(),
            describe(v)
        )));
    };
    let mut out = [0f32; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = number(&t.raw_get::<Value>(i + 1)?, ty)? as f32;
    }
    Ok(out)
}

fn locstring(v: &Value) -> mlua::Result<LocString> {
    match v {
        Value::Integer(_) | Value::Number(_) => {
            let n = integer(v, FieldType::LocString)?;
            u32::try_from(n)
                .map(LocString::strref)
                .map_err(|_| bad(format!("{n} is not a strref")))
        }
        Value::String(s) => {
            let mut l = LocString::none();
            l.set(0, s.as_bytes().to_vec());
            Ok(l)
        }
        Value::Table(t) => {
            let mut l = match t.get::<Option<i64>>("strref")? {
                Some(n) => LocString::strref(
                    u32::try_from(n).map_err(|_| bad(format!("{n} is not a strref")))?,
                ),
                None => LocString::none(),
            };
            if let Some(strings) = t.get::<Option<Table>>("strings")? {
                let mut subs: Vec<(u32, Vec<u8>)> = Vec::new();
                for pair in strings.pairs::<i64, mlua::LuaString>() {
                    let (id, text) = pair?;
                    let id = u32::try_from(id)
                        .map_err(|_| bad(format!("{id} is not a substring id")))?;
                    subs.push((id, text.as_bytes().to_vec()));
                }
                subs.sort();
                for (id, text) in subs {
                    l.set(id, text);
                }
            }
            Ok(l)
        }
        other => Err(bad(format!(
            "a CExoLocString field takes a strref, a string or {{strref = N, strings = {{[id] = text}}}}, not {}",
            describe(other)
        ))),
    }
}

macro_rules! narrow {
    ($v:expr, $ty:expr, $t:ty, $variant:ident) => {{
        let n = integer($v, $ty)?;
        <$t>::try_from(n)
            .map(GValue::$variant)
            .map_err(|_| bad(format!("{n} does not fit a {} field", $ty.name())))
    }};
}

/// A plain Lua value as a value of type `ty`.
pub fn typed(v: &Value, ty: FieldType) -> mlua::Result<GValue> {
    match ty {
        FieldType::Byte => narrow!(v, ty, u8, Byte),
        FieldType::Char => narrow!(v, ty, i8, Char),
        FieldType::Word => narrow!(v, ty, u16, Word),
        FieldType::Short => narrow!(v, ty, i16, Short),
        FieldType::Dword => narrow!(v, ty, u32, Dword),
        FieldType::Int => narrow!(v, ty, i32, Int),
        FieldType::Dword64 => narrow!(v, ty, u64, Dword64),
        FieldType::Int64 => narrow!(v, ty, i64, Int64),
        FieldType::StrRef => narrow!(v, ty, u32, StrRef),
        FieldType::Float => Ok(GValue::Float(number(v, ty)? as f32)),
        FieldType::Double => Ok(GValue::Double(number(v, ty)?)),
        FieldType::String => Ok(GValue::String(byte_string(v, ty)?)),
        FieldType::Void => Ok(GValue::Void(byte_string(v, ty)?)),
        FieldType::ResRef => {
            let b = byte_string(v, ty)?;
            GValue::resref(&String::from_utf8_lossy(&b)).lift()
        }
        FieldType::LocString => Ok(GValue::LocString(locstring(v)?)),
        FieldType::Vector => Ok(GValue::Vector(floats::<3>(v, ty)?)),
        FieldType::Orientation => Ok(GValue::Orientation(floats::<4>(v, ty)?)),
        FieldType::Struct | FieldType::List => Err(bad(format!(
            "a {} field is not set from a plain value; copy one from another GFF with get_value",
            ty.name()
        ))),
    }
}

/// The value a script gives for a field: a typed value as it is, a plain
/// one in the type the field holds now. A field that does not exist yet
/// takes a typed value.
pub fn value_for(v: Value, holds: Option<FieldType>, path: &str) -> mlua::Result<GValue> {
    if let Value::UserData(u) = &v
        && let Ok(t) = u.borrow::<LValue>()
    {
        return Ok(t.0.clone());
    }
    match holds {
        Some(ty) => typed(&v, ty),
        None => Err(bad(format!(
            "{path} does not exist yet, so its type is not known: give a typed value, as rakata.gff.byte(5) or rakata.gff.string(\"x\")"
        ))),
    }
}

fn gff_path(path: &str) -> mlua::Result<GffPath> {
    GffPath::parse(path).lift()
}

/// The type the field at `path` holds, if it exists.
pub fn holds(g: &Gff, path: &str) -> Option<FieldType> {
    let p = GffPath::parse(path).ok()?;
    g.get(&p).ok().map(GValue::field_type)
}

impl UserData for LGff {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_method("file_type", |_, this, ()| Ok(this.0.file_type_str()));
        m.add_method("get", |lua, this, path: String| {
            plain(lua, this.0.get(&gff_path(&path)?).lift()?)
        });
        m.add_method("get_value", |_, this, path: String| {
            Ok(LValue(this.0.get(&gff_path(&path)?).lift()?.clone()))
        });
        m.add_method("type", |_, this, path: String| {
            Ok(this.0.get(&gff_path(&path)?).lift()?.field_type().name())
        });
        m.add_method("has", |_, this, path: String| {
            let p = gff_path(&path)?;
            Ok(this.0.get(&p).is_ok() || this.0.struct_at(&p).is_ok())
        });
        m.add_method("len", |_, this, path: String| {
            match this.0.get(&gff_path(&path)?).lift()? {
                GValue::List(items) => Ok(items.len()),
                other => Err(bad(format!(
                    "{path} holds a {}, not a List",
                    other.field_type().name()
                ))),
            }
        });
        m.add_method("fields", |lua, this, path: Option<String>| {
            let p = gff_path(path.as_deref().unwrap_or(""))?;
            let s = this.0.struct_at(&p).lift()?;
            lua.create_sequence_from(s.fields.iter().map(|f| f.label.to_string()))
        });
        m.add_method_mut("set", |lua, this, (path, value): (String, Value)| {
            let v = value_for(value, holds(&this.0, &path), &path)?;
            let old = this.0.set(&gff_path(&path)?, v).lift()?;
            plain(lua, &old)
        });
        m.add_method("clone", |_, this, ()| Ok(LGff(this.0.clone())));
        m.add_method("semantic_eq", |_, this, other: UserDataRef<LGff>| {
            Ok(this.0.semantic_eq(&other.0))
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!("Gff({})", this.0.file_type_str().trim_end()))
        });
    }
}

fn file_type(text: &str) -> mlua::Result<[u8; 4]> {
    let b = text.as_bytes();
    if b.is_empty() || b.len() > 4 {
        return Err(bad(format!(
            "a GFF file type is one to four characters, as UTC or DLG, not `{text}`"
        )));
    }
    let mut out = *b"    ";
    out[..b.len()].copy_from_slice(b);
    Ok(out)
}

// ---- LIP ----

/// A lip-sync track.
pub struct LLip(pub Lip);

/// A shape by its label (`rest`, `ee`, ..., `k`) or its byte, 0 to 15.
pub fn shape(v: &Value) -> mlua::Result<Shape> {
    let found = match v {
        Value::String(s) => Shape::from_label(&s.to_str()?),
        Value::Integer(i) => u8::try_from(*i).ok().and_then(Shape::from_byte),
        _ => None,
    };
    found.ok_or_else(|| {
        bad(format!(
            "a mouth shape is a label (rest ee eh ah oh oo y s f n th m t sh l k) or its number 0 to 15, not {}",
            describe(v)
        ))
    })
}

impl UserData for LLip {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("length", |_, this| Ok(this.0.length));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_method("keys", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, k) in this.0.keys.iter().enumerate() {
                let kt = lua.create_table()?;
                kt.set("time", k.time)?;
                kt.set("shape", k.shape.label())?;
                t.raw_set(i + 1, kt)?;
            }
            Ok(t)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!(
                "Lip(length {}, {} keyframes)",
                this.0.length,
                this.0.keys.len()
            ))
        });
    }
}

// ---- TLK ----

/// A talk table.
pub struct LTlk(pub Tlk);

impl UserData for LTlk {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("len", |_, this, ()| Ok(this.0.len()));
        m.add_meta_method("__len", |_, this, ()| Ok(this.0.len()));
        m.add_method("text", |_, this, strref: u32| {
            Ok(this
                .0
                .text(rakata::res::StrRef::new(strref))
                .map(str::to_owned))
        });
        m.add_method_mut("append", |_, this, text: String| {
            let s = this.0.append(rakata::tlk::Entry::new(text));
            Ok(s.raw())
        });
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!("Tlk({} entries)", this.0.len()))
        });
    }
}

/// The format tables.
pub fn install(lua: &Lua, rakata: &Table) -> mlua::Result<()> {
    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| {
            Ok(LTwoDa(TwoDa::parse(&b.as_bytes()).lift()?))
        })?,
    )?;
    t.set(
        "new",
        lua.create_function(|_, columns: Vec<String>| Ok(LTwoDa(TwoDa::new(columns))))?,
    )?;
    rakata.set("twoda", t)?;

    let g = lua.create_table()?;
    g.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| Ok(LGff(Gff::parse(&b.as_bytes()).lift()?)))?,
    )?;
    g.set(
        "new",
        lua.create_function(|_, ty: String| Ok(LGff(Gff::new(file_type(&ty)?))))?,
    )?;
    for (name, ty) in [
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
        ("resref", FieldType::ResRef),
        ("locstring", FieldType::LocString),
        ("void", FieldType::Void),
        ("vector", FieldType::Vector),
        ("orientation", FieldType::Orientation),
        ("strref", FieldType::StrRef),
    ] {
        g.set(
            name,
            lua.create_function(move |_, v: Value| Ok(LValue(typed(&v, ty)?)))?,
        )?;
    }
    rakata.set("gff", g)?;

    let l = lua.create_table()?;
    l.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| Ok(LLip(Lip::parse(&b.as_bytes()).lift()?)))?,
    )?;
    l.set(
        "from_cues",
        lua.create_function(|_, (cues, length): (Table, f32)| {
            Ok(LLip(Lip::from_cues(cue_list(&cues)?, length)))
        })?,
    )?;
    rakata.set("lip", l)?;

    let k = lua.create_table()?;
    k.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| Ok(LTlk(Tlk::parse(&b.as_bytes()).lift()?)))?,
    )?;
    rakata.set("tlk", k)?;
    Ok(())
}

/// `{{time, shape}, ...}`, each entry also accepted as `{time = t, shape = s}`.
pub fn cue_list(cues: &Table) -> mlua::Result<Vec<(f32, Shape)>> {
    let mut out = Vec::new();
    for c in cues.sequence_values::<Table>() {
        let c = c?;
        let time: Value = match c.get::<Value>("time")? {
            Value::Nil => c.raw_get(1)?,
            v => v,
        };
        let s: Value = match c.get::<Value>("shape")? {
            Value::Nil => c.raw_get(2)?,
            v => v,
        };
        out.push((number(&time, FieldType::Float)? as f32, shape(&s)?));
    }
    Ok(out)
}
