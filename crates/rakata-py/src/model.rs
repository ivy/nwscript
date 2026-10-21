//! Binary models (`rakata::mdl`): a model's MDL and MDX read from bytes, a
//! path or the install, its node tree, meshes, skins, lights, emitters,
//! sabers, controllers and animations as Python objects, and the pair written back as bytes.
//!
//! The reader borrows every name and array from its inputs, so the class
//! copies what it reads into objects of its own once, when it is made, and
//! keeps the two inputs for the writer. A mesh's vertex attributes are
//! handed out as lists of tuples, and its interleaved MDX block whole as
//! `bytes` with each attribute's offset in the stride, for a caller that
//! reads them in bulk.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rakata::mdl::{
    Animation, Attribute, BoundingBox, Classification, Controller, Emitter, Light, Mesh, Model,
    ModelFile, Node, NodeKind, Quaternion, Saber, Skin, controller_types,
};

use crate::fail;
use crate::game::{PyInstall, load, read_file};

type V3 = (f32, f32, f32);
type Q = (f32, f32, f32, f32);

fn v3([x, y, z]: [f32; 3]) -> V3 {
    (x, y, z)
}

fn v2([x, y]: [f32; 2]) -> (f32, f32) {
    (x, y)
}

fn v4([a, b, c, d]: [f32; 4]) -> Q {
    (a, b, c, d)
}

/// W, X, Y, Z.
fn quat(q: Quaternion) -> Q {
    (q.w, q.x, q.y, q.z)
}

fn bbox(b: BoundingBox) -> (V3, V3) {
    (v3(b.min), v3(b.max))
}

fn classification_name(c: Classification) -> &'static str {
    match c {
        Classification::Other => "other",
        Classification::Effect => "effect",
        Classification::Tile => "tile",
        Classification::Character => "character",
        Classification::Door => "door",
        Classification::Lightsaber => "lightsaber",
        Classification::Placeable => "placeable",
        Classification::Unknown(_) => "unknown",
    }
}

pub(crate) fn kind_name(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Dummy => "dummy",
        NodeKind::Light => "light",
        NodeKind::Emitter => "emitter",
        NodeKind::Reference => "reference",
        NodeKind::Trimesh => "trimesh",
        NodeKind::Skin => "skin",
        NodeKind::AnimMesh => "animmesh",
        NodeKind::Dangly => "dangly",
        NodeKind::Aabb => "aabb",
        NodeKind::Saber => "saber",
        NodeKind::Other(_) => "other",
    }
}

/// One triangle of a mesh, as the MDL's face record stores it.
#[pyclass(name = "MeshFace", module = "rakata", frozen, get_all)]
pub struct PyMeshFace {
    normal: V3,
    distance: f32,
    /// The surface (material) id.
    surface: u32,
    /// The faces across each edge, or -1.
    adjacent: (i16, i16, i16),
    vertices: (u16, u16, u16),
}

#[pymethods]
impl PyMeshFace {
    fn __repr__(&self) -> String {
        format!("MeshFace({:?})", self.vertices)
    }
}

/// The mesh part of a node: textures, flags, faces and vertices.
#[pyclass(name = "Mesh", module = "rakata", frozen)]
pub struct PyMesh {
    #[pyo3(get)]
    texture: Option<String>,
    #[pyo3(get)]
    lightmap: Option<String>,
    #[pyo3(get)]
    texture_count: u16,
    #[pyo3(get)]
    lightmapped: bool,
    #[pyo3(get)]
    render: bool,
    #[pyo3(get)]
    shadow: bool,
    #[pyo3(get)]
    transparency_hint: u32,
    #[pyo3(get)]
    diffuse: V3,
    #[pyo3(get)]
    ambient: V3,
    #[pyo3(get)]
    bounding_box: (V3, V3),
    #[pyo3(get)]
    radius: f32,
    #[pyo3(get)]
    average: V3,
    #[pyo3(get)]
    vertex_count: usize,
    #[pyo3(get)]
    mdx_flags: u32,
    #[pyo3(get)]
    stride: usize,
    /// The MDX block as stored, `vertex_count * stride` bytes.
    #[pyo3(get)]
    vertex_buffer: Py<PyBytes>,
    /// Each attribute read from `vertex_buffer`, by name, and its offset
    /// within the stride.
    #[pyo3(get)]
    offsets: BTreeMap<&'static str, usize>,
    positions: Vec<[f32; 3]>,
    normals: Option<Vec<[f32; 3]>>,
    uv1: Option<Vec<[f32; 2]>>,
    uv2: Option<Vec<[f32; 2]>>,
    faces: Vec<rakata::mdl::Face>,
    indices: Option<Vec<u16>>,
}

