//! Comparing two programs at three levels: their bytes, their instructions
//! with jumps resolved to the instruction they reach, and their behaviour
//! under an engine.

use crate::snapshot::CallRecord;
use nwscript_ncs::{Operands, Program};
use nwscript_vm::{Engine, Limits, Routine, run};
use serde::Serialize;

/// How two programs compare at one level.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Level {
    Same,
    /// Where they first differ.
    Different {
        at: String,
    },
    Skipped {
        why: String,
    },
}

impl Level {
    pub fn is_different(&self) -> bool {
        matches!(self, Level::Different { .. })
    }
}

/// The first byte at which two files differ.
pub fn bytes(a: &[u8], b: &[u8]) -> Level {
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(i) => Level::Different {
            at: format!("byte {i}: {:#04x} against {:#04x}", a[i], b[i]),
        },
        None if a.len() != b.len() => Level::Different {
            at: format!("lengths {} and {}", a.len(), b.len()),
        },
        None => Level::Same,
    }
}

/// An instruction with its jump as the index of the instruction it reaches,
/// so programs laid out at different offsets compare by meaning.
fn normalized(p: &Program) -> Vec<String> {
    p.iter()
        .map(|(at, i)| match (&i.operands, i.jump_target(at)) {
            (Operands::Jump(_), Some(t)) => match p.index_at(t) {
                Some(idx) => format!("{} @{idx}", i.opcode.mnemonic(i.ty)),
                None => i.to_string(),
            },
            _ => i.to_string(),
        })
        .collect()
}

/// The first instruction at which two programs differ.
pub fn instructions(a: &Program, b: &Program) -> Level {
    let (na, nb) = (normalized(a), normalized(b));
    for (k, (x, y)) in na.iter().zip(&nb).enumerate() {
        if x != y {
            return Level::Different {
                at: format!(
                    "instruction {k} (offsets {} and {}): {} against {}",
                    a.offsets()[k],
                    b.offsets()[k],
                    a.instructions()[k],
                    b.instructions()[k]
                ),
            };
        }
    }
    if na.len() != nb.len() {
        return Level::Different {
            at: format!("{} instructions against {}", na.len(), nb.len()),
        };
    }
    Level::Same
}

/// The first engine call, result or stop at which two programs' runs differ,
/// each against a fresh engine from `engine`.
pub fn trace(
    a: &Program,
    b: &Program,
    routines: &[Routine],
    engine: &mut dyn FnMut() -> Box<dyn Engine>,
) -> Level {
    let ra = run(a, routines, engine().as_mut(), Limits::default());
    let rb = run(b, routines, engine().as_mut(), Limits::default());
    let (ca, cb): (Vec<CallRecord>, Vec<CallRecord>) = (
        ra.trace.iter().map(CallRecord::from).collect(),
        rb.trace.iter().map(CallRecord::from).collect(),
    );
    if let Some(i) = ca.iter().zip(&cb).position(|(x, y)| x != y) {
        return Level::Different {
            at: format!("call {}: {} against {}", i + 1, ca[i], cb[i]),
        };
    }
    if ca.len() != cb.len() {
        let (more, extra) = if ca.len() > cb.len() {
            ("first", &ca[cb.len()])
        } else {
            ("second", &cb[ca.len()])
        };
        return Level::Different {
            at: format!(
                "the {more} makes more calls, starting with call {}: {extra}",
                ca.len().min(cb.len()) + 1
            ),
        };
    }
    if ra.result != rb.result {
        return Level::Different {
            at: format!(
                "results {:?} and {:?}",
                ra.result.map(|v| v.to_string()),
                rb.result.map(|v| v.to_string())
            ),
        };
    }
    let (fa, fb) = (
        ra.fault.map(|f| f.to_string()),
        rb.fault.map(|f| f.to_string()),
    );
    if fa != fb {
        return Level::Different {
            at: format!("stops {fa:?} and {fb:?}"),
        };
    }
    Level::Same
}
