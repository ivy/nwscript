//! `rakata.author`: the authoring API (`rakata::author`). `Mod` is the
//! install opened for a mod's writes; the edits are the submodules
//! `twoda`, `globals`, `blueprint`, `dialog`, `lip`, `scripts` and `voice`,
//! one per module of the Rust API, with the same functions.

use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use rakata::author::{self, Mod};
use rakata::game::Install;
use rakata::res::{ContentHash, ResRef};

use crate::dialog::{PyDialog, PyFinding, PyNodeRef, key};
use crate::formats::PyGff;
use crate::game::{PyInstall, res_id};
use crate::{fail, submodule};

/// A copy of a resource as the game would load it.
#[pyclass(name = "Loaded", module = "rakata.author", frozen)]
pub struct PyLoaded {
    bytes: Py<PyBytes>,
    /// Where the bytes lie, relative to the install:
    /// `Override/x.2da`, `modules/danm14ac_s.rim:k_pdan_juhani31.ncs`.
    #[pyo3(get)]
    location: String,
    /// Whether the copy is one this mod wrote.
    #[pyo3(get)]
    written: bool,
}

#[pymethods]
impl PyLoaded {
    #[getter]
    fn bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        self.bytes.clone_ref(py)
    }

    fn __repr__(&self) -> String {
        format!("Loaded({})", self.location)
    }
}

/// A file the mod wrote.
#[pyclass(name = "Written", module = "rakata.author", frozen, get_all)]
pub struct PyWritten {
    path: String,
    /// The SHA-256 of what was written, in hex.
    hash: String,
    len: usize,
    /// For a dialogue, the references its base copy already lacked: the
    /// game's or another mod's, reported and not refused.
    warnings: Vec<PyFinding>,
}

impl From<author::Written> for PyWritten {
    fn from(w: author::Written) -> Self {
        PyWritten {
            path: w.path.display().to_string(),
            hash: w.hash.to_hex(),
            len: w.len,
            warnings: w.warnings.into_iter().map(PyFinding::from).collect(),
        }
    }
}

#[pymethods]
impl PyWritten {
    fn __repr__(&self) -> String {
        format!("Written({})", self.path)
    }
}

/// An install opened for a mod's writes, in one module.
#[pyclass(name = "Mod", module = "rakata.author")]
pub struct PyMod {
    install: Arc<Install>,
    inner: Mod,
}

#[pymethods]
impl PyMod {
    /// Open `install` for a mod working in `module` (`"danm14ac"`), or in
    /// the global layers alone. Raises `RE0403` when the install has no
    /// capsule of that module.
    #[staticmethod]
    #[pyo3(signature = (install, module = None))]
    fn open(install: &PyInstall, module: Option<&str>) -> PyResult<Self> {
        let i = install.inner.clone();
        Ok(PyMod {
            inner: Mod::open(i.clone(), module).map_err(fail)?,
            install: i,
        })
    }

    #[getter]
    fn install(&self) -> PyInstall {
        PyInstall {
            inner: self.install.clone(),
        }
    }

    #[getter]
    fn module(&self) -> Option<String> {
        self.inner.module().map(ToString::to_string)
    }

    /// The copy of `name` (`NAME.EXT`) the game loads: this mod's own write
    /// when it made one, else the install's in the engine's order. Raises
    /// `RE0404` when no layer holds it.
    fn loaded(&self, py: Python<'_>, name: &str) -> PyResult<PyLoaded> {
        let l = self.inner.loaded(res_id(name, None)?).map_err(fail)?;
        Ok(PyLoaded {
            bytes: PyBytes::new(py, &l.bytes).unbind(),
            location: l.location,
            written: l.written,
        })
    }

    /// Guard a resource the mod replaces: every copy in the module's
    /// capsules must have this SHA-256 (hex), and `Override/` must hold
    /// none. Raises `RE1401`, `RE1402` or `RE1403`.
    fn guard_replaced(&self, name: &str, sha256: &str) -> PyResult<()> {
        let expected: ContentHash = sha256
            .parse()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        self.inner
            .guard_replaced(res_id(name, None)?, expected)
            .map_err(fail)
    }

    /// The node of `dialog` a key names, text keys compared through the
    /// install's talk table as well as the text a node embeds.
    fn find(&mut self, dialog: &PyDialog, key: &Bound<'_, PyAny>) -> PyResult<PyNodeRef> {
        let k = self::key(key)?;
        Ok(self.inner.find(&dialog.inner, &k).map_err(fail)?.into())
    }