fn same<const N: usize>(a: &Attribute<'_, N>, buffer: &[u8]) -> bool {
    !buffer.is_empty() && a.buffer().as_ptr() == buffer.as_ptr()
}

impl PyMesh {
    fn new(py: Python<'_>, m: &Mesh<'_>) -> Self {
        let mut offsets = BTreeMap::new();
        if same(&m.positions, m.vertex_buffer) {
            offsets.insert("position", m.positions.offset());
        }
        if let Some(a) = m.normals.as_ref().filter(|a| same(a, m.vertex_buffer)) {
            offsets.insert("normal", a.offset());
        }
        if let Some(a) = m.uv1.as_ref().filter(|a| same(a, m.vertex_buffer)) {
            offsets.insert("uv1", a.offset());
        }
        if let Some(a) = m.uv2.as_ref().filter(|a| same(a, m.vertex_buffer)) {
            offsets.insert("uv2", a.offset());
        }
        PyMesh {
            texture: m.texture.map(str::to_owned),
            lightmap: m.lightmap.map(str::to_owned),
            texture_count: m.texture_count,
            lightmapped: m.lightmapped,
            render: m.render,
            shadow: m.shadow,
            transparency_hint: m.transparency_hint,
            diffuse: v3(m.diffuse),
            ambient: v3(m.ambient),
            bounding_box: bbox(m.bounding_box),
            radius: m.radius,
            average: v3(m.average),
            vertex_count: m.vertex_count,
            mdx_flags: m.mdx_flags,
            stride: m.stride,
            vertex_buffer: PyBytes::new(py, m.vertex_buffer).unbind(),
            offsets,
            positions: m.positions.iter().collect(),
            normals: m.normals.map(|a| a.iter().collect()),
            uv1: m.uv1.map(|a| a.iter().collect()),
            uv2: m.uv2.map(|a| a.iter().collect()),
            faces: m.faces.iter().collect(),
            indices: m.indices.map(|i| i.iter().collect()),
        }
    }
}

#[pymethods]
impl PyMesh {
    /// Each vertex's position, in the node's coordinates.
    #[getter]
    fn positions(&self) -> Vec<V3> {
        self.positions.iter().copied().map(v3).collect()
    }

    #[getter]
    fn normals(&self) -> Option<Vec<V3>> {
        self.normals
            .as_ref()
            .map(|v| v.iter().copied().map(v3).collect())
    }

    /// The first texture coordinates.
    #[getter]
    fn uv1(&self) -> Option<Vec<(f32, f32)>> {
        self.uv1
            .as_ref()
            .map(|v| v.iter().copied().map(v2).collect())
    }

    /// The second texture coordinates: the lightmap's, on a lightmapped
    /// mesh.
    #[getter]
    fn uv2(&self) -> Option<Vec<(f32, f32)>> {
        self.uv2
            .as_ref()
            .map(|v| v.iter().copied().map(v2).collect())
    }

    #[getter]
    fn faces(&self) -> Vec<PyMeshFace> {
        self.faces
            .iter()
            .map(|f| PyMeshFace {
                normal: v3(f.normal),
                distance: f.distance,
                surface: f.surface,
                adjacent: (f.adjacent[0], f.adjacent[1], f.adjacent[2]),
                vertices: (f.vertices[0], f.vertices[1], f.vertices[2]),
            })
            .collect()
    }

    /// The triangle list, three vertex indices a face, or `None`.
    #[getter]
    fn indices(&self) -> Option<Vec<u16>> {
        self.indices.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Mesh({}, {} vertices, {} faces)",
            self.texture.as_deref().unwrap_or("no texture"),
            self.vertex_count,
            self.faces.len()
        )
    }
}

