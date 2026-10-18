//! What the install holds, by name and type, for the dangling-reference
//! rule: every resource the engine can load from somewhere, and in which
//! module.
//!
//! The global layers are `Override/` (its top level, as
//! [`Install::override_ids`] lists it), `patch.erf`, the BIFs `chitin.key`
//! indexes, the texture packs the install searches
//! ([`Install::texture_pack_ids`]), the capsules under `rims/`, `lips/`
//! capsules no module owns, and the loose files under `streamwaves/`,
//! `streammusic/`, `streamsounds/` and `movies/`. Each module adds its own
//! capsules ([`Install::module_ids`]). The capsule files themselves are
//! names too: a value naming a module is checked against the modules.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rakata_archive::{Erf, Rim, Source};
use rakata_game::{Install, ModuleName};
use rakata_res::{ResId, ResType};

/// What a site names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Want {
    /// A resource of this type.
    Type(ResType),
    /// A module, by the stem its capsules share.
    Module,
    /// A blueprint of any kind (`utc`, `uti`, `utp`, …).
    Blueprint,
    /// A resource of any type.
    Any,
}

impl Want {
    /// The text a finding shows: the extension, `module`, `blueprint` or
    /// `any`.
    pub(crate) fn name(self) -> String {
        match self {
            Want::Type(t) => t.to_string(),
            Want::Module => "module".into(),
            Want::Blueprint => "blueprint".into(),
            Want::Any => "any".into(),
        }
    }

    /// `"mdl"` → a type, `"module"` → a module.
    pub(crate) fn parse(s: &str) -> Option<Want> {
        match s {
            "module" => Some(Want::Module),
            "blueprint" => Some(Want::Blueprint),
            "any" => Some(Want::Any),
            ext => ResType::from_extension(ext).map(Want::Type),
        }
    }
}

const BLUEPRINTS: [ResType; 9] = [
    ResType::Utc,
    ResType::Uti,
    ResType::Utp,
    ResType::Utd,
    ResType::Utm,
    ResType::Utw,
    ResType::Utt,
    ResType::Ute,
    ResType::Uts,
];

/// Where a resource is loaded, and so what it can name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    /// A member of one of this module's capsules: the global layers and
    /// the module's own capsules.
    Module(String),
    /// Anywhere else (`Override/`, the BIFs, `patch.erf`): it can be loaded
    /// in any module, so a name any module holds resolves.
    Anywhere,
}

type Names = HashMap<String, Vec<ResType>>;

fn add(names: &mut Names, name: &str, ty: ResType) {
    let v = names.entry(name.to_ascii_lowercase()).or_default();
    if !v.contains(&ty) {
        v.push(ty);
    }
}

fn add_id(names: &mut Names, id: &ResId) {
    add(names, &String::from_utf8_lossy(id.name.as_bytes()), id.ty);
}

/// The formats a texture is loaded from: a site that names a `tpc` is
/// satisfied by any of them, since the game reads a `.tga` or `.dds` in
/// `Override/` where it has no `.tpc` (and crashes on both, RE1901).
const TEXTURES: [ResType; 3] = [ResType::Tpc, ResType::Tga, ResType::Dds];

/// The names a scope holds.
fn holds(names: &Names, name: &str, want: Want, modules: &HashSet<String>) -> bool {
    match want {
        Want::Module => modules.contains(name),
        want => names.get(name).is_some_and(|types| match want {
            Want::Type(t) if TEXTURES.contains(&t) => types.iter().any(|t| TEXTURES.contains(t)),
            Want::Type(t) => types.contains(&t),
            Want::Blueprint => types.iter().any(|t| BLUEPRINTS.contains(t)),
            _ => true,
        }),
    }
}

/// Every name the install holds, by scope.
pub(crate) struct Universe {
    global: Names,
    modules: HashMap<String, Names>,
    anywhere: Names,
    module_names: HashSet<String>,
}

/// The entry of `dir` named `name` in any case.
pub(crate) fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
        .collect();
    found.sort();
    found.into_iter().next()
}

fn files_of(dir: &Path, deep: bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            if deep {
                files_of(&p, deep, out);
            }
        } else {
            out.push(p);
        }
    }
}

/// A file's stem, lowercase, and its type by extension.
fn ident(p: &Path) -> Option<(String, ResType)> {
    let name = p.file_name()?.to_string_lossy().to_ascii_lowercase();
    let (stem, ext) = name.rsplit_once('.')?;
    Some((stem.to_string(), ResType::from_extension(ext)?))
}