    /// The reference checks over `dialog`, against the install and this
    /// mod's writes: what `write_override` runs before writing one.
    fn check_dialog(&mut self, dialog: &PyDialog) -> Vec<PyFinding> {
        self.inner
            .check_dialog(&dialog.inner)
            .into_iter()
            .map(PyFinding::from)
            .collect()
    }

    /// Write `data` to `Override/` as `name`, read back as its type first;
    /// a dialogue is refused (`RE1407`) when a node or link it adds names
    /// what the install lacks. A refused write leaves the disk as it was.
    fn write_override(&mut self, name: &str, data: &[u8]) -> PyResult<PyWritten> {
        Ok(self.inner.write_override(name, data).map_err(fail)?.into())
    }

    /// Write a voice-over to `streamwaves/` as `resref.wav`. Raises `RE1413`
    /// for bytes that are not RIFF WAVE.
    fn write_streamwaves(&mut self, resref: &str, data: &[u8]) -> PyResult<PyWritten> {
        Ok(self
            .inner
            .write_streamwaves(resref, data)
            .map_err(fail)?
            .into())
    }

    fn __repr__(&self) -> String {
        format!(
            "Mod({:?}, {:?})",
            self.install.root().display().to_string(),
            self.inner.module().map(ToString::to_string)
        )
    }
}

/// `rakata.author.twoda`.
mod twoda {
    use super::*;
    use crate::formats::{PyTwoDa, Row, pairs};
    use rakata::author::twoda::{self, RowKey};

    fn row_key(r: Row) -> RowKey {
        match r {
            Row::Index(i) => RowKey::Index(i),
            Row::Label(l) => RowKey::Label(l),
        }
    }

    /// The one row `key` names: an index, or a label (the `label` column
    /// where the table has one, the row label otherwise). Raises `RE0705`
    /// when none matches and `RE1405` when a label matches several.
    #[pyfunction]
    fn find(table: &PyTwoDa, key: Row) -> PyResult<usize> {
        twoda::find(&table.inner, &row_key(key)).map_err(fail)
    }

    /// Append a copy of the row `source`, labelled `new_label`, with
    /// `cells` changed; its index. Raises `RE1405` when a row already
    /// carries `new_label`.
    #[pyfunction]
    #[pyo3(signature = (table, source, new_label, cells = None))]
    fn add_row_copying(
        mut table: PyRefMut<'_, PyTwoDa>,
        source: Row,
        new_label: &str,
        cells: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<usize> {
        let c = pairs(cells)?;
        twoda::add_row_copying(&mut table.inner, &row_key(source), new_label, &c).map_err(fail)
    }

    pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(find, m)?)?;
        m.add_function(wrap_pyfunction!(add_row_copying, m)?)?;
        Ok(())
    }
}

/// `rakata.author.globals`.
mod globals {
    use super::*;
    use crate::formats::PyTwoDa;
    use rakata::author::globals::{self, GlobalType};

    /// Declare the global `name` of type `"Boolean"` or `"Number"` in
    /// `globalcat.2da`'s table; the row's index. Raises `RE1404` when a row
    /// already declares the name in any case.
    #[pyfunction]
    #[pyo3(signature = (table, name, r#type = "Boolean"))]
    fn declare(mut table: PyRefMut<'_, PyTwoDa>, name: &str, r#type: &str) -> PyResult<usize> {
        let ty = match r#type {
            "Boolean" => GlobalType::Boolean,
            "Number" => GlobalType::Number,
            other => {
                return Err(PyValueError::new_err(format!(
                    "a global's type is \"Boolean\" or \"Number\", not {other:?}"
                )));
            }
        };
        globals::declare(&mut table.inner, name, ty).map_err(fail)
    }

    pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(declare, m)?)?;
        Ok(())
    }
}

/// `rakata.author.blueprint`.
mod blueprint {
    use super::*;
    use crate::formats::set_path;
    use crate::value::{field_type, from_py, to_py};
    use rakata::author::blueprint;
    use rakata::gff::{Path as GffPath, Value};

