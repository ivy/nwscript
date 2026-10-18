//! Walkmeshes (`rakata::walkmesh`): an area room's WOK, a placeable's PWK
//! and a door's DWK, read from bytes, a path or the install and written
//! back as bytes.
//!
//! The reader borrows the vertex array from its input, so the class owns
//! that array and the trailer apart from the rest, and lends them to a
//! `Walkmesh` again to write. Vertices are handed out as tuples, and whole
//! as `bytes` for a caller that reads them in bulk.

use std::path::PathBuf;

use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rakata::walkmesh::{EdgeRef, Kind, SurfaceMaterials, Vertices, Walkmesh};

use crate::fail;
use crate::formats::PyTwoDa;
use crate::game::{PyInstall, load, read_file};

type V3 = (f32, f32, f32);

fn v3([x, y, z]: [f32; 3]) -> V3 {
    (x, y, z)
}

fn edge(e: EdgeRef) -> (u32, u8) {
    (e.face, e.edge)
}

/// One triangle of a walkmesh.
#[pyclass(name = "WalkmeshFace", module = "rakata", frozen, get_all)]
pub struct PyWalkmeshFace {
    /// Indices into the vertices, counter-clockwise seen from the normal.
    vertices: (u32, u32, u32),
    /// The face's row of `surfacemat.2da`.
    material: u32,
    normal: V3,
    distance: f32,
}

#[pymethods]
impl PyWalkmeshFace {
    fn __repr__(&self) -> String {
        format!(
            "WalkmeshFace({:?}, material {})",
            self.vertices, self.material
        )
    }
}

/// A node of a walkmesh's AABB tree, root first, in pre-order.
#[pyclass(name = "AabbNode", module = "rakata", frozen, get_all)]
pub struct PyAabbNode {
    min: V3,
    max: V3,
    /// At a leaf, the face it holds.
    face: Option<u32>,
    unknown: u32,
    /// At an inner node, the direction from the left child's box to the
    /// right's: 1 +X, 2 +Y, 4 +Z, 8 -X, 16 -Y, 32 -Z; 0 at a leaf.
    plane: u32,
    left: Option<u32>,
    right: Option<u32>,
}

#[pymethods]
impl PyAabbNode {
    fn __repr__(&self) -> String {
        format!("AabbNode({:?}..{:?})", self.min, self.max)
    }
}

/// A walkmesh: a WOK, PWK or DWK.
#[pyclass(name = "Walkmesh", module = "rakata")]
#[derive(Clone)]
pub struct PyWalkmesh {
    /// Everything but the vertices and the trailer, which are empty here.
    parts: Walkmesh<'static>,
    vertices: Vec<u8>,
    trailer: Vec<u8>,
}

impl PyWalkmesh {
    fn new(data: &[u8]) -> PyResult<Self> {
        let w = Walkmesh::parse(data).map_err(fail)?;
        Ok(PyWalkmesh {
            vertices: w.vertices.as_bytes().to_vec(),
            trailer: w.trailer.to_vec(),
            parts: Walkmesh {
                kind: w.kind,
                relative_points: w.relative_points,
                absolute_points: w.absolute_points,
                position: w.position,
                vertices: Vertices::from_bytes(&[]).expect("no vertices"),
                faces: w.faces,
                adjacency: w.adjacency,
                perimeter_edges: w.perimeter_edges,
                perimeters: w.perimeters,
                aabb: w.aabb,
                unknown: w.unknown,
                trailer: &[],
            },
        })
    }

    /// The walkmesh whole, borrowing the vertices and trailer.
    fn walkmesh(&self) -> Walkmesh<'_> {
        Walkmesh {
            vertices: Vertices::from_bytes(&self.vertices).expect("twelve bytes a vertex"),
            trailer: &self.trailer,
            ..self.parts.clone()
        }
    }
}

#[pymethods]
impl PyWalkmesh {
    /// Read a WOK, PWK or DWK. Raises `RE2501` and the format's other codes.
    #[staticmethod]
    fn parse(data: &[u8]) -> PyResult<Self> {
        Self::new(data)
    }

    #[staticmethod]
    fn read(path: PathBuf) -> PyResult<Self> {
        Self::new(&read_file(&path)?)
    }

    /// The walkmesh the install loads by this name: `"m01aa_01a"` is the
    /// room's `.wok`; name a placeable's or door's with its extension
    /// (`"plc_chair1.pwk"`). Raises `RE0404`.
    #[staticmethod]
    #[pyo3(signature = (install, name, module = None))]
    fn load(install: &PyInstall, name: &str, module: Option<&str>) -> PyResult<Self> {
        Self::new(&load(&install.inner, name, Some("wok"), module)?)
    }

