//! `Install`: one model of the install, and the one answer to which copy of
//! a resource the engine loads.
//!
//! The search order is the engine's as the public documents and the
//! proposal state it, and as the Juhani mod's `loaded()` measured it over a
//! modded build: `Override/` first; then the module's capsules, `STEM.mod`
//! over `STEM.rim` over `STEM_s.rim` (KotOR 2 adds `STEM_dlg.erf`) and the
//! lip-sync pack `lips/STEM_loc.mod`; then KotOR 1's `patch.erf`; then the
//! texture packs, the selected quality pack over `swpc_tex_gui.erf`; then
//! the BIFs `chitin.key` indexes. Every copy the install holds is a
//! [`Candidate`], with where it lies, its content hash and why it lost.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rakata_archive::Key;
use rakata_res::{ContentHash, ResId, ResType};

use crate::archive::{BifFile, Capsule};
use crate::listing::Listing;
use crate::texture_pack::{self, TexturePack};
use crate::{Error, Profile, Result};

/// A module's name: the stem its capsules share, `danm14ac` for
/// `danm14ac.mod`, `danm14ac.rim` and `danm14ac_s.rim`. Compared without
/// regard to case.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleName(String);

impl ModuleName {
    /// A module name as the user or a capsule file spells it; the case is
    /// folded. A capsule's file name is accepted and its suffix dropped.
    pub fn new(name: &str) -> Self {
        let lower = name.to_ascii_lowercase();
        let stem = ["_loc.mod", "_dlg.erf", "_s.rim", ".mod", ".rim"]
            .iter()
            .find_map(|s| lower.strip_suffix(s))
            .unwrap_or(&lower);
        Self(stem.to_owned())
    }

    /// The name, lowercase.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The layer of the install a candidate came from, in the engine's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Layer {
    /// A loose file under `Override/`.
    Override,
    /// A member of one of the module's capsules.
    Module,
    /// A member of `patch.erf`.
    Patch,
    /// A member of a texture pack under `TexturePacks/`: the selected
    /// quality pack, then `swpc_tex_gui.erf`.
    TexturePack,
    /// A member of a BIF, through `chitin.key`.
    Chitin,
}

impl Layer {
    /// The layer's name, as the JSON output spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Layer::Override => "override",
            Layer::Module => "module",
            Layer::Patch => "patch",
            Layer::TexturePack => "texturepack",
            Layer::Chitin => "chitin",
        }
    }
}

/// Where a copy of a resource lies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// A loose file.
    File {
        /// Its path.
        path: PathBuf,
    },
    /// A member of a capsule: a module's `.mod`, `.rim` or `_s.rim`, a
    /// lip-sync pack, `patch.erf`, or a texture pack.
    Capsule {
        /// The capsule.
        path: PathBuf,
        /// The member's name and type as looked up; the capsule's own
        /// spelling may differ in case.
        member: ResId,
    },
    /// A member of a BIF.
    Bif {
        /// The BIF.
        path: PathBuf,
        /// The KEY that names it.
        key: PathBuf,
        /// The member's position in the BIF's table.
        index: u32,
    },
}

impl Location {
    /// The file the bytes are read from.
    pub fn path(&self) -> &Path {
        match self {
            Location::File { path }
            | Location::Capsule { path, .. }
            | Location::Bif { path, .. } => path,
        }
    }

    /// A short name for a message: the file's name, with the member or
    /// index where there is one.
    pub fn short(&self) -> String {
        let name = |p: &Path| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };
        match self {
            Location::File { path } => name(path),
            Location::Capsule { path, member } => format!("{}:{member}", name(path)),
            Location::Bif { path, index, .. } => format!("{}[{index}]", name(path)),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::File { path } => write!(f, "{}", path.display()),
            Location::Capsule { path, member } => write!(f, "{}:{member}", path.display()),
            Location::Bif { path, index, .. } => write!(f, "{}[{index}]", path.display()),
        }
    }
}