/// The skin part of a skin node: its bones and each vertex's weights.
#[pyclass(name = "Skin", module = "rakata", frozen)]
pub struct PySkin {
    /// The node numbers of the bones, by local bone index.
    #[pyo3(get)]
    bones: Vec<u16>,
    weights: Vec<[f32; 4]>,
    bone_indices: Vec<[f32; 4]>,
    bone_map: Vec<Option<u16>>,
    qbones: Vec<Option<Quaternion>>,
    tbones: Vec<Option<[f32; 3]>>,
}

impl PySkin {
    fn new(s: &Skin<'_>) -> Self {
        let n = u16::try_from(s.bone_map_len()).unwrap_or(u16::MAX);
        PySkin {
            bones: s.bones.clone(),
            weights: s.weights.iter().collect(),
            bone_indices: s.bone_indices.iter().collect(),
            bone_map: (0..n).map(|i| s.bone_index(i)).collect(),
            qbones: (0..n).map(|i| s.qbone(i)).collect(),
            tbones: (0..n).map(|i| s.tbone(i)).collect(),
        }
    }
}

#[pymethods]
impl PySkin {
    /// Each vertex's four weights.
    #[getter]
    fn weights(&self) -> Vec<Q> {
        self.weights.iter().copied().map(v4).collect()
    }

    /// Each vertex's four local bone indices, -1 where a slot names none.
    #[getter]
    fn bone_indices(&self) -> Vec<Q> {
        self.bone_indices.iter().copied().map(v4).collect()
    }

    /// The local bone index of node number `node`, if it is a bone.
    fn bone_index(&self, node: usize) -> Option<u16> {
        self.bone_map.get(node).copied().flatten()
    }

    /// The bind rotation stored for node number `node`, `(w, x, y, z)`.
    fn qbone(&self, node: usize) -> Option<Q> {
        self.qbones.get(node).copied().flatten().map(quat)
    }

    /// The bind translation stored for node number `node`.
    fn tbone(&self, node: usize) -> Option<V3> {
        self.tbones.get(node).copied().flatten().map(v3)
    }

    fn __repr__(&self) -> String {
        format!(
            "Skin({} bones, {} vertices)",
            self.bones.len(),
            self.weights.len()
        )
    }
}

/// A light's header: its flares, priority and flags.
#[pyclass(name = "Light", module = "rakata", frozen, get_all)]
pub struct PyLight {
    flare_radius: f32,
    /// Each flare as `(texture, size, position, colour shift)`.
    flares: Vec<(String, f32, f32, V3)>,
    priority: i32,
    ambient_only: bool,
    dynamic_type: u32,
    affect_dynamic: bool,
    shadow: bool,
    generate_flare: bool,
    fading: bool,
}

impl PyLight {
    fn new(l: &Light<'_>) -> Self {
        PyLight {
            flare_radius: l.flare_radius,
            flares: l
                .flares
                .iter()
                .map(|f| (f.texture.to_owned(), f.size, f.position, v3(f.color_shift)))
                .collect(),
            priority: l.priority,
            ambient_only: l.ambient_only,
            dynamic_type: l.dynamic_type,
            affect_dynamic: l.affect_dynamic,
            shadow: l.shadow,
            generate_flare: l.generate_flare,
            fading: l.fading,
        }
    }
}

#[pymethods]
impl PyLight {
    fn __repr__(&self) -> String {
        format!("Light({} flares)", self.flares.len())
    }
}

/// A particle emitter's header.
#[pyclass(name = "Emitter", module = "rakata", frozen, get_all)]
pub struct PyEmitter {
    dead_space: f32,
    blast_radius: f32,
    blast_length: f32,
    lightning_words: (u32, u32),
    x_grid: u32,
    y_grid: u32,
    space_type: u32,
    update: String,
    render: String,
    blend: String,
    texture: Option<String>,
    chunk: Option<String>,
    two_sided: bool,
    looping: bool,
    render_order: u16,
    name_187: String,
    flags: u32,
}

impl PyEmitter {
    fn new(e: &Emitter<'_>) -> Self {
        PyEmitter {
            dead_space: e.dead_space,
            blast_radius: e.blast_radius,
            blast_length: e.blast_length,
            lightning_words: (e.lightning_words[0], e.lightning_words[1]),
            x_grid: e.x_grid,
            y_grid: e.y_grid,
            space_type: e.space_type,
            update: e.update.to_owned(),
            render: e.render.to_owned(),
            blend: e.blend.to_owned(),
            texture: e.texture.map(str::to_owned),
            chunk: e.chunk.map(str::to_owned),
            two_sided: e.two_sided,
            looping: e.looping,
            render_order: e.render_order,
            name_187: e.name_187.to_owned(),
            flags: e.flags,
        }
    }
}

