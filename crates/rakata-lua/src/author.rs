//! `rakata.author`: the authoring operations, each the façade's function of
//! the same name, over the values the format tables hold.

use mlua::{Lua, Table, UserDataRef, UserDataRefMut, Value};
use rakata::author::dialog as dlg;
use rakata::author::globals::{self, GlobalType};
use rakata::author::scripts::{self, Flags, Source};
use rakata::author::twoda::{self, RowKey};
use rakata::author::voice::{self, Line, Take};
use rakata::author::{blueprint, lip};
use rakata::res::ContentHash;

use crate::dialog::{LDialog, LNodeRef, kind, new_node, node_key};
use crate::error::{Lift, bad};
use crate::formats::{LGff, LLip, LRow, LTwoDa, LValue, cells, cue_list, holds, plain, value_for};
use crate::install::LMod;
use crate::values::{Opts, bytes};

fn row_key(v: Value) -> mlua::Result<RowKey> {
    Ok(match LRow::from_lua(v)? {
        LRow::Index(i) => RowKey::Index(i),
        LRow::Label(l) => RowKey::Label(l),
    })
}

fn global_type(name: &str) -> mlua::Result<GlobalType> {
    match name.to_ascii_lowercase().as_str() {
        "boolean" => Ok(GlobalType::Boolean),
        "number" => Ok(GlobalType::Number),
        _ => Err(bad(format!(
            "`{name}` is not a global's type: Boolean or Number"
        ))),
    }
}

/// `{condition = "k_c", at = 0}`, both optional.
fn link_opts(t: Option<Table>, what: &str) -> mlua::Result<(Option<String>, Option<usize>)> {
    let o = Opts::new(t, what, &["condition", "at"])?;
    Ok((o.get("condition")?, o.get("at")?))
}

