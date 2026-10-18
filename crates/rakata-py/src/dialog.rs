//! Dialogues (`rakata-dialog`): the model read from a DLG, nodes by index
//! (`NodeRef`) and by stable key (`NodeKey`), and the reference checks'
//! findings.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rakata::dialog::{Kind, NodeKey, NodeRef};
use rakata::res::ResRef;
use rakata_dialog::{Dialog, Finding, Link, Node};

use crate::fail;
use crate::formats::PyGff;

pub(crate) fn kind(name: &str) -> PyResult<Kind> {
    match name {
        "entry" => Ok(Kind::Entry),
        "reply" => Ok(Kind::Reply),
        other => Err(PyValueError::new_err(format!(
            "kind is \"entry\" or \"reply\", not {other:?}"
        ))),
    }
}

/// A node by its list and index: `E3` is `EntryList[3]`, `R0` `ReplyList[0]`.
#[pyclass(name = "NodeRef", module = "rakata", frozen, eq, hash)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyNodeRef {
    pub(crate) inner: NodeRef,
}

#[pymethods]
impl PyNodeRef {
    #[new]
    fn new(kind: &str, index: usize) -> PyResult<Self> {
        Ok(PyNodeRef {
            inner: NodeRef {
                kind: self::kind(kind)?,
                index,
            },
        })
    }

    /// `"entry"` or `"reply"`.
    #[getter]
    fn kind(&self) -> &'static str {
        self.inner.kind.name()
    }

    #[getter]
    fn index(&self) -> usize {
        self.inner.index
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("NodeRef({})", self.inner)
    }
}

impl From<NodeRef> for PyNodeRef {
    fn from(inner: NodeRef) -> Self {
        PyNodeRef { inner }
    }
}

/// How to find one node: by its voice-over, its strref, its text, or
/// (fragile) its index.
#[pyclass(name = "NodeKey", module = "rakata", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyNodeKey {
    pub(crate) inner: NodeKey,
}

#[pymethods]
impl PyNodeKey {
    /// `voice:RESREF`, `strref:N`, `text:TEXT`, or an index as `E3`, `R0`,
    /// `entry:3`, `reply:0`. Raises `RE1104` on anything else.
    #[staticmethod]
    fn parse(text: &str) -> PyResult<Self> {
        Ok(PyNodeKey {
            inner: text.parse().map_err(fail)?,
        })
    }

    /// The node voiced by this resref, without regard to case.
    #[staticmethod]
    fn voice(resref: &str) -> PyResult<Self> {
        Ok(PyNodeKey {
            inner: NodeKey::Voice(ResRef::new(resref).map_err(fail)?),
        })
    }

    /// The node whose text is this strref.
    #[staticmethod]
    fn strref(strref: u32) -> Self {
        PyNodeKey {
            inner: NodeKey::StrRef(strref),
        }
    }

    /// The node whose text, trimmed, is this text, trimmed.
    #[staticmethod]
    fn text(text: String) -> Self {
        PyNodeKey {
            inner: NodeKey::Text(text),
        }
    }

    /// The node at this index; fragile, since indices move on a reorder.
    #[staticmethod]
    fn index(kind: &str, index: usize) -> PyResult<Self> {
        Ok(PyNodeKey {
            inner: NodeKey::Index(self::kind(kind)?, index),
        })
    }

    /// The stable key that names the node at `at` and no other.
    #[staticmethod]
    fn of(dialog: &PyDialog, at: &PyNodeRef) -> Self {
        PyNodeKey {
            inner: NodeKey::of(&dialog.inner, at.inner),
        }
    }

    /// Whether the key names a position rather than content.
    fn is_fragile(&self) -> bool {
        self.inner.is_fragile()
    }

    /// The one node of `dialog` the key names, text compared with the text
    /// a node embeds (`Mod.find` compares through the talk table too).
    /// Raises `RE1102` when no node matches and `RE1103` when several do.
    fn resolve(&self, dialog: &PyDialog) -> PyResult<PyNodeRef> {
        Ok(self.inner.resolve(&dialog.inner).map_err(fail)?.into())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("NodeKey({})", self.inner)
    }
}

/// A key from a `NodeKey`, a `NodeRef` (its index key, as an edit that
/// just made the node returns it) or the text `NodeKey.parse` takes.
pub(crate) fn key(obj: &Bound<'_, PyAny>) -> PyResult<NodeKey> {
    if let Ok(k) = obj.cast::<PyNodeKey>() {
        return Ok(k.get().inner.clone());
    }
    if let Ok(r) = obj.cast::<PyNodeRef>() {
        return Ok(NodeKey::from(r.get().inner));
    }
    if let Ok(s) = obj.extract::<&str>() {
        return s.parse().map_err(fail);
    }
    Err(PyTypeError::new_err(
        "a node key is a NodeKey, a NodeRef or str",
    ))
}

/// A link: from the start, or from a node to a node of the other kind.
#[pyclass(name = "Link", module = "rakata", frozen, get_all)]
#[derive(Clone)]
pub struct PyLink {
    /// The node it leads to.
    target: PyNodeRef,
    /// Its position in the list it lies in.
    position: usize,
    /// The condition script (`Active`), or `None`.
    condition: Option<String>,
    /// `IsChild`.
    is_child: bool,
}

impl From<&Link<'_>> for PyLink {
    fn from(l: &Link<'_>) -> Self {
        PyLink {
            target: l.target.into(),
            position: l.position,
            condition: l.condition().map(|r| r.to_string()),
            is_child: l.is_child(),
        }
    }
}

#[pymethods]
impl PyLink {
    fn __repr__(&self) -> String {
        format!("Link(-> {})", self.target.inner)
    }
}