    /// The value at `path`.
    #[pyfunction]
    fn get<'py>(py: Python<'py>, gff: &PyGff, path: &str) -> PyResult<Bound<'py, PyAny>> {
        to_py(py, &blueprint::get(&gff.inner, path).map_err(fail)?)
    }

    /// Set the field at `path`; what it held, or `None` for a field added.
    #[pyfunction]
    #[pyo3(signature = (gff, path, value, r#type = None))]
    fn set<'py>(
        mut gff: PyRefMut<'_, PyGff>,
        path: &str,
        value: &Bound<'py, PyAny>,
        r#type: Option<&str>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        set_path(&mut gff.inner, path, value, r#type)
    }

    /// Set `label` to `value` in every element of the list at `list`,
    /// adding it where an element lacks it; how many elements there are (0
    /// when the path names no list). The type is the one the elements'
    /// field holds, or `type` when none holds it.
    #[pyfunction]
    #[pyo3(signature = (gff, list, label, value, r#type = None))]
    fn set_each(
        mut gff: PyRefMut<'_, PyGff>,
        list: &str,
        label: &str,
        value: &Bound<'_, PyAny>,
        r#type: Option<&str>,
    ) -> PyResult<usize> {
        let ty = match r#type {
            Some(t) => Some(field_type(t)?),
            None => {
                let p = GffPath::parse(list).map_err(fail)?;
                match gff.inner.get(&p) {
                    Ok(Value::List(items)) => items
                        .iter()
                        .find_map(|s| s.get(label))
                        .map(Value::field_type),
                    _ => return Ok(0),
                }
            }
        };
        let Some(ty) = ty else {
            return Err(PyValueError::new_err(format!(
                "no element of {list} holds {label}; give its type"
            )));
        };
        blueprint::set_each(&mut gff.inner, list, label, from_py(value, ty)?).map_err(fail)
    }

    /// A copy of `source` whose `TemplateResRef` is `template`.
    #[pyfunction]
    fn clone(source: &PyGff, template: &str) -> PyResult<PyGff> {
        Ok(PyGff {
            inner: blueprint::clone(&source.inner, template).map_err(fail)?,
        })
    }

    pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(get, m)?)?;
        m.add_function(wrap_pyfunction!(set, m)?)?;
        m.add_function(wrap_pyfunction!(set_each, m)?)?;
        m.add_function(wrap_pyfunction!(clone, m)?)?;
        Ok(())
    }
}

/// `rakata.author.dialog`.
mod dialog {
    use super::*;
    use crate::dialog::kind;
    use pyo3::exceptions::PyTypeError;
    use rakata::author::dialog::{self, CAMERA_STATIC, NO_DELAY, Text};
    use rakata_dialog::NewNode;

    fn resref(s: Option<&str>) -> PyResult<Option<ResRef>> {
        match s {
            None | Some("") => Ok(None),
            Some(s) => ResRef::new(s).map(Some).map_err(fail),
        }
    }

