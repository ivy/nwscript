//! The engine calls a program makes, with the arguments that are literals,
//! and the string constants it holds: what an index records of a script
//! without decompiling it.
//!
//! Arguments are pushed last first, so the instruction just before an
//! `ACTION` pushes its first argument. Each argument is read on its own: it
//! is a literal when one `CONST*` instruction pushes its stack slot (three
//! `CONSTF`s for a vector) and nothing between it and the `ACTION` pops,
//! changes or overwrites that slot, and unknown otherwise. The slots come
//! from the stack depth before each instruction
//! ([`stack_depths`](crate::cfg::stack_depths)), which needs the size of
//! each routine's parameters and return value; where a depth is unknown,
//! the literals read are those the `CONST*` instructions directly before
//! the `ACTION` push, up to the first argument something else computes. A
//! routine's number names it only through the engine's table, which the
//! caller supplies as the type of each parameter and return value.

use crate::cfg::{self, Cfg, Depths};
use crate::{Opcode, Operands, Program, Type};

/// A parameter's or return value's type, as far as reading a literal and
/// sizing a stack slot need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Int,
    Float,
    String,
    Object,
    /// Three floats.
    Vector,
    /// An `action`: a deferred call, which takes no stack slot.
    Action,
    /// No value: what a routine that returns nothing returns.
    Void,
    /// An engine structure, or anything else of one stack element that no
    /// constant pushes.
    Other,
}

impl ArgType {
    /// Bytes on the stack.
    pub const fn size(self) -> i32 {
        match self {
            ArgType::Vector => 12,
            ArgType::Action | ArgType::Void => 0,
            _ => 4,
        }
    }
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
/// type of argument `k` of routine `r` and `ret` the type routine `r`
/// returns, `None` where the table does not know; an argument past the
/// table's parameters is taken to be one stack element.
pub fn calls(
    program: &Program,
    param: impl Fn(u16, usize) -> Option<ArgType>,
    ret: impl Fn(u16) -> Option<ArgType>,
) -> Vec<Call> {
    let ins = program.instructions();
    let size = |r: u16, k: usize| param(r, k).map_or(4, ArgType::size);
    let arg_bytes = |r: u16, argc: u8| -> i32 { (0..usize::from(argc)).map(|k| size(r, k)).sum() };
    let graph = cfg::build(program);
    let depths = cfg::stack_depths(program, &graph, &|r, argc| {
        ret(r).map(|t| t.size() - arg_bytes(r, argc))
    });
    let reader = Slots {
        program,
        graph: &graph,
        depths: &depths,
        arg_bytes: &arg_bytes,
    };
    let mut out = Vec::new();
    for (i, x) in ins.iter().enumerate() {
        let Operands::Action { routine, argc } = x.operands else {
            continue;
        };
        let mut args = adjacent(program, i, routine, argc, &param);
        if let Some(depth) = depths.before[i] {
            let mut hi = depth;
            for (k, arg) in args.iter_mut().enumerate() {
                let lo = hi - size(routine, k);
                if arg.is_none() && lo < hi {
                    *arg = reader.literal(i, lo, hi, param(routine, k));
                }
                hi = lo;
            }
        }
        out.push(Call {
            offset: program.offsets()[i],
            routine,
            args,
        });
    }
    out
}

/// Every string constant in `program` (each `CONSTS`), with the
/// instruction's file offset, in instruction order.
pub fn strings(program: &Program) -> Vec<(u32, Vec<u8>)> {
    program
        .iter()
        .filter_map(|(at, x)| match (&x.opcode, &x.operands) {
            (Opcode::Const, Operands::String(s)) => Some((at, s.clone())),
            _ => None,
        })
        .collect()
}

/// The arguments of the `ACTION` at `at` that the `CONST*` instructions
/// directly before it push, up to the first computed one; `None` from
/// there on.
fn adjacent(
    program: &Program,
    at: usize,
    routine: u16,
    argc: u8,
    param: &impl Fn(u16, usize) -> Option<ArgType>,
) -> Vec<Option<Literal>> {
    let ins = program.instructions();
    let mut args = Vec::with_capacity(usize::from(argc));
    let mut j = at;
    for k in 0..usize::from(argc) {
        let lit = match param(routine, k) {
            Some(ArgType::Action) => None,
            Some(ArgType::Vector) => {
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
            }
            _ => {
                let lit = j.checked_sub(1).and_then(|at| constant(&ins[at].operands));
                if lit.is_some() {
                    j -= 1;
                }
                lit
            }
        };
        match lit {
            Some(l) => args.push(Some(l)),
            None => break,
        }
    }
    args.resize(usize::from(argc), None);
    args
}

/// The literal a `CONST*` instruction's operands push.
fn constant(o: &Operands) -> Option<Literal> {
    match o {
        Operands::Int(v) => Some(Literal::Int(*v)),
        Operands::Float(v) => Some(Literal::Float(*v)),
        Operands::String(s) => Some(Literal::String(s.clone())),
        Operands::Object(v) => Some(Literal::Object(*v)),
        _ => None,
    }
}

/// What reading a stack slot needs: the program, its graph and depths, and
/// the bytes of arguments each `ACTION` takes.
struct Slots<'a> {
    program: &'a Program,
    graph: &'a Cfg,
    depths: &'a Depths,
    arg_bytes: &'a dyn Fn(u16, u8) -> i32,
}

