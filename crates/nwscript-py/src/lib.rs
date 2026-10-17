//! Python bindings of the nwscript façade, the same shape as the Lua
//! binding: each function is one library call, and results are plain
//! dicts, lists, bytes and strings.
//!
//! ```python
//! import nwscript
//! out = nwscript.compile(open("k_x.nss", "rb").read(), include_dirs=["scripts"], mode="bioware")
//! if out["ok"]:
//!     r = nwscript.run(out["ncs"], nwscript="scripts/nwscript.nss")
//!     print(r["result"], [c["name"] for c in r["trace"]])
//! ```

use nwscript::diag::{Diagnostic, SourceMap, json::to_json};
use nwscript::modes::{Mode, finish};
use nwscript::snapshot::CallRecord;
use nwscript::{Checker, Declarations, FsResolver, Game, Resolver};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use std::path::PathBuf;

fn to_py(py: Python<'_>, v: &serde_json::Value) -> PyResult<Py<PyAny>> {
    Ok(match v {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_pyobject(py)?.to_owned().into_any().unbind(),
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(i) => i.into_pyobject(py)?.into_any().unbind(),
            None => n
                .as_f64()
                .unwrap_or(0.0)
                .into_pyobject(py)?
                .into_any()
                .unbind(),
        },
        serde_json::Value::String(s) => s.into_pyobject(py)?.into_any().unbind(),
        serde_json::Value::Array(a) => {
            let l = PyList::empty(py);
            for x in a {
                l.append(to_py(py, x)?)?;
            }
            l.into_any().unbind()
        }
        serde_json::Value::Object(o) => {
            let d = PyDict::new(py);
            for (k, x) in o {
                d.set_item(k, to_py(py, x)?)?;
            }
            d.into_any().unbind()
        }
    })
}

fn game(g: &str) -> PyResult<Game> {
    match g {
        "k1" => Ok(Game::K1),
        "k2" => Ok(Game::K2),
        other => Err(PyValueError::new_err(format!(
            "game is \"k1\" or \"k2\", not {other:?}"
        ))),
    }
}

fn mode(m: &str) -> PyResult<Mode> {
    Mode::parse(m).ok_or_else(|| {
        PyValueError::new_err(format!(
            "mode is \"strict\", \"nwnnsscomp\" or \"bioware\", not {m:?}"
        ))
    })
}

fn declarations(d: &str) -> PyResult<Declarations> {
    match d {
        "shipped" => Ok(Declarations::Shipped),
        "bioware" => Ok(Declarations::BioWare),
        other => Err(PyValueError::new_err(format!(
            "declarations is \"shipped\" or \"bioware\", not {other:?}"
        ))),
    }
}

fn diags_json(ds: &[Diagnostic], map: &SourceMap) -> serde_json::Value {
    serde_json::to_value(ds.iter().map(|d| to_json(d, map)).collect::<Vec<_>>())
        .expect("diagnostics serialize")
}

struct Checked {
    map: SourceMap,
    diags: Vec<Diagnostic>,
    program: Option<nwscript::sema::Analysis>,
    inputs: Vec<nwscript::InputRecord>,
}

#[allow(clippy::too_many_arguments)]
fn check_source(
    source: &[u8],
    name: &str,
    include_dirs: Vec<PathBuf>,
    nwscript: Option<PathBuf>,
    g: Game,
    m: Mode,
    decl: Declarations,
) -> Checked {
    let mut map = SourceMap::new();
    let mut resolver = FsResolver::new(include_dirs, nwscript);
    let out = Checker::with_declarations(decl, g).check(&mut resolver, &mut map, name, source);
    let diags = finish(out.analysis.as_ref(), out.diagnostics, &map, &m.levels());
    Checked {
        inputs: resolver.inputs().to_vec(),
        map,
        diags,
        program: out.analysis,
    }
}

/// Check a script: front end, semantic analysis and lints. Returns
/// `{"ok", "diagnostics", "inputs"}`.
#[pyfunction]
#[pyo3(signature = (source, name="script.nss", include_dirs=vec![], nwscript=None, game="k1", mode="strict", declarations="shipped"))]
#[allow(clippy::too_many_arguments)]
fn check(
    py: Python<'_>,
    source: &[u8],
    name: &str,
    include_dirs: Vec<PathBuf>,
    nwscript: Option<PathBuf>,
    game: &str,
    mode: &str,
    declarations: &str,
) -> PyResult<Py<PyAny>> {
    let c = check_source(
        source,
        name,
        include_dirs,
        nwscript,
        self::game(game)?,
        self::mode(mode)?,
        self::declarations(declarations)?,
    );
    let ok = !nwscript::diag::has_errors(&c.diags);
    to_py(
        py,
        &serde_json::json!({ "ok": ok, "diagnostics": diags_json(&c.diags, &c.map), "inputs": c.inputs }),
    )
}

