//! `rakata::game`: the install and which copy of a resource it loads.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rakata::game::{
    Candidate, Install, Location, ModuleName, Profile, Resolution, TexturePack, Verdict,
};
use rakata::res::ResId;

use crate::fail;

pub(crate) fn profile(name: &str) -> PyResult<Profile> {
    match name {
        "k1" => Ok(Profile::K1),
        "k2" => Ok(Profile::K2),
        other => Err(PyValueError::new_err(format!(
            "profile is \"k1\" or \"k2\", not {other:?}"
        ))),
    }
}

fn profile_name(p: Profile) -> &'static str {
    match p {
        Profile::K1 => "k1",
        Profile::K2 => "k2",
    }
}

/// A resource id from `NAME.EXT`, or from `NAME` and `EXT` apart.
pub(crate) fn res_id(name: &str, ext: Option<&str>) -> PyResult<ResId> {
    let text = match ext {
        Some(e) => format!("{name}.{}", e.trim_start_matches('.')),
        None => name.to_string(),
    };
    ResId::parse(&text).map_err(fail)
}

/// The bytes of the copy of `name` the install loads, in `module`'s
/// capsules too when it names one; `ext` is the extension `name` takes when
/// it has none of its own. Raises `RE0404` when no layer holds it.
pub(crate) fn load(
    install: &Install,
    name: &str,
    ext: Option<&str>,
    module: Option<&str>,
) -> PyResult<Vec<u8>> {
    let id = match ext {
        Some(e) if !name.contains('.') => res_id(name, Some(e))?,
        _ => res_id(name, None)?,
    };
    let module = module.map(ModuleName::new);
    match install.read(id, module.as_ref()).map_err(fail)? {
        Some((_, bytes)) => Ok(bytes),
        None => Err(fail(rakata::game::Error::NotFound {
            id,
            module: module.as_ref().map(ToString::to_string),
        })),
    }
}

/// A file's bytes; one that cannot be read raises `RE0001`.
pub(crate) fn read_file(path: &Path) -> PyResult<Vec<u8>> {
    std::fs::read(path)
        .map_err(|e| crate::raise("RE0001", format!("cannot read `{}`: {e}", path.display())))
}

/// A game install: the folder holding `chitin.key`, opened once.
#[pyclass(name = "Install", module = "rakata", frozen)]
pub struct PyInstall {
    pub(crate) inner: Arc<Install>,
}

#[pymethods]
impl PyInstall {
    /// Open the install at `path`, its texture layer searching the quality
    /// pack `texture_pack` names (`"tpa"`, `"tpb"` or `"tpc"`; `"tpa"` when
    /// none is named) over `swpc_tex_gui.erf`. Raises `RE0401` without a
    /// `chitin.key`, `RE0402` when a BIF it names is missing, and `RE0405`
    /// for a pack that is none of the three.
    #[new]
    #[pyo3(signature = (path, profile = "k1", texture_pack = None))]
    fn new(path: PathBuf, profile: &str, texture_pack: Option<&str>) -> PyResult<Self> {
        let profile = self::profile(profile)?;
        let pack = match texture_pack {
            Some(name) => name.parse::<TexturePack>().map_err(fail)?,
            None => profile.default_texture_pack(),
        };
        let install = Install::open(&path, profile)
            .map_err(fail)?
            .with_texture_pack(pack);
        Ok(PyInstall {
            inner: Arc::new(install),
        })
    }

    /// The install's folder.
    #[getter]
    fn root(&self) -> String {
        self.inner.root().display().to_string()
    }

