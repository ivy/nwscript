//! The engine calls a program makes, with the arguments that are literals:
//! what an index records of a script without decompiling it.
//!
//! Arguments are pushed last first, so the instruction just before an
//! `ACTION` pushes its first argument. Walking back from the `ACTION`, each
//! argument that a single `CONST*` instruction pushes (three `CONSTF`s for a
//! vector) is a literal; the walk stops at the first argument something
//! else computes, and that argument and every later one are unknown. A
//! routine's number names it only through the engine's table, which the
//! caller supplies as the type of each parameter.

use crate::{Operands, Program};

/// A parameter's type, as far as reading a literal needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Int,
    Float,
    String,
    Object,
    /// Three floats.
    Vector,
    /// An engine structure, an action, or anything else no constant pushes.
    Other,
}

/// A literal argument.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i32),
    Float(f32),
    /// The string's bytes, Windows-1252.
    String(Vec<u8>),
    /// An object constant: `0` is `OBJECT_SELF`, `1` is `OBJECT_INVALID`.
    Object(i32),
    Vector([f32; 3]),
}

/// One `ACTION`.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The instruction's file offset.
    pub offset: u32,
    /// The routine's number in the engine's table.
    pub routine: u16,
    /// One entry per argument the instruction passes: the literal, or
    /// `None` where the value is computed.
    pub args: Vec<Option<Literal>>,
}

/// Every engine call in `program`, in instruction order. `param` gives the
/// type of argument `k` of routine `r`, `None` when the table does not
/// know; only a vector parameter changes how a literal is read.
pub fn calls(program: &Program, param: impl Fn(u16, usize) -> Option<ArgType>) -> Vec<Call> {
    let ins = program.instructions();
    let mut out = Vec::new();
    for (i, x) in ins.iter().enumerate() {
        let Operands::Action { routine, argc } = x.operands else {
            continue;
        };
        let mut args = Vec::with_capacity(usize::from(argc));
        let mut j = i;
        for k in 0..usize::from(argc) {
            let lit = if param(routine, k) == Some(ArgType::Vector) {
                let comps: Option<Vec<f32>> = (1..=3)
                    .map(|n| match j.checked_sub(n).map(|at| &ins[at].operands) {
                        Some(Operands::Float(v)) => Some(*v),
                        _ => None,
                    })
                    .collect();
                comps.map(|zyx| {
                    j -= 3;
                    Literal::Vector([zyx[2], zyx[1], zyx[0]])
                })
            } else {
                let lit = match j.checked_sub(1).map(|at| &ins[at].operands) {
                    Some(Operands::Int(v)) => Some(Literal::Int(*v)),
                    Some(Operands::Float(v)) => Some(Literal::Float(*v)),
                    Some(Operands::String(s)) => Some(Literal::String(s.clone())),
                    Some(Operands::Object(v)) => Some(Literal::Object(*v)),
                    _ => None,
                };
                if lit.is_some() {
                    j -= 1;
                }
                lit
            };
            match lit {
                Some(l) => args.push(Some(l)),
                None => break,
            }
        }
        args.resize(usize::from(argc), None);
        out.push(Call {
            offset: program.offsets()[i],
            routine,
            args,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nwscript_diag::FileId;

    #[test]
    fn literals_before_the_action_and_none_after_a_computed_argument() {
        // ExecuteScript("k_pman_aioff", oLocal, -1): the object is computed,
        // so the default after it is unknown too; SetFacingPoint takes a
        // vector literal; GetHitDice(OBJECT_SELF) an object constant.
        let program = crate::asm::parse(
            FileId(1),
            b"JSR main\nRETN\nmain:\nCONSTI -1\nCPTOPSP -8, 4\nCONSTS \"k_pman_aioff\"\nACTION 0, 3\nCONSTF 1.0\nCONSTF 2.0\nCONSTF 3.0\nACTION 1, 1\nCONSTO 0\nACTION 2, 1\nMOVSP -4\nRETN\n",
        )
        .unwrap();
        let param = |r: u16, k: usize| match (r, k) {
            (0, 0) => Some(ArgType::String),
            (0, 1) => Some(ArgType::Object),
            (0, 2) => Some(ArgType::Int),
            (1, 0) => Some(ArgType::Vector),
            (2, 0) => Some(ArgType::Object),
            _ => None,
        };
        let c = calls(&program, param);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].routine, 0);
        assert_eq!(
            c[0].args,
            [Some(Literal::String(b"k_pman_aioff".to_vec())), None, None]
        );
        assert_eq!(c[1].args, [Some(Literal::Vector([1.0, 2.0, 3.0]))]);
        assert_eq!(c[2].args, [Some(Literal::Object(0))]);
        assert!(c[0].offset < c[1].offset && c[1].offset < c[2].offset);
    }
}
