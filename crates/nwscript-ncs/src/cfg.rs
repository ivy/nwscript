//! The control-flow view of a program: basic blocks, the subroutines they
//! belong to, and the static stack depth before each instruction.
//!
//! Subroutines are the program's entry, every `JSR` target, and every
//! `STORE_STATE` resume point (a deferred action's body, which the engine
//! runs later with the saved stack). A block belongs to the subroutine from
//! whose entry it is reached without following a call.
//!
//! Stack depth is in bytes, relative to the subroutine's entry. An `ACTION`
//! changes the depth by an amount only the engine function table knows, so
//! [`stack_depths`] takes it from the caller; where it is unknown, so is the
//! depth after it.

use crate::{Opcode, Operands, Program, Type};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubKind {
    /// Where execution starts: the first instruction.
    Entry,
    /// The target of a `JSR`.
    Call,
    /// A `STORE_STATE` resume point: the body of a deferred action.
    Closure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// Index of the first instruction.
    pub start: usize,
    /// Index one past the last instruction.
    pub end: usize,
    pub succs: Vec<usize>,
    pub preds: Vec<usize>,
    /// The subroutine this block belongs to, when any subroutine reaches it.
    pub sub: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sub {
    pub kind: SubKind,
    /// The block the subroutine starts at.
    pub entry: usize,
    pub blocks: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Cfg {
    pub blocks: Vec<Block>,
    pub subs: Vec<Sub>,
    /// The block of each instruction, by instruction index.
    pub block_of: Vec<usize>,
    /// Blocks reached from more than one subroutine, which `sub` cannot name.
    pub shared: Vec<usize>,
    /// Jump edges that can never be taken, left out of `succs`: (from block, to block).
    pub infeasible: Vec<(usize, usize)>,
}

/// `CPTOPSP -4, 4`: a copy of the value on top of the stack.
fn is_dup(i: &crate::Instruction) -> bool {
    i.opcode == Opcode::CpTopSp
        && i.operands
            == Operands::Stack {
                offset: -4,
                size: 4,
            }
}

/// The resume point of a `STORE_STATE` at `at`: its type byte is the
/// distance to it.
fn resume_target(at: u32, ty: Type) -> u32 {
    at + u32::from(ty.0)
}

/// Build the blocks and subroutines of a program.
pub fn build(p: &Program) -> Cfg {
    let n = p.len();
    if n == 0 {
        return Cfg::default();
    }
    let ins = p.instructions();
    let offs = p.offsets();
    let index_of = |off: u32| p.index_at(off);

    let mut leaders = BTreeSet::from([0usize]);
    let mut sub_entries: BTreeMap<usize, SubKind> = BTreeMap::from([(0, SubKind::Entry)]);
    for (i, ins_i) in ins.iter().enumerate() {
        let at = offs[i];
        if let Some(t) = ins_i.jump_target(at).and_then(index_of) {
            leaders.insert(t);
            if ins_i.opcode == Opcode::Jsr {
                sub_entries.entry(t).or_insert(SubKind::Call);
            }
        }
        if ins_i.opcode == Opcode::StoreState
            && let Some(t) = index_of(resume_target(at, ins_i.ty))
        {
            leaders.insert(t);
            sub_entries.insert(t, SubKind::Closure);
        }
        let ends = ins_i.opcode.is_jump() || matches!(ins_i.opcode, Opcode::Retn);
        if ends && i + 1 < n {
            leaders.insert(i + 1);
        }
    }
    let starts: Vec<usize> = leaders.into_iter().collect();
    let mut blocks: Vec<Block> = starts
        .iter()
        .enumerate()
        .map(|(b, &s)| Block {
            start: s,
            end: starts.get(b + 1).copied().unwrap_or(n),
            succs: Vec::new(),
            preds: Vec::new(),
            sub: None,
        })
        .collect();
    let mut infeasible = Vec::new();
    let mut block_of = vec![0; n];
    for (b, blk) in blocks.iter().enumerate() {
        block_of[blk.start..blk.end].fill(b);
    }
    for b in 0..blocks.len() {
        let last = blocks[b].end - 1;
        let li = &ins[last];
        let mut succs = Vec::new();
        let fall = (blocks[b].end < n).then(|| block_of[blocks[b].end]);
        // BioWare's `||`: copy the left operand and `JZ` to the right one; on
        // the fall-through, copy the same (now known nonzero) value again and
        // `JZ` to the `LOGOR`. That second jump is never taken.
        let never_taken = li.opcode == Opcode::Jz
            && blocks[b].end - blocks[b].start == 2
            && is_dup(&ins[blocks[b].start])
            && blocks[b].start >= 2
            && ins[blocks[b].start - 1].opcode == Opcode::Jz
            && is_dup(&ins[blocks[b].start - 2]);
        match li.opcode {
            Opcode::Retn => {}
            Opcode::Jmp => succs.extend(
                li.jump_target(offs[last])
                    .and_then(index_of)
                    .map(|t| block_of[t]),
            ),
            Opcode::Jz | Opcode::Jnz => {
                let taken = li
                    .jump_target(offs[last])
                    .and_then(index_of)
                    .map(|t| block_of[t]);
                if never_taken {
                    infeasible.extend(taken.map(|t| (b, t)));
                } else {
                    succs.extend(taken);
                }
                succs.extend(fall);
            }
            // A call returns to the next instruction; the callee is its own subroutine.
            _ => succs.extend(fall),
        }
        succs.dedup();
        for &s in &succs {
            blocks[s].preds.push(b);
        }
        blocks[b].succs = succs;
    }
    let mut subs = Vec::new();
    let mut shared = BTreeSet::new();
    for (&entry_ins, &kind) in &sub_entries {
        let id = subs.len();
        let entry = block_of[entry_ins];
        let mut seen = BTreeSet::from([entry]);
        let mut queue = VecDeque::from([entry]);
        while let Some(b) = queue.pop_front() {
            for &s in &blocks[b].succs {
                if seen.insert(s) {
                    queue.push_back(s);
                }
            }
        }
        for &b in &seen {
            match blocks[b].sub {
                None => blocks[b].sub = Some(id),
                Some(other) if other != id => {
                    shared.insert(b);
                }
                _ => {}
            }
        }
        subs.push(Sub {
            kind,
            entry,
            blocks: seen.into_iter().collect(),
        });
    }
    Cfg {
        blocks,
        subs,
        block_of,
        shared: shared.into_iter().collect(),
        infeasible,
    }
}

/// What the stack-depth analysis found.
#[derive(Clone, Debug, Default)]
pub struct Depths {
    /// The depth in bytes before each instruction, relative to its
    /// subroutine's entry; `None` where it cannot be determined.
    pub before: Vec<Option<i32>>,
    /// Each subroutine's net change of the caller's stack, when known.
    pub net: Vec<Option<i32>>,
    /// Instructions reached with two different depths, and what they were.
    pub conflicts: Vec<(usize, i32, i32)>,
}

/// The change of stack depth an instruction makes on its own, in bytes;
/// `None` for `ACTION` and `JSR`, which depend on what they call.
pub fn effect(op: Opcode, ty: Type, operands: &Operands) -> Option<i32> {
    use Opcode::*;
    let pair = |t: Type| -> i32 {
        match t {
            Type::VECTOR_VECTOR => -12,
            Type::VECTOR_FLOAT | Type::FLOAT_VECTOR => -4,
            _ => -4,
        }
    };
    Some(match (op, operands) {
        (RsAdd | Const, _) => 4,
        (CpTopSp | CpTopBp, Operands::Stack { size, .. }) => i32::from(*size),
        (CpDownSp | CpDownBp, _) => 0,
        (MovSp, Operands::Offset(o)) => *o,
        (Equal | NEqual, Operands::Size(s)) => 4 - 2 * i32::from(*s),
        (
            LogAnd | LogOr | IncOr | ExcOr | BoolAnd | Equal | NEqual | Geq | Gt | Lt | Leq
            | ShLeft | ShRight | UShRight | Mod,
            _,
        ) => -4,
        (Add | Sub | Mul | Div, _) => pair(ty),
        (Neg | Comp | Not | Nop, _) => 0,
        (DecISp | IncISp | DecIBp | IncIBp, _) => 0,
        (Jz | Jnz, _) => -4,
        (Jmp | Retn | StoreState | StoreStateAll, _) => 0,
        (
            Destruct,
            Operands::Destruct {
                size, keep_size, ..
            },
        ) => i32::from(*keep_size) - i32::from(*size),
        (SaveBp, _) => 4,
        (RestoreBp, _) => -4,
        (Action | Jsr, _) => return None,
        _ => return None,
    })
}

/// Stack depths, given the net effect of each `ACTION` (its return value's
/// size minus its arguments' sizes; `action` arguments are not on the
/// stack) as `action(routine, argc)`.
pub fn stack_depths(p: &Program, cfg: &Cfg, action: &dyn Fn(u16, u8) -> Option<i32>) -> Depths {
    let n = p.len();
    let ins = p.instructions();
    let offs = p.offsets();
    let sub_at: BTreeMap<usize, usize> = cfg
        .subs
        .iter()
        .enumerate()
        .map(|(i, s)| (cfg.blocks[s.entry].start, i))
        .collect();
    let mut net: Vec<Option<i32>> = vec![None; cfg.subs.len()];
    let mut out = Depths::default();
    // Callee effects feed callers, so iterate until the subroutines' nets settle.
    for _round in 0..=cfg.subs.len() {
        let mut before: Vec<Option<i32>> = vec![None; n];
        let mut conflicts = Vec::new();
        let mut new_net = vec![None; cfg.subs.len()];
        for (sid, sub) in cfg.subs.iter().enumerate() {
            let mut queue = VecDeque::from([(cfg.blocks[sub.entry].start, 0i32)]);
            let mut ret_depth: Option<i32> = None;
            let mut ret_conflict = false;
            while let Some((start, depth)) = queue.pop_front() {
                let b = cfg.block_of[start];
                match before[start] {
                    Some(d) if d == depth => continue,
                    Some(d) => {
                        conflicts.push((start, d, depth));
                        continue;
                    }
                    None => {}
                }
                let mut d = Some(depth);
                let blk = &cfg.blocks[b];
                for i in blk.start..blk.end {
                    let Some(cur) = d else { break };
                    if before[i].is_some_and(|x| x != cur) {
                        conflicts.push((i, before[i].unwrap(), cur));
                    }
                    before[i] = Some(cur);
                    let ins_i = &ins[i];
                    let e = match (&ins_i.opcode, &ins_i.operands) {
                        (Opcode::Action, Operands::Action { routine, argc }) => {
                            action(*routine, *argc)
                        }
                        (Opcode::Jsr, _) => ins_i
                            .jump_target(offs[i])
                            .and_then(|t| p.index_at(t))
                            .and_then(|t| sub_at.get(&t))
                            .and_then(|&s| net[s]),
                        _ => effect(ins_i.opcode, ins_i.ty, &ins_i.operands),
                    };
                    if ins_i.opcode == Opcode::Retn {
                        match ret_depth {
                            None => ret_depth = Some(cur),
                            Some(r) if r != cur => ret_conflict = true,
                            _ => {}
                        }
                    }
                    d = e.map(|e| cur + e);
                }
                if let Some(d) = d {
                    for &s in &blk.succs {
                        if cfg.blocks[s].sub == Some(sid)
                            || !sub_at.contains_key(&cfg.blocks[s].start)
                        {
                            queue.push_back((cfg.blocks[s].start, d));
                        }
                    }
                }
            }
            if ret_conflict {
                conflicts.push((
                    cfg.blocks[sub.entry].start,
                    ret_depth.unwrap_or(0),
                    i32::MIN,
                ));
            }
            new_net[sid] = if ret_conflict { None } else { ret_depth };
        }
        let settled = new_net == net;
        net = new_net;
        out = Depths {
            before,
            net: net.clone(),
            conflicts,
        };
        if settled {
            break;
        }
    }
    out
}

/// The graph as Graphviz: one cluster per subroutine, one node per block
/// listing its instructions.
pub fn to_dot(p: &Program, cfg: &Cfg) -> String {
    let mut s = String::from("digraph ncs {\n  node [shape=box fontname=monospace];\n");
    let ins = p.instructions();
    let offs = p.offsets();
    for (sid, sub) in cfg.subs.iter().enumerate() {
        let _ = writeln!(
            s,
            "  subgraph cluster_{sid} {{\n    label=\"{:?} at {}\";",
            sub.kind, offs[cfg.blocks[sub.entry].start]
        );
        for &b in &sub.blocks {
            if cfg.blocks[b].sub != Some(sid) {
                continue;
            }
            let blk = &cfg.blocks[b];
            let mut label = String::new();
            for i in blk.start..blk.end {
                let _ = write!(
                    label,
                    "{}: {}\\l",
                    offs[i],
                    ins[i]
                        .to_string()
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                );
            }
            let _ = writeln!(s, "    b{b} [label=\"{label}\"];");
        }
        s.push_str("  }\n");
    }
    let dead: Vec<usize> = (0..cfg.blocks.len())
        .filter(|&b| cfg.blocks[b].sub.is_none())
        .collect();
    if !dead.is_empty() {
        s.push_str("  subgraph cluster_dead {\n    label=\"unreachable\"; style=dashed;\n");
        for b in dead {
            let blk = &cfg.blocks[b];
            let mut label = String::new();
            for i in blk.start..blk.end {
                let _ = write!(
                    label,
                    "{}: {}\\l",
                    offs[i],
                    ins[i]
                        .to_string()
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                );
            }
            let _ = writeln!(s, "    b{b} [label=\"{label}\"];");
        }
        s.push_str("  }\n");
    }
    for (b, blk) in cfg.blocks.iter().enumerate() {
        for &t in &blk.succs {
            let _ = writeln!(s, "  b{b} -> b{t};");
        }
        let last = &ins[blk.end - 1];
        if last.opcode == Opcode::Jsr
            && let Some(t) = last
                .jump_target(offs[blk.end - 1])
                .and_then(|t| p.index_at(t))
        {
            let _ = writeln!(
                s,
                "  b{b} -> b{} [style=dashed label=call];",
                cfg.block_of[t]
            );
        }
    }
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Instruction, Operands as O, Type as T};

    fn i(op: Opcode, ty: T, o: O) -> Instruction {
        Instruction::new(op, ty, o)
    }

    /// `int StartingConditional() { return a || b; }` in BioWare's shape, with
    /// the never-taken second `JZ`.
    fn or_program() -> Program {
        use Opcode::*;
        Program::new(vec![
            i(RsAdd, T::INT, O::None),   // 13
            i(Jsr, T::VOID, O::Jump(8)), // 15 -> 23
            i(Retn, T::VOID, O::None),   // 21
            i(Const, T::INT, O::Int(0)), // 23 a
            i(
                CpTopSp,
                T::STACK,
                O::Stack {
                    offset: -4,
                    size: 4,
                },
            ), // 29
            i(Jz, T::VOID, O::Jump(20)), // 37 -> 57
            i(
                CpTopSp,
                T::STACK,
                O::Stack {
                    offset: -4,
                    size: 4,
                },
            ), // 43
            i(Jz, T::VOID, O::Jump(12)), // 51 -> 63, never taken
            i(Const, T::INT, O::Int(1)), // 57 b
            i(LogOr, T::INT_INT, O::None), // 63
            i(
                CpDownSp,
                T::STACK,
                O::Stack {
                    offset: -8,
                    size: 4,
                },
            ), // 65
            i(MovSp, T::VOID, O::Offset(-4)), // 73
            i(Retn, T::VOID, O::None),
        ])
    }

    #[test]
    fn blocks_subs_and_the_never_taken_or_jump() {
        let p = or_program();
        let cfg = build(&p);
        assert_eq!(cfg.subs.len(), 2);
        assert_eq!(cfg.subs[0].kind, SubKind::Entry);
        assert_eq!(cfg.subs[1].kind, SubKind::Call);
        assert_eq!(cfg.infeasible.len(), 1, "{:?}", cfg.blocks);
        let d = stack_depths(&p, &cfg, &|_, _| None);
        assert!(d.conflicts.is_empty(), "{:?}", d.conflicts);
        // main: a(4) b(8) LOGOR(4) CPDOWNSP(4) MOVSP(0): net 0.
        assert_eq!(d.net[1], Some(0));
        assert_eq!(d.before[p.len() - 1], Some(0));
    }

    #[test]
    fn closures_are_their_own_subroutines() {
        use Opcode::*;
        let p = Program::new(vec![
            i(
                StoreState,
                T::STORE_STATE,
                O::StoreState {
                    globals: 0,
                    locals: 0,
                },
            ), // 13, resumes at 29
            i(Jmp, T::VOID, O::Jump(8)), // 23 -> 31
            i(Retn, T::VOID, O::None),   // 29
            i(Retn, T::VOID, O::None),   // 31
        ]);
        let cfg = build(&p);
        let kinds: Vec<SubKind> = cfg.subs.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, [SubKind::Entry, SubKind::Closure]);
        assert!(to_dot(&p, &cfg).starts_with("digraph ncs {"));
    }
}