/// How a candidate fared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The copy the engine loads.
    Wins,
    /// A case variant of the winner in the same directory: the engine's
    /// choice between them is not established, so the winner is marked
    /// ambiguous and this one is its rival.
    Ambiguous,
    /// Outranked by the candidate at `by` in the resolution's list, for
    /// the reason given.
    Shadowed {
        /// The index of the candidate that outranks this one.
        by: usize,
        /// The precedence rule, as a sentence fragment.
        reason: String,
    },
}

/// One copy of a resource the install holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The layer it lies in.
    pub layer: Layer,
    /// Where it lies.
    pub location: Location,
    /// Its length in bytes.
    pub len: usize,
    /// The SHA-256 of its bytes.
    pub hash: ContentHash,
    /// Whether it wins, and if not why.
    pub verdict: Verdict,
}

/// The answer to which copy of a resource the engine loads: every copy the
/// install holds, in the engine's order, the first the winner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The resource asked for.
    pub id: ResId,
    /// The module asked in, if any.
    pub module: Option<ModuleName>,
    /// Every copy, the winner first; empty when the install holds none.
    pub candidates: Vec<Candidate>,
}

impl Resolution {
    /// The copy the engine loads, if the install holds one.
    pub fn winner(&self) -> Option<&Candidate> {
        self.candidates.first()
    }

    /// Whether the winner has a case variant beside it.
    pub fn is_ambiguous(&self) -> bool {
        self.candidates
            .iter()
            .any(|c| c.verdict == Verdict::Ambiguous)
    }

    /// The winner and its case variants, when the resolution is ambiguous.
    pub fn case_variants(&self) -> Vec<&Candidate> {
        if !self.is_ambiguous() {
            return Vec::new();
        }
        self.candidates
            .iter()
            .filter(|c| matches!(c.verdict, Verdict::Wins | Verdict::Ambiguous))
            .collect()
    }

    /// The error a caller reports when there is no winner.
    pub fn not_found(&self) -> Option<Error> {
        self.candidates.is_empty().then(|| Error::NotFound {
            id: self.id,
            module: self.module.as_ref().map(ToString::to_string),
        })
    }
}

/// A module's capsules, in the engine's precedence.
#[derive(Debug)]
struct Module {
    capsules: Vec<Capsule>,
}

/// A game install: the folder holding `chitin.key`, opened once.
pub struct Install {
    root: PathBuf,
    profile: Profile,
    key_path: PathBuf,
    key: Key,
    bifs: Vec<Option<BifFile>>,
    override_dir: Option<Listing>,
    modules_dir: Option<Listing>,
    modules: HashMap<ModuleName, Module>,
    module_names: Vec<ModuleName>,
    patch: Option<Capsule>,
    texture_packs_dir: Option<Listing>,
    texture_pack: TexturePack,
    /// The selected quality pack, then the GUI pack, those present.
    texture_packs: Vec<Capsule>,
    pub(crate) inputs: Vec<nwscript::InputRecord>,
    pub(crate) resolver_module: Option<ModuleName>,
}

impl fmt::Debug for Install {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Install")
            .field("root", &self.root)
            .field("profile", &self.profile)
            .field("modules", &self.module_names.len())
            .finish_non_exhaustive()
    }
}

