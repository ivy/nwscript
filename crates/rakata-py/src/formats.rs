//! The formats: GFF (`rakata::gff`), 2DA (`rakata::twoda`), TLK
//! (`rakata::tlk`) and LIP (`rakata::lip`), each read from bytes and
//! written back as bytes.

use std::path::PathBuf;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use rakata::author::blueprint;
use rakata::gff::{Gff, Path as GffPath};
use rakata::lip::{Lip, Shape};
use rakata::res::{Language, ResRef, StrRef};
use rakata::tlk::{Entry, Tlk};
use rakata::twoda::{RowRef, TwoDa};

use crate::fail;
use crate::value::{field_type, from_py, to_py};

/// A GFF file: a blueprint, a dialogue's tree, a module's GIT.
#[pyclass(name = "Gff", module = "rakata")]
#[derive(Clone)]
pub struct PyGff {
    pub(crate) inner: Gff,
}

/// The value at `path` set to `value`, the field's type given by `ty` or
/// by the field it replaces; the value the field held, or `None`.
pub(crate) fn set_path<'py>(
    gff: &mut Gff,
    path: &str,
    value: &Bound<'py, PyAny>,
    ty: Option<&str>,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let py = value.py();
    let ty = match ty {
        Some(t) => field_type(t)?,
        None => gff
            .get(&GffPath::parse(path).map_err(fail)?)
            .map_err(fail)?
            .field_type(),
    };
    let old = blueprint::set(gff, path, from_py(value, ty)?).map_err(fail)?;
    old.map(|v| to_py(py, &v)).transpose()
}

#[pymethods]
impl PyGff {
    /// Read a GFF's bytes. Raises `RE0601` and the format's other codes.
    #[staticmethod]
    fn parse(data: &[u8]) -> PyResult<Self> {
        Ok(PyGff {
            inner: Gff::parse(data).map_err(fail)?,
        })
    }

    /// An empty file of this type (`"UTC"`), with a root struct and no fields.
    #[staticmethod]
    fn new(file_type: &str) -> PyResult<Self> {
        let b = file_type.as_bytes();
        if b.len() > 4 {
            return Err(PyValueError::new_err(format!(
                "a GFF file type is at most four bytes, not {file_type:?}"
            )));
        }
        let mut t = *b"    ";
        t[..b.len()].copy_from_slice(b);
        Ok(PyGff { inner: Gff::new(t) })
    }

    /// The file type without its trailing space: `"UTC"`, `"DLG"`.
    #[getter]
    fn file_type(&self) -> String {
        self.inner.file_type_str()
    }

    /// The value at `path` (`Tag`, `ItemList[0].Dropable`). Raises `RE0606`
    /// when the path does not parse and `RE0607` when it names nothing.
    fn get<'py>(&self, py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyAny>> {
        to_py(py, &blueprint::get(&self.inner, path).map_err(fail)?)
    }

    /// The type of the field at `path`, as the format documents name it.
    fn field_type(&self, path: &str) -> PyResult<&'static str> {
        let p = GffPath::parse(path).map_err(fail)?;
        Ok(self.inner.get(&p).map_err(fail)?.field_type().name())
    }

    /// Set the field at `path` to `value` and return what it held, or `None`
    /// for a field added. The type is the field's own, or `type` (`"BYTE"`,
    /// `"CResRef"`) for a field the struct lacks; a `type` other than the
    /// field's raises `RE1406`.
    #[pyo3(signature = (path, value, r#type = None))]
    fn set<'py>(
        &mut self,
        path: &str,
        value: &Bound<'py, PyAny>,
        r#type: Option<&str>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        set_path(&mut self.inner, path, value, r#type)
    }

    /// The bytes, in BioWare's layout; a file read and not changed writes
    /// back as it was read.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn __repr__(&self) -> String {
        format!("Gff({:?})", self.inner.file_type_str())
    }
}

/// A row by index or by label.
#[derive(FromPyObject)]
pub(crate) enum Row {
    Index(usize),
    Label(String),
}