impl Slots<'_> {
    /// The literal in the stack slot `lo..hi` (bytes from the subroutine's
    /// entry) when the `ACTION` at `at` runs: what the `CONST*`
    /// instructions that push it push, provided every instruction from
    /// them to the `ACTION` is of the same subroutine and of known depth,
    /// and leaves the slot as it is.
    fn literal(&self, at: usize, lo: i32, hi: i32, ty: Option<ArgType>) -> Option<Literal> {
        let ins = self.program.instructions();
        let before = &self.depths.before;
        let sub = self.graph.blocks[self.graph.block_of[at]].sub?;
        let same = |m: usize| self.graph.blocks[self.graph.block_of[m]].sub == Some(sub);
        // The instruction that pushes the slot: the last one before the
        // ACTION that starts at or below the slot's bottom.
        let mut p = at;
        loop {
            p = p.checked_sub(1)?;
            if !same(p) {
                return None;
            }
            if before[p]? <= lo {
                break;
            }
        }
        if before[p]? != lo {
            return None;
        }
        let (lit, n) = if ty == Some(ArgType::Vector) {
            let mut v = [0f32; 3];
            for (c, x) in v.iter_mut().enumerate() {
                let m = p + c;
                match (&ins.get(m)?.opcode, &ins[m].operands) {
                    (Opcode::Const, Operands::Float(f)) if before[m] == Some(lo + 4 * c as i32) => {
                        *x = *f
                    }
                    _ => return None,
                }
            }
            (Literal::Vector(v), 3)
        } else if ins[p].opcode == Opcode::Const && hi - lo == 4 {
            (constant(&ins[p].operands)?, 1)
        } else {
            return None;
        };
        for m in p + n..at {
            let d = before[m]?;
            if !same(m) || d - self.pops(m)? < hi {
                return None;
            }
            // A copy into the slot, or an increment of it, changes it.
            let write = match (&ins[m].opcode, &ins[m].operands) {
                (Opcode::CpDownSp, Operands::Stack { offset, size }) => {
                    Some((d + offset, i32::from(*size)))
                }
                (Opcode::DecISp | Opcode::IncISp, Operands::Offset(o)) => Some((d + o, 4)),
                _ => None,
            };
            if let Some((start, len)) = write
                && start < hi
                && start + len > lo
            {
                return None;
            }
        }
        Some(lit)
    }

    /// The bytes instruction `m` takes off the stack before it pushes what
    /// it pushes; `None` where that is not known.
    fn pops(&self, m: usize) -> Option<i32> {
        use Opcode::*;
        let x = &self.program.instructions()[m];
        Some(match (x.opcode, &x.operands) {
            (
                Const | RsAdd | CpTopSp | CpTopBp | CpDownSp | CpDownBp | SaveBp | Nop | Jmp
                | StoreState | StoreStateAll | DecISp | IncISp | DecIBp | IncIBp,
                _,
            ) => 0,
            (MovSp, Operands::Offset(o)) => (-o).max(0),
            (Jz | Jnz | RestoreBp | Neg | Comp | Not, _) => 4,
            (Destruct, Operands::Destruct { size, .. }) => i32::from(*size),
            (Action, Operands::Action { routine, argc }) => (self.arg_bytes)(*routine, *argc),
            (Jsr, _) => (self.depths.before[m]? - self.depths.before.get(m + 1).copied()??).max(0),
            (Equal | NEqual, Operands::Size(s)) => 2 * i32::from(*s),
            (Add | Sub | Mul | Div, _) => match x.ty {
                Type::VECTOR_VECTOR => 24,
                Type::VECTOR_FLOAT | Type::FLOAT_VECTOR => 16,
                _ => 8,
            },
            (
                LogAnd | LogOr | IncOr | ExcOr | BoolAnd | Equal | NEqual | Geq | Gt | Lt | Leq
                | ShLeft | ShRight | UShRight | Mod,
                _,
            ) => 8,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nwscript_diag::FileId;

    fn program(asm: &str) -> Program {
        crate::asm::parse(FileId(1), asm.as_bytes()).unwrap()
    }

    /// Routine 0 `ExecuteScript(string, object, int)`, 1
    /// `SetFacingPoint(vector)`, 2 `GetHitDice(object)` returning `int`, 3
    /// `GetFirstPC()` returning `object`, 4 `GetItemPossessedBy(object,
    /// string)` returning `object`.
    fn param(r: u16, k: usize) -> Option<ArgType> {
        match (r, k) {
            (0, 0) => Some(ArgType::String),
            (0, 1) => Some(ArgType::Object),
            (0, 2) => Some(ArgType::Int),
            (1, 0) => Some(ArgType::Vector),
            (2, 0) => Some(ArgType::Object),
            (4, 0) => Some(ArgType::Object),
            (4, 1) => Some(ArgType::String),
            _ => None,
        }
    }

    fn ret(r: u16) -> Option<ArgType> {
        match r {
            0 | 1 => Some(ArgType::Void),
            2 => Some(ArgType::Int),
            3 | 4 => Some(ArgType::Object),
            _ => None,
        }
    }

    fn string(s: &str) -> Option<Literal> {
        Some(Literal::String(s.as_bytes().to_vec()))
    }

    #[test]
    fn literals_before_the_action_and_after_a_computed_argument() {
        // ExecuteScript("k_pman_aioff", oLocal, -1): the object is a local's
        // copy, and the default after it is still read; SetFacingPoint takes
        // a vector literal; GetHitDice(OBJECT_SELF) an object constant.
        let p = program(
            "JSR main\nRETN\nmain:\nCONSTI -1\nCPTOPSP -8, 4\nCONSTS \"k_pman_aioff\"\nACTION 0, 3\nCONSTF 1.0\nCONSTF 2.0\nCONSTF 3.0\nACTION 1, 1\nCONSTO 0\nACTION 2, 1\nMOVSP -4\nRETN\n",
        );
        let c = calls(&p, param, ret);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].routine, 0);
        assert_eq!(
            c[0].args,
            [string("k_pman_aioff"), None, Some(Literal::Int(-1))]
        );
        assert_eq!(c[1].args, [Some(Literal::Vector([1.0, 2.0, 3.0]))]);
        assert_eq!(c[2].args, [Some(Literal::Object(0))]);
        assert!(c[0].offset < c[1].offset && c[1].offset < c[2].offset);
    }

    #[test]
    fn a_literal_after_a_stack_operand_or_a_nested_call_is_read() {
        // object oPC = GetFirstPC();
        // GetItemPossessedBy(oPC, "k37_itm_freedont");
        // GetItemPossessedBy(GetFirstPC(), "k37_itm_freedont");
        let p = program(
            "JSR main\nRETN\nmain:\nACTION 3, 0\nCONSTS \"k37_itm_freedont\"\nCPTOPSP -8, 4\nACTION 4, 2\nMOVSP -4\nCONSTS \"k37_itm_freedont\"\nACTION 3, 0\nACTION 4, 2\nMOVSP -4\nMOVSP -4\nRETN\n",
        );
        let c = calls(&p, param, ret);
        assert_eq!(c.len(), 4);
        assert_eq!(c[1].args, [None, string("k37_itm_freedont")]);
        assert_eq!(c[3].args, [None, string("k37_itm_freedont")]);
    }

    #[test]
    fn a_computed_or_overwritten_slot_is_not_a_literal() {
        // GetItemPossessedBy(oPC, "k37_" + "itm"): the slot's first
        // constant is popped by the addition.
        let p = program(
            "JSR main\nRETN\nmain:\nACTION 3, 0\nCONSTS \"k37_\"\nCONSTS \"itm\"\nADDSS\nCPTOPSP -8, 4\nACTION 4, 2\nMOVSP -8\nRETN\n",
        );
        assert_eq!(calls(&p, param, ret)[1].args, [None, None]);
        // ExecuteScript("a", OBJECT_SELF, -1) with the int slot negated
        // and the string slot overwritten from a local before the call.
        let p = program(
            "JSR main\nRETN\nmain:\nCONSTS \"x\"\nCONSTI 1\nNEGI\nCONSTO 0\nCONSTS \"a\"\nCPTOPSP -16, 4\nCPDOWNSP -8, 4\nMOVSP -4\nACTION 0, 3\nMOVSP -4\nRETN\n",
        );
        assert_eq!(
            calls(&p, param, ret)[0].args,
            [None, Some(Literal::Object(0)), None]
        );
    }

    #[test]
    fn without_routine_types_the_adjacent_literals_are_read() {
        let p = program(
            "JSR main\nRETN\nmain:\nACTION 9, 0\nCONSTS \"tag\"\nCPTOPSP -8, 4\nACTION 4, 2\nCONSTS \"k_pman_aioff\"\nACTION 4, 1\nMOVSP -8\nRETN\n",
        );
        let c = calls(&p, |_, _| None, |_| None);
        assert_eq!(c[1].args, [None, None]);
        assert_eq!(c[2].args, [string("k_pman_aioff")]);
    }

    #[test]
    fn every_string_constant_with_its_offset() {
        let p = program(
            "JSR main\nRETN\nmain:\nCONSTS \"a\"\nCONSTI 1\nCONSTS \"b\"\nMOVSP -12\nRETN\n",
        );
        let s = strings(&p);
        assert_eq!(
            s.iter().map(|(_, v)| v.as_slice()).collect::<Vec<_>>(),
            [b"a".as_slice(), b"b"]
        );
        assert!(s[0].0 < s[1].0);
    }
}