#[pymethods]
impl PyEmitter {
    fn __repr__(&self) -> String {
        format!("Emitter({:?}, {:?})", self.update, self.render)
    }
}

/// A saber blade's vertices and its surface as triangles.
#[pyclass(name = "Saber", module = "rakata", frozen, get_all)]
pub struct PySaber {
    positions: Vec<V3>,
    uv: Vec<(f32, f32)>,
    normals: Vec<V3>,
    /// The blade's surface: twelve triangles over the vertices, wound as a
    /// mesh's faces are. The mesh's stored faces are not the blade.
    triangles: Vec<(u16, u16, u16)>,
    words: (u32, u32),
}

impl PySaber {
    fn new(s: &Saber<'_>) -> Self {
        PySaber {
            positions: s.positions.iter().map(v3).collect(),
            uv: s.uv.iter().map(v2).collect(),
            normals: s.normals.iter().map(v3).collect(),
            triangles: s
                .blade_triangles()
                .into_iter()
                .map(|[a, b, c]| (a, b, c))
                .collect(),
            words: (s.words[0], s.words[1]),
        }
    }
}

#[pymethods]
impl PySaber {
    fn __repr__(&self) -> String {
        format!("Saber({} vertices)", self.positions.len())
    }
}

/// One controller of a node: a type and its keys.
#[pyclass(name = "Controller", module = "rakata", frozen, get_all)]
pub struct PyController {
    /// The controller type: 8 position, 20 orientation, 36 scale, ...
    kind: u32,
    /// The type's name on the kind of node that carries it (`radius` on a
    /// light, `birth_rate` on an emitter); `None` when not established.
    name: Option<&'static str>,
    /// The u16 after the type, as stored.
    field: u16,
    /// The column byte as stored, 0x10 set on a bezier-keyed controller.
    columns: u8,
    is_bezier: bool,
    /// Whether this is an orientation whose keys are each a compressed word.
    is_compressed: bool,
    /// Each key as `(time, values)`: an orientation's rotation as `(w, x,
    /// y, z)`, a compressed one decoded; any other controller's row of
    /// floats as stored, a bezier key's value then its two control values.
    keys: Vec<(f32, Vec<f32>)>,
}

impl PyController {
    fn new(c: &Controller<'_>, node: NodeKind) -> Self {
        let keys = match c.orientation_keys() {
            Some(keys) => keys
                .map(|k| (k.time, vec![k.value.w, k.value.x, k.value.y, k.value.z]))
                .collect(),
            None => (0..c.len())
                .map(|i| {
                    (
                        c.time(i).unwrap_or_default(),
                        c.floats(i).map(Iterator::collect).unwrap_or_default(),
                    )
                })
                .collect(),
        };
        PyController {
            kind: c.kind,
            name: controller_types::name(node, c.kind),
            field: c.field,
            columns: c.columns,
            is_bezier: c.is_bezier(),
            is_compressed: c.is_compressed(),
            keys,
        }
    }
}

#[pymethods]
impl PyController {
    fn __repr__(&self) -> String {
        format!("Controller({}, {} keys)", self.kind, self.keys.len())
    }
}

/// A node of a model's geometry or of an animation.
#[pyclass(name = "ModelNode", module = "rakata", frozen, get_all)]
pub struct PyModelNode {
    name: String,
    /// `"dummy"`, `"light"`, `"emitter"`, `"reference"`, `"trimesh"`,
    /// `"skin"`, `"animmesh"`, `"dangly"`, `"aabb"`, `"saber"` or
    /// `"other"`; `flags` is the word it is read from.
    kind: &'static str,
    flags: u16,
    /// The node's number: the index of its name in the model's names.
    number: u16,
    supernode: u16,
    /// The parent's index in the list the node is in.
    parent: Option<usize>,
    children: Vec<usize>,
    /// Relative to the parent.
    position: V3,
    /// Relative to the parent, `(w, x, y, z)`.
    orientation: Q,
    mesh: Option<Py<PyMesh>>,
    skin: Option<Py<PySkin>>,
    light: Option<Py<PyLight>>,
    emitter: Option<Py<PyEmitter>>,
    saber: Option<Py<PySaber>>,
    controllers: Vec<Py<PyController>>,
}