/// Compile a script to NCS. Returns `{"ok", "ncs" (bytes or None), "diagnostics", "inputs"}`.
#[pyfunction]
#[pyo3(signature = (source, name="script.nss", include_dirs=vec![], nwscript=None, game="k1", mode="strict", declarations="shipped"))]
#[allow(clippy::too_many_arguments)]
fn compile(
    py: Python<'_>,
    source: &[u8],
    name: &str,
    include_dirs: Vec<PathBuf>,
    nwscript: Option<PathBuf>,
    game: &str,
    mode: &str,
    declarations: &str,
) -> PyResult<Py<PyAny>> {
    let g = self::game(game)?;
    let mut c = check_source(
        source,
        name,
        include_dirs,
        nwscript,
        g,
        self::mode(mode)?,
        self::declarations(declarations)?,
    );
    let mut ncs = None;
    if !nwscript::diag::has_errors(&c.diags)
        && let Some(a) = &c.program
    {
        match nwscript::compile::compile(&a.program, g) {
            Ok(p) => ncs = Some(nwscript::ncs::encode(&p)),
            Err(d) => c.diags.push(d),
        }
    }
    let d = PyDict::new(py);
    d.set_item("ok", ncs.is_some())?;
    d.set_item("ncs", ncs.map(|b| PyBytes::new(py, &b).unbind()))?;
    d.set_item("diagnostics", to_py(py, &diags_json(&c.diags, &c.map))?)?;
    d.set_item(
        "inputs",
        to_py(
            py,
            &serde_json::to_value(&c.inputs).expect("inputs serialize"),
        )?,
    )?;
    Ok(d.into_any().unbind())
}

fn table(nwscript: Option<PathBuf>) -> PyResult<Option<nwscript::sema::EngineTable>> {
    let Some(p) = nwscript else { return Ok(None) };
    let bytes =
        std::fs::read(&p).map_err(|e| PyValueError::new_err(format!("{}: {e}", p.display())))?;
    Ok(Some(
        nwscript::sema::EngineTable::from_nwscript(nwscript::diag::FileId(0), &bytes).0,
    ))
}

fn decode(ncs: &[u8]) -> PyResult<nwscript::ncs::Program> {
    nwscript::ncs::decode(nwscript::diag::FileId(0), ncs)
        .map_err(|d| PyValueError::new_err(d.message))
}

/// NCS bytes as assembly text; routine names come from `nwscript` when given.
#[pyfunction]
#[pyo3(signature = (ncs, nwscript=None))]
fn disassemble(ncs: &[u8], nwscript: Option<PathBuf>) -> PyResult<String> {
    let program = decode(ncs)?;
    let t = table(nwscript)?;
    let names = |r: u16| {
        t.as_ref()
            .and_then(|t| t.functions.get(usize::from(r)))
            .map(|f| f.name.clone())
    };
    let opts = nwscript::bytecode::PrintOptions {
        raw: false,
        routine_name: t.as_ref().map(|_| &names as &dyn Fn(u16) -> Option<String>),
    };
    Ok(nwscript::ncs::asm::print(&program, &opts))
}

/// Assembly text as NCS bytes.
#[pyfunction]
fn assemble(py: Python<'_>, text: &str) -> PyResult<Py<PyBytes>> {
    let p =
        nwscript::ncs::asm::parse(nwscript::diag::FileId(0), text.as_bytes()).map_err(|ds| {
            PyValueError::new_err(
                ds.iter()
                    .map(|d| d.message.clone())
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
    Ok(PyBytes::new(py, &nwscript::ncs::encode(&p)).unbind())
}

/// Run NCS under the recording engine or an empty world. Returns
/// `{"result", "trace", "unmodelled", "fault"}`; each call is
/// `{"name", "args", "ret", "modelled", "deferred"}` with values as text.
#[pyfunction]
#[pyo3(signature = (ncs, nwscript, engine="recording", seed=0))]
fn run(
    py: Python<'_>,
    ncs: &[u8],
    nwscript: PathBuf,
    engine: &str,
    seed: u64,
) -> PyResult<Py<PyAny>> {
    let program = decode(ncs)?;
    let t = table(Some(nwscript))?.expect("a path was given");
    let routines = nwscript::engine::routines(&t);
    let mut rec = nwscript::vm::Recording;
    let mut world = nwscript::vm::World::new(seed);
    let e: &mut dyn nwscript::vm::Engine = match engine {
        "recording" => &mut rec,
        "world" => &mut world,
        other => {
            return Err(PyValueError::new_err(format!(
                "engine is \"recording\" or \"world\", not {other:?}"
            )));
        }
    };
    let r = nwscript::vm::run(&program, &routines, e, nwscript::vm::Limits::default());
    let calls: Vec<CallRecord> = r.trace.iter().map(CallRecord::from).collect();
    to_py(
        py,
        &serde_json::json!({
            "result": r.result.map(|v| v.to_string()),
            "trace": calls.iter().map(|c| serde_json::json!({ "name": c.name, "args": c.args, "ret": c.ret, "modelled": c.modelled, "deferred": c.deferred })).collect::<Vec<_>>(),
            "unmodelled": r.unmodelled,
            "fault": r.fault.map(|f| f.to_string()),
        }),
    )
}

/// Compare two NCS files by bytes, instructions and, with `nwscript`, trace.
#[pyfunction]
#[pyo3(signature = (a, b, nwscript=None))]
fn diff(py: Python<'_>, a: &[u8], b: &[u8], nwscript: Option<PathBuf>) -> PyResult<Py<PyAny>> {
    let (pa, pb) = (decode(a)?, decode(b)?);
    let mut out = serde_json::json!({
        "bytes": nwscript::compare::bytes(a, b),
        "instructions": nwscript::compare::instructions(&pa, &pb),
    });
    if let Some(t) = table(nwscript)? {
        let routines = nwscript::engine::routines(&t);
        let mut make = || -> Box<dyn nwscript::vm::Engine> { Box::new(nwscript::vm::Recording) };
        out["trace"] =
            serde_json::to_value(nwscript::compare::trace(&pa, &pb, &routines, &mut make))
                .expect("serializes");
    }
    to_py(py, &out)
}

#[pymodule(name = "nwscript")]
fn nwscript_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(disassemble, m)?)?;
    m.add_function(wrap_pyfunction!(assemble, m)?)?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(diff, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