impl Install {
    /// Open the install at `dir`, the folder holding `chitin.key`. Reads the
    /// KEY and lists `Override/`, `modules/`, `lips/` and `TexturePacks/`;
    /// every archive is opened on first use. The texture layer searches the
    /// profile's [default pack](Profile::default_texture_pack);
    /// [`Install::with_texture_pack`] selects another. Fails with
    /// [`Error::NotAnInstall`] when there is no `chitin.key`, and
    /// [`Error::MissingBif`] when the KEY names a BIF the install lacks.
    pub fn open(dir: impl AsRef<Path>, profile: Profile) -> Result<Self> {
        let root = dir.as_ref().to_path_buf();
        let listing = Listing::read(&root).map_err(|source| Error::Io {
            path: root.clone(),
            source,
        })?;
        let key_path = listing
            .files("chitin.key")
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotAnInstall { dir: root.clone() })?;
        let key_bytes = fs::read(&key_path).map_err(|source| Error::Io {
            path: key_path.clone(),
            source,
        })?;
        let key = Key::parse(&key_bytes).map_err(|source| Error::Archive {
            path: key_path.clone(),
            source,
        })?;

        let mut listings: HashMap<PathBuf, Listing> = HashMap::new();
        listings.insert(root.clone(), listing);
        let mut bifs = Vec::with_capacity(key.bifs().len());
        for bif in key.bifs() {
            let mut dir = root.clone();
            let mut found = None;
            let parts: Vec<String> = bif.components().collect();
            for (i, part) in parts.iter().enumerate() {
                let here = match listings.get(&dir) {
                    Some(l) => l,
                    None => {
                        let l = Listing::read(&dir).map_err(|source| Error::Io {
                            path: dir.clone(),
                            source,
                        })?;
                        listings.entry(dir.clone()).or_insert(l)
                    }
                };
                if i + 1 == parts.len() {
                    found = here.files(part).into_iter().next();
                } else {
                    match here.dir(part) {
                        Some(d) => dir = d,
                        None => break,
                    }
                }
            }
            match found {
                Some(path) => bifs.push(Some(BifFile::new(path))),
                None => {
                    // The KEY names BIFs no KotOR install ships
                    // (`_newbif.bif`'s neighbours differ by platform); a
                    // missing one fails only when a resource needs it.
                    bifs.push(None);
                }
            }
        }

        let root_listing = &listings[&root];
        let dir_listing = |name: &str| -> Result<Option<Listing>> {
            match root_listing.dir(name) {
                Some(path) => Listing::read(&path)
                    .map(Some)
                    .map_err(|source| Error::Io { path, source }),
                None => Ok(None),
            }
        };
        let override_dir = dir_listing("Override")?;
        let modules_dir = dir_listing("modules")?;
        let lips_dir = dir_listing("lips")?;
        let texture_packs_dir = dir_listing(texture_pack::DIR)?;
        let patch = profile
            .has_patch_erf()
            .then(|| root_listing.files("patch.erf").into_iter().next())
            .flatten()
            .map(Capsule::new);

        let mut modules: HashMap<ModuleName, Module> = HashMap::new();
        if let Some(dir) = &modules_dir {
            let mut stems: Vec<ModuleName> = dir
                .file_names()
                .map(|n| n.to_string_lossy())
                .filter(|n| {
                    profile
                        .module_suffixes()
                        .iter()
                        .any(|s| n.len() > s.len() && n.to_ascii_lowercase().ends_with(s))
                })
                .map(|n| ModuleName::new(&n))
                .collect();
            stems.sort();
            stems.dedup();
            for stem in stems {
                let module = Module {
                    capsules: module_capsules(profile, &stem, dir, lips_dir.as_ref()),
                };
                modules.insert(stem, module);
            }
        }
        let mut module_names: Vec<ModuleName> = modules.keys().cloned().collect();
        module_names.sort();

