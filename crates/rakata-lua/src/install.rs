//! `rakata.Install` and `rakata.Mod`: the install `--game` names, read in
//! the engine's order, and opened for a mod's writes.

use std::sync::Arc;

use mlua::{Lua, Table, UserData, UserDataMethods, UserDataRef, Value};
use rakata::author::{Mod, Written};
use rakata::game::{Install, ModuleName, Verdict};
use rakata::res::ContentHash;

use crate::dialog::{LDialog, LNodeRef, node_key};
use crate::error::{Lift, bad, codes, raise};
use crate::sandbox::inputs;
use crate::values::{Opts, bytes, finding, location, profile, profile_name, res_id};

/// The profile `rakata lua --profile` names, the default of
/// `Install.open`.
#[derive(Debug, Clone, Copy)]
pub struct DefaultProfile(pub rakata::game::Profile);

/// An install, shared with the mods opened on it.
pub struct LInstall(pub Arc<Install>);

impl UserData for LInstall {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("root", |_, this, ()| {
            Ok(this.0.root().display().to_string())
        });
        m.add_method("profile", |_, this, ()| Ok(profile_name(this.0.profile())));
        m.add_method("modules", |lua, this, ()| {
            lua.create_sequence_from(this.0.modules().iter().map(ToString::to_string))
        });
        m.add_method(
            "resolve",
            |lua, this, (name, module): (String, Option<String>)| {
                let module = module.map(|m| ModuleName::new(&m));
                let r = this.0.resolve(res_id(&name)?, module.as_ref()).lift()?;
                let t = lua.create_table()?;
                t.set("id", r.id.to_string())?;
                t.set("module", r.module.as_ref().map(ToString::to_string))?;
                let candidates = lua.create_table()?;
                for (i, c) in r.candidates.iter().enumerate() {
                    let ct = lua.create_table()?;
                    ct.set("layer", c.layer.name())?;
                    ct.set("location", location(&this.0, &c.location))?;
                    ct.set("len", c.len)?;
                    ct.set("hash", c.hash.to_hex())?;
                    match &c.verdict {
                        Verdict::Wins => ct.set("verdict", "wins")?,
                        Verdict::Ambiguous => ct.set("verdict", "ambiguous")?,
                        Verdict::Shadowed { by, reason } => {
                            ct.set("verdict", "shadowed")?;
                            ct.set("by", *by)?;
                            ct.set("reason", reason.as_str())?;
                        }
                    }
                    candidates.raw_set(i + 1, ct)?;
                }
                t.set("candidates", candidates)?;
                t.set("ambiguous", r.is_ambiguous())?;
                Ok(t)
            },
        );
        m.add_method(
            "read",
            |lua, this, (name, module): (String, Option<String>)| {
                let module = module.map(|m| ModuleName::new(&m));
                match this.0.read(res_id(&name)?, module.as_ref()).lift()? {
                    None => Ok(Value::Nil),
                    Some((l, b)) => {
                        let t = lua.create_table()?;
                        t.set("bytes", bytes(lua, &b)?)?;
                        t.set("location", location(&this.0, &l))?;
                        Ok(Value::Table(t))
                    }
                }
            },
        );
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!("Install({})", this.0.root().display()))
        });
    }
}

/// `Install.open([{profile = "k1"}])`: the install `--game` names.
pub fn open_install(lua: &Lua, opts: Option<Table>) -> mlua::Result<LInstall> {
    let o = Opts::new(opts, "Install.open", &["profile"])?;
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
            "the script opened the install, and no install was named: pass --game DIR, the folder holding chitin.key",
        ));
    };
    Ok(LInstall(Arc::new(Install::open(&game, p).lift()?)))
}

/// An install opened for a mod's writes.
pub struct LMod {
    pub install: Arc<Install>,
    pub inner: Mod,
}

fn written(lua: &Lua, w: &Written, dialog: &str) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("path", w.path.display().to_string())?;
    t.set("hash", w.hash.to_hex())?;
    t.set("len", w.len)?;
    let warnings = lua.create_table()?;
    for (i, f) in w.warnings.iter().enumerate() {
        warnings.raw_set(i + 1, finding(lua, dialog, f)?)?;
    }
    t.set("warnings", warnings)?;
    Ok(t)
}