    /// `"k1"` or `"k2"`.
    #[getter]
    fn profile(&self) -> &'static str {
        profile_name(self.inner.profile())
    }

    /// The quality pack the texture layer searches: `"tpa"`, `"tpb"` or
    /// `"tpc"`.
    #[getter]
    fn texture_pack(&self) -> &'static str {
        self.inner.texture_pack().name()
    }

    /// The modules under `modules/`, by capsule stem.
    fn modules(&self) -> Vec<String> {
        self.inner
            .modules()
            .iter()
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Which copy of a resource the engine loads, and every other copy the
    /// install holds: `which("globalcat", "2da")` or `which("globalcat.2da")`,
    /// in a module's capsules too when `module` names one. Raises `RE0404`
    /// when no layer holds it, as `rakata which` reports.
    #[pyo3(signature = (name, ext = None, module = None))]
    fn which(&self, name: &str, ext: Option<&str>, module: Option<&str>) -> PyResult<PyResolution> {
        let id = res_id(name, ext)?;
        let module = module.map(ModuleName::new);
        let r = self.inner.resolve(id, module.as_ref()).map_err(fail)?;
        if let Some(e) = r.not_found() {
            return Err(fail(e));
        }
        Ok(PyResolution::from(&r))
    }

    /// The bytes of the copy the engine loads: `read("m01aa", "lyt")` or
    /// `read("m01aa.lyt")`, in a module's capsules too when `module` names
    /// one. Raises `RE0404` when no layer holds it.
    #[pyo3(signature = (name, ext = None, module = None))]
    fn read<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        ext: Option<&str>,
        module: Option<&str>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let id = res_id(name, ext)?;
        let b = load(&self.inner, &id.to_string(), None, module)?;
        Ok(PyBytes::new(py, &b))
    }

    fn __repr__(&self) -> String {
        format!("Install({:?})", self.inner.root().display().to_string())
    }
}

/// One copy of a resource the install holds.
#[pyclass(name = "Candidate", module = "rakata", frozen, get_all)]
#[derive(Clone)]
pub struct PyCandidate {
    /// `"override"`, `"module"`, `"patch"`, `"texturepack"` or `"chitin"`.
    layer: &'static str,
    /// The file, with `:member` for a capsule's or `[index]` for a BIF's.
    location: String,
    /// The file the bytes are read from.
    path: String,
    /// The member's name in a capsule.
    member: Option<String>,
    /// The member's position in a BIF.
    index: Option<u32>,
    len: usize,
    /// The SHA-256 of the bytes, in hex.
    hash: String,
    /// `"wins"`, `"ambiguous"` or `"shadowed"`.
    verdict: &'static str,
    /// For a shadowed copy, the precedence rule that outranks it.
    reason: Option<String>,
}

impl From<&Candidate> for PyCandidate {
    fn from(c: &Candidate) -> Self {
        let (member, index) = match &c.location {
            Location::File { .. } => (None, None),
            Location::Capsule { member, .. } => (Some(member.to_string()), None),
            Location::Bif { index, .. } => (None, Some(*index)),
        };
        let (verdict, reason) = match &c.verdict {
            Verdict::Wins => ("wins", None),
            Verdict::Ambiguous => ("ambiguous", None),
            Verdict::Shadowed { reason, .. } => ("shadowed", Some(reason.clone())),
        };
        PyCandidate {
            layer: c.layer.name(),
            location: c.location.to_string(),
            path: c.location.path().display().to_string(),
            member,
            index,
            len: c.len,
            hash: c.hash.to_hex(),
            verdict,
            reason,
        }
    }
}

#[pymethods]
impl PyCandidate {
    fn __repr__(&self) -> String {
        format!(
            "Candidate({} {} {})",
            self.verdict, self.location, self.hash
        )
    }
}

/// Which copy of a resource the engine loads: every copy, the winner first.
#[pyclass(name = "Resolution", module = "rakata", frozen, get_all)]
pub struct PyResolution {
    /// `NAME.EXT`.
    id: String,
    module: Option<String>,
    candidates: Vec<PyCandidate>,
    /// Whether the winner has a case variant beside it in `Override/`.
    ambiguous: bool,
}

impl From<&Resolution> for PyResolution {
    fn from(r: &Resolution) -> Self {
        PyResolution {
            id: r.id.to_string(),
            module: r.module.as_ref().map(ToString::to_string),
            candidates: r.candidates.iter().map(PyCandidate::from).collect(),
            ambiguous: r.is_ambiguous(),
        }
    }
}

impl PyResolution {
    fn first(&self) -> &PyCandidate {
        // `which` raises rather than returning a resolution without a copy.
        &self.candidates[0]
    }
}

#[pymethods]
impl PyResolution {
    /// The copy the engine loads.
    #[getter]
    fn winner(&self) -> PyCandidate {
        self.first().clone()
    }

    /// The winner's SHA-256, in hex: `rakata which`'s `winner.sha256`.
    #[getter]
    fn hash(&self) -> String {
        self.first().hash.clone()
    }

    fn __repr__(&self) -> String {
        format!("Resolution({} -> {})", self.id, self.first().location)
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyInstall>()?;
    m.add_class::<PyResolution>()?;
    m.add_class::<PyCandidate>()?;
    Ok(())
}