        Ok(Self {
            root,
            profile,
            key_path,
            key,
            bifs,
            override_dir,
            modules_dir,
            modules,
            module_names,
            patch,
            texture_packs: texture_capsules(
                texture_packs_dir.as_ref(),
                profile.default_texture_pack(),
            ),
            texture_packs_dir,
            texture_pack: profile.default_texture_pack(),
            inputs: Vec::new(),
            resolver_module: None,
        })
    }

    /// The install with `pack` as the quality pack its texture layer
    /// searches, over `swpc_tex_gui.erf`.
    pub fn with_texture_pack(mut self, pack: TexturePack) -> Self {
        if pack != self.texture_pack {
            self.texture_pack = pack;
            self.texture_packs = texture_capsules(self.texture_packs_dir.as_ref(), pack);
        }
        self
    }

    /// The quality pack the texture layer searches.
    pub fn texture_pack(&self) -> TexturePack {
        self.texture_pack
    }

    /// The texture packs the layer searches, in order, by path: the
    /// selected quality pack and `swpc_tex_gui.erf`, those the install
    /// holds.
    pub fn texture_pack_paths(&self) -> Vec<&Path> {
        self.texture_packs.iter().map(Capsule::path).collect()
    }

    /// Where the selected quality pack would lie, when `TexturePacks/`
    /// exists and does not hold it (`RW0402`): the texture layer then holds
    /// only the GUI pack.
    pub fn missing_texture_pack(&self) -> Option<PathBuf> {
        let dir = self.texture_packs_dir.as_ref()?;
        let name = self.texture_pack.file_name();
        dir.files(name).is_empty().then(|| dir.path().join(name))
    }

    /// Every member of the texture packs the layer searches, in search
    /// order then table order, without duplicates.
    pub fn texture_pack_ids(&self) -> Result<Vec<ResId>> {
        let mut out = Vec::new();
        for capsule in &self.texture_packs {
            out.extend(capsule.ids()?);
        }
        let mut seen = std::collections::HashSet::new();
        out.retain(|id| seen.insert(*id));
        Ok(out)
    }

    /// The folder the install was opened at.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The game.
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// The KEY, as read.
    pub fn key(&self) -> &Key {
        &self.key
    }

    /// Every module with a capsule under `modules/`, sorted.
    pub fn modules(&self) -> &[ModuleName] {
        &self.module_names
    }

    /// The module's capsules in the engine's precedence, by path.
    pub fn module_capsules(&self, module: &ModuleName) -> Result<Vec<&Path>> {
        Ok(self
            .module(module)?
            .capsules
            .iter()
            .map(Capsule::path)
            .collect())
    }

    /// Every member of every capsule of the module, in capsule precedence
    /// then table order, without duplicates.
    pub fn module_ids(&self, module: &ModuleName) -> Result<Vec<ResId>> {
        let mut out = Vec::new();
        for capsule in &self.module(module)?.capsules {
            out.extend(capsule.ids()?);
        }
        let mut seen = std::collections::HashSet::new();
        out.retain(|id| seen.insert(*id));
        Ok(out)
    }

    /// Every resource under `Override/` whose extension names a type, one
    /// per name and type, sorted.
    pub fn override_ids(&self) -> Vec<ResId> {
        let Some(dir) = &self.override_dir else {
            return Vec::new();
        };
        let mut ids: Vec<ResId> = dir
            .file_names()
            .filter_map(|n| ResId::parse(&n.to_string_lossy()).ok())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// The groups of `Override/` files whose names differ only in case,
    /// each sorted by name, the groups sorted by their folded name. Which
    /// of a group the engine opens is not established; `rakata which`
    /// reports the resolution ambiguous and names them all.
    pub fn override_case_variants(&self) -> Vec<Vec<PathBuf>> {
        self.override_dir
            .as_ref()
            .map(Listing::case_variants)
            .unwrap_or_default()
    }

    fn module(&self, module: &ModuleName) -> Result<&Module> {
        self.modules.get(module).ok_or_else(|| Error::NoSuchModule {
            module: module.to_string(),
            dir: self.modules_dir.as_ref().map(|l| l.path().to_path_buf()),
        })
    }

    /// Every copy of `id` the install holds, in the engine's order, with
    /// the winner first and each loser's reason; the module's capsules are
    /// searched when `module` names one. Fails when the module is unknown
    /// or an archive that holds a copy cannot be read.
    pub fn resolve(&self, id: ResId, module: Option<&ModuleName>) -> Result<Resolution> {
        let mut candidates = Vec::new();
        self.search(id, module, &mut |layer, location, bytes| {
            candidates.push(Candidate {
                layer,
                location,
                len: bytes.len(),
                hash: ContentHash::of(bytes),
                verdict: Verdict::Wins,
            });
            Ok(true)
        })?;
        for i in 1..candidates.len() {
            let (winner, rest) = candidates.split_at_mut(1);
            let winner = &winner[0];
            let loser = &mut rest[i - 1];
            loser.verdict = if loser.layer == Layer::Override && winner.layer == Layer::Override {
                Verdict::Ambiguous
            } else {
                Verdict::Shadowed {
                    by: 0,
                    reason: precedence(winner, loser),
                }
            };
        }
        Ok(Resolution {
            id,
            module: module.cloned(),
            candidates,
        })
    }

    /// The bytes of the copy the engine loads, if the install holds one:
    /// the search stops at the first hit.
    pub fn read(
        &self,
        id: ResId,
        module: Option<&ModuleName>,
    ) -> Result<Option<(Location, Vec<u8>)>> {
        let mut found = None;
        self.search(id, module, &mut |_, location, bytes| {
            found = Some((location, bytes.to_vec()));
            Ok(false)
        })?;
        Ok(found)
    }

    /// Walk the layers in order, offering each copy to `visit`, which says
    /// whether to go on.
    fn search(&self, id: ResId, module: Option<&ModuleName>, visit: &mut Visit<'_>) -> Result<()> {
        let module = module.map(|m| self.module(m)).transpose()?;
        if let Some(dir) = &self.override_dir {
            let name = loose_name(id);
            for path in dir.files(&name) {
                let bytes = fs::read(&path).map_err(|source| Error::Io {
                    path: path.clone(),
                    source,
                })?;
                if !visit(Layer::Override, Location::File { path }, &bytes)? {
                    return Ok(());
                }
            }
        }
        if let Some(module) = module {
            for capsule in &module.capsules {
                if let Some(bytes) = capsule.get(id)? {
                    let location = Location::Capsule {
                        path: capsule.path().to_path_buf(),
                        member: id,
                    };
                    if !visit(Layer::Module, location, bytes)? {
                        return Ok(());
                    }
                }
            }
        }
        if let Some(patch) = &self.patch
            && let Some(bytes) = patch.get(id)?
        {
            let location = Location::Capsule {
                path: patch.path().to_path_buf(),
                member: id,
            };
            if !visit(Layer::Patch, location, bytes)? {
                return Ok(());
            }
        }
        for pack in &self.texture_packs {
            if let Some(bytes) = pack.get(id)? {
                let location = Location::Capsule {
                    path: pack.path().to_path_buf(),
                    member: id,
                };
                if !visit(Layer::TexturePack, location, bytes)? {
                    return Ok(());
                }
            }
        }
        if let Some(entry) = self.key.get(id) {
            let bif = self
                .bifs
                .get(entry.bif as usize)
                .and_then(Option::as_ref)
                .ok_or_else(|| Error::MissingBif {
                    name: self
                        .key
                        .bifs()
                        .get(entry.bif as usize)
                        .map(|b| b.name_str())
                        .unwrap_or_else(|| format!("BIF #{}", entry.bif)),
                    key: self.key_path.clone(),
                })?;
            if let Some(bytes) = bif.get(entry.index as usize)? {
                let location = Location::Bif {
                    path: bif.path().to_path_buf(),
                    key: self.key_path.clone(),
                    index: entry.index,
                };
                visit(Layer::Chitin, location, bytes)?;
            }
        }
        Ok(())
    }
}