    /// `None`, no text; an `int`, the game's line by strref; a `str`, a line
    /// of the mod's own in English.
    fn text(t: Option<&Bound<'_, PyAny>>) -> PyResult<Text> {
        let Some(t) = t.filter(|t| !t.is_none()) else {
            return Ok(Text::None);
        };
        if let Ok(s) = t.extract::<String>() {
            return Ok(Text::English(s));
        }
        if let Ok(n) = t.extract::<u32>() {
            return Ok(Text::StrRef(n));
        }
        Err(PyTypeError::new_err(
            "a node's text is None, an int strref or a str",
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn new_node(
        text: Option<&Bound<'_, PyAny>>,
        speaker: &str,
        listener: &str,
        voice: Option<&str>,
        script: Option<&str>,
        animations: Vec<(String, u16)>,
        quest: Option<(String, u32)>,
        camera: Option<i32>,
        comment: &str,
    ) -> PyResult<NewNode> {
        Ok(NewNode {
            text: self::text(text)?,
            speaker: speaker.to_string(),
            listener: listener.to_string(),
            voice: resref(voice)?,
            script: resref(script)?,
            animations,
            quest,
            camera,
            comment: comment.to_string(),
        })
    }

    /// Append an entry, a line of the owner's or of `speaker`'s; where it
    /// lies.
    #[pyfunction]
    #[pyo3(signature = (dialog, text = None, *, speaker = "", listener = "", voice = None, script = None, animations = vec![], quest = None, camera = None, comment = ""))]
    #[allow(clippy::too_many_arguments)]
    fn add_entry(
        mut dialog: PyRefMut<'_, PyDialog>,
        text: Option<&Bound<'_, PyAny>>,
        speaker: &str,
        listener: &str,
        voice: Option<&str>,
        script: Option<&str>,
        animations: Vec<(String, u16)>,
        quest: Option<(String, u32)>,
        camera: Option<i32>,
        comment: &str,
    ) -> PyResult<PyNodeRef> {
        let n = new_node(
            text, speaker, listener, voice, script, animations, quest, camera, comment,
        )?;
        Ok(dialog::add_entry(&mut dialog.inner, &n)
            .map_err(fail)?
            .into())
    }

    /// Append a reply, a line of the player's (without text, the
    /// conversation goes on without a choice); where it lies.
    #[pyfunction]
    #[pyo3(signature = (dialog, text = None, *, listener = "", voice = None, script = None, comment = ""))]
    fn add_reply(
        mut dialog: PyRefMut<'_, PyDialog>,
        text: Option<&Bound<'_, PyAny>>,
        listener: &str,
        voice: Option<&str>,
        script: Option<&str>,
        comment: &str,
    ) -> PyResult<PyNodeRef> {
        let n = new_node(
            text,
            "",
            listener,
            voice,
            script,
            Vec::new(),
            None,
            None,
            comment,
        )?;
        Ok(dialog::add_reply(&mut dialog.inner, &n)
            .map_err(fail)?
            .into())
    }

    /// Link `source` to `to`, of the other kind, under `condition`, at `at`
    /// among `source`'s links or after the last; the link's position.
    #[pyfunction]
    #[pyo3(signature = (dialog, source, to, condition = None, at = None))]
    fn link(
        mut dialog: PyRefMut<'_, PyDialog>,
        source: &Bound<'_, PyAny>,
        to: &Bound<'_, PyAny>,
        condition: Option<&str>,
        at: Option<usize>,
    ) -> PyResult<usize> {
        let (f, t) = (key(source)?, key(to)?);
        dialog::link(&mut dialog.inner, &f, &t, condition, at).map_err(fail)
    }

    /// Remove every link out of `source`.
    #[pyfunction]
    fn unlink_all(mut dialog: PyRefMut<'_, PyDialog>, source: &Bound<'_, PyAny>) -> PyResult<()> {
        let f = key(source)?;
        dialog::unlink_all(&mut dialog.inner, &f).map_err(fail)
    }

    /// A way into the conversation at the entry `to`, under `condition`, at
    /// `at` in the starting list, or first; its position.
    #[pyfunction]
    #[pyo3(signature = (dialog, to, condition = None, at = None))]
    fn start(
        mut dialog: PyRefMut<'_, PyDialog>,
        to: &Bound<'_, PyAny>,
        condition: Option<&str>,
        at: Option<usize>,
    ) -> PyResult<usize> {
        let t = key(to)?;
        dialog::start(&mut dialog.inner, &t, condition, at).map_err(fail)
    }

    /// Reorder a list (`"entry"` or `"reply"`): `order[i]` is the old index
    /// of the node that goes to `i`. Raises `RE1107` unless `order` is a
    /// permutation of the list.
    #[pyfunction]
    fn reorder(mut dialog: PyRefMut<'_, PyDialog>, kind: &str, order: Vec<usize>) -> PyResult<()> {
        let k = self::kind(kind)?;
        dialog::reorder(&mut dialog.inner, k, &order).map_err(fail)
    }

    /// A new dialogue named `name` with the settings of `like` and no lines.
    #[pyfunction]
    fn empty_like(like: &PyDialog, name: &str) -> PyDialog {
        PyDialog {
            inner: dialog::empty_like(&like.inner, name),
        }
    }

    pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(add_entry, m)?)?;
        m.add_function(wrap_pyfunction!(add_reply, m)?)?;
        m.add_function(wrap_pyfunction!(link, m)?)?;
        m.add_function(wrap_pyfunction!(unlink_all, m)?)?;
        m.add_function(wrap_pyfunction!(start, m)?)?;
        m.add_function(wrap_pyfunction!(reorder, m)?)?;
        m.add_function(wrap_pyfunction!(empty_like, m)?)?;
        m.add("NO_DELAY", NO_DELAY)?;
        m.add("CAMERA_STATIC", CAMERA_STATIC)?;
        Ok(())
    }
}

/// `rakata.author.lip`.
mod lip {
    use super::*;
    use crate::formats::{PyLip, cues};
    use rakata::author::lip;