/// Sources as `{{name = "k_x", text = "..."}, ...}` or `{k_x = "..."}`.
fn sources(t: Option<Table>) -> mlua::Result<Vec<Source>> {
    let mut out = Vec::new();
    let Some(t) = t else {
        return Ok(out);
    };
    for pair in t.pairs::<Value, Value>() {
        let (k, v) = pair?;
        match (k, v) {
            (Value::Integer(_), Value::Table(s)) => {
                let name: String = s.get("name")?;
                let text: mlua::LuaString = s.get("text")?;
                out.push(Source {
                    name,
                    text: text.as_bytes().to_vec(),
                });
            }
            (Value::String(name), Value::String(text)) => out.push(Source {
                name: name.to_str()?.to_string(),
                text: text.as_bytes().to_vec(),
            }),
            _ => {
                return Err(bad(
                    "sources are {{name = NAME, text = SOURCE}, ...} or {NAME = SOURCE}",
                ));
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// `rakata.author` and its tables.
pub fn install(lua: &Lua, rakata: &Table) -> mlua::Result<()> {
    let author = lua.create_table()?;

    let t = lua.create_table()?;
    t.set(
        "find",
        lua.create_function(|_, (table, key): (UserDataRef<LTwoDa>, Value)| {
            twoda::find(&table.0, &row_key(key)?).lift()
        })?,
    )?;
    t.set(
        "add_row_copying",
        lua.create_function(
            |_,
             (mut table, from, label, c): (
                UserDataRefMut<LTwoDa>,
                Value,
                String,
                Option<Table>,
            )| {
                let from = row_key(from)?;
                twoda::add_row_copying(&mut table.0, &from, &label, &cells(c)?).lift()
            },
        )?,
    )?;
    author.set("twoda", t)?;

    let g = lua.create_table()?;
    g.set(
        "declare",
        lua.create_function(
            |_, (mut table, name, ty): (UserDataRefMut<LTwoDa>, String, String)| {
                globals::declare(&mut table.0, &name, global_type(&ty)?).lift()
            },
        )?,
    )?;
    author.set("globals", g)?;

    let b = lua.create_table()?;
    b.set(
        "get",
        lua.create_function(|lua, (g, path): (UserDataRef<LGff>, String)| {
            plain(lua, &blueprint::get(&g.0, &path).lift()?)
        })?,
    )?;
    b.set(
        "get_value",
        lua.create_function(|_, (g, path): (UserDataRef<LGff>, String)| {
            Ok(LValue(blueprint::get(&g.0, &path).lift()?))
        })?,
    )?;
    b.set(
        "set",
        lua.create_function(
            |lua, (mut g, path, value): (UserDataRefMut<LGff>, String, Value)| {
                let v = value_for(value, holds(&g.0, &path), &path)?;
                match blueprint::set(&mut g.0, &path, v).lift()? {
                    Some(old) => plain(lua, &old),
                    None => Ok(Value::Nil),
                }
            },
        )?,
    )?;
    b.set(
        "set_each",
        lua.create_function(
            |_, (mut g, list, label, value): (UserDataRefMut<LGff>, String, String, Value)| {
                let first = format!("{list}[0].{label}");
                let v = value_for(value, holds(&g.0, &first), &first)?;
                blueprint::set_each(&mut g.0, &list, &label, v).lift()
            },
        )?,
    )?;
    b.set(
        "clone",
        lua.create_function(|_, (g, template): (UserDataRef<LGff>, String)| {
            Ok(LGff(blueprint::clone(&g.0, &template).lift()?))
        })?,
    )?;
    author.set("blueprint", b)?;

    let d = lua.create_table()?;
    d.set(
        "add_entry",
        lua.create_function(
            |_, (mut dg, node): (UserDataRefMut<LDialog>, Option<Table>)| {
                Ok(LNodeRef(
                    dlg::add_entry(&mut dg.0, &new_node(node)?).lift()?,
                ))
            },
        )?,
    )?;
    d.set(
        "add_reply",
        lua.create_function(
            |_, (mut dg, node): (UserDataRefMut<LDialog>, Option<Table>)| {
                Ok(LNodeRef(
                    dlg::add_reply(&mut dg.0, &new_node(node)?).lift()?,
                ))
            },
        )?,
    )?;
    d.set(
        "link",
        lua.create_function(
            |_, (mut dg, from, to, opts): (UserDataRefMut<LDialog>, Value, Value, Option<Table>)| {
                let (condition, at) = link_opts(opts, "author.dialog.link")?;
                let (from, to) = (node_key(from)?, node_key(to)?);
                dlg::link(&mut dg.0, &from, &to, condition.as_deref(), at).lift()
            },
        )?,
    )?;
    d.set(
        "unlink_all",
        lua.create_function(|_, (mut dg, from): (UserDataRefMut<LDialog>, Value)| {
            dlg::unlink_all(&mut dg.0, &node_key(from)?).lift()
        })?,
    )?;
    d.set(
        "start",
        lua.create_function(
            |_, (mut dg, to, opts): (UserDataRefMut<LDialog>, Value, Option<Table>)| {
                let (condition, at) = link_opts(opts, "author.dialog.start")?;
                dlg::start(&mut dg.0, &node_key(to)?, condition.as_deref(), at).lift()
            },
        )?,
    )?;
    d.set(
        "reorder",
        lua.create_function(
            |_, (mut dg, k, order): (UserDataRefMut<LDialog>, String, Vec<usize>)| {
                dlg::reorder(&mut dg.0, kind(&k)?, &order).lift()
            },
        )?,
    )?;
    d.set(
        "empty_like",
        lua.create_function(|_, (like, name): (UserDataRef<LDialog>, String)| {
            Ok(LDialog(dlg::empty_like(&like.0, &name)))
        })?,
    )?;
    d.set("CAMERA_STATIC", dlg::CAMERA_STATIC)?;
    d.set("NO_DELAY", dlg::NO_DELAY)?;
    author.set("dialog", d)?;

    let l = lua.create_table()?;
    l.set(
        "from_cues",
        lua.create_function(|_, (cues, length): (Table, f32)| {
            Ok(LLip(lip::from_cues(&cue_list(&cues)?, length)))
        })?,
    )?;
    l.set(
        "rhubarb_shape",
        lua.create_function(|_, letter: String| {
            let mut chars = letter.chars();
            Ok(match (chars.next(), chars.next()) {
                (Some(c), None) => lip::rhubarb_shape(c).map(|s| s.label()),
                _ => None,
            })
        })?,
    )?;
    author.set("lip", l)?;

    let s = lua.create_table()?;
    s.set(
        "compile",
        lua.create_function(
            |lua, (m, srcs, opts): (UserDataRef<LMod>, Option<Table>, Option<Table>)| {
                let o = Opts::new(opts, "author.scripts.compile", &["includes", "flags"])?;
                let flags = match o.get::<String>("flags")?.as_deref() {
                    None | Some("default") => Flags::default(),
                    Some("bioware") => Flags::BIOWARE,
                    Some(other) => {
                        return Err(bad(format!(
                            "`{other}` is not a set of compile flags: default or bioware"
                        )));
                    }
                };
                let out = scripts::compile(
                    &m.inner,
                    &sources(srcs)?,
                    &sources(o.get("includes")?)?,
                    flags,
                )
                .lift()?;
                let t = lua.create_table()?;
                for (i, c) in out.iter().enumerate() {
                    let ct = lua.create_table()?;
                    ct.set("name", c.name.as_str())?;
                    ct.set("ncs", bytes(lua, &c.ncs)?)?;
                    ct.set("warnings", c.warnings.as_str())?;
                    let inputs = lua.create_table()?;
                    for (j, r) in c.inputs.iter().enumerate() {
                        let rt = lua.create_table()?;
                        rt.set("role", r.role.as_str())?;
                        rt.set("path", r.path.as_str())?;
                        rt.set("sha256", r.sha256.as_str())?;
                        inputs.raw_set(j + 1, rt)?;
                    }
                    ct.set("inputs", inputs)?;
                    t.raw_set(i + 1, ct)?;
                }
                Ok(t)
            },
        )?,
    )?;
    author.set("scripts", s)?;

    let v = lua.create_table()?;
    v.set(
        "take",
        lua.create_function(
            |lua, (name, audio, transcript): (String, mlua::LuaString, String)| {
                let take = Take::from_transcript(&name, &audio.as_bytes(), &transcript).lift()?;
                take_table(lua, &take)
            },
        )?,
    )?;
    v.set(
        "spans",
        lua.create_function(|lua, (lines, takes, lead): (Table, Table, Option<f64>)| {
            let lines = lines
                .sequence_values::<Table>()
                .map(|l| {
                    let l = l?;
                    Ok(Line {
                        resref: l.get("resref")?,
                        take: l.get("take")?,
                        text: l.get::<Option<String>>("text")?.unwrap_or_default(),
                        first: l.get("first")?,
                        last: l.get("last")?,
                    })
                })
                .collect::<mlua::Result<Vec<_>>>()?;
            let takes = takes
                .sequence_values::<Table>()
                .map(|t| take_of(&t?))
                .collect::<mlua::Result<Vec<_>>>()?;
            let spans = voice::spans(&lines, &takes, lead.unwrap_or(0.0)).lift()?;
            let t = lua.create_table()?;
            for (i, s) in spans.iter().enumerate() {
                let st = lua.create_table()?;
                st.set("resref", s.resref.to_string())?;
                st.set("take", s.take.as_str())?;
                st.set("text", s.text.as_str())?;
                st.set("start", s.start)?;
                st.set("end", s.end)?;
                st.set("start_text", s.start_text())?;
                st.set("end_text", s.end_text())?;
                st.set("key", s.key.as_str())?;
                t.raw_set(i + 1, st)?;
            }
            Ok(t)
        })?,
    )?;
    v.set(
        "take_cues",
        lua.create_function(|_, json: String| Ok(LLip(voice::take_cues(&json).lift()?)))?,
    )?;
    v.set(
        "streamwave",
        lua.create_function(|lua, mp3: mlua::LuaString| {
            bytes(lua, &voice::streamwave(&mp3.as_bytes()))
        })?,
    )?;
    author.set("voice", v)?;

    rakata.set("author", author)?;
    Ok(())
}

fn take_table(lua: &Lua, take: &Take) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("name", take.name.as_str())?;
    t.set("hash", take.hash.to_hex())?;
    let segs = lua.create_table()?;
    for (i, (a, b)) in take.segments.iter().enumerate() {
        segs.raw_set(i + 1, lua.create_sequence_from([*a, *b])?)?;
    }
    t.set("segments", segs)?;
    Ok(t)
}

fn take_of(t: &Table) -> mlua::Result<Take> {
    let hash: String = t.get("hash")?;
    let hash: ContentHash = hash
        .parse()
        .map_err(|e| bad(format!("a take's hash is a SHA-256 in hex: {e}")))?;
    let segments = t
        .get::<Table>("segments")?
        .sequence_values::<Vec<f64>>()
        .map(|s| {
            let s = s?;
            match s.as_slice() {
                [a, b] => Ok((*a, *b)),
                _ => Err(bad("a take's segment is {start, end}")),
            }
        })
        .collect::<mlua::Result<Vec<_>>>()?;
    Ok(Take {
        name: t.get("name")?,
        hash,
        segments,
    })
}
