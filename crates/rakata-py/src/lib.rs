//! Python bindings of the `rakata` façade: the module `rakata`, the same
//! names and operations as the Rust API, with `rakata.nwscript` (the
//! `nwscript-py` functions, mounted from that crate) and `rakata.author`
//! (the authoring API, `rakata::author`, by its submodules).
//!
//! ```python
//! import rakata
//! install = rakata.Install("/path/to/swkotor")
//! print(install.which("globalcat", "2da").hash)
//! m = rakata.Mod.open(install, "danm14ac")
//! table = rakata.TwoDa.parse(m.loaded("globalcat.2da").bytes)
//! rakata.author.globals.declare(table, "OAJ_DONE", "Boolean")
//! m.write_override("globalcat.2da", table.to_bytes())
//! ```
//!
//! Every refusal is raised as `rakata.RakataError`, whose `code` is the
//! registered diagnostic code the `rakata` binary prints and whose
//! `message` is the text after it; `str()` of it is `CODE: message`.
//! Arguments of the wrong Python type raise `TypeError` or `ValueError`.

mod author;
mod dialog;
mod formats;
mod game;
mod value;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyDict;

create_exception!(
    rakata,
    RakataError,
    PyException,
    "A refusal of the library, with the registered diagnostic code (`code`) and the message after it (`message`)."
);

/// The exception for a refusal with this code and message.
pub(crate) fn raise(code: &str, message: String) -> PyErr {
    Python::attach(|py| {
        let err = RakataError::new_err(format!("{code}: {message}"));
        let v = err.value(py);
        // Setting an attribute on a fresh exception instance does not fail.
        let _ = v.setattr("code", code);
        let _ = v.setattr("message", message);
        err
    })
}

/// The exception for any of the library's errors, by the authoring API's
/// error, which every format's and the install's convert into with their
/// codes unchanged.
pub(crate) fn fail(e: impl Into<rakata::author::Error>) -> PyErr {
    let e = e.into();
    raise(e.code(), e.message())
}

/// A module `name` (`rakata.author.twoda`) as the attribute `short` of
/// `parent`, and in `sys.modules`, so `import rakata.author.twoda` finds it.
pub(crate) fn submodule<'py>(
    parent: &Bound<'py, PyModule>,
    name: &str,
    short: &str,
) -> PyResult<Bound<'py, PyModule>> {
    let py = parent.py();
    let m = PyModule::new(py, name)?;
    parent.add(short, &m)?;
    py.import("sys")?
        .getattr("modules")?
        .cast_into::<PyDict>()?
        .set_item(name, &m)?;
    Ok(m)
}

#[pymodule(name = "rakata")]
fn rakata_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("__version__", rakata::VERSION)?;
    m.add("VERSION", rakata::VERSION)?;
    m.add("RakataError", py.get_type::<RakataError>())?;
    game::register(m)?;
    formats::register(m)?;
    dialog::register(m)?;
    let nws = submodule(m, "rakata.nwscript", "nwscript")?;
    nwscript_py::register(&nws)?;
    author::register(m)?;
    Ok(())
}
