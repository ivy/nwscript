//! `rakata.dialog`: dialogues, the nodes in them by `NodeRef` and by
//! stable `NodeKey`, and the `Session` that loads them from the install and
//! builds their transcripts.

use mlua::{Lua, Table, UserData, UserDataFields, UserDataMethods, UserDataRef, Value};
use rakata::dialog::{Kind, NodeKey, NodeRef, Session, TranscriptOptions};
use rakata::game::ModuleName;
use rakata::res::ResRef;
use rakata_dialog::{Dialog, NewNode, Text};

use crate::error::{Lift, bad, codes, raise};
use crate::formats::LGff;
use crate::install::DefaultProfile;
use crate::sandbox::inputs;
use crate::values::{Opts, bytes, json, key_text, profile, res_id};

/// A dialogue.
pub struct LDialog(pub Dialog);

/// A node by its list and index.
#[derive(Clone, Copy)]
pub struct LNodeRef(pub NodeRef);

/// A node by what it says.
#[derive(Clone)]
pub struct LNodeKey(pub NodeKey);

/// `entry` or `reply` (`E` and `R` too).
pub fn kind(name: &str) -> mlua::Result<Kind> {
    match name.to_ascii_lowercase().as_str() {
        "entry" | "e" => Ok(Kind::Entry),
        "reply" | "r" => Ok(Kind::Reply),
        _ => Err(bad(format!("`{name}` is not a node kind: entry or reply"))),
    }
}

/// A key from Lua: a `NodeKey`, a `NodeRef`, or text as `NodeKey.parse`
/// reads it.
pub fn node_key(v: Value) -> mlua::Result<NodeKey> {
    match &v {
        Value::String(s) => s.to_str()?.parse::<NodeKey>().lift(),
        Value::UserData(u) => {
            if let Ok(k) = u.borrow::<LNodeKey>() {
                return Ok(k.0.clone());
            }
            if let Ok(r) = u.borrow::<LNodeRef>() {
                return Ok(NodeKey::from(r.0));
            }
            Err(bad(
                "a node key is a NodeKey, a NodeRef or text such as `voice:R`, `strref:N`, `text:T` or `E3`",
            ))
        }
        _ => Err(bad(format!(
            "a node key is a NodeKey, a NodeRef or text such as `voice:R`, `strref:N`, `text:T` or `E3`, not a {}",
            v.type_name()
        ))),
    }
}

/// A node reference from Lua: a `NodeRef`, or text as `E3`.
fn node_ref(d: &Dialog, v: Value) -> mlua::Result<NodeRef> {
    if let Value::UserData(u) = &v
        && let Ok(r) = u.borrow::<LNodeRef>()
    {
        return Ok(r.0);
    }
    node_key(v)?.resolve(d).lift()
}

impl UserData for LNodeRef {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("kind", |_, this| Ok(this.0.kind.name()));
        f.add_field_method_get("index", |_, this| Ok(this.0.index));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method("__tostring", |_, this, ()| Ok(this.0.to_string()));
        m.add_meta_method("__eq", |_, this, other: UserDataRef<LNodeRef>| {
            Ok(this.0 == other.0)
        });
    }
}

impl UserData for LNodeKey {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("is_fragile", |_, this, ()| Ok(this.0.is_fragile()));
        m.add_method("resolve", |_, this, d: UserDataRef<LDialog>| {
            Ok(LNodeRef(this.0.resolve(&d.0).lift()?))
        });
        m.add_meta_method("__tostring", |_, this, ()| Ok(key_text(&this.0)));
        m.add_meta_method("__eq", |_, this, other: UserDataRef<LNodeKey>| {
            Ok(this.0 == other.0)
        });
    }
}

fn resref_text(r: Option<ResRef>) -> Option<String> {
    r.map(|r| r.to_string())
}

fn links_table(lua: &Lua, links: &[rakata_dialog::Link<'_>]) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for (i, l) in links.iter().enumerate() {
        let lt = lua.create_table()?;
        lt.set("target", LNodeRef(l.target))?;
        lt.set("condition", resref_text(l.condition()))?;
        lt.set("is_child", l.is_child())?;
        t.raw_set(i + 1, lt)?;
    }
    Ok(t)
}