impl Row {
    fn as_ref(&self) -> RowRef<'_> {
        match self {
            Row::Index(i) => RowRef::Index(*i),
            Row::Label(l) => RowRef::Label(l),
        }
    }
}

/// `{column: value}` as the pairs the table's calls take.
pub(crate) fn pairs(d: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<(String, String)>> {
    let Some(d) = d else {
        return Ok(Vec::new());
    };
    d.iter()
        .map(|(k, v)| Ok((k.extract()?, v.extract()?)))
        .collect()
}

fn borrowed(p: &[(String, String)]) -> Vec<(&str, &str)> {
    p.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
}

/// A 2DA table: column names, and rows of a label and one cell per column.
#[pyclass(name = "TwoDa", module = "rakata")]
#[derive(Clone)]
pub struct PyTwoDa {
    pub(crate) inner: TwoDa,
}

#[pymethods]
impl PyTwoDa {
    /// Read a 2DA V2.b table. Raises `RE0701` and the format's other codes.
    #[staticmethod]
    fn parse(data: &[u8]) -> PyResult<Self> {
        Ok(PyTwoDa {
            inner: TwoDa::parse(data).map_err(fail)?,
        })
    }

    /// An empty table with these columns.
    #[staticmethod]
    fn new(columns: Vec<String>) -> Self {
        PyTwoDa {
            inner: TwoDa::new(columns),
        }
    }

    #[getter]
    fn columns(&self) -> Vec<String> {
        self.inner.columns.clone()
    }

    /// Every row as `(label, cells)`, in file order.
    #[getter]
    fn rows(&self) -> Vec<(String, Vec<String>)> {
        self.inner
            .rows
            .iter()
            .map(|r| (r.label.clone(), r.cells.clone()))
            .collect()
    }

    fn column_index(&self, name: &str) -> Option<usize> {
        self.inner.column_index(name)
    }

    /// The index itself when in range, or the first row whose row label is
    /// `row`.
    fn row_index(&self, row: Row) -> Option<usize> {
        self.inner.row_index(row.as_ref())
    }

    /// The cell; raises `RE0705` for a missing row, `RE0706` for a column.
    fn get(&self, row: Row, column: &str) -> PyResult<String> {
        Ok(self
            .inner
            .get(row.as_ref(), column)
            .map_err(fail)?
            .to_string())
    }

    fn set(&mut self, row: Row, column: &str, value: String) -> PyResult<()> {
        self.inner.set(row.as_ref(), column, value).map_err(fail)
    }

    /// Append a row of blank cells with `changes` applied; its index.
    #[pyo3(signature = (label, changes = None))]
    fn add_row(&mut self, label: String, changes: Option<&Bound<'_, PyDict>>) -> PyResult<usize> {
        let p = pairs(changes)?;
        self.inner.add_row(label, &borrowed(&p)).map_err(fail)
    }

    /// Append a copy of `source` (by index, or by row label) labelled
    /// `new_label` with `changes` applied; its index. The authoring call
    /// that names rows by a `label` column is `author.twoda.add_row_copying`.
    #[pyo3(signature = (source, new_label, changes = None))]
    fn add_row_copying(
        &mut self,
        source: Row,
        new_label: String,
        changes: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<usize> {
        let p = pairs(changes)?;
        self.inner
            .add_row_copying(source.as_ref(), new_label, &borrowed(&p))
            .map_err(fail)
    }

    /// Append a column, every row taking `fill`; its index.
    #[pyo3(signature = (name, fill = ""))]
    fn add_column(&mut self, name: String, fill: &str) -> PyResult<usize> {
        self.inner.add_column(name, fill).map_err(fail)
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn __len__(&self) -> usize {
        self.inner.rows.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "TwoDa({} columns, {} rows)",
            self.inner.columns.len(),
            self.inner.rows.len()
        )
    }
}

/// A talk table: `dialog.tlk`'s strings by strref.
#[pyclass(name = "Tlk", module = "rakata")]
#[derive(Clone)]
pub struct PyTlk {
    pub(crate) inner: Tlk,
}

#[pymethods]
impl PyTlk {
    #[staticmethod]
    fn open(path: PathBuf) -> PyResult<Self> {
        Ok(PyTlk {
            inner: Tlk::open(path).map_err(fail)?,
        })
    }

    #[staticmethod]
    fn parse(data: &[u8]) -> PyResult<Self> {
        Ok(PyTlk {
            inner: Tlk::parse(data).map_err(fail)?,
        })
    }

    /// An empty table for the language with this id (0, English).
    #[staticmethod]
    #[pyo3(signature = (language = 0))]
    fn new(language: u32) -> Self {
        PyTlk {
            inner: Tlk::new(Language::from_id(language)),
        }
    }

    /// The language id.
    #[getter]
    fn language(&self) -> u32 {
        self.inner.language.id()
    }

    /// The text of a strref, or `None` past the end or without text.
    fn text(&self, strref: u32) -> Option<String> {
        self.inner.text(StrRef::new(strref)).map(str::to_owned)
    }

    /// Append an entry, voiced by `sound` for `length` seconds when given;
    /// its strref.
    #[pyo3(signature = (text, sound = None, length = 0.0))]
    fn append(&mut self, text: String, sound: Option<&str>, length: f32) -> PyResult<u32> {
        let mut e = Entry::new(text);
        if let Some(s) = sound {
            e = e.with_sound(ResRef::new(s).map_err(fail)?, length);
        }
        Ok(self.inner.append(e).raw())
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn save(&self, path: PathBuf) -> PyResult<()> {
        self.inner.save(path).map_err(fail)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
}

pub(crate) fn shape(label: &str) -> PyResult<Shape> {
    Shape::from_label(label).ok_or_else(|| {
        let names: Vec<&str> = Shape::ALL.iter().map(|s| s.label()).collect();
        PyValueError::new_err(format!(
            "{label:?} is not a mouth shape; the shapes are {}",
            names.join(", ")
        ))
    })
}

pub(crate) fn cues(c: &[(f32, String)]) -> PyResult<Vec<(f32, Shape)>> {
    c.iter().map(|(t, s)| Ok((*t, shape(s)?))).collect()
}

/// A lip-sync track: the line's length in seconds and its keyframes.
#[pyclass(name = "Lip", module = "rakata")]
#[derive(Clone)]
pub struct PyLip {
    pub(crate) inner: Lip,
}

#[pymethods]
impl PyLip {
    #[staticmethod]
    fn parse(data: &[u8]) -> PyResult<Self> {
        Ok(PyLip {
            inner: Lip::parse(data).map_err(fail)?,
        })
    }

    #[staticmethod]
    fn read(path: PathBuf) -> PyResult<Self> {
        Ok(PyLip {
            inner: Lip::read(path).map_err(fail)?,
        })
    }

    /// A track over `length` seconds from `(time, shape)` cues, closing on
    /// a rest at `length`. Shapes by label: `"rest"`, `"ee"`, `"m"`, ...
    #[staticmethod]
    fn from_cues(cues: Vec<(f32, String)>, length: f32) -> PyResult<Self> {
        Ok(PyLip {
            inner: Lip::from_cues(self::cues(&cues)?, length),
        })
    }

    #[getter]
    fn length(&self) -> f32 {
        self.inner.length
    }

    /// The keyframes as `(time, shape label)`, in file order.
    #[getter]
    fn keys(&self) -> Vec<(f32, &'static str)> {
        self.inner
            .keys
            .iter()
            .map(|k| (k.time, k.shape.label()))
            .collect()
    }

    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn write(&self, path: PathBuf) -> PyResult<()> {
        self.inner.write(path).map_err(fail)
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGff>()?;
    m.add_class::<PyTwoDa>()?;
    m.add_class::<PyTlk>()?;
    m.add_class::<PyLip>()?;
    Ok(())
}
