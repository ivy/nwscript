//! What the inspection commands (`cat`, `gff`, `2da`, `tlk`, `lip`,
//! `texture`, `lyt`, `walkmesh`, `mdl`) share: the flags that say where a
//! `NAME.EXT` is looked up, reading one named resource through
//! [`rakata::inspect::load`] into the envelope's inputs, finding a
//! resource beside another by name, and writing a file into its outputs.

use std::path::{Path, PathBuf};

use clap::Args;

use super::Ctx;
use crate::cli::ProfileArg;
use rakata::diag::Diagnostic;
use rakata::game::Profile;
use rakata::inspect::{Loaded, Lookup};
use rakata::res::ContentHash;
use rakata::{Envelope, InputRecord, OutputRecord};

/// The forms a resource argument takes, for each command's help.
macro_rules! resource_forms {
    () => {
        "\
A resource is named in one of three forms:
  NAME.EXT          with --game, the copy the engine loads, found as
                    `rakata which` finds it (--module adds the module's capsules)
  PATH              any name holding a `/`, and any name at all without --game,
                    is a file on disk (write ./NAME.EXT for a file here)
  ARCHIVE!MEMBER    a member of an ERF, MOD, SAV, HAK or RIM by name.ext, or of
                    a BIF by position as #N or #N.ext; members nest
                    (a.mod!b.erf!c.utc); ARCHIVE is a path, else a path within
                    --game in any case (modules/danm14ac.mod!dan14_juhani.utc)"
    };
}
pub(crate) use resource_forms;

/// How the two sides of a diff are looked up, for each diff's help.
macro_rules! diff_sides {
    () => {
        "\
Each side is named in any of those forms. --game-a and --game-b name an
install per side, so one name compares across two installs; --module-a and
--module-b likewise. A side without its own falls back to --game and
--module."
    };
}
pub(crate) use diff_sides;

/// Where a `NAME.EXT` argument is looked up, beside the global `--game`.
#[derive(Args, Debug, Clone)]
pub struct LookupArgs {
    /// The module whose capsules a NAME.EXT is also searched in: the stem of modules/STEM.mod, STEM.rim and STEM_s.rim.
    #[arg(long, short, value_name = "STEM")]
    pub module: Option<String>,
    /// The game the install belongs to.
    #[arg(long, value_enum, default_value_t = ProfileArg::K1)]
    pub profile: ProfileArg,
}

pub fn profile(p: ProfileArg) -> Profile {
    match p {
        ProfileArg::K1 => Profile::K1,
        ProfileArg::K2 => Profile::K2,
    }
}

impl LookupArgs {
    /// The lookup over `--game`.
    pub fn lookup<'a>(&'a self, ctx: &'a Ctx) -> Lookup<'a> {
        Lookup {
            game: ctx.game.as_deref(),
            module: self.module.as_deref(),
            profile: profile(self.profile),
        }
    }
}

/// Read the resource `name` names and record it as an input under `role`.
pub fn load(
    env: &mut Envelope,
    role: &str,
    name: &str,
    lookup: &Lookup<'_>,
) -> Result<Loaded, Diagnostic> {
    crate::working_on(name);
    let loaded = rakata::inspect::load(name, lookup)?;
    env.inputs.push(InputRecord {
        role: role.to_owned(),
        path: loaded.source.clone(),
        sha256: ContentHash::of(&loaded.bytes).to_hex(),
    });
    Ok(loaded)
}

/// Write `bytes` to `path` and record it as an output.
pub fn write(env: &mut Envelope, path: &Path, bytes: &[u8]) -> Result<(), Diagnostic> {
    std::fs::write(path, bytes).map_err(|e| {
        Diagnostic::new(
            &rakata::codes::RE1306,
            format!("cannot write `{}`: {e}", path.display()),
        )
    })?;
    env.outputs.push(OutputRecord {
        path: path.display().to_string(),
        sha256: ContentHash::of(bytes).to_hex(),
        bytes: bytes.len(),
    });
    Ok(())
}

/// One side of a diff: its name and where it is looked up.
pub struct Side<'a> {
    pub name: &'a str,
    pub game: Option<&'a PathBuf>,
    pub module: Option<&'a String>,
}

/// The flags of a two-sided command: each side's install and module,
/// falling back to `--game` and `--module`.
#[derive(Args, Debug, Clone)]
pub struct DiffArgs {
    /// The first resource.
    #[arg(value_name = "A")]
    pub a: String,
    /// The second resource.
    #[arg(value_name = "B")]
    pub b: String,
    /// The install A's NAME.EXT is resolved in, in place of --game.
    #[arg(long, value_name = "DIR")]
    pub game_a: Option<PathBuf>,
    /// The install B's NAME.EXT is resolved in, in place of --game.
    #[arg(long, value_name = "DIR")]
    pub game_b: Option<PathBuf>,
    /// The module searched for A, in place of --module.
    #[arg(long, value_name = "STEM")]
    pub module_a: Option<String>,
    /// The module searched for B, in place of --module.
    #[arg(long, value_name = "STEM")]
    pub module_b: Option<String>,
    #[command(flatten)]
    pub lookup: LookupArgs,
}