/// One node of a dialogue, read from its struct when asked for.
#[pyclass(name = "Node", module = "rakata", frozen, get_all)]
pub struct PyNode {
    at: PyNodeRef,
    speaker: Option<String>,
    listener: Option<String>,
    /// The talk-table reference of its text.
    strref: Option<u32>,
    /// The text it embeds.
    text: Option<String>,
    voice: Option<String>,
    sound: Option<String>,
    script: Option<String>,
    delay: Option<u32>,
    /// `(participant, animation)` in order.
    animations: Vec<(String, u32)>,
    camera_angle: Option<u32>,
    camera_id: Option<i32>,
    quest: Option<String>,
    quest_entry: Option<u32>,
    comment: Option<String>,
    links: Vec<PyLink>,
}

impl From<&Node<'_>> for PyNode {
    fn from(n: &Node<'_>) -> Self {
        let s = |r: Option<ResRef>| r.map(|r| r.to_string());
        PyNode {
            at: n.at.into(),
            speaker: n.speaker(),
            listener: n.listener(),
            strref: n.strref(),
            text: n.embedded_text(),
            voice: s(n.voice()),
            sound: s(n.sound()),
            script: s(n.script()),
            delay: n.delay(),
            animations: n
                .animations()
                .into_iter()
                .map(|a| (a.participant, a.animation))
                .collect(),
            camera_angle: n.camera_angle(),
            camera_id: n.camera_id(),
            quest: n.quest(),
            quest_entry: n.quest_entry(),
            comment: n.comment(),
            links: n.links().iter().map(PyLink::from).collect(),
        }
    }
}

#[pymethods]
impl PyNode {
    fn __repr__(&self) -> String {
        format!("Node({})", self.at.inner)
    }
}

/// A dialogue: a DLG's GFF kept whole, read through the model.
#[pyclass(name = "Dialog", module = "rakata")]
#[derive(Clone)]
pub struct PyDialog {
    pub(crate) inner: Dialog,
}

#[pymethods]
impl PyDialog {
    /// Read a DLG's bytes, named `name` in messages (`banter.dlg`). Raises
    /// `RE1101` for a GFF of another type.
    #[staticmethod]
    fn parse(name: String, data: &[u8]) -> PyResult<Self> {
        Ok(PyDialog {
            inner: Dialog::parse(name, data).map_err(fail)?,
        })
    }

    /// The dialogue in a GFF of type `DLG`.
    #[staticmethod]
    #[pyo3(name = "new")]
    fn from_gff(name: String, gff: &PyGff) -> PyResult<Self> {
        Ok(PyDialog {
            inner: Dialog::new(name, gff.inner.clone()).map_err(fail)?,
        })
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// A copy of the GFF beneath.
    #[getter]
    fn gff(&self) -> PyGff {
        PyGff {
            inner: self.inner.gff().clone(),
        }
    }

    /// How many nodes of a kind (`"entry"`, `"reply"`) there are.
    fn len(&self, kind: &str) -> PyResult<usize> {
        Ok(self.inner.len(self::kind(kind)?))
    }

    /// The node at `at`, or `None` past the end of its list.
    fn node(&self, at: &PyNodeRef) -> Option<PyNode> {
        self.inner.node(at.inner).as_ref().map(PyNode::from)
    }

    /// Every node of a kind, in index order.
    fn iter(&self, kind: &str) -> PyResult<Vec<PyNode>> {
        Ok(self
            .inner
            .iter(self::kind(kind)?)
            .map(|n| PyNode::from(&n))
            .collect())
    }

    /// `StartingList`: the links into the entries, tried in order.
    fn starts(&self) -> Vec<PyLink> {
        self.inner.starts().iter().map(PyLink::from).collect()
    }

    /// The bytes: for a dialogue read and not changed, the bytes it was
    /// read from.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn __repr__(&self) -> String {
        format!("Dialog({:?})", self.inner.name())
    }
}

/// One reference of a dialogue the install cannot satisfy.
#[pyclass(name = "Finding", module = "rakata", frozen)]
#[derive(Clone)]
pub struct PyFinding {
    /// `RW1102` (script), `RW1104` (voice-over), `RW1105` (strref) or
    /// `RW1106` (global).
    #[pyo3(get)]
    code: &'static str,
    /// Where it lies: `E3`, `the link E3 -> R1`, `the start into E0`,
    /// `EndConversation`, `EndConverAbort`.
    #[pyo3(get)]
    place: String,
    /// The stable key of the node it lies on; `None` for the dialogue's
    /// end script.
    #[pyo3(get)]
    key: Option<PyNodeKey>,
    /// What is referred to.
    #[pyo3(get)]
    subject: String,
    /// For an undeclared global, the condition that reads it.
    #[pyo3(get)]
    via: Option<String>,
    inner: Finding,
}

impl From<Finding> for PyFinding {
    fn from(f: Finding) -> Self {
        PyFinding {
            code: f.code,
            place: f.place.to_string(),
            key: f.key.clone().map(|inner| PyNodeKey { inner }),
            subject: f.subject.clone(),
            via: f.via.clone(),
            inner: f,
        }
    }
}

#[pymethods]
impl PyFinding {
    /// The message without the code, naming the dialogue `dialog`.
    fn message(&self, dialog: &str) -> String {
        self.inner.message(dialog)
    }

    fn __repr__(&self) -> String {
        format!("Finding({} {} {})", self.code, self.place, self.subject)
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDialog>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<PyLink>()?;
    m.add_class::<PyNodeRef>()?;
    m.add_class::<PyNodeKey>()?;
    m.add_class::<PyFinding>()?;
    Ok(())
}