    /// A track over `length` seconds from `(time, shape)` cues, closing on
    /// a rest at `length`.
    #[pyfunction]
    fn from_cues(cues: Vec<(f32, String)>, length: f32) -> PyResult<PyLip> {
        Ok(PyLip {
            inner: lip::from_cues(&self::cues(&cues)?, length),
        })
    }

    /// The game's shape's label for one of Rhubarb Lip Sync's mouth shapes
    /// (`A` to `H`, `X`); `None` for another letter.
    #[pyfunction]
    fn rhubarb_shape(letter: char) -> Option<&'static str> {
        lip::rhubarb_shape(letter).map(|s| s.label())
    }

    pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(from_cues, m)?)?;
        m.add_function(wrap_pyfunction!(rhubarb_shape, m)?)?;
        Ok(())
    }
}

/// `rakata.author.scripts`.
mod scripts {
    use super::*;
    use nwscript::Declarations;
    use nwscript::modes::Mode;
    use pyo3::types::PyList;
    use rakata::author::scripts::{self, Flags, Source};

    /// A script's source: its name without `.nss` and its text.
    #[pyclass(name = "Source", module = "rakata.author.scripts", frozen)]
    #[derive(Clone)]
    pub struct PySource {
        inner: Source,
    }

    #[pymethods]
    impl PySource {
        /// `text` as `bytes`, or `str` encoded as Windows-1252. Raises
        /// `RE0101` for a name over sixteen bytes.
        #[new]
        fn new(name: String, text: &Bound<'_, PyAny>) -> PyResult<Self> {
            let text = if let Ok(s) = text.extract::<String>() {
                rakata::tlk::codepage::encode(&s).map_err(|c| {
                    PyValueError::new_err(format!(
                        "{c:?} has no byte in the game's code page (Windows-1252)"
                    ))
                })?
            } else {
                text.extract::<Vec<u8>>()?
            };
            Ok(PySource {
                inner: Source::new(name, text).map_err(fail)?,
            })
        }

        #[getter]
        fn name(&self) -> &str {
            self.inner.name()
        }

        #[getter]
        fn text<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
            PyBytes::new(py, self.inner.text())
        }
    }

    /// How to compile, as `nws`'s `--declarations` and `--compat` say it.
    #[pyclass(name = "Flags", module = "rakata.author.scripts", frozen)]
    #[derive(Clone, Copy)]
    pub struct PyFlags {
        inner: Flags,
    }

    #[pymethods]
    impl PyFlags {
        /// `declarations` is `"shipped"` or `"bioware"`; `mode` is
        /// `"strict"`, `"nwnnsscomp"` or `"bioware"`.
        #[new]
        #[pyo3(signature = (declarations = "shipped", mode = "strict"))]
        fn new(declarations: &str, mode: &str) -> PyResult<Self> {
            let declarations = match declarations {
                "shipped" => Declarations::Shipped,
                "bioware" => Declarations::BioWare,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "declarations is \"shipped\" or \"bioware\", not {other:?}"
                    )));
                }
            };
            let mode = Mode::parse(mode).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "mode is \"strict\", \"nwnnsscomp\" or \"bioware\", not {mode:?}"
                ))
            })?;
            Ok(PyFlags {
                inner: Flags { declarations, mode },
            })
        }

        /// `--declarations bioware --compat bioware`: a decompiled BioWare
        /// script compiles to BioWare's bytes.
        #[classattr]
        #[allow(non_snake_case)]
        fn BIOWARE() -> PyFlags {
            PyFlags {
                inner: Flags::BIOWARE,
            }
        }
    }

    /// A script compiled.
    #[pyclass(name = "Compiled", module = "rakata.author.scripts", frozen)]
    pub struct PyCompiled {
        /// The script's name, without `.ncs`.
        #[pyo3(get)]
        name: String,
        ncs: Vec<u8>,
        /// The warnings, as `nws` prints them; empty when there were none.
        #[pyo3(get)]
        warnings: String,
        inputs: Vec<nwscript::InputRecord>,
    }

    #[pymethods]
    impl PyCompiled {
        /// The bytecode, for `Mod.write_override` as `NAME.ncs`.
        #[getter]
        fn ncs<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
            PyBytes::new(py, &self.ncs)
        }

        /// Every include and `nwscript.nss` read: `{"role", "path", "sha256"}`.
        #[getter]
        fn inputs<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
            let l = PyList::empty(py);
            for i in &self.inputs {
                let d = PyDict::new(py);
                d.set_item("role", &i.role)?;
                d.set_item("path", &i.path)?;
                d.set_item("sha256", &i.sha256)?;
                l.append(d)?;
            }
            Ok(l)
        }
    }

    fn sources(s: &[PyRef<'_, PySource>]) -> Vec<Source> {
        s.iter().map(|s| s.inner.clone()).collect()
    }

    /// Compile each of `sources` for the install's game, includes found
    /// among `sources` and `includes`, then in the install. Nothing is
    /// written. Raises `RE1408` on the first script with an error, its
    /// diagnostics in the message.
    #[pyfunction]
    #[pyo3(signature = (m, sources, includes = vec![], flags = None))]
    fn compile(
        m: &PyMod,
        sources: Vec<PyRef<'_, PySource>>,
        includes: Vec<PyRef<'_, PySource>>,
        flags: Option<&PyFlags>,
    ) -> PyResult<Vec<PyCompiled>> {
        let flags = flags.map(|f| f.inner).unwrap_or_default();
        let out = scripts::compile(
            &m.inner,
            &self::sources(&sources),
            &self::sources(&includes),
            flags,
        )
        .map_err(fail)?;
        Ok(out
            .into_iter()
            .map(|c| PyCompiled {
                name: c.name,
                ncs: c.ncs,
                warnings: c.warnings,
                inputs: c.inputs,
            })
            .collect())
    }

    pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<PySource>()?;
        m.add_class::<PyFlags>()?;
        m.add_class::<PyCompiled>()?;
        m.add_function(wrap_pyfunction!(compile, m)?)?;
        Ok(())
    }
}