/// A visitor over the copies `Install::search` finds: the layer, where the
/// copy lies and its bytes; `Ok(true)` to go on, `Ok(false)` to stop.
type Visit<'a> = dyn FnMut(Layer, Location, &[u8]) -> Result<bool> + 'a;

/// `name.ext` as a loose file spells it.
fn loose_name(id: ResId) -> String {
    match id.ty {
        ResType::Unknown(n) => format!("{}.type{n}", id.name),
        ty => format!("{}.{}", id.name, ty.extension().unwrap_or_default()),
    }
}

/// The module's capsules in the engine's precedence: each suffix of the
/// profile under `modules/`, every case variant of each, then the lip-sync
/// pack under `lips/`.
fn module_capsules(
    profile: Profile,
    stem: &ModuleName,
    modules: &Listing,
    lips: Option<&Listing>,
) -> Vec<Capsule> {
    let mut out = Vec::new();
    for suffix in profile.module_suffixes() {
        for path in modules.files(&format!("{stem}{suffix}")) {
            out.push(Capsule::new(path));
        }
    }
    if let Some(lips) = lips {
        for path in lips.files(&format!("{stem}{}", profile.lips_suffix())) {
            out.push(Capsule::new(path));
        }
    }
    out
}

/// The texture packs the layer searches under `TexturePacks/`: `pack`, then
/// `swpc_tex_gui.erf`, each in every case variant the folder holds.
fn texture_capsules(dir: Option<&Listing>, pack: TexturePack) -> Vec<Capsule> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    [pack.file_name(), texture_pack::GUI]
        .into_iter()
        .flat_map(|name| dir.files(name))
        .map(Capsule::new)
        .collect()
}

