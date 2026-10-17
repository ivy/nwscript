//! Checking recovered functions by recompiling them.
//!
//! The recovered script is compiled whole by a [`Compiler`] the caller
//! supplies, and each function's instructions are compared with the
//! original's. Functions are matched by the call graph, not by position: the
//! first function (the entry stub's target) with the first, then each pair's
//! calls in order. Within a matched pair, instructions must be equal, except
//! that a `JSR` matches when it calls the matching function.

use nwscript_ncs::{Instruction, Opcode as Op, Program};
use std::collections::{HashMap, VecDeque};

/// Why a compiler produced no bytecode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileError {
    /// What the compiler reported, first error first.
    pub message: String,
    /// The 1-based line of the first error, when the compiler names one.
    pub line: Option<u32>,
}

/// A compiler to verify with: a whole script's source bytes in, NCS bytes
/// out. Where includes and `nwscript.nss` come from is the implementation's
/// business; recovered scripts include nothing but need the engine's
/// declarations. Any `FnMut(&[u8]) -> Result<Vec<u8>, CompileError>` is one.
pub trait Compiler {
    fn compile(&mut self, source: &[u8]) -> Result<Vec<u8>, CompileError>;
}

impl<F: FnMut(&[u8]) -> Result<Vec<u8>, CompileError>> Compiler for F {
    fn compile(&mut self, source: &[u8]) -> Result<Vec<u8>, CompileError> {
        self(source)
    }
}

/// The functions of a program as contiguous instruction ranges: one starts
/// at the entry stub's target and at every `JSR` target; the first range
/// takes in the entry stub.
pub(crate) fn function_ranges(p: &Program) -> Vec<(usize, usize)> {
    let ins = p.instructions();
    let mut starts: Vec<usize> = ins
        .iter()
        .enumerate()
        .filter(|(_, x)| x.opcode == Op::Jsr)
        .filter_map(|(i, x)| x.jump_target(p.offsets()[i]).and_then(|t| p.index_at(t)))
        .collect();
    starts.sort_unstable();
    starts.dedup();
    let mut out = Vec::new();
    for (k, &s) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(ins.len());
        out.push((if k == 0 { 0 } else { s }, end));
    }
    out
}

fn calls(p: &Program, ranges: &[(usize, usize)], r: (usize, usize)) -> Vec<Option<usize>> {
    (r.0..r.1)
        .filter(|&i| p.instructions()[i].opcode == Op::Jsr)
        .map(|i| {
            let t = p.instructions()[i]
                .jump_target(p.offsets()[i])
                .and_then(|t| p.index_at(t))?;
            func_starting(ranges, t)
        })
        .collect()
}

fn func_starting(ranges: &[(usize, usize)], t: usize) -> Option<usize> {
    ranges.iter().position(|&(s, e)| s <= t && t < e)
}

/// For each original function, the recompiled function it matches.
/// Function ranges of the original and the recompiled program, and the
/// pairing between them.
type Pairing = (
    Vec<(usize, usize)>,
    Vec<(usize, usize)>,
    HashMap<usize, usize>,
);

fn pair(orig: &Program, re: &Program) -> Pairing {
    let ro = function_ranges(orig);
    let rr = function_ranges(re);
    let mut map = HashMap::new();
    if ro.is_empty() || rr.is_empty() {
        return (ro, rr, map);
    }
    map.insert(0, 0);
    let mut queue = VecDeque::from([(0usize, 0usize)]);
    while let Some((a, b)) = queue.pop_front() {
        let ca = calls(orig, &ro, ro[a]);
        let cb = calls(re, &rr, rr[b]);
        for (x, y) in ca.into_iter().zip(cb) {
            if let (Some(x), Some(y)) = (x, y)
                && !map.contains_key(&x)
            {
                map.insert(x, y);
                queue.push_back((x, y));
            }
        }
    }
    (ro, rr, map)
}

/// The outcome of comparing one original function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Compared {
    Same,
    /// The original offset of the first instruction that differs.
    DiffersAt(u32),
    /// No function of the recompiled program corresponds to it.
    Missing,
}

/// Compare every original function with its match in `re`.
pub(crate) fn compare(orig: &Program, re: &Program) -> Vec<Compared> {
    let (ro, rr, map) = pair(orig, re);
    let mut out = Vec::new();
    for (a, &(s, e)) in ro.iter().enumerate() {
        let Some(&b) = map.get(&a) else {
            out.push(Compared::Missing);
            continue;
        };
        let (s2, e2) = rr[b];
        let oi = &orig.instructions()[s..e];
        let ri = &re.instructions()[s2..e2];
        let mut res = Compared::Same;
        for k in 0..oi.len().max(ri.len()) {
            let same = match (oi.get(k), ri.get(k)) {
                (Some(x), Some(y)) => {
                    same_instruction(orig, s + k, x, re, s2 + k, y, &ro, &rr, &map)
                }
                _ => false,
            };
            if !same {
                let at = orig
                    .offsets()
                    .get(s + k)
                    .copied()
                    .unwrap_or(orig.file_len());
                res = Compared::DiffersAt(
                    at.min(orig.offsets().get(e).copied().unwrap_or(orig.file_len())),
                );
                break;
            }
        }
        out.push(res);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn same_instruction(
    orig: &Program,
    i: usize,
    x: &Instruction,
    re: &Program,
    j: usize,
    y: &Instruction,
    ro: &[(usize, usize)],
    rr: &[(usize, usize)],
    map: &HashMap<usize, usize>,
) -> bool {
    if x.opcode == Op::Jsr && y.opcode == Op::Jsr {
        let tx = x
            .jump_target(orig.offsets()[i])
            .and_then(|t| orig.index_at(t));
        let ty = y.jump_target(re.offsets()[j]).and_then(|t| re.index_at(t));
        let (Some(tx), Some(ty)) = (tx, ty) else {
            return false;
        };
        // A call matches when it calls the start of the matching function.
        let fx = func_starting(ro, tx);
        let fy = func_starting(rr, ty);
        return match (fx, fy) {
            (Some(fx), Some(fy)) => map.get(&fx) == Some(&fy) && tx - ro[fx].0 == ty - rr[fy].0,
            _ => false,
        };
    }
    x == y
}