/// `rakata.author.voice`.
mod voice {
    use super::*;
    use crate::formats::PyLip;
    use rakata::author::voice::{self, LEAD, Line, Span, Take, WAV_HEADER};

    /// One recording: its name, the SHA-256 of its audio, and its
    /// transcript's segments as `(start, end)` seconds.
    #[pyclass(name = "Take", module = "rakata.author.voice", frozen)]
    #[derive(Clone)]
    pub struct PyTake {
        inner: Take,
    }

    #[pymethods]
    impl PyTake {
        #[new]
        fn new(name: String, hash: &str, segments: Vec<(f64, f64)>) -> PyResult<Self> {
            let hash: ContentHash = hash
                .parse()
                .map_err(|e| PyValueError::new_err(format!("{e}")))?;
            Ok(PyTake {
                inner: Take {
                    name,
                    hash,
                    segments,
                },
            })
        }

        /// A take from its audio's bytes and its AWS Transcribe JSON.
        /// Raises `RE1412` without `results.audio_segments`.
        #[staticmethod]
        fn from_transcript(name: &str, audio: &[u8], transcript: &str) -> PyResult<Self> {
            Ok(PyTake {
                inner: Take::from_transcript(name, audio, transcript).map_err(fail)?,
            })
        }

        #[getter]
        fn name(&self) -> String {
            self.inner.name.clone()
        }

        #[getter]
        fn hash(&self) -> String {
            self.inner.hash.to_hex()
        }

        #[getter]
        fn segments(&self) -> Vec<(f64, f64)> {
            self.inner.segments.clone()
        }
    }

    /// A voice line: the resref it is played by, the take it is cut from,
    /// its subtitle, and its segments of the take, inclusive from 0
    /// (`None` for `first`: the whole take; for `last`: to the end).
    #[pyclass(name = "Line", module = "rakata.author.voice", frozen, get_all)]
    #[derive(Clone)]
    pub struct PyLine {
        resref: String,
        take: String,
        text: String,
        first: Option<usize>,
        last: Option<usize>,
    }

    #[pymethods]
    impl PyLine {
        #[new]
        #[pyo3(signature = (resref, take, text, first = None, last = None))]
        fn new(
            resref: String,
            take: String,
            text: String,
            first: Option<usize>,
            last: Option<usize>,
        ) -> Self {
            PyLine {
                resref,
                take,
                text,
                first,
                last,
            }
        }
    }

