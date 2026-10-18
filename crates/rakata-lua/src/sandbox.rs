//! What a script may touch: the install `--game` names, its own directory
//! (for `require`, and for reading), and the paths its arguments name. The
//! state is built without `io`, `os`, `package` or `debug`, and without
//! `dofile` and `loadfile`, so the only way to a file is through here.

use std::cell::RefCell;
use std::fs;
use std::path::{Component, Path, PathBuf};

use mlua::{Lua, MultiValue, Table, Value};

use crate::error::{bad, codes, raise};

/// Where a script's printed lines go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Print {
    /// Straight to standard output.
    Stdout,
    /// Kept, for the JSON envelope ([`crate::Outcome::output`]).
    Capture,
}

/// The paths a script may read and write.
#[derive(Debug, Clone)]
pub struct Inputs {
    /// The install, as `--game` named it, and normalized.
    pub game: Option<(PathBuf, PathBuf)>,
    /// The script's directory, normalized.
    pub dir: PathBuf,
    /// Each argument as a path, normalized.
    pub args: Vec<PathBuf>,
}

/// An absolute path with `.` and `..` folded and the longest existing
/// prefix's symbolic links resolved, so a link cannot lead a path out of
/// the directory it appears to lie in.
pub fn normalize(p: &Path) -> PathBuf {
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(p)
    };
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    let mut existing = out.clone();
    let mut rest = Vec::new();
    while !existing.exists() {
        match (
            existing.file_name().map(ToOwned::to_owned),
            existing.parent(),
        ) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                existing = parent.to_path_buf();
            }
            _ => return out,
        }
    }
    let mut real = fs::canonicalize(&existing).unwrap_or(existing);
    for name in rest.into_iter().rev() {
        real.push(name);
    }
    real
}

impl Inputs {
    pub fn new(game: Option<&Path>, script: &Path, args: &[String]) -> Self {
        let dir = script
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        Inputs {
            game: game.map(|g| (g.to_path_buf(), normalize(g))),
            dir: normalize(dir),
            args: args
                .iter()
                .filter(|a| !a.is_empty())
                .map(|a| normalize(Path::new(a)))
                .collect(),
        }
    }

    fn writable(&self, p: &Path) -> bool {
        self.game.iter().any(|(_, g)| p.starts_with(g))
            || self.args.iter().any(|a| p.starts_with(a))
    }

    fn readable(&self, p: &Path) -> bool {
        self.writable(p) || p.starts_with(&self.dir)
    }

    /// `path`, when the script may read it; else `RE1801`.
    pub fn check_read(&self, path: &str) -> mlua::Result<PathBuf> {
        let n = normalize(Path::new(path));
        if self.readable(&n) {
            Ok(PathBuf::from(path))
        } else {
            Err(raise(
                codes::NOT_AN_INPUT,
                format!(
                    "`{path}` is not one of the script's inputs: a script reads only below --game, its own directory, and the paths its arguments name"
                ),
            ))
        }
    }

    /// `path`, when the script may write it; else `RE1801`.
    pub fn check_write(&self, path: &str) -> mlua::Result<PathBuf> {
        let n = normalize(Path::new(path));
        if self.writable(&n) {
            Ok(PathBuf::from(path))
        } else {
            Err(raise(
                codes::NOT_AN_INPUT,
                format!(
                    "`{path}` is not one of the script's inputs: a script writes only below --game and the paths its arguments name"
                ),
            ))
        }
    }
}

/// The inputs of the state's script.
pub fn inputs(lua: &Lua) -> mlua::Result<mlua::AppDataRef<'_, Inputs>> {
    lua.app_data_ref::<Inputs>()
        .ok_or_else(|| mlua::Error::runtime("the rakata state has no inputs"))
}

/// The lines `print` kept, under [`Print::Capture`].
#[derive(Default)]
pub struct Captured(pub RefCell<String>);

fn module_path(dir: &Path, name: &str) -> mlua::Result<PathBuf> {
    let parts: Vec<&str> = name.split('.').collect();
    let plain = |p: &&str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    };
    if !parts.iter().all(plain) {
        return Err(raise(
            codes::NO_SUCH_MODULE,
            format!(
                "`{name}` is not a module name: dotted words naming a file below the script's directory, as `lib.scenes` names lib/scenes.lua"
            ),
        ));
    }
    let mut p = dir.to_path_buf();
    for part in &parts {
        p.push(part);
    }
    p.set_extension("lua");
    Ok(p)
}

