//! An area's room layout and room visibility (`rakata::lyt`), each read
//! from bytes, a path or the install and written back as bytes.

use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rakata::lyt::{DoorHook, Lyt, Placement, Vis};

use crate::fail;
use crate::game::{PyInstall, load, read_file};

type V3 = (f32, f32, f32);

fn v3([x, y, z]: [f32; 3]) -> V3 {
    (x, y, z)
}

/// A room, swoop track or obstacle: its model and where it stands.
#[pyclass(name = "Placement", module = "rakata", frozen, get_all)]
pub struct PyPlacement {
    model: String,
    /// X, Y, Z in metres, Z up.
    position: V3,
}

impl From<&Placement> for PyPlacement {
    fn from(p: &Placement) -> Self {
        PyPlacement {
            model: p.model.clone(),
            position: v3(p.position),
        }
    }
}

#[pymethods]
impl PyPlacement {
    fn __repr__(&self) -> String {
        format!("Placement({:?} at {:?})", self.model, self.position)
    }
}

/// A door hook: a named point of a room where a door stands.
#[pyclass(name = "DoorHook", module = "rakata", frozen, get_all)]
pub struct PyDoorHook {
    room: String,
    name: String,
    /// The integer after the name, kept as read.
    unknown: i32,
    position: V3,
    /// W, X, Y, Z.
    orientation: (f32, f32, f32, f32),
}

impl From<&DoorHook> for PyDoorHook {
    fn from(d: &DoorHook) -> Self {
        let [w, x, y, z] = d.orientation.to_array();
        PyDoorHook {
            room: d.room.clone(),
            name: d.name.clone(),
            unknown: d.unknown,
            position: v3(d.position),
            orientation: (w, x, y, z),
        }
    }
}

#[pymethods]
impl PyDoorHook {
    fn __repr__(&self) -> String {
        format!("DoorHook({:?} in {:?})", self.name, self.room)
    }
}

fn placements(ps: &[Placement]) -> Vec<PyPlacement> {
    ps.iter().map(PyPlacement::from).collect()
}

/// An area's room layout: its rooms' models and positions, swoop tracks,
/// obstacles and door hooks.
#[pyclass(name = "Lyt", module = "rakata")]
#[derive(Clone)]
pub struct PyLyt {
    inner: Lyt,
}

#[pymethods]
impl PyLyt {
    /// Read a layout. Raises `RE2301` and the format's other codes.
    #[staticmethod]
    fn parse(data: &[u8]) -> PyResult<Self> {
        Ok(PyLyt {
            inner: Lyt::parse(data).map_err(fail)?,
        })
    }

    #[staticmethod]
    fn read(path: PathBuf) -> PyResult<Self> {
        Self::parse(&read_file(&path)?)
    }

    /// The `.lyt` the install loads by this name (`"m01aa"`, the area's
    /// name), in a module's capsules too when `module` names one. Raises
    /// `RE0404`.
    #[staticmethod]
    #[pyo3(signature = (install, name, module = None))]
    fn load(install: &PyInstall, name: &str, module: Option<&str>) -> PyResult<Self> {
        Self::parse(&load(&install.inner, name, Some("lyt"), module)?)
    }

    /// The `filedependancy` line's value, as the file spells it.
    #[getter]
    fn file_dependency(&self) -> Option<String> {
        self.inner.file_dependency.clone()
    }

    #[getter]
    fn rooms(&self) -> Vec<PyPlacement> {
        placements(&self.inner.rooms)
    }

    #[getter]
    fn tracks(&self) -> Vec<PyPlacement> {
        placements(&self.inner.tracks)
    }

    #[getter]
    fn obstacles(&self) -> Vec<PyPlacement> {
        placements(&self.inner.obstacles)
    }

    #[getter]
    fn door_hooks(&self) -> Vec<PyDoorHook> {
        self.inner.door_hooks.iter().map(PyDoorHook::from).collect()
    }

    /// The room whose model is `model`, in any case.
    fn room(&self, model: &str) -> Option<PyPlacement> {
        self.inner.room(model).map(PyPlacement::from)
    }

    /// Whether the two hold the same dependency, rooms, tracks, obstacles
    /// and door hooks, floats by their bits; how the file spelled them
    /// aside.
    fn semantic_eq(&self, other: &PyLyt) -> bool {
        self.inner.semantic_eq(&other.inner)
    }

    /// The text; a layout read and not changed writes back as it was read.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn __repr__(&self) -> String {
        format!("Lyt({} rooms)", self.inner.rooms.len())
    }
}

/// An area's room visibility: for each room, the rooms drawn while the
/// camera is in it.
#[pyclass(name = "Vis", module = "rakata")]
#[derive(Clone)]
pub struct PyVis {
    inner: Vis,
}

#[pymethods]
impl PyVis {
    /// Read a visibility file. Raises `RE2311` and `RE2312`.
    #[staticmethod]
    fn parse(data: &[u8]) -> PyResult<Self> {
        Ok(PyVis {
            inner: Vis::parse(data).map_err(fail)?,
        })
    }

    #[staticmethod]
    fn read(path: PathBuf) -> PyResult<Self> {
        Self::parse(&read_file(&path)?)
    }

    /// The `.vis` the install loads by this name. Raises `RE0404`.
    #[staticmethod]
    #[pyo3(signature = (install, name, module = None))]
    fn load(install: &PyInstall, name: &str, module: Option<&str>) -> PyResult<Self> {
        Self::parse(&load(&install.inner, name, Some("vis"), module)?)
    }

    /// Every room as `(room, visible rooms)`, in file order.
    #[getter]
    fn rooms(&self) -> Vec<(String, Vec<String>)> {
        self.inner
            .rooms
            .iter()
            .map(|r| (r.room.clone(), r.visible.clone()))
            .collect()
    }

    /// The rooms visible from `room`, in any case, or `None` when the file
    /// does not list it.
    fn visible(&self, room: &str) -> Option<Vec<String>> {
        self.inner.visible(room).map(<[String]>::to_vec)
    }

    fn semantic_eq(&self, other: &PyVis) -> bool {
        self.inner.semantic_eq(&other.inner)
    }

    /// The text; a file read and not changed writes back as it was read.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn __len__(&self) -> usize {
        self.inner.rooms.len()
    }

    fn __repr__(&self) -> String {
        format!("Vis({} rooms)", self.inner.rooms.len())
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLyt>()?;
    m.add_class::<PyVis>()?;
    m.add_class::<PyPlacement>()?;
    m.add_class::<PyDoorHook>()?;
    Ok(())
}