    /// The stretch of a take to cut for one line.
    #[pyclass(name = "Span", module = "rakata.author.voice", frozen)]
    pub struct PySpan {
        inner: Span,
    }

    #[pymethods]
    impl PySpan {
        #[getter]
        fn resref(&self) -> String {
            self.inner.resref.to_string()
        }

        #[getter]
        fn take(&self) -> String {
            self.inner.take.clone()
        }

        #[getter]
        fn text(&self) -> String {
            self.inner.text.clone()
        }

        /// Seconds into the take, the lead included.
        #[getter]
        fn start(&self) -> f64 {
            self.inner.start
        }

        #[getter]
        fn end(&self) -> f64 {
            self.inner.end
        }

        /// The cache key: SHA-256 hex of the take's hash and the span.
        #[getter]
        fn key(&self) -> String {
            self.inner.key.clone()
        }

        /// The start as ffmpeg's `-ss` takes it, to the millisecond.
        fn start_text(&self) -> String {
            self.inner.start_text()
        }

        /// The end as ffmpeg's `-to` takes it, to the millisecond.
        fn end_text(&self) -> String {
            self.inner.end_text()
        }

        fn __repr__(&self) -> String {
            format!(
                "Span({} {} {}..{})",
                self.inner.resref,
                self.inner.take,
                self.inner.start_text(),
                self.inner.end_text()
            )
        }
    }

    /// The span of every line, in order, `lead` seconds kept on each side.
    /// Raises `RE0101`, `RE1411` (two lines of one resref) or `RE1410` (a
    /// take or segments the transcript does not hold).
    #[pyfunction]
    #[pyo3(signature = (lines, takes, lead = LEAD))]
    fn spans(
        lines: Vec<PyRef<'_, PyLine>>,
        takes: Vec<PyRef<'_, PyTake>>,
        lead: f64,
    ) -> PyResult<Vec<PySpan>> {
        let lines: Vec<Line> = lines
            .iter()
            .map(|l| Line {
                resref: l.resref.clone(),
                take: l.take.clone(),
                text: l.text.clone(),
                first: l.first,
                last: l.last,
            })
            .collect();
        let takes: Vec<Take> = takes.iter().map(|t| t.inner.clone()).collect();
        Ok(voice::spans(&lines, &takes, lead)
            .map_err(fail)?
            .into_iter()
            .map(|inner| PySpan { inner })
            .collect())
    }

    /// The lip-sync file from Rhubarb Lip Sync's JSON output. Raises
    /// `RE1412` when it does not read.
    #[pyfunction]
    fn take_cues(json: &str) -> PyResult<PyLip> {
        Ok(PyLip {
            inner: voice::take_cues(json).map_err(fail)?,
        })
    }

    /// The voice file the game plays for an MP3 stream: `WAV_HEADER`, then
    /// the stream.
    #[pyfunction]
    fn streamwave<'py>(py: Python<'py>, mp3: &[u8]) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &voice::streamwave(mp3))
    }

    pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<PyTake>()?;
        m.add_class::<PyLine>()?;
        m.add_class::<PySpan>()?;
        m.add_function(wrap_pyfunction!(spans, m)?)?;
        m.add_function(wrap_pyfunction!(take_cues, m)?)?;
        m.add_function(wrap_pyfunction!(streamwave, m)?)?;
        m.add("LEAD", LEAD)?;
        m.add("WAV_HEADER", PyBytes::new(m.py(), &WAV_HEADER))?;
        Ok(())
    }
}

pub fn register(top: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = top.py();
    top.add_class::<PyMod>()?;
    let m = submodule(top, "rakata.author", "author")?;
    m.add("Mod", py.get_type::<PyMod>())?;
    m.add_class::<PyLoaded>()?;
    m.add_class::<PyWritten>()?;
    twoda::register(&submodule(&m, "rakata.author.twoda", "twoda")?)?;
    globals::register(&submodule(&m, "rakata.author.globals", "globals")?)?;
    blueprint::register(&submodule(&m, "rakata.author.blueprint", "blueprint")?)?;
    dialog::register(&submodule(&m, "rakata.author.dialog", "dialog")?)?;
    lip::register(&submodule(&m, "rakata.author.lip", "lip")?)?;
    scripts::register(&submodule(&m, "rakata.author.scripts", "scripts")?)?;
    voice::register(&submodule(&m, "rakata.author.voice", "voice")?)?;
    Ok(())
}