impl PyModelNode {
    fn new(py: Python<'_>, n: &Node<'_>) -> PyResult<Self> {
        Ok(PyModelNode {
            name: n.name.to_owned(),
            kind: kind_name(n.kind),
            flags: n.flags,
            number: n.number,
            supernode: n.supernode,
            parent: n.parent,
            children: n.children.clone(),
            position: v3(n.position),
            orientation: quat(n.orientation),
            mesh: n
                .mesh
                .as_ref()
                .map(|m| Py::new(py, PyMesh::new(py, m)))
                .transpose()?,
            skin: n
                .skin
                .as_ref()
                .map(|s| Py::new(py, PySkin::new(s)))
                .transpose()?,
            light: n
                .light
                .as_ref()
                .map(|l| Py::new(py, PyLight::new(l)))
                .transpose()?,
            emitter: n
                .emitter
                .as_ref()
                .map(|e| Py::new(py, PyEmitter::new(e)))
                .transpose()?,
            saber: n
                .saber
                .as_ref()
                .map(|s| Py::new(py, PySaber::new(s)))
                .transpose()?,
            controllers: n
                .controllers
                .iter()
                .map(|c| Py::new(py, PyController::new(c, n.kind)))
                .collect::<PyResult<_>>()?,
        })
    }
}

#[pymethods]
impl PyModelNode {
    fn __repr__(&self) -> String {
        format!("ModelNode({:?}, {})", self.name, self.kind)
    }
}

fn nodes(py: Python<'_>, ns: &[Node<'_>]) -> PyResult<Vec<Py<PyModelNode>>> {
    ns.iter()
        .map(|n| Py::new(py, PyModelNode::new(py, n)?))
        .collect()
}

fn find(py: Python<'_>, ns: &[Py<PyModelNode>], name: &str) -> Option<Py<PyModelNode>> {
    ns.iter()
        .find(|n| n.get().name.eq_ignore_ascii_case(name))
        .map(|n| n.clone_ref(py))
}

/// One of a model's own animations: its nodes carry the keys.
#[pyclass(name = "Animation", module = "rakata", frozen, get_all)]
pub struct PyAnimation {
    name: String,
    /// In seconds.
    length: f32,
    /// The blend into it, in seconds.
    transition: f32,
    /// The node the animation is rooted at.
    root: String,
    /// The events as `(time, name)`.
    events: Vec<(f32, String)>,
    nodes: Vec<Py<PyModelNode>>,
}

impl PyAnimation {
    fn new(py: Python<'_>, a: &Animation<'_>) -> PyResult<Self> {
        Ok(PyAnimation {
            name: a.name.to_owned(),
            length: a.length,
            transition: a.transition,
            root: a.root.to_owned(),
            events: a
                .events
                .iter()
                .map(|e| (e.time, e.name.to_owned()))
                .collect(),
            nodes: nodes(py, &a.nodes)?,
        })
    }
}

#[pymethods]
impl PyAnimation {
    /// The first node named `name`, compared without case.
    fn node(&self, py: Python<'_>, name: &str) -> Option<Py<PyModelNode>> {
        find(py, &self.nodes, name)
    }

    fn __repr__(&self) -> String {
        format!("Animation({:?}, {}s)", self.name, self.length)
    }
}

/// A binary model: its MDL's header and node tree, and its MDX's vertices.
#[pyclass(name = "Model", module = "rakata", frozen)]
pub struct PyModel {
    mdl: Vec<u8>,
    mdx: Vec<u8>,
    #[pyo3(get)]
    name: String,
    /// `"other"` (rooms), `"effect"`, `"tile"`, `"character"`, `"door"`,
    /// `"lightsaber"`, `"placeable"` or `"unknown"`.
    #[pyo3(get)]
    classification: &'static str,
    #[pyo3(get)]
    subclassification: u8,
    #[pyo3(get)]
    fog: bool,
    #[pyo3(get)]
    supermodel: Option<String>,
    #[pyo3(get)]
    bounding_box: (V3, V3),
    #[pyo3(get)]
    radius: f32,
    #[pyo3(get)]
    animation_scale: f32,
    /// Every node the geometry's tree reaches, the root first.
    #[pyo3(get)]
    nodes: Vec<Py<PyModelNode>>,
    #[pyo3(get)]
    animations: Vec<Py<PyAnimation>>,
}