/// The members of an ERF or RIM capsule; empty when it does not read.
fn capsule_ids(p: &Path) -> Vec<ResId> {
    let Ok(src) = Source::map(p) else {
        return Vec::new();
    };
    let bytes = src.bytes();
    if let Ok(rim) = Rim::parse(bytes) {
        rim.members().iter().map(|m| m.id).collect()
    } else if let Ok(erf) = Erf::parse(bytes) {
        erf.members().iter().map(|m| m.id).collect()
    } else {
        Vec::new()
    }
}

impl Universe {
    /// Everything the install at `root` holds.
    pub(crate) fn open(install: &Install) -> Universe {
        let root = install.root();
        let mut global = Names::new();
        for e in install.key().entries() {
            add_id(&mut global, &e.id);
        }
        for id in install.override_ids() {
            add_id(&mut global, &id);
        }
        if install.profile().has_patch_erf()
            && let Some(p) = child_ci(root, "patch.erf")
        {
            for id in capsule_ids(&p) {
                add_id(&mut global, &id);
            }
        }
        let installed: HashSet<String> = install
            .modules()
            .iter()
            .map(|m| m.as_str().to_string())
            .collect();
        let mut module_names = installed.clone();
        let mut capsules = Vec::new();
        for (dir, own) in [("modules", true), ("rims", false), ("lips", true)] {
            let Some(d) = child_ci(root, dir) else {
                continue;
            };
            let mut files = Vec::new();
            files_of(&d, false, &mut files);
            for f in files {
                let Some((stem, ty)) = ident(&f) else {
                    continue;
                };
                add(&mut global, &stem, ty);
                if dir == "rims" && ty == ResType::Rim {
                    module_names.insert(ModuleName::new(&stem).as_str().to_string());
                }
                let owned = own && installed.contains(ModuleName::new(&stem).as_str());
                if !owned && matches!(ty, ResType::Rim | ResType::Mod | ResType::Erf) {
                    capsules.push(f);
                }
            }
        }
        if let Some(d) = child_ci(root, "texturepacks") {
            let mut files = Vec::new();
            files_of(&d, false, &mut files);
            for f in files {
                if let Some((stem, ty)) = ident(&f) {
                    add(&mut global, &stem, ty);
                }
            }
        }
        for id in install.texture_pack_ids().unwrap_or_default() {
            add_id(&mut global, &id);
        }
        for c in capsules {
            for id in capsule_ids(&c) {
                add_id(&mut global, &id);
            }
        }
        for dir in ["streamwaves", "streammusic", "streamsounds", "movies"] {
            if let Some(d) = child_ci(root, dir) {
                let mut files = Vec::new();
                files_of(&d, true, &mut files);
                for f in files {
                    if let Some((stem, ty)) = ident(&f) {
                        add(&mut global, &stem, ty);
                    }
                }
            }
        }
        let mut anywhere = global.clone();
        let mut modules = HashMap::new();
        for m in install.modules() {
            let mut names = Names::new();
            for id in install.module_ids(m).unwrap_or_default() {
                add_id(&mut names, &id);
                add_id(&mut anywhere, &id);
            }
            modules.insert(m.as_str().to_string(), names);
        }
        Universe {
            global,
            modules,
            anywhere,
            module_names,
        }
    }

    /// Whether `scope` can load what `name` names; `name` is lowercase.
    pub(crate) fn resolves(&self, name: &str, want: Want, scope: &Scope) -> bool {
        match scope {
            Scope::Anywhere => holds(&self.anywhere, name, want, &self.module_names),
            Scope::Module(m) => {
                holds(&self.global, name, want, &self.module_names)
                    || self
                        .modules
                        .get(m)
                        .is_some_and(|n| holds(n, name, want, &self.module_names))
            }
        }
    }

    /// The scope of a file at `rel`, a path relative to the tree: a module
    /// capsule's members load in that module, everything else anywhere.
    pub(crate) fn scope_of(&self, rel: &str) -> Scope {
        let lower = rel.to_ascii_lowercase();
        if let Some((dir, file)) = lower.rsplit_once('/')
            && (dir == "modules" || (dir == "lips" && file.ends_with("_loc.mod")))
        {
            let m = ModuleName::new(file);
            if self.modules.contains_key(m.as_str()) {
                return Scope::Module(m.as_str().to_string());
            }
        }
        Scope::Anywhere
    }
}