    /// `"area"` (a WOK) or `"object"` (a PWK or DWK).
    #[getter]
    fn kind(&self) -> &'static str {
        match self.parts.kind {
            Kind::Area => "area",
            Kind::Object => "object",
        }
    }

    #[getter]
    fn relative_points(&self) -> (V3, V3) {
        let [a, b] = self.parts.relative_points;
        (v3(a), v3(b))
    }

    #[getter]
    fn absolute_points(&self) -> (V3, V3) {
        let [a, b] = self.parts.absolute_points;
        (v3(a), v3(b))
    }

    #[getter]
    fn position(&self) -> V3 {
        v3(self.parts.position)
    }

    /// The header's field at offset 108, as read.
    #[getter]
    fn unknown(&self) -> u32 {
        self.parts.unknown
    }

    /// Every vertex as `(x, y, z)`, metres, Z up.
    #[getter]
    fn vertices(&self) -> Vec<V3> {
        self.walkmesh().vertices.iter().map(v3).collect()
    }

    /// The vertices as stored: twelve bytes each, three little-endian
    /// floats.
    #[getter]
    fn vertex_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.vertices)
    }

    #[getter]
    fn faces(&self) -> Vec<PyWalkmeshFace> {
        self.parts
            .faces
            .iter()
            .map(|f| PyWalkmeshFace {
                vertices: (f.vertices[0], f.vertices[1], f.vertices[2]),
                material: f.material,
                normal: v3(f.normal),
                distance: f.distance,
            })
            .collect()
    }

    /// For each walkable face, the `(face, edge)` across each of its three
    /// edges, or `None` where no walkable face is.
    #[getter]
    #[allow(clippy::type_complexity)]
    fn adjacency(&self) -> Vec<(Option<(u32, u8)>, Option<(u32, u8)>, Option<(u32, u8)>)> {
        self.parts
            .adjacency
            .iter()
            .map(|[a, b, c]| (a.map(edge), b.map(edge), c.map(edge)))
            .collect()
    }

    /// The edges with no walkable face across, loop by loop, as
    /// `((face, edge), transition)`: the index in the area's LYT room list
    /// of the room the edge leads into, or `None`.
    #[getter]
    fn perimeter_edges(&self) -> Vec<((u32, u8), Option<u32>)> {
        self.parts
            .perimeter_edges
            .iter()
            .map(|p| (edge(p.edge), p.transition))
            .collect()
    }

    /// Where each loop of `perimeter_edges` ends.
    #[getter]
    fn perimeters(&self) -> Vec<u32> {
        self.parts.perimeters.clone()
    }

    #[getter]
    fn aabb(&self) -> Vec<PyAabbNode> {
        self.parts
            .aabb
            .iter()
            .map(|n| PyAabbNode {
                min: v3(n.min),
                max: v3(n.max),
                face: n.face,
                unknown: n.unknown,
                plane: n.plane,
                left: n.left,
                right: n.right,
            })
            .collect()
    }

    /// The indices of the faces whose material `surfacemat` (the table
    /// `surfacemat.2da` holds) says is walkable. Raises `RE2505` for a
    /// table without its `Walk` column.
    fn walkable_faces(&self, surfacemat: &PyTwoDa) -> PyResult<Vec<usize>> {
        let table = SurfaceMaterials::from_2da(&surfacemat.inner).map_err(fail)?;
        Ok(self.walkmesh().walkable_faces(&table).collect())
    }

    /// Set face `face`'s material, its row of `surfacemat.2da`; the
    /// material it had.
    fn set_material(&mut self, face: usize, material: u32) -> PyResult<u32> {
        let n = self.parts.faces.len();
        let f = self.parts.faces.get_mut(face).ok_or_else(|| {
            PyIndexError::new_err(format!("face {face} of a walkmesh with {n} faces"))
        })?;
        Ok(std::mem::replace(&mut f.material, material))
    }

    /// Whether the two hold the same data, floats by their bits.
    fn semantic_eq(&self, other: &PyWalkmesh) -> bool {
        self.walkmesh().semantic_eq(&other.walkmesh())
    }

    /// The walkmesh in BioWare's layout; one read and not changed writes
    /// back as it was read. Raises `RE2506`.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.walkmesh().to_bytes().map_err(fail)?))
    }

    fn __repr__(&self) -> String {
        format!(
            "Walkmesh({}, {} vertices, {} faces)",
            self.kind(),
            self.vertices.len() / 12,
            self.parts.faces.len()
        )
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWalkmesh>()?;
    m.add_class::<PyWalkmeshFace>()?;
    m.add_class::<PyAabbNode>()?;
    Ok(())
}