/// Why `winner` outranks `loser`, as a sentence fragment.
fn precedence(winner: &Candidate, loser: &Candidate) -> String {
    match (winner.layer, loser.layer) {
        (Layer::Override, Layer::Module) => "Override outranks the module's capsules".into(),
        (Layer::Override, Layer::Patch) => "Override outranks patch.erf".into(),
        (Layer::Override, Layer::TexturePack) => "Override outranks the texture packs".into(),
        (Layer::Override, Layer::Chitin) => "Override outranks the BIFs".into(),
        (Layer::Module, Layer::Module) => format!(
            "{} outranks {}",
            file_name(&winner.location),
            file_name(&loser.location)
        ),
        (Layer::Module, Layer::Patch) => "the module's capsules outrank patch.erf".into(),
        (Layer::Module, Layer::TexturePack) => {
            "the module's capsules outrank the texture packs".into()
        }
        (Layer::Module, Layer::Chitin) => "the module's capsules outrank the BIFs".into(),
        (Layer::Patch, Layer::TexturePack) => "patch.erf outranks the texture packs".into(),
        (Layer::Patch, Layer::Chitin) => "patch.erf outranks the BIFs".into(),
        (Layer::TexturePack, Layer::TexturePack) => format!(
            "{} outranks {}",
            file_name(&winner.location),
            file_name(&loser.location)
        ),
        (Layer::TexturePack, Layer::Chitin) => "the texture packs outrank the BIFs".into(),
        (w, l) => format!("{} outranks {}", w.name(), l.name()),
    }
}

fn file_name(location: &Location) -> String {
    location
        .path()
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_name_folds_case_and_drops_a_capsule_suffix() {
        assert_eq!(ModuleName::new("DANM14AC").as_str(), "danm14ac");
        assert_eq!(ModuleName::new("danm14ac_s.rim").as_str(), "danm14ac");
        assert_eq!(ModuleName::new("danm14ac.mod").as_str(), "danm14ac");
        assert_eq!(ModuleName::new("danm14ac_loc.mod").as_str(), "danm14ac");
        assert_eq!(ModuleName::new("unk_m44ac").as_str(), "unk_m44ac");
    }

    #[test]
    fn a_loose_name_is_name_dot_extension() {
        let id = ResId::parse("Appearance.2DA").unwrap();
        assert_eq!(loose_name(id), "Appearance.2da");
    }
}