impl DiffArgs {
    /// Read both sides, recorded as inputs `a` and `b`.
    pub fn load(&self, ctx: &Ctx, env: &mut Envelope) -> Result<(Loaded, Loaded), Diagnostic> {
        let sides = [
            Side {
                name: &self.a,
                game: self.game_a.as_ref().or(ctx.game.as_ref()),
                module: self.module_a.as_ref().or(self.lookup.module.as_ref()),
            },
            Side {
                name: &self.b,
                game: self.game_b.as_ref().or(ctx.game.as_ref()),
                module: self.module_b.as_ref().or(self.lookup.module.as_ref()),
            },
        ];
        let mut out = Vec::with_capacity(2);
        for (role, side) in ["a", "b"].into_iter().zip(sides) {
            let lookup = Lookup {
                game: side.game.map(PathBuf::as_path),
                module: side.module.map(String::as_str),
                profile: profile(self.lookup.profile),
            };
            out.push(load(env, role, side.name, &lookup)?);
        }
        let b = out.pop().expect("two sides");
        let a = out.pop().expect("two sides");
        Ok((a, b))
    }
}

/// Read the resource `name` names when it exists, recorded under `role`;
/// `None` when nothing answers to the name (no file, no member, no copy in
/// the install), an error for anything else.
pub fn load_if_found(
    env: &mut Envelope,
    role: &str,
    name: &str,
    lookup: &Lookup<'_>,
) -> Result<Option<Loaded>, Diagnostic> {
    match load(env, role, name, lookup) {
        Ok(l) => Ok(Some(l)),
        Err(d) if ["RE0001", "RE0404", "RE1301"].contains(&d.code.id) => Ok(None),
        Err(d) => Err(d),
    }
}

/// `name` with its last extension replaced by `ext`, in the case of the
/// extension it had: `models/a.MDL` to `models/a.MDX`.
pub fn with_extension(name: &str, ext: &str) -> Option<String> {
    let (stem, old) = name.rsplit_once('.')?;
    if old.is_empty() || old.contains(['/', '\\', '!']) {
        return None;
    }
    let ext = if old.chars().all(|c| c.is_ascii_uppercase()) {
        ext.to_ascii_uppercase()
    } else {
        ext.to_owned()
    };
    Some(format!("{stem}.{ext}"))
}

/// The name of the resource `file` beside the one `name` names: in the same
/// folder for a path, the same archive for a member, and `file` itself for
/// a name the install resolves.
pub fn beside(name: &str, file: &str) -> String {
    match name.rfind(['/', '\\', '!']) {
        Some(at) => format!("{}{file}", &name[..=at]),
        None => file.to_owned(),
    }
}

/// The diagnostic for a resource `command` does not read, by its extension.
pub fn unread_type(loaded: &Loaded, command: &str, reads: &str) -> Diagnostic {
    let ext = loaded.extension().unwrap_or_default();
    Diagnostic::new(
        &rakata::codes::RE1307,
        format!(
            "`{}` is a .{ext}, which `rakata {command}` does not read",
            loaded.source
        ),
    )
    .help(match reader(&ext) {
        Some(r) => format!("`rakata {command}` reads {reads}; a .{ext} is read by `rakata {r}`"),
        None => format!("`rakata {command}` reads {reads}"),
    })
}

/// The inspection command that reads a resource of extension `ext`, for
/// the formats that are not GFF.
fn reader(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "tpc" | "txi" | "tga" => "texture show",
        "lyt" | "vis" => "lyt show",
        "wok" | "pwk" | "dwk" => "walkmesh show",
        "mdl" | "mdx" => "mdl show",
        "lip" => "lip show",
        "2da" => "2da show",
        _ => return None,
    })
}

/// Three floats as `(x, y, z)`.
pub fn point(v: [f32; 3]) -> String {
    format!("({}, {}, {})", v[0], v[1], v[2])
}

/// `N WORD`, the word taking an `s` (or `es` after an `s` or `h`) for
/// any count but one; `vertex` becomes `vertices`.
pub fn count(n: usize, word: &str) -> String {
    if n == 1 {
        return format!("1 {word}");
    }
    let plural = match word.strip_suffix("ex") {
        Some(stem) => format!("{stem}ices"),
        None if word.ends_with(['s', 'h']) => format!("{word}es"),
        None => format!("{word}s"),
    };
    format!("{n} {plural}")
}