/// `require`, `print` and `rakata.fs`, and the dangerous base functions
/// removed.
pub fn install(lua: &Lua, rakata: &Table, print: Print) -> mlua::Result<()> {
    let g = lua.globals();
    g.set("dofile", Value::Nil)?;
    g.set("loadfile", Value::Nil)?;

    let loaded = lua.create_table()?;
    lua.set_named_registry_value("rakata.loaded", loaded)?;
    g.set(
        "require",
        lua.create_function(|lua, name: String| {
            if name == "rakata" {
                return lua.globals().get::<Value>("rakata");
            }
            let loaded: Table = lua.named_registry_value("rakata.loaded")?;
            let have: Value = loaded.get(name.as_str())?;
            if !have.is_nil() {
                return Ok(have);
            }
            let path = module_path(&inputs(lua)?.dir, &name)?;
            let code = fs::read(&path).map_err(|e| {
                raise(
                    codes::NO_SUCH_MODULE,
                    format!(
                        "no module `{name}` in the script's directory: {}: {e}",
                        path.display()
                    ),
                )
            })?;
            let v: Value = lua
                .load(&code)
                .set_name(format!("@{}", path.display()))
                .call(name.as_str())?;
            let v = if v.is_nil() { Value::Boolean(true) } else { v };
            loaded.set(name.as_str(), v.clone())?;
            Ok(v)
        })?,
    )?;

    lua.set_app_data(Captured::default());
    g.set(
        "print",
        lua.create_function(move |lua, args: MultiValue| {
            let mut line = String::new();
            for (i, v) in args.iter().enumerate() {
                if i > 0 {
                    line.push('\t');
                }
                let s: String = lua
                    .globals()
                    .get::<mlua::Function>("tostring")?
                    .call::<mlua::LuaString>(v.clone())?
                    .to_string_lossy();
                line.push_str(&s);
            }
            line.push('\n');
            match print {
                Print::Stdout => {
                    use std::io::Write;
                    let mut out = std::io::stdout().lock();
                    let _ = out.write_all(line.as_bytes());
                }
                Print::Capture => {
                    if let Some(c) = lua.app_data_ref::<Captured>() {
                        c.0.borrow_mut().push_str(&line);
                    }
                }
            }
            Ok(())
        })?,
    )?;

    let fsys = lua.create_table()?;
    fsys.set(
        "read",
        lua.create_function(|lua, path: String| {
            let p = inputs(lua)?.check_read(&path)?;
            let bytes = fs::read(&p)
                .map_err(|e| raise(codes::READ_FAILED, format!("cannot read `{path}`: {e}")))?;
            lua.create_string(bytes)
        })?,
    )?;
    fsys.set(
        "write",
        lua.create_function(|lua, (path, bytes): (String, mlua::LuaString)| {
            let p = inputs(lua)?.check_write(&path)?;
            let fail = |e: std::io::Error| {
                raise(codes::WRITE_FAILED, format!("cannot write `{path}`: {e}"))
            };
            if let Some(dir) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
                fs::create_dir_all(dir).map_err(fail)?;
            }
            fs::write(&p, &bytes.as_bytes()[..]).map_err(fail)?;
            Ok(())
        })?,
    )?;
    fsys.set(
        "exists",
        lua.create_function(|lua, path: String| {
            let p = inputs(lua)?.check_read(&path)?;
            Ok(p.exists())
        })?,
    )?;
    fsys.set(
        "list",
        lua.create_function(|lua, path: String| {
            let p = inputs(lua)?.check_read(&path)?;
            let rd = fs::read_dir(&p)
                .map_err(|e| raise(codes::READ_FAILED, format!("cannot list `{path}`: {e}")))?;
            let mut names: Vec<String> = rd
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            lua.create_sequence_from(names)
        })?,
    )?;
    fsys.set(
        "join",
        lua.create_function(|_, parts: MultiValue| {
            let mut p = PathBuf::new();
            for v in parts {
                match v {
                    Value::String(s) => p.push(s.to_str()?.to_string()),
                    other => {
                        return Err(bad(format!(
                            "rakata.fs.join takes strings, not {}",
                            other.type_name()
                        )));
                    }
                }
            }
            Ok(p.display().to_string())
        })?,
    )?;
    rakata.set("fs", fsys)?;
    Ok(())
}