impl UserData for LDialog {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("name", |_, this, ()| Ok(this.0.name().to_string()));
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_method("len", |_, this, k: String| Ok(this.0.len(kind(&k)?)));
        m.add_method("gff", |_, this, ()| Ok(LGff(this.0.gff().clone())));
        m.add_method("clone", |_, this, ()| Ok(LDialog(this.0.clone())));
        m.add_method("node", |lua, this, at: Value| {
            let at = node_ref(&this.0, at)?;
            let Some(n) = this.0.node(at) else {
                return Ok(Value::Nil);
            };
            let t = lua.create_table()?;
            t.set("at", LNodeRef(n.at))?;
            t.set("key", LNodeKey(NodeKey::of(&this.0, n.at)))?;
            t.set("speaker", n.speaker())?;
            t.set("listener", n.listener())?;
            t.set("strref", n.strref())?;
            t.set("text", n.embedded_text())?;
            t.set("voice", resref_text(n.voice()))?;
            t.set("sound", resref_text(n.sound()))?;
            t.set("script", resref_text(n.script()))?;
            t.set("delay", n.delay())?;
            t.set("camera_angle", n.camera_angle())?;
            t.set("camera_id", n.camera_id())?;
            t.set("quest", n.quest())?;
            t.set("quest_entry", n.quest_entry())?;
            t.set("comment", n.comment())?;
            let anims = lua.create_table()?;
            for (i, a) in n.animations().iter().enumerate() {
                let at = lua.create_table()?;
                at.set("participant", a.participant.as_str())?;
                at.set("animation", a.animation)?;
                anims.raw_set(i + 1, at)?;
            }
            t.set("animations", anims)?;
            t.set("links", links_table(lua, &n.links())?)?;
            Ok(Value::Table(t))
        });
        m.add_method("starts", |lua, this, ()| links_table(lua, &this.0.starts()));
        m.add_method("key_of", |_, this, at: Value| {
            let at = node_ref(&this.0, at)?;
            Ok(LNodeKey(NodeKey::of(&this.0, at)))
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!(
                "Dialog({}, {} entries, {} replies)",
                this.0.name(),
                this.0.len(Kind::Entry),
                this.0.len(Kind::Reply)
            ))
        });
    }
}

/// A new node from `{text = ..., speaker = ..., ...}`: text is a string
/// (English, embedded), an integer (a strref), or absent.
pub fn new_node(t: Option<Table>) -> mlua::Result<NewNode> {
    let o = Opts::new(
        t,
        "a new node",
        &[
            "text",
            "speaker",
            "listener",
            "voice",
            "script",
            "animations",
            "quest",
            "camera",
            "comment",
        ],
    )?;
    let text = match o.get::<Value>("text")? {
        None | Some(Value::Nil) => Text::None,
        Some(Value::String(s)) => Text::English(s.to_str()?.to_string()),
        Some(Value::Integer(i)) => {
            Text::StrRef(u32::try_from(i).map_err(|_| bad(format!("{i} is not a strref")))?)
        }
        Some(other) => {
            return Err(bad(format!(
                "a node's text is a string or a strref, not a {}",
                other.type_name()
            )));
        }
    };
    let resref = |v: Option<String>| -> mlua::Result<Option<ResRef>> {
        match v.as_deref() {
            None | Some("") => Ok(None),
            Some(r) => ResRef::new(r).map(Some).lift(),
        }
    };
    let mut animations = Vec::new();
    if let Some(list) = o.get::<Table>("animations")? {
        for a in list.sequence_values::<Table>() {
            let a = a?;
            let who: Value = match a.get::<Value>("participant")? {
                Value::Nil => a.raw_get(1)?,
                v => v,
            };
            let n: Value = match a.get::<Value>("animation")? {
                Value::Nil => a.raw_get(2)?,
                v => v,
            };
            let (Value::String(who), Value::Integer(n)) = (who, n) else {
                return Err(bad(
                    "an animation is {participant, animation}: a tag and the engine's number",
                ));
            };
            let n = u16::try_from(n).map_err(|_| bad(format!("{n} is not an animation number")))?;
            animations.push((who.to_str()?.to_string(), n));
        }
    }
    let quest = match o.get::<Table>("quest")? {
        None => None,
        Some(q) => {
            let category: String = match q.get::<Option<String>>("category")? {
                Some(c) => c,
                None => q.raw_get(1)?,
            };
            let entry: u32 = match q.get::<Option<u32>>("entry")? {
                Some(e) => e,
                None => q.raw_get(2)?,
            };
            Some((category, entry))
        }
    };
    Ok(NewNode {
        text,
        speaker: o.get::<String>("speaker")?.unwrap_or_default(),
        listener: o.get::<String>("listener")?.unwrap_or_default(),
        voice: resref(o.get("voice")?)?,
        script: resref(o.get("script")?)?,
        animations,
        quest,
        camera: o.get("camera")?,
        comment: o.get::<String>("comment")?.unwrap_or_default(),
    })
}

/// A session over the install `--game` names.
pub struct LSession(pub Session);