impl UserData for LMod {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("install", |_, this, ()| Ok(LInstall(this.install.clone())));
        m.add_method("module", |_, this, ()| {
            Ok(this.inner.module().map(ToString::to_string))
        });
        m.add_method("loaded", |lua, this, name: String| {
            let l = this.inner.loaded(res_id(&name)?).lift()?;
            let t = lua.create_table()?;
            t.set("bytes", bytes(lua, &l.bytes)?)?;
            t.set("location", l.location)?;
            t.set("written", l.written)?;
            Ok(t)
        });
        m.add_method(
            "guard_replaced",
            |_, this, (name, hash): (String, String)| {
                let expected: ContentHash = hash
                    .parse()
                    .map_err(|e| bad(format!("`{hash}` is not a SHA-256 in hex: {e}")))?;
                this.inner.guard_replaced(res_id(&name)?, expected).lift()
            },
        );
        m.add_method_mut(
            "write_override",
            |lua, this, (name, b): (String, mlua::LuaString)| {
                let w = this.inner.write_override(&name, &b.as_bytes()).lift()?;
                written(lua, &w, &name)
            },
        );
        m.add_method_mut(
            "write_module",
            |lua, this, (name, b): (String, mlua::LuaString)| {
                let w = this.inner.write_module(&name, &b.as_bytes()).lift()?;
                written(lua, &w, &name)
            },
        );
        m.add_method_mut("set_module", |_, this, module: Option<String>| {
            this.inner.set_module(module.as_deref()).lift()
        });
        m.add_method("guard_other_modules", |_, this, name: String| {
            this.inner.guard_other_modules(res_id(&name)?).lift()
        });
        m.add_method_mut(
            "write_streamwaves",
            |lua, this, (resref, b): (String, mlua::LuaString)| {
                let w = this
                    .inner
                    .write_streamwaves(&resref, &b.as_bytes())
                    .lift()?;
                written(lua, &w, &resref)
            },
        );
        m.add_method_mut(
            "find",
            |_, this, (d, key): (UserDataRef<LDialog>, Value)| {
                let key = node_key(key)?;
                Ok(LNodeRef(this.inner.find(&d.0, &key).lift()?))
            },
        );
        m.add_method_mut("check_dialog", |lua, this, d: UserDataRef<LDialog>| {
            let findings = this.inner.check_dialog(&d.0);
            let t = lua.create_table()?;
            for (i, f) in findings.iter().enumerate() {
                t.raw_set(i + 1, finding(lua, d.0.name(), f)?)?;
            }
            Ok(t)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(match this.inner.module() {
                Some(m) => format!("Mod({}, {m})", this.install.root().display()),
                None => format!("Mod({})", this.install.root().display()),
            })
        });
    }
}

/// `Mod.open(install, [module])`.
pub fn open_mod(install: UserDataRef<LInstall>, module: Option<String>) -> mlua::Result<LMod> {
    let arc = install.0.clone();
    let inner = Mod::open(arc.clone(), module.as_deref()).lift()?;
    Ok(LMod {
        install: arc,
        inner,
    })
}

/// `rakata.Install`, `rakata.Mod` and `rakata.res`.
pub fn install(lua: &Lua, rakata: &Table) -> mlua::Result<()> {
    let class = lua.create_table()?;
    class.set(
        "open",
        lua.create_function(|lua, opts: Option<Table>| open_install(lua, opts))?,
    )?;
    rakata.set("Install", class)?;

    let class = lua.create_table()?;
    class.set(
        "open",
        lua.create_function(
            |_, (install, module): (UserDataRef<LInstall>, Option<String>)| {
                open_mod(install, module)
            },
        )?,
    )?;
    rakata.set("Mod", class)?;

    let res = lua.create_table()?;
    res.set(
        "hash",
        lua.create_function(|_, b: mlua::LuaString| Ok(ContentHash::of(&b.as_bytes()).to_hex()))?,
    )?;
    res.set(
        "id",
        lua.create_function(|_, name: String| Ok(res_id(&name)?.to_string()))?,
    )?;
    rakata.set("res", res)?;
    Ok(())
}