impl PyModel {
    fn new(py: Python<'_>, mdl: Vec<u8>, mdx: Vec<u8>) -> PyResult<Self> {
        let m = Model::parse(&mdl, &mdx).map_err(fail)?;
        let nodes = nodes(py, &m.nodes)?;
        let animations = m
            .animations
            .iter()
            .map(|a| Py::new(py, PyAnimation::new(py, a)?))
            .collect::<PyResult<_>>()?;
        Ok(PyModel {
            name: m.name.to_owned(),
            classification: classification_name(m.classification),
            subclassification: m.subclassification,
            fog: m.fog,
            supermodel: m.supermodel.map(str::to_owned),
            bounding_box: bbox(m.bounding_box),
            radius: m.radius,
            animation_scale: m.animation_scale,
            nodes,
            animations,
            mdl,
            mdx,
        })
    }
}

#[pymethods]
impl PyModel {
    /// Read a model from its MDL and MDX; the MDX may be empty for a model
    /// whose meshes have none. Raises `RE2401` and the format's other codes.
    #[staticmethod]
    #[pyo3(signature = (mdl, mdx = Vec::new()))]
    fn parse(py: Python<'_>, mdl: Vec<u8>, mdx: Vec<u8>) -> PyResult<Self> {
        Self::new(py, mdl, mdx)
    }

    /// Read `mdl` and the MDX beside it (the same path with the extension
    /// `.mdx`), or `mdx` when given.
    #[staticmethod]
    #[pyo3(signature = (mdl, mdx = None))]
    fn read(py: Python<'_>, mdl: PathBuf, mdx: Option<PathBuf>) -> PyResult<Self> {
        let mdx = mdx.unwrap_or_else(|| mdl.with_extension("mdx"));
        Self::new(py, read_file(&mdl)?, read_file(&mdx)?)
    }

    /// The `.mdl` and `.mdx` the install loads by this name (`"m01aa_01a"`),
    /// in a module's capsules too when `module` names one. Raises `RE0404`.
    #[staticmethod]
    #[pyo3(signature = (install, name, module = None))]
    fn load(
        py: Python<'_>,
        install: &PyInstall,
        name: &str,
        module: Option<&str>,
    ) -> PyResult<Self> {
        let stem = name.split_once('.').map_or(name, |(s, _)| s);
        let mdl = load(&install.inner, stem, Some("mdl"), module)?;
        let mdx = load(&install.inner, stem, Some("mdx"), module)?;
        Self::new(py, mdl, mdx)
    }

    /// The first node named `name`, compared without case.
    fn node(&self, py: Python<'_>, name: &str) -> Option<Py<PyModelNode>> {
        find(py, &self.nodes, name)
    }

    /// The model's own animation named `name`, compared without case.
    fn animation(&self, py: Python<'_>, name: &str) -> Option<Py<PyAnimation>> {
        self.animations
            .iter()
            .find(|a| a.get().name.eq_ignore_ascii_case(name))
            .map(|a| a.clone_ref(py))
    }

    /// The MDL and MDX, laid out as BioWare's compiler lays them out; a
    /// vanilla model writes back byte for byte. Raises `RE2414` and the
    /// writer's other codes.
    fn to_bytes<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<(Bound<'py, PyBytes>, Bound<'py, PyBytes>)> {
        let file = ModelFile::parse(&self.mdl, &self.mdx).map_err(fail)?;
        let (mdl, mdx) = file.to_bytes().map_err(fail)?;
        Ok((PyBytes::new(py, &mdl), PyBytes::new(py, &mdx)))
    }

    fn __repr__(&self) -> String {
        format!("Model({:?}, {} nodes)", self.name, self.nodes.len())
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyModel>()?;
    m.add_class::<PyModelNode>()?;
    m.add_class::<PyMesh>()?;
    m.add_class::<PyMeshFace>()?;
    m.add_class::<PySkin>()?;
    m.add_class::<PyController>()?;
    m.add_class::<PyAnimation>()?;
    Ok(())
}