impl UserData for LSession {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method_mut("load", |lua, this, what: String| {
            // A path, with a directory in it, is a file the inputs check
            // admits; anything else is a name the install resolves.
            let l = if what.contains('/') || what.contains('\\') {
                inputs(lua)?.check_read(&what)?;
                this.0.load(&what).lift()?
            } else {
                this.0.load_id(res_id(&what)?).lift()?
            };
            let t = lua.create_table()?;
            t.set("location", l.location)?;
            t.set("module", l.module)?;
            t.set("dialog", LDialog(l.dialog))?;
            Ok(t)
        });
        m.add_method_mut("set_module", |_, this, module: Option<String>| {
            this.0.set_module(module.map(|m| ModuleName::new(&m)));
            Ok(())
        });
        m.add_method("module", |_, this, ()| {
            Ok(this.0.module().map(ToString::to_string))
        });
        m.add_method_mut(
            "transcript",
            |lua, this, (d, opts): (UserDataRef<LDialog>, Option<Table>)| {
                let o = Opts::new(opts, "Session:transcript", &["scripts"])?;
                let options = TranscriptOptions {
                    scripts: o.get::<bool>("scripts")?.unwrap_or(false),
                };
                let tr = this.0.transcript(&d.0, options);
                let data = serde_json::to_value(&tr)
                    .map_err(|e| mlua::Error::runtime(format!("the transcript: {e}")))?;
                let v = json(lua, &data)?;
                if let Value::Table(t) = &v {
                    t.set("text", tr.text())?;
                }
                Ok(v)
            },
        );
        m.add_method_mut(
            "resolve",
            |_, this, (d, key): (UserDataRef<LDialog>, Value)| {
                let key = node_key(key)?;
                Ok(LNodeRef(this.0.resolve(&d.0, &key).lift()?))
            },
        );
    }
}

/// `rakata.dialog`, with `NodeKey`, `NodeRef` and `Session`.
pub fn install(lua: &Lua, rakata: &Table) -> mlua::Result<()> {
    let d = lua.create_table()?;
    d.set(
        "parse",
        lua.create_function(|_, (name, b): (String, mlua::LuaString)| {
            Ok(LDialog(Dialog::parse(name, &b.as_bytes()).lift()?))
        })?,
    )?;

    let key = lua.create_table()?;
    key.set(
        "parse",
        lua.create_function(|_, text: String| Ok(LNodeKey(text.parse::<NodeKey>().lift()?)))?,
    )?;
    key.set(
        "voice",
        lua.create_function(|_, r: String| Ok(LNodeKey(NodeKey::Voice(ResRef::new(&r).lift()?))))?,
    )?;
    key.set(
        "strref",
        lua.create_function(|_, s: u32| Ok(LNodeKey(NodeKey::StrRef(s))))?,
    )?;
    key.set(
        "text",
        lua.create_function(|_, t: String| Ok(LNodeKey(NodeKey::Text(t))))?,
    )?;
    key.set(
        "index",
        lua.create_function(|_, (k, i): (String, usize)| {
            Ok(LNodeKey(NodeKey::Index(kind(&k)?, i)))
        })?,
    )?;
    key.set(
        "of",
        lua.create_function(|_, (d, at): (UserDataRef<LDialog>, Value)| {
            let at = node_ref(&d.0, at)?;
            Ok(LNodeKey(NodeKey::of(&d.0, at)))
        })?,
    )?;
    d.set("NodeKey", key)?;

    let r = lua.create_table()?;
    r.set(
        "new",
        lua.create_function(|_, (k, i): (String, usize)| {
            Ok(LNodeRef(NodeRef {
                kind: kind(&k)?,
                index: i,
            }))
        })?,
    )?;
    r.set(
        "entry",
        lua.create_function(|_, i: usize| Ok(LNodeRef(NodeRef::entry(i))))?,
    )?;
    r.set(
        "reply",
        lua.create_function(|_, i: usize| Ok(LNodeRef(NodeRef::reply(i))))?,
    )?;
    d.set("NodeRef", r)?;

    let s = lua.create_table()?;
    s.set(
        "open",
        lua.create_function(|lua, opts: Option<Table>| {
            let o = Opts::new(opts, "Session.open", &["profile", "module"])?;
            let p = match o.get::<String>("profile")? {
                Some(p) => profile(&p)?,
                None => lua
                    .app_data_ref::<DefaultProfile>()
                    .map(|d| d.0)
                    .unwrap_or(rakata::game::Profile::K1),
            };
            let game = inputs(lua)?.game.as_ref().map(|(g, _)| g.clone());
            let Some(game) = game else {
                return Err(raise(
                    codes::NO_GAME,
                    "the script opened a dialogue session, and no install was named: pass --game DIR, the folder holding chitin.key",
                ));
            };
            let mut session = Session::open(&game, p).lift()?;
            if let Some(m) = o.get::<String>("module")? {
                session.set_module(Some(ModuleName::new(&m)));
            }
            Ok(LSession(session))
        })?,
    )?;
    d.set("Session", s)?;
    rakata.set("dialog", d)?;
    Ok(())
}
