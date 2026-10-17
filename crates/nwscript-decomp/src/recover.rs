//! Recovering source structure from bytecode.
//!
//! BioWare's compiler is syntax-directed: each construct compiles to a fixed
//! shape, so recovery reads the instructions back the way they were written.
//! A symbolic stack follows every instruction: variables (one entry per
//! reserved slot), temporaries holding the expression that computed them,
//! the return slot, and markers for the copies `&&`, `||` and `switch` make.
//! Statements end where the stack returns to its variables; a `JZ` that
//! consumes a condition starts an `if` or a loop, whose extent the jumps
//! give; a `MOVSP` that pops variables ends their scope. What does not fit
//! a known shape stops the function with a reason, never a guess: the
//! function is then reported as not recovered.

use crate::ast::*;
use nwscript_ncs::cfg;
use nwscript_ncs::{Instruction, Opcode as Op, Operands as O, Program, Type as T};
use nwscript_sema::EngineTable;
use std::collections::{BTreeMap, HashMap};

type Res<X> = Result<X, String>;

/// The expression a [`Slot::Dup`] stands for when an operator consumes it:
/// the value it copies, which only a `case` test compares.
const DUP: VarId = VarId::MAX;

fn is_dup_expr(e: &Expr) -> bool {
    matches!(e, Expr::Place(Place::Var(DUP)))
}

/// Where each function is and what the bytecode says about its frame.
#[derive(Clone, Debug)]
struct Layout {
    kind: FuncKind,
    /// Index of the first instruction of its body (a `JSR` target).
    entry: usize,
    /// Instruction range, the entry stub included for the first function.
    first: usize,
    end: usize,
    params: Option<u32>,
    ret: u32,
}

/// What the call sites say about a function's parameters and result.
#[derive(Clone, Debug, Default)]
struct Calls {
    /// Each argument's size and type, from the first call parsed.
    args: Option<Vec<(u32, Option<Ty>)>>,
    ret: Option<Ty>,
}

struct Shared<'a> {
    ins: &'a [Instruction],
    offs: &'a [u32],
    prog: &'a Program,
    table: &'a EngineTable,
    layout: Vec<Layout>,
    /// Function index by entry instruction index.
    by_entry: HashMap<usize, usize>,
    script: Script,
    calls: Vec<Calls>,
    /// Parameter variables of the functions parsed so far.
    params: Vec<Option<Vec<VarId>>>,
    /// The globals' slot variables, first to last, once the initialiser is read.
    globals: Vec<VarId>,
    /// The slots each joined variable is made of.
    parts: HashMap<VarId, Vec<VarId>>,
    /// Back jumps by target: candidates for `do`/`while`.
    back_jumps: HashMap<usize, Vec<usize>>,
}

/// Recover the program. The error is for bytecode with no recognisable
/// entry point; anything narrower is reported per function.
pub fn recover(prog: &Program, table: &EngineTable) -> Res<Script> {
    let ins = prog.instructions();
    if ins.is_empty() {
        return Err("the program has no instructions".into());
    }
    let layout = layout(prog, table)?;
    let mut by_entry = HashMap::new();
    for (i, l) in layout.iter().enumerate() {
        by_entry.insert(l.entry, i);
    }
    let mut back_jumps: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, x) in ins.iter().enumerate() {
        if x.opcode == Op::Jmp
            && let Some(t) = x
                .jump_target(prog.offsets()[i])
                .and_then(|t| prog.index_at(t))
            && t <= i
        {
            back_jumps.entry(t).or_default().push(i);
        }
    }
    let n = layout.len();
    let mut sh = Shared {
        ins,
        offs: prog.offsets(),
        prog,
        table,
        by_entry,
        script: Script::default(),
        calls: vec![Calls::default(); n],
        params: vec![None; n],
        globals: Vec::new(),
        parts: HashMap::new(),
        back_jumps,
        layout,
    };
    let entry_is_sc = ins[0].opcode == Op::RsAdd;
    for i in 0..n {
        let l = sh.layout[i].clone();
        let start = sh.offs[l.first];
        let end = sh.offs.get(l.end).copied().unwrap_or(prog.file_len());
        let name = match l.kind {
            FuncKind::Globals => "(globals)".to_string(),
            FuncKind::Entry if entry_is_sc => "StartingConditional".into(),
            FuncKind::Entry => "main".into(),
            FuncKind::User => format!("sub_{:x}", sh.offs[l.entry]),
        };
        sh.script.funcs.push(Func {
            name,
            kind: l.kind,
            start,
            end,
            params: Vec::new(),
            defaults: Vec::new(),
            ret: (l.kind == FuncKind::Entry && entry_is_sc).then_some(Ty::Int),
            body: Err("not reached".into()),
        });
    }
    for i in 0..n {
        let body = parse_function(&mut sh, i);
        let f = &mut sh.script.funcs[i];
        f.body = body;
        if f.params.is_empty()
            && let Some(ps) = &sh.params[i]
        {
            f.params = ps.clone();
        }
    }
    finish(&mut sh);
    Ok(sh.script)
}

/// The byte size of an engine routine's parameter or return type on the stack.
fn sema_size(t: &nwscript_sema::Type) -> Option<u32> {
    use nwscript_sema::Type as S;
    match t {
        S::Void | S::Action => Some(0),
        S::Vector => Some(12),
        S::Struct(_) => None,
        _ => Some(4),
    }
}

/// The net stack effect of `ACTION routine, argc`.
fn action_effect(table: &EngineTable, routine: u16, argc: u8) -> Option<i32> {
    let f = table.functions.get(usize::from(routine))?;
    let mut e = sema_size(&f.ret)? as i32;
    let argc = usize::from(argc);
    for p in f.params.iter().take(argc) {
        e -= sema_size(&p.ty)? as i32;
    }
    // Arguments past the declaration (bytecode compiled against an older
    // one) are taken to be one stack element each.
    e -= 4 * argc.saturating_sub(f.params.len()) as i32;
    Some(e)
}

fn jump_index(prog: &Program, i: usize) -> Option<usize> {
    let x = &prog.instructions()[i];
    x.jump_target(prog.offsets()[i])
        .and_then(|t| prog.index_at(t))
}

/// Find the functions: the entry stub's target, every `JSR` target, the
/// globals initialiser (the target holding `SAVEBP`), and each one's frame.
fn layout(prog: &Program, table: &EngineTable) -> Res<Vec<Layout>> {
    let ins = prog.instructions();
    let stub_jsr = match (ins.first(), ins.get(1), ins.get(2)) {
        (Some(a), Some(b), _) if a.opcode == Op::Jsr && b.opcode == Op::Retn => 0,
        (Some(a), Some(b), Some(c))
            if a.opcode == Op::RsAdd
                && a.ty == T::INT
                && b.opcode == Op::Jsr
                && c.opcode == Op::Retn =>
        {
            1
        }
        _ => return Err("the program does not start with an entry stub (`JSR`, `RETN`)".into()),
    };
    let first = jump_index(prog, stub_jsr).ok_or("the entry stub's JSR has no target")?;
    let mut starts: Vec<usize> = vec![first];
    for (i, x) in ins.iter().enumerate() {
        if x.opcode == Op::Jsr
            && let Some(t) = jump_index(prog, i)
        {
            starts.push(t);
        }
    }
    starts.sort_unstable();
    starts.dedup();
    let graph = cfg::build(prog);
    let depths = cfg::stack_depths(prog, &graph, &|r, a| action_effect(table, r, a));
    let sub_of: HashMap<usize, usize> = graph
        .subs
        .iter()
        .enumerate()
        .map(|(k, s)| (graph.blocks[s.entry].start, k))
        .collect();
    let mut out = Vec::new();
    for (k, &s) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(ins.len());
        let has_savebp = ins[s..end].iter().any(|x| x.opcode == Op::SaveBp);
        let kind = if s == first {
            if has_savebp {
                FuncKind::Globals
            } else {
                FuncKind::Entry
            }
        } else {
            FuncKind::User
        };
        let sub = sub_of.get(&s).copied();
        let params = sub
            .and_then(|sid| depths.net[sid])
            .and_then(|n| u32::try_from(-n).ok());
        let mut ret = 0u32;
        if let (Some(sid), Some(p)) = (sub, params) {
            #[allow(clippy::needless_range_loop)]
            for i in s..end {
                if graph.blocks[graph.block_of[i]].sub != Some(sid) {
                    continue;
                }
                if let (Op::CpDownSp, O::Stack { offset, size }, Some(d)) =
                    (ins[i].opcode, &ins[i].operands, depths.before[i])
                {
                    let at = d + offset;
                    if at < -(p as i32) {
                        ret = ret.max((-(p as i32) - at) as u32).max(u32::from(*size));
                    }
                }
            }
        }
        out.push(Layout {
            kind,
            entry: s,
            first: if k == 0 { 0 } else { s },
            end,
            params,
            ret,
        });
    }
    // The entry point the globals initialiser calls after SAVEBP.
    if out.first().is_some_and(|l| l.kind == FuncKind::Globals) {
        let g = &out[0];
        let save = (g.entry..g.end)
            .find(|&i| ins[i].opcode == Op::SaveBp)
            .expect("found above");
        let main = (save..g.end)
            .find(|&i| ins[i].opcode == Op::Jsr)
            .and_then(|i| jump_index(prog, i))
            .ok_or("the globals initialiser calls no entry point")?;
        let m = out
            .iter_mut()
            .find(|l| l.entry == main)
            .ok_or("the globals initialiser calls into the middle of a function")?;
        m.kind = FuncKind::Entry;
    }
    Ok(out)
}

/// One entry of the symbolic stack.
#[derive(Clone, Debug)]
struct Entry {
    size: u32,
    slot: Slot,
}

#[derive(Clone, Debug)]
enum Slot {
    /// A variable's slot (one `RSADD`, or one parameter).
    Var(VarId),
    /// The function's return value.
    Ret,
    /// A value computed by this expression.
    Temp(Expr, Option<Ty>),
    /// A user call's return slot, reserved before its arguments.
    Reserved(Option<Ty>),
    /// The copy of the value below it that `&&`, `||` and `case` make.
    Dup,
    /// A deferred action (`STORE_STATE`): an argument of no size.
    Closure(Expr),
    /// The value a `switch` tests.
    Subject,
}

/// What one pass over a simple statement found.
enum Event {
    Stmt(Stmt),
    /// A condition consumed by the `JZ` at this index.
    Cond(Expr, usize),
    /// A `case` test: the `JNZ` at this index consumed it.
    CaseTest(Expr, usize),
}

/// A construct `break` and `continue` leave.
enum Ctl {
    Loop { head: Option<usize>, end: usize },
    Switch { breaks: Vec<usize> },
}

/// How a block ends.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Until {
    /// At this instruction index.
    At(usize),
    /// At the condition a `do`/`while` loop tests with the `JZ` at this index.
    DoCond(usize),
    /// Where a `MOVSP` pops below the block: the end of a `switch` body.
    SwitchEnd,
    /// At `SAVEBP`: the globals initialiser's declarations.
    SaveBp(usize),
}

struct Fp<'s, 'a> {
    sh: &'s mut Shared<'a>,
    func: usize,
    stack: Vec<Entry>,
    /// Entries below this index are the frame: return slot and parameters.
    frame: usize,
    pc: usize,
    /// The instruction a `return` jumps to.
    epilogue: usize,
    ctl: Vec<Ctl>,
}

fn parse_function(sh: &mut Shared, f: usize) -> Res<Vec<Stmt>> {
    let l = sh.layout[f].clone();
    let p = l
        .params
        .ok_or("the function's stack effect could not be determined")?;
    let ins = sh.ins;
    let last = l.end - 1;
    if ins[last].opcode != Op::Retn {
        return Err("the function does not end with RETN".into());
    }
    let mut stack = Vec::new();
    if l.ret > 0 {
        stack.push(Entry {
            size: l.ret,
            slot: Slot::Ret,
        });
    }
    // Parameters: the first is on top, so they go on in reverse.
    let sizes: Vec<(u32, Option<Ty>)> = match &sh.calls[f].args {
        Some(a) if a.iter().map(|x| x.0).sum::<u32>() == p => a.clone(),
        _ => (0..p / 4).map(|_| (4, None)).collect(),
    };
    if l.kind == FuncKind::User && sh.params[f].is_none() {
        let mut vars = Vec::new();
        for (size, ty) in &sizes {
            vars.push(new_param(sh, f, *size, *ty));
        }
        sh.params[f] = Some(vars);
    }
    if let Some(vars) = sh.params[f].clone() {
        for &v in vars.iter().rev() {
            match sh.parts.get(&v) {
                Some(parts) => {
                    for &s in parts {
                        stack.push(Entry {
                            size: 4,
                            slot: Slot::Var(s),
                        })
                    }
                }
                None => stack.push(Entry {
                    size: sh.script.vars[v].size,
                    slot: Slot::Var(v),
                }),
            }
        }
    }
    let epilogue = if p > 0 {
        if !matches!((ins[last - 1].opcode, &ins[last - 1].operands), (Op::MovSp, O::Offset(o)) if *o == -(p as i32))
        {
            return Err("the function does not end by popping its parameters".into());
        }
        last - 1
    } else {
        last
    };
    if sh.layout[f].ret > 0 && sh.script.funcs[f].ret.is_none() {
        let r = sh.calls[f]
            .ret
            .or(if l.ret == 12 { Some(Ty::Vector) } else { None });
        sh.script.funcs[f].ret = r.or(Some(Ty::Int));
    }
    let frame = stack.len();
    let mut fp = Fp {
        sh,
        func: f,
        stack,
        frame,
        pc: l.entry,
        epilogue,
        ctl: Vec::new(),
    };
    match l.kind {
        FuncKind::Globals => fp.globals(l.end),
        _ => {
            let body = fp.block(Until::At(epilogue), None)?;
            if fp.pc != epilogue {
                return Err(fp.at("the body ends before the epilogue"));
            }
            Ok(body)
        }
    }
}

fn new_param(sh: &mut Shared, f: usize, size: u32, ty: Option<Ty>) -> VarId {
    if size == 12 {
        let parts: Vec<VarId> = (0..3)
            .map(|_| new_var(sh, VarKind::Param, Some(Ty::Float), 4, Some(f)))
            .collect();
        join(sh, &parts, Ty::Vector)
    } else {
        new_var(sh, VarKind::Param, ty, size, Some(f))
    }
}

fn new_var(
    sh: &mut Shared,
    kind: VarKind,
    ty: Option<Ty>,
    size: u32,
    owner: Option<usize>,
) -> VarId {
    sh.script.vars.push(Var {
        kind,
        ty,
        size,
        name: String::new(),
        part_of: None,
        owner,
    });
    sh.script.vars.len() - 1
}

/// Join slot variables into one vector or struct variable.
fn join(sh: &mut Shared, parts: &[VarId], ty: Ty) -> VarId {
    let first = &sh.script.vars[parts[0]];
    let (kind, owner) = (first.kind, first.owner);
    let size = parts.iter().map(|&p| sh.script.vars[p].size).sum();
    let whole = new_var(sh, kind, Some(ty), size, owner);
    for (n, &p) in parts.iter().enumerate() {
        sh.script.vars[p].part_of = Some((whole, n));
    }
    sh.parts.insert(whole, parts.to_vec());
    whole
}

impl Fp<'_, '_> {
    fn at(&self, what: &str) -> String {
        let pc = self.pc.min(self.sh.ins.len() - 1);
        format!(
            "{what} at offset {} ({})",
            self.sh.offs[pc], self.sh.ins[pc]
        )
    }

    fn ins(&self, i: usize) -> Res<&Instruction> {
        self.sh
            .ins
            .get(i)
            .ok_or_else(|| "the code runs past the end of the program".to_string())
    }

    fn target(&self, i: usize) -> Res<usize> {
        jump_index(self.sh.prog, i).ok_or_else(|| {
            format!(
                "the jump at offset {} does not land on an instruction",
                self.sh.offs[i]
            )
        })
    }

    fn top_bytes(&self) -> u32 {
        self.stack.iter().map(|e| e.size).sum()
    }

    fn bytes_above(&self, idx: usize) -> u32 {
        self.stack[idx..].iter().map(|e| e.size).sum()
    }

    /// The entries `bytes` bytes from the top cover, as a range of entry
    /// indices, when entry boundaries fall there.
    fn span(&self, from_top: i32, size: u32) -> Option<std::ops::Range<usize>> {
        let total = self.top_bytes() as i64;
        let start = total + i64::from(from_top);
        let end = start + i64::from(size);
        if start < 0 || end > total {
            return None;
        }
        let mut at = 0i64;
        let mut first = None;
        for (k, e) in self.stack.iter().enumerate() {
            if at == start && first.is_none() {
                first = Some(k);
            }
            at += i64::from(e.size);
            if at == end
                && let Some(f) = first
            {
                return Some(f..k + 1);
            }
            if at > end {
                return None;
            }
        }
        None
    }

    /// The variable `size` bytes at `from_top` name: one slot, or slots
    /// joined (now, if need be) into a vector or struct.
    fn var_at(&mut self, from_top: i32, size: u32) -> Res<VarId> {
        let r = self
            .span(from_top, size)
            .ok_or_else(|| self.at("a copy that does not cover whole stack entries"))?;
        let mut slots = Vec::new();
        for e in &self.stack[r] {
            match e.slot {
                Slot::Var(v) => slots.push(v),
                _ => return Err(self.at("a copy that is not of a variable")),
            }
        }
        self.whole(&slots)
    }

    /// The variable these slot variables make up.
    fn whole(&mut self, slots: &[VarId]) -> Res<VarId> {
        if let [one] = slots {
            return Ok(*one);
        }
        let first = self.sh.script.vars[slots[0]].part_of;
        if let Some((w, 0)) = first {
            if self.sh.parts.get(&w).is_some_and(|p| p == slots) {
                return Ok(w);
            }
            return Err(self.at("an access across parts of different variables"));
        }
        if slots
            .iter()
            .any(|&s| self.sh.script.vars[s].part_of.is_some())
        {
            return Err(self.at("an access across parts of different variables"));
        }
        let tys: Vec<Option<Ty>> = slots.iter().map(|&s| self.sh.script.vars[s].ty).collect();
        let ty = if slots.len() == 3 && tys.iter().all(|t| *t == Some(Ty::Float) || t.is_none()) {
            Ty::Vector
        } else {
            let fields: Vec<Ty> = tys.iter().map(|t| t.unwrap_or(Ty::Int)).collect();
            let idx = match self
                .sh
                .script
                .structs
                .iter()
                .position(|s| s.fields == fields)
            {
                Some(i) => i,
                None => {
                    let n = self.sh.script.structs.len();
                    self.sh.script.structs.push(StructDef {
                        name: format!("Struct{}", n + 1),
                        fields,
                    });
                    n
                }
            };
            Ty::Struct(idx)
        };
        Ok(join(self.sh, slots, ty))
    }

    /// The global `size` bytes at BP offset `off`.
    fn global_at(&mut self, off: i32, size: u32) -> Res<VarId> {
        let total: u32 = self
            .sh
            .globals
            .iter()
            .map(|&g| self.sh.script.vars[g].size)
            .sum();
        let start = i64::from(total) + i64::from(off);
        let mut at = 0i64;
        let mut slots = Vec::new();
        for &g in &self.sh.globals {
            if at >= start && at < start + i64::from(size) {
                slots.push(g);
            }
            at += i64::from(self.sh.script.vars[g].size);
        }
        let covered: u32 = slots.iter().map(|&g| self.sh.script.vars[g].size).sum();
        if slots.is_empty() || covered != size || start < 0 {
            return Err(self.at("a global access that does not cover whole globals"));
        }
        // The first slot must start exactly at `start`.
        let mut at = 0i64;
        for &g in &self.sh.globals {
            if g == slots[0] {
                break;
            }
            at += i64::from(self.sh.script.vars[g].size);
        }
        if at != start {
            return Err(self.at("a global access that does not cover whole globals"));
        }
        self.whole(&slots)
    }

    fn var_ty(&self, v: VarId) -> Option<Ty> {
        self.sh.script.vars[v].ty
    }

    fn refine(&mut self, e: &Expr, ty: Option<Ty>) {
        if let (Expr::Place(Place::Var(v)), Some(t)) = (e, ty)
            && *v != DUP
        {
            let var = &mut self.sh.script.vars[*v];
            if var.ty.is_none() && var.size == t.size().unwrap_or(0) {
                var.ty = Some(t);
            }
        }
    }

    fn push(&mut self, size: u32, slot: Slot) {
        self.stack.push(Entry { size, slot });
    }

    fn pop_temp(&mut self) -> Res<(Expr, Option<Ty>, u32)> {
        match self.stack.pop() {
            Some(Entry {
                size,
                slot: Slot::Temp(e, t),
            }) => Ok((e, t, size)),
            Some(Entry {
                slot: Slot::Dup, ..
            }) => Ok((Expr::Place(Place::Var(DUP)), Some(Ty::Int), 4)),
            other => {
                if let Some(o) = other {
                    self.stack.push(o);
                }
                Err(self.at("an operation on something that is not a value"))
            }
        }
    }

    /// Does the `RSADD` at `pc` (with `size` bytes, from `n` instructions)
    /// reserve a user call's return value? Follows the stack depth until
    /// a `JSR` fills the slot or something pops it.
    fn is_return_slot(&self, pc: usize, n: usize, size: u32) -> bool {
        let ins = self.sh.ins;
        let mut depth: i64 = 0;
        let mut i = pc + n;
        while i < ins.len() {
            let x = &ins[i];
            match x.opcode {
                Op::Jsr => {
                    let Some(f) =
                        jump_index(self.sh.prog, i).and_then(|t| self.sh.by_entry.get(&t))
                    else {
                        return false;
                    };
                    let l = &self.sh.layout[*f];
                    let Some(p) = l.params else { return false };
                    if depth - i64::from(p) - i64::from(l.ret) == -i64::from(size) {
                        return l.ret == size;
                    }
                    depth -= i64::from(p);
                }
                Op::Action => {
                    let O::Action { routine, argc } = x.operands else {
                        return false;
                    };
                    match action_effect(self.sh.table, routine, argc) {
                        Some(e) => depth += i64::from(e),
                        None => return false,
                    }
                }
                Op::StoreState => {
                    match ins.get(i + 1).and_then(|_| jump_index(self.sh.prog, i + 1)) {
                        Some(t) => {
                            i = t;
                            continue;
                        }
                        None => return false,
                    }
                }
                Op::Jz => {
                    // Only the jumps of `&&` and `||` stay inside an expression.
                    let dup = i > 0 && is_dup(&ins[i - 1]);
                    if !dup {
                        return false;
                    }
                    depth -= 4;
                }
                Op::Jmp | Op::Jnz | Op::Retn | Op::SaveBp | Op::RestoreBp => return false,
                _ => match cfg::effect(x.opcode, x.ty, &x.operands) {
                    Some(e) => depth += i64::from(e),
                    None => return false,
                },
            }
            if depth < 0 {
                return false;
            }
            i += 1;
        }
        false
    }

    /// A local declaration for each slot the `RSADD` at `pc` reserves.
    fn declare(&mut self, out: &mut Vec<Stmt>) -> Res<()> {
        let x = self.ins(self.pc)?;
        let ty = Ty::of_rsadd(x.ty).ok_or_else(|| self.at("an RSADD of an unknown type"))?;
        let v = new_var(self.sh, VarKind::Local, Some(ty), 4, Some(self.func));
        self.push(4, Slot::Var(v));
        out.push(Stmt::Decl(v));
        self.pc += 1;
        Ok(())
    }

    /// The globals initialiser: declarations up to `SAVEBP`.
    fn globals(&mut self, end: usize) -> Res<Vec<Stmt>> {
        let save = (self.pc..end)
            .find(|&i| self.sh.ins[i].opcode == Op::SaveBp)
            .ok_or("no SAVEBP")?;
        let body = self.block(Until::SaveBp(save), None)?;
        let mut globals = Vec::new();
        for e in &self.stack[self.frame..] {
            match e.slot {
                Slot::Var(v) => {
                    self.sh.script.vars[v].kind = VarKind::Global;
                    self.sh.script.vars[v].owner = None;
                    if let Some((w, _)) = self.sh.script.vars[v].part_of {
                        self.sh.script.vars[w].kind = VarKind::Global;
                        self.sh.script.vars[w].owner = None;
                    }
                    globals.push(v);
                }
                _ => return Err(self.at("a value left on the stack among the globals")),
            }
        }
        self.sh.globals = globals;
        for s in &body {
            match s {
                Stmt::Decl(_) | Stmt::Expr(Expr::Assign(..)) => {}
                _ => return Err("the globals initialiser does more than initialise globals".into()),
            }
        }
        Ok(body)
    }

    /// The `do`/`while` loop whose body starts here: the back jump to `pc`
    /// that a `JZ` past it precedes.
    fn do_loop_at(&self, pc: usize, end: usize) -> Option<usize> {
        let js = self.sh.back_jumps.get(&pc)?;
        js.iter()
            .copied()
            .filter(|&j| j < end && j > pc)
            .filter(|&j| {
                self.sh.ins[j - 1].opcode == Op::Jz
                    && jump_index(self.sh.prog, j - 1) == Some(j + 1)
            })
            .max()
    }

    fn block(
        &mut self,
        until: Until,
        labels: Option<&mut BTreeMap<usize, Vec<Stmt>>>,
    ) -> Res<Vec<Stmt>> {
        Ok(self.block_cond(until, labels)?.0)
    }

    /// Statements until `until`; for [`Until::DoCond`], also the condition.
    fn block_cond(
        &mut self,
        until: Until,
        mut labels: Option<&mut BTreeMap<usize, Vec<Stmt>>>,
    ) -> Res<(Vec<Stmt>, Option<Expr>)> {
        let base = self.stack.len();
        let mut out: Vec<Stmt> = Vec::new();
        let limit = match until {
            Until::At(e) => e,
            Until::DoCond(jz) => jz + 1,
            Until::SwitchEnd => self.epilogue,
            Until::SaveBp(s) => s,
        };
        loop {
            if let Some(m) = labels.as_deref_mut()
                && let Some(ls) = m.remove(&self.pc)
            {
                out.extend(ls);
            }
            if self.pc >= limit {
                if self.pc > limit {
                    return Err(self.at("a statement runs past the end of its block"));
                }
                if matches!(until, Until::DoCond(_)) {
                    return Err(self.at("a do/while loop without its condition"));
                }
                break;
            }
            if let Some(j) = self.do_loop_at(self.pc, limit) {
                let s = self.do_while(j)?;
                out.push(s);
                continue;
            }
            let start = self.pc;
            let x = self.ins(start)?.clone();
            match x.opcode {
                Op::RsAdd => {
                    let vec3 = x.ty == T::FLOAT
                        && (1..3).all(|k| {
                            self.sh
                                .ins
                                .get(start + k)
                                .is_some_and(|y| y.opcode == Op::RsAdd && y.ty == T::FLOAT)
                        });
                    let ret = (vec3 && self.is_return_slot(start, 3, 12))
                        || self.is_return_slot(start, 1, 4);
                    if !ret {
                        self.declare(&mut out)?;
                        continue;
                    }
                }
                Op::MovSp => {
                    let O::Offset(o) = x.operands else {
                        unreachable!()
                    };
                    let n =
                        u32::try_from(-o).map_err(|_| self.at("a MOVSP that grows the stack"))?;
                    // `return;` with locals: pop everything above the frame, jump out.
                    if start + 1 < limit
                        && self.is_return_jump(start + 1)
                        && n == self.bytes_above(self.frame)
                        && n > 0
                    {
                        self.pc = start + 2;
                        out.push(Stmt::Return(None));
                        continue;
                    }
                    let above = self.bytes_above(base);
                    if n > above {
                        if until == Until::SwitchEnd && above == 0 {
                            break;
                        }
                        return Err(self.at("a MOVSP that pops below its block"));
                    }
                    // Pop whole variables of this block.
                    let mut k = self.stack.len();
                    let mut got = 0;
                    while got < n {
                        k -= 1;
                        got += self.stack[k].size;
                    }
                    if got != n
                        || self.stack[k..]
                            .iter()
                            .any(|e| !matches!(e.slot, Slot::Var(_)))
                    {
                        return Err(self.at("a MOVSP that does not pop whole variables"));
                    }
                    let popped: Vec<VarId> = self.stack[k..]
                        .iter()
                        .map(|e| match e.slot {
                            Slot::Var(v) => v,
                            _ => unreachable!(),
                        })
                        .collect();
                    self.stack.truncate(k);
                    self.pc = start + 1;
                    let ends_block = k == base
                        && match until {
                            Until::At(e) => self.pc == e,
                            Until::SwitchEnd => self.pops_below(self.pc, base),
                            _ => false,
                        };
                    if ends_block {
                        break;
                    }
                    // A nested block: from the first popped variable's declaration.
                    let at = out
                        .iter()
                        .position(|s| matches!(s, Stmt::Decl(v) if popped.contains(v)))
                        .ok_or_else(|| self.at("a scope end without its declarations"))?;
                    let inner: Vec<Stmt> = out.drain(at..).collect();
                    if inner
                        .iter()
                        .any(|s| matches!(s, Stmt::Case(_) | Stmt::Default))
                    {
                        return Err(self.at("a scope that spans case labels"));
                    }
                    out.push(Stmt::Block(inner));
                    continue;
                }
                Op::Jmp => {
                    let t = self.target(start)?;
                    self.pc = start + 1;
                    if t == self.epilogue {
                        out.push(Stmt::Return(None));
                        continue;
                    }
                    match self.ctl.last_mut() {
                        Some(Ctl::Loop { end, .. }) if t == *end => out.push(Stmt::Break),
                        Some(Ctl::Loop { head: Some(h), .. }) if t == *h => {
                            out.push(Stmt::Continue)
                        }
                        Some(Ctl::Switch { breaks }) => {
                            breaks.push(t);
                            out.push(Stmt::Break);
                        }
                        _ => {
                            self.pc = start;
                            return Err(self.at("a jump that is no break, continue or return"));
                        }
                    }
                    continue;
                }
                _ => {}
            }
            match self.simple(start)? {
                Event::Stmt(s) => out.push(s),
                Event::Cond(c, jz) => {
                    if let Until::DoCond(j) = until
                        && jz == j
                    {
                        if self.stack.len() != base {
                            return Err(self.at("a loop condition with locals still in scope"));
                        }
                        return Ok((out, Some(c)));
                    }
                    let s = self.if_or_while(start, c, jz)?;
                    out.push(s);
                }
                Event::CaseTest(c, jnz) => {
                    let s = self.switch(c, jnz)?;
                    out.push(s);
                }
            }
        }
        if let Until::SaveBp(_) = until {
            return Ok((out, None));
        }
        // Variables still in scope: their cleanup came before a `return` that
        // ends the block, so their scope was an inner block that closed first.
        if self.stack.len() > base {
            let vars: Vec<VarId> = self.stack[base..]
                .iter()
                .filter_map(|e| match e.slot {
                    Slot::Var(v) => Some(v),
                    _ => None,
                })
                .collect();
            if vars.len() != self.stack.len() - base
                || !matches!(out.last(), Some(Stmt::Return(None)))
            {
                return Err(self.at("variables still in scope at the end of a block"));
            }
            let at = out
                .iter()
                .position(|s| matches!(s, Stmt::Decl(v) if vars.contains(v)))
                .ok_or_else(|| self.at("a scope without its declarations"))?;
            let ret = out.pop().expect("checked above");
            let inner: Vec<Stmt> = out.drain(at..).collect();
            out.push(Stmt::Block(inner));
            out.push(ret);
            self.stack.truncate(base);
        }
        Ok((out, None))
    }

    /// Whether the instruction at `i` is a `MOVSP` that pops below `base`.
    fn pops_below(&self, i: usize, base: usize) -> bool {
        match self.sh.ins.get(i) {
            Some(Instruction {
                opcode: Op::MovSp,
                operands: O::Offset(o),
                ..
            }) => -i64::from(*o) > i64::from(self.bytes_above(base)),
            _ => false,
        }
    }

    fn is_return_jump(&self, i: usize) -> bool {
        self.sh.ins.get(i).is_some_and(|x| x.opcode == Op::Jmp)
            && jump_index(self.sh.prog, i) == Some(self.epilogue)
    }

    /// `if`/`else` or `while`, given the condition the `JZ` at `jz` tested;
    /// `start` is where the condition began.
    fn if_or_while(&mut self, start: usize, cond: Expr, jz: usize) -> Res<Stmt> {
        let t = self.target(jz)?;
        if t <= jz + 1 || t > self.sh.ins.len() {
            return Err(self.at("a conditional jump backwards"));
        }
        let j = t - 1;
        if self.ins(j)?.opcode != Op::Jmp {
            self.pc = j;
            return Err(self.at("an if without its closing jump"));
        }
        let e = self.target(j)?;
        if e == start {
            self.ctl.push(Ctl::Loop {
                head: Some(start),
                end: t,
            });
            let body = self.block(Until::At(j), None);
            self.ctl.pop();
            let body = body?;
            self.pc = t;
            return Ok(Stmt::While(cond, body));
        }
        if e < t {
            self.pc = j;
            return Err(self.at("an if whose closing jump goes backwards"));
        }
        let then = self.block(Until::At(j), None)?;
        self.pc = t;
        let els = if e > t {
            Some(self.block(Until::At(e), None)?)
        } else {
            None
        };
        if self.pc != e {
            return Err(self.at("an else that does not end where the if jumps"));
        }
        Ok(Stmt::If(cond, then, els))
    }

    /// `do { ... } while (c);` whose back jump is at `j`.
    fn do_while(&mut self, j: usize) -> Res<Stmt> {
        self.ctl.push(Ctl::Loop {
            head: None,
            end: j + 1,
        });
        let r = self.block_cond(Until::DoCond(j - 1), None);
        self.ctl.pop();
        let (body, cond) = r?;
        let cond = cond.ok_or_else(|| self.at("a do/while loop without its condition"))?;
        if self.pc != j {
            return Err(self.at("a do/while condition that does not end at its jump"));
        }
        self.pc = j + 1;
        Ok(Stmt::DoWhile(body, cond))
    }

    /// A `switch`: the subject is on the stack, the first test was consumed
    /// by the `JNZ` at `jnz`.
    fn switch(&mut self, first: Expr, jnz: usize) -> Res<Stmt> {
        let subject = match self.stack.pop() {
            Some(Entry {
                size: 4,
                slot: Slot::Temp(e, _),
            }) => e,
            _ => return Err(self.at("a case test without a switch value")),
        };
        let mut cases: Vec<(i32, usize)> = Vec::new();
        let case_value = |c: &Expr| match c {
            Expr::Binary(BinOp::Eq, l, r) if is_dup_expr(l) => match **r {
                Expr::Int(v) => Some(v),
                Expr::Unary(UnOp::Neg, ref x) => match **x {
                    Expr::Int(v) => Some(v.wrapping_neg()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let v = case_value(&first).ok_or_else(|| self.at("a case label that is not an integer"))?;
        cases.push((v, self.target(jnz)?));
        self.push(4, Slot::Subject);
        let base = self.stack.len();
        loop {
            let x = self.ins(self.pc)?;
            if !is_dup(x) {
                break;
            }
            match self.simple(self.pc)? {
                Event::CaseTest(c, j) if self.stack.len() == base => {
                    let v = case_value(&c)
                        .ok_or_else(|| self.at("a case label that is not an integer"))?;
                    cases.push((v, self.target(j)?));
                }
                _ => return Err(self.at("a switch test that is not a case")),
            }
        }
        if self.ins(self.pc)?.opcode != Op::Jmp {
            return Err(self.at("switch tests not followed by a jump"));
        }
        let default = self.target(self.pc)?;
        self.pc += 1;
        let mut labels: BTreeMap<usize, Vec<Stmt>> = BTreeMap::new();
        for &(v, t) in &cases {
            labels.entry(t).or_default().push(Stmt::Case(v));
        }
        labels.entry(default).or_default().push(Stmt::Default);
        self.ctl.push(Ctl::Switch { breaks: Vec::new() });
        let body = self.block(Until::SwitchEnd, Some(&mut labels));
        let Some(Ctl::Switch { breaks }) = self.ctl.pop() else {
            unreachable!()
        };
        let mut body = body?;
        let end = self.pc;
        if breaks.iter().any(|&b| b != end) {
            return Err(self.at("a break that does not leave its switch"));
        }
        // Labels at the end itself: a default there is no default at all.
        if let Some(ls) = labels.remove(&end) {
            body.extend(ls.into_iter().filter(|s| !matches!(s, Stmt::Default)));
        }
        if !labels.is_empty() {
            return Err(self.at("a case label that is not at the top of its switch"));
        }
        match self.ins(end)? {
            Instruction {
                opcode: Op::MovSp,
                operands: O::Offset(-4),
                ..
            } => {}
            _ => return Err(self.at("a switch that does not end by popping its value")),
        }
        self.stack.pop();
        self.pc = end + 1;
        Ok(Stmt::Switch(subject, body))
    }

    /// One simple statement, or the condition a structured statement tests.
    fn simple(&mut self, start: usize) -> Res<Event> {
        let base = self.stack.len();
        self.pc = start;
        loop {
            let pc = self.pc;
            let x = self.ins(pc)?.clone();
            self.pc += 1;
            match (x.opcode, &x.operands) {
                (Op::RsAdd, _) => {
                    if x.ty == T::FLOAT
                        && (1..3).all(|k| {
                            self.sh
                                .ins
                                .get(pc + k)
                                .is_some_and(|y| y.opcode == Op::RsAdd && y.ty == T::FLOAT)
                        })
                        && self.is_return_slot(pc, 3, 12)
                    {
                        self.push(12, Slot::Reserved(Some(Ty::Vector)));
                        self.pc = pc + 3;
                    } else if self.is_return_slot(pc, 1, 4) {
                        self.push(4, Slot::Reserved(Ty::of_rsadd(x.ty)));
                    } else {
                        self.pc = pc;
                        return Err(self.at("a declaration inside an expression"));
                    }
                }
                (Op::Const, o) => {
                    let (e, t) = match o {
                        O::Int(i) => (Expr::Int(*i), Ty::Int),
                        O::Float(f) => (Expr::Float(*f), Ty::Float),
                        O::String(s) => (Expr::Str(s.clone()), Ty::String),
                        O::Object(v) => (Expr::Object(*v), Ty::Object),
                        _ => unreachable!("CONST layouts"),
                    };
                    if let Expr::Object(v) = e
                        && v != 0
                        && v != 1
                    {
                        self.pc = pc;
                        return Err(
                            self.at("an object constant other than OBJECT_SELF or OBJECT_INVALID")
                        );
                    }
                    self.push(4, Slot::Temp(e, Some(t)));
                }
                (Op::CpTopSp, O::Stack { offset, size }) => {
                    let size = u32::from(*size);
                    let top_is_value = matches!(
                        self.stack.last(),
                        Some(Entry {
                            slot: Slot::Temp(..) | Slot::Subject,
                            size: 4
                        })
                    );
                    if *offset == -4 && size == 4 && top_is_value {
                        self.push(4, Slot::Dup);
                        continue;
                    }
                    self.pc = pc;
                    let v = self.var_at(*offset, size)?;
                    self.pc = pc + 1;
                    let ty = self.var_ty(v);
                    self.push(size, Slot::Temp(Expr::Place(Place::Var(v)), ty));
                }
                (Op::CpTopBp, O::Stack { offset, size }) => {
                    self.pc = pc;
                    let v = self.global_at(*offset, u32::from(*size))?;
                    self.pc = pc + 1;
                    let ty = self.var_ty(v);
                    self.push(u32::from(*size), Slot::Temp(Expr::Place(Place::Var(v)), ty));
                }
                (Op::CpDownSp, O::Stack { offset, size }) => {
                    let size = u32::from(*size);
                    self.pc = pc;
                    let r = self.span(*offset, size).ok_or_else(|| {
                        self.at("an assignment that does not cover whole stack entries")
                    })?;
                    if matches!(self.stack[r.start].slot, Slot::Ret) {
                        return self.return_value(pc);
                    }
                    let v = self.var_at(*offset, size)?;
                    self.pc = pc + 1;
                    let (e, t, s) = self.pop_temp()?;
                    if s != size {
                        return Err(self.at("an assignment of a value of another size"));
                    }
                    self.refine_var(v, t);
                    self.push(
                        size,
                        Slot::Temp(Expr::Assign(Place::Var(v), Box::new(e)), t),
                    );
                }
                (Op::CpDownBp, O::Stack { offset, size }) => {
                    let size = u32::from(*size);
                    self.pc = pc;
                    let v = self.global_at(*offset, size)?;
                    self.pc = pc + 1;
                    let (e, t, s) = self.pop_temp()?;
                    if s != size {
                        return Err(self.at("an assignment of a value of another size"));
                    }
                    self.push(
                        size,
                        Slot::Temp(Expr::Assign(Place::Var(v), Box::new(e)), t),
                    );
                }
                (Op::Action, O::Action { routine, argc }) => {
                    self.pc = pc;
                    let e = self.action(*routine, *argc)?;
                    self.pc = pc + 1;
                    if let Some(s) = e {
                        if self.stack.len() == base {
                            return Ok(Event::Stmt(Stmt::Expr(s)));
                        }
                        self.pc = pc;
                        return Err(self.at("a void call inside an expression"));
                    }
                }
                (Op::Jsr, _) => {
                    self.pc = pc;
                    let e = self.call(pc)?;
                    self.pc = pc + 1;
                    if let Some(s) = e {
                        if self.stack.len() == base {
                            return Ok(Event::Stmt(Stmt::Expr(s)));
                        }
                        self.pc = pc;
                        return Err(self.at("a void call inside an expression"));
                    }
                }
                (Op::LogAnd | Op::LogOr, _) => {
                    let (r, _, _) = self.pop_temp()?;
                    let (l, _, _) = self.pop_temp()?;
                    let op = if x.opcode == Op::LogAnd {
                        BinOp::LogAnd
                    } else {
                        BinOp::LogOr
                    };
                    self.push(
                        4,
                        Slot::Temp(Expr::Binary(op, Box::new(l), Box::new(r)), Some(Ty::Int)),
                    );
                }
                (Op::Jz, _) => match self.stack.pop() {
                    Some(Entry {
                        slot: Slot::Dup, ..
                    }) => {}
                    Some(Entry {
                        slot: Slot::Temp(e, _),
                        size: 4,
                    }) if self.stack.len() == base && !has_dup(&e) => {
                        return Ok(Event::Cond(e, pc));
                    }
                    other => {
                        if let Some(o) = other {
                            self.stack.push(o);
                        }
                        self.pc = pc;
                        return Err(self.at("a conditional jump inside an expression"));
                    }
                },
                (Op::Jnz, _) => match self.stack.pop() {
                    Some(Entry {
                        slot: Slot::Temp(e, _),
                        size: 4,
                    }) => return Ok(Event::CaseTest(e, pc)),
                    other => {
                        if let Some(o) = other {
                            self.stack.push(o);
                        }
                        self.pc = pc;
                        return Err(self.at("a JNZ that is not a case test"));
                    }
                },
                (Op::MovSp, O::Offset(o)) => {
                    let n = -i64::from(*o);
                    if self.stack.len() == base + 1
                        && let Some(Entry {
                            size,
                            slot: Slot::Temp(..),
                        }) = self.stack.last()
                        && i64::from(*size) == n
                    {
                        let (e, _, _) = self.pop_temp()?;
                        return Ok(Event::Stmt(Stmt::Expr(e)));
                    }
                    self.pc = pc;
                    return Err(self.at("a MOVSP inside an expression"));
                }
                (Op::IncISp | Op::DecISp, O::Offset(off)) => {
                    self.pc = pc;
                    let inc = x.opcode == Op::IncISp;
                    let v = self.var_at(*off, 4)?;
                    let off = *off;
                    self.incdec(v, inc, pc, |y| {
                        matches!((y.opcode, &y.operands), (Op::CpTopSp, O::Stack { offset, size: 4 }) if *offset == off)
                    })?;
                }
                (Op::IncIBp | Op::DecIBp, O::Offset(off)) => {
                    self.pc = pc;
                    let inc = x.opcode == Op::IncIBp;
                    let v = self.global_at(*off, 4)?;
                    let off = *off;
                    self.incdec(v, inc, pc, |y| {
                        matches!((y.opcode, &y.operands), (Op::CpTopBp, O::Stack { offset, size: 4 }) if *offset == off)
                    })?;
                }
                (Op::StoreState, O::StoreState { locals, .. }) => {
                    self.pc = pc;
                    let e = self.closure(pc, *locals)?;
                    self.push(0, Slot::Closure(e));
                }
                (
                    Op::Destruct,
                    O::Destruct {
                        size,
                        keep_offset,
                        keep_size,
                    },
                ) => {
                    let (e, _, s) = self.pop_temp()?;
                    let whole = match &e {
                        Expr::Place(Place::Var(v))
                            if s == u32::from(*size) && self.sh.parts.contains_key(v) =>
                        {
                            *v
                        }
                        // A component of a vector value: `f().x`.
                        _ if s == 12 && *size == 12 && *keep_size == 4 && keep_offset % 4 == 0 => {
                            self.pc = pc + 1;
                            self.push(
                                4,
                                Slot::Temp(
                                    Expr::Member(Box::new(e), usize::from(*keep_offset / 4)),
                                    Some(Ty::Float),
                                ),
                            );
                            continue;
                        }
                        _ => {
                            self.pc = pc;
                            return Err(self.at("a DESTRUCT of something that is not a variable"));
                        }
                    };
                    let parts = self.sh.parts.get(&whole).cloned().unwrap_or_default();
                    let mut at = 0u32;
                    let mut picked = Vec::new();
                    for p in parts {
                        let ps = self.sh.script.vars[p].size;
                        if at >= u32::from(*keep_offset)
                            && at + ps <= u32::from(*keep_offset) + u32::from(*keep_size)
                        {
                            picked.push(p);
                        }
                        at += ps;
                    }
                    if picked.len() != 1
                        || self.sh.script.vars[picked[0]].size != u32::from(*keep_size)
                    {
                        self.pc = pc;
                        return Err(self.at("a DESTRUCT that keeps no single member"));
                    }
                    let ty = self.var_ty(picked[0]);
                    self.push(
                        u32::from(*keep_size),
                        Slot::Temp(Expr::Place(Place::Var(picked[0])), ty),
                    );
                }
                (Op::Nop, _) => {}
                (op, _) => {
                    self.pc = pc;
                    if let Some(u) = unary(op) {
                        let (e, t, s) = self.pop_temp()?;
                        let ty = match op {
                            Op::Neg => t,
                            _ => Some(Ty::Int),
                        };
                        self.pc = pc + 1;
                        self.push(s, Slot::Temp(Expr::Unary(u, Box::new(e)), ty));
                        continue;
                    }
                    let Some(b) = binary(op) else {
                        return Err(self.at("an instruction no statement shape explains"));
                    };
                    let (r, rt, _) = self.pop_temp()?;
                    let (l, lt, _) = self.pop_temp()?;
                    if is_dup_expr(&r) || (is_dup_expr(&l) && b != BinOp::Eq) {
                        return Err(self.at("a copied value used other than by a case test"));
                    }
                    let (lty, rty, res) = operand_types(op, x.ty, &x.operands);
                    self.refine(&l, lty);
                    self.refine(&r, rty);
                    let _ = (lt, rt);
                    let size = res.and_then(|t| t.size()).unwrap_or(4);
                    self.pc = pc + 1;
                    self.push(
                        size,
                        Slot::Temp(Expr::Binary(b, Box::new(l), Box::new(r)), res),
                    );
                }
            }
        }
    }

    fn refine_var(&mut self, v: VarId, t: Option<Ty>) {
        let e = Expr::Place(Place::Var(v));
        self.refine(&e, t);
    }

    /// `++`/`--` on `v`: postfix when the value copied just before is `v`
    /// (`CPTOPSP v; INCISP`), prefix when `copies_back` matches the next
    /// instruction (`INCISP; CPTOPSP v`).
    fn incdec(
        &mut self,
        v: VarId,
        inc: bool,
        pc: usize,
        copies_back: impl Fn(&Instruction) -> bool,
    ) -> Res<()> {
        if let Some(Entry {
            slot: Slot::Temp(Expr::Place(Place::Var(w)), _),
            ..
        }) = self.stack.last()
            && *w == v
        {
            self.stack.pop();
            self.push(
                4,
                Slot::Temp(
                    Expr::IncDec {
                        place: Place::Var(v),
                        inc,
                        post: true,
                    },
                    Some(Ty::Int),
                ),
            );
            self.pc = pc + 1;
            return Ok(());
        }
        if self.sh.ins.get(pc + 1).is_some_and(copies_back) {
            self.push(
                4,
                Slot::Temp(
                    Expr::IncDec {
                        place: Place::Var(v),
                        inc,
                        post: false,
                    },
                    Some(Ty::Int),
                ),
            );
            self.pc = pc + 2;
            return Ok(());
        }
        Err(self.at("an increment without the copy ++ or -- makes"))
    }

    /// `return e;`: the value is copied into the return slot at `pc`, then
    /// everything above the frame is popped and the jump goes to the epilogue;
    /// the pop of the value that would have followed stays behind as dead code.
    fn return_value(&mut self, pc: usize) -> Res<Event> {
        let (e, t, size) = self.pop_temp()?;
        if let Some(Slot::Ret) = self.stack.first().map(|s| s.slot.clone())
            && self.sh.script.funcs[self.func].ret.is_none()
        {
            self.sh.script.funcs[self.func].ret = t;
        }
        let above = self.bytes_above(self.frame) + size;
        self.pc = pc + 1;
        match self.ins(self.pc)? {
            Instruction {
                opcode: Op::MovSp,
                operands: O::Offset(o),
                ..
            } if -i64::from(*o) == i64::from(above) => self.pc += 1,
            _ => return Err(self.at("a return that does not pop its frame")),
        }
        if !self.is_return_jump(self.pc) {
            return Err(self.at("a return that does not jump to the epilogue"));
        }
        self.pc += 1;
        if let Some(Instruction {
            opcode: Op::MovSp,
            operands: O::Offset(o),
            ..
        }) = self.sh.ins.get(self.pc)
            && -i64::from(*o) == i64::from(size)
        {
            self.pc += 1;
        }
        Ok(Event::Stmt(Stmt::Return(Some(e))))
    }

    /// An engine call: arguments from the stack, first on top. Returns the
    /// call when it leaves no value (a statement), else pushes it.
    fn action(&mut self, routine: u16, argc: u8) -> Res<Option<Expr>> {
        let table = self.sh.table;
        let f = table
            .functions
            .get(usize::from(routine))
            .ok_or_else(|| self.at("a call to an engine routine nwscript.nss does not declare"))?;
        let declared = f.params.len().min(usize::from(argc));
        let mut args = Vec::new();
        for p in &f.params[..declared] {
            let need =
                sema_size(&p.ty).ok_or_else(|| self.at("an engine parameter of unknown size"))?;
            if p.ty == nwscript_sema::Type::Action {
                match self.stack.pop() {
                    Some(Entry {
                        slot: Slot::Closure(e),
                        ..
                    }) => args.push(e),
                    _ => return Err(self.at("an action argument that is not a deferred call")),
                }
                continue;
            }
            // A defaulted vector is pushed as three float constants.
            if need == 12
                && let Some(v) = self.three_floats()
            {
                args.push(Expr::Vector(v));
                continue;
            }
            let (e, t, s) = self.pop_temp()?;
            if s != need {
                return Err(self.at("an engine call whose arguments do not match its declaration"));
            }
            let want = Ty::of_sema(&p.ty);
            self.refine(&e, want);
            let _ = t;
            args.push(e);
        }
        // Arguments past the declaration: whatever single values the bytecode pushed.
        for _ in declared..usize::from(argc) {
            let (e, _, s) = self.pop_temp()?;
            if s != 4 {
                return Err(self.at("an engine call with more arguments than its declaration"));
            }
            args.push(e);
        }
        let call = Expr::Engine { routine, args };
        match sema_size(&f.ret) {
            Some(0) => Ok(Some(call)),
            Some(n) => {
                self.push(n, Slot::Temp(call, Ty::of_sema(&f.ret)));
                Ok(None)
            }
            None => Err(self.at("an engine routine returning a struct")),
        }
    }

    /// Three float constants on top of the stack, as a vector (z on the bottom).
    fn three_floats(&mut self) -> Option<[f32; 3]> {
        let n = self.stack.len();
        if n < 3 {
            return None;
        }
        // x was pushed first.
        let mut v = [0.0; 3];
        for (k, e) in self.stack[n - 3..].iter().enumerate() {
            match e.slot {
                Slot::Temp(Expr::Float(f), _) => v[k] = f,
                _ => return None,
            }
        }
        self.stack.truncate(n - 3);
        Some(v)
    }

    /// A user call at `pc`. Returns the call when it leaves no value.
    fn call(&mut self, pc: usize) -> Res<Option<Expr>> {
        let t = self.target(pc)?;
        let f = *self
            .sh
            .by_entry
            .get(&t)
            .ok_or_else(|| self.at("a call into the middle of a function"))?;
        let l = self.sh.layout[f].clone();
        let p = l
            .params
            .ok_or_else(|| self.at("a call to a function of unknown stack effect"))?;
        let mut args = Vec::new();
        let mut got = 0;
        while got < p {
            let (e, ty, s) = self.pop_temp()?;
            got += s;
            args.push((e, ty, s));
        }
        if got != p {
            return Err(self.at("a call whose arguments do not fill its parameters"));
        }
        // What this call says about the callee's parameters.
        let known = self.sh.calls[f].args.clone();
        match known {
            None => {
                self.sh.calls[f].args = Some(args.iter().map(|(_, t, s)| (*s, *t)).collect());
            }
            Some(mut k) => {
                for (slot, (_, t, s)) in k.iter_mut().zip(&args) {
                    if slot.1.is_none() && slot.0 == *s {
                        slot.1 = *t;
                    }
                }
                self.sh.calls[f].args = Some(k);
            }
        }
        if let Some(ps) = self.sh.params[f].clone() {
            for (v, (_, t, _)) in ps.iter().zip(&args) {
                let e = Expr::Place(Place::Var(*v));
                self.refine(&e, *t);
            }
        }
        let call = Expr::User {
            func: f,
            args: args.into_iter().map(|a| a.0).collect(),
        };
        if l.ret == 0 {
            return Ok(Some(call));
        }
        match self.stack.pop() {
            Some(Entry {
                size,
                slot: Slot::Reserved(ty),
            }) if size == l.ret => {
                if self.sh.calls[f].ret.is_none() {
                    self.sh.calls[f].ret = ty;
                }
                let ty = ty.or(self.sh.script.funcs[f].ret);
                self.push(size, Slot::Temp(call, ty));
                Ok(None)
            }
            _ => Err(self.at("a call without its reserved return slot")),
        }
    }

    /// The deferred action a `STORE_STATE` at `pc` saves: its body, between
    /// the `JMP` after it and that jump's target, is one call.
    fn closure(&mut self, pc: usize, locals: u32) -> Res<Expr> {
        if self.ins(pc + 1)?.opcode != Op::Jmp {
            return Err(self.at("a STORE_STATE not followed by a jump over its body"));
        }
        let after = self.target(pc + 1)?;
        if after < pc + 3 || self.ins(after - 1)?.opcode != Op::Retn {
            return Err(self.at("a deferred action whose body does not end with RETN"));
        }
        let stack: Vec<Entry> = if locals == 0 {
            Vec::new()
        } else {
            let r = self
                .span(-(locals as i32), locals)
                .ok_or_else(|| self.at("a STORE_STATE that saves part of a stack entry"))?;
            self.stack[r].to_vec()
        };
        let mut sub = Fp {
            sh: &mut *self.sh,
            func: self.func,
            frame: stack.len(),
            stack,
            pc: pc + 2,
            epilogue: after - 1,
            ctl: Vec::new(),
        };
        let body = sub.block(Until::At(after - 1), None)?;
        match <[Stmt; 1]>::try_from(body) {
            Ok([Stmt::Expr(e @ (Expr::Engine { .. } | Expr::User { .. }))]) => {
                self.pc = after;
                Ok(e)
            }
            _ => Err(self.at("a deferred action that is not a single call")),
        }
    }
}

/// Whether an expression holds a [`DUP`] anywhere.
fn has_dup(e: &Expr) -> bool {
    match e {
        Expr::Place(Place::Var(v)) => *v == DUP,
        Expr::Engine { args, .. } | Expr::User { args, .. } => args.iter().any(has_dup),
        Expr::Unary(_, x) | Expr::Assign(_, x) | Expr::Member(x, _) => has_dup(x),
        Expr::Binary(_, l, r) => has_dup(l) || has_dup(r),
        _ => false,
    }
}

fn is_dup(x: &Instruction) -> bool {
    x.opcode == Op::CpTopSp
        && x.operands
            == (O::Stack {
                offset: -4,
                size: 4,
            })
}

fn unary(op: Op) -> Option<UnOp> {
    Some(match op {
        Op::Neg => UnOp::Neg,
        Op::Not => UnOp::Not,
        Op::Comp => UnOp::Comp,
        _ => return None,
    })
}

fn binary(op: Op) -> Option<BinOp> {
    Some(match op {
        Op::IncOr => BinOp::BitOr,
        Op::ExcOr => BinOp::BitXor,
        Op::BoolAnd => BinOp::BitAnd,
        Op::Equal => BinOp::Eq,
        Op::NEqual => BinOp::Ne,
        Op::Geq => BinOp::Ge,
        Op::Gt => BinOp::Gt,
        Op::Lt => BinOp::Lt,
        Op::Leq => BinOp::Le,
        Op::ShLeft => BinOp::Shl,
        Op::ShRight => BinOp::Shr,
        Op::UShRight => BinOp::UShr,
        Op::Add => BinOp::Add,
        Op::Sub => BinOp::Sub,
        Op::Mul => BinOp::Mul,
        Op::Div => BinOp::Div,
        Op::Mod => BinOp::Mod,
        _ => return None,
    })
}

/// The operand types and result type of a binary instruction.
fn operand_types(op: Op, ty: T, operands: &O) -> (Option<Ty>, Option<Ty>, Option<Ty>) {
    let pair = match ty {
        T::INT_INT => (Some(Ty::Int), Some(Ty::Int)),
        T::FLOAT_FLOAT => (Some(Ty::Float), Some(Ty::Float)),
        T::OBJECT_OBJECT => (Some(Ty::Object), Some(Ty::Object)),
        T::STRING_STRING => (Some(Ty::String), Some(Ty::String)),
        T::INT_FLOAT => (Some(Ty::Int), Some(Ty::Float)),
        T::FLOAT_INT => (Some(Ty::Float), Some(Ty::Int)),
        T::VECTOR_VECTOR => (Some(Ty::Vector), Some(Ty::Vector)),
        T::VECTOR_FLOAT => (Some(Ty::Vector), Some(Ty::Float)),
        T::FLOAT_VECTOR => (Some(Ty::Float), Some(Ty::Vector)),
        t => match t.engine_pair() {
            Some(n) => (Some(Ty::Engine(n)), Some(Ty::Engine(n))),
            None => (None, None),
        },
    };
    let _ = operands;
    let res = match op {
        Op::Equal | Op::NEqual | Op::Geq | Op::Gt | Op::Lt | Op::Leq => Some(Ty::Int),
        Op::Add | Op::Sub | Op::Mul | Op::Div => match ty {
            T::INT_INT => Some(Ty::Int),
            T::STRING_STRING => Some(Ty::String),
            T::VECTOR_VECTOR | T::VECTOR_FLOAT | T::FLOAT_VECTOR => Some(Ty::Vector),
            _ => Some(Ty::Float),
        },
        _ => Some(Ty::Int),
    };
    (pair.0, pair.1, res)
}

/// After every function is read: parameter types from the call sites,
/// defaults a call relies on, and names.
fn finish(sh: &mut Shared) {
    for f in 0..sh.script.funcs.len() {
        if let (Some(ps), Some(args)) = (sh.params[f].clone(), sh.calls[f].args.clone()) {
            for (v, (_, t)) in ps.iter().zip(args) {
                if sh.script.vars[*v].ty.is_none() {
                    sh.script.vars[*v].ty = t;
                }
            }
        }
        if sh.script.funcs[f].ret.is_none() && sh.layout[f].ret > 0 {
            sh.script.funcs[f].ret = sh.calls[f].ret.or(Some(Ty::Int));
        }
        let n = sh.script.funcs[f].params.len();
        sh.script.funcs[f].defaults = vec![None; n];
    }
    // A trailing argument with no literal form (a negative float, a vector)
    // can only have come from a default the callee declares.
    let mut needs: Vec<(usize, usize, Expr)> = Vec::new();
    for f in &sh.script.funcs {
        if let Ok(body) = &f.body {
            visit_calls(body, &mut |func, args| {
                for (k, a) in args.iter().enumerate().rev() {
                    let unwritable = match a {
                        Expr::Float(x) => !crate::print::float_writable(*x),
                        Expr::Vector(_) => true,
                        _ => false,
                    };
                    if !unwritable {
                        break;
                    }
                    needs.push((func, k, a.clone()));
                }
            });
        }
    }
    for (func, k, e) in needs {
        let d = &mut sh.script.funcs[func].defaults;
        if k < d.len() && d[k].is_none() {
            d[k] = Some(e);
        }
    }
    // Defaults must be trailing: fill the gaps after the first one with
    // what every call passes, or drop the ones before a gap.
    for f in &mut sh.script.funcs {
        let d = &mut f.defaults;
        if let Some(first) = d.iter().position(Option::is_some)
            && d[first..].iter().any(Option::is_none)
        {
            let last_gap = d.iter().rposition(Option::is_none).unwrap_or(0);
            for x in d[..=last_gap].iter_mut() {
                *x = None;
            }
        }
    }
    for f in &mut sh.script.funcs {
        if let Ok(b) = &mut f.body {
            fors(b);
        }
    }
    name_vars(sh);
}

fn visit_calls(body: &[Stmt], f: &mut impl FnMut(usize, &[Expr])) {
    fn expr(e: &Expr, f: &mut impl FnMut(usize, &[Expr])) {
        match e {
            Expr::User { func, args } => {
                f(*func, args);
                args.iter().for_each(|a| expr(a, f));
            }
            Expr::Engine { args, .. } => args.iter().for_each(|a| expr(a, f)),
            Expr::Unary(_, x) | Expr::Assign(_, x) | Expr::Member(x, _) => expr(x, f),
            Expr::Binary(_, l, r) => {
                expr(l, f);
                expr(r, f);
            }
            _ => {}
        }
    }
    for s in body {
        match s {
            Stmt::Expr(e) | Stmt::Return(Some(e)) => expr(e, f),
            Stmt::If(c, t, e) => {
                expr(c, f);
                visit_calls(t, f);
                if let Some(e) = e {
                    visit_calls(e, f);
                }
            }
            Stmt::While(c, b) | Stmt::DoWhile(b, c) | Stmt::Switch(c, b) => {
                expr(c, f);
                visit_calls(b, f);
            }
            Stmt::For(i, c, st, b) => {
                expr(i, f);
                expr(c, f);
                expr(st, f);
                visit_calls(b, f);
            }
            Stmt::Block(b) => visit_calls(b, f),
            _ => {}
        }
    }
}

/// Name variables in the order the source declares them: globals, then
/// each function's parameters and locals, numbered per function.
fn name_vars(sh: &mut Shared) {
    fn decls(body: &[Stmt], out: &mut Vec<VarId>) {
        for st in body {
            match st {
                Stmt::Decl(v) => out.push(*v),
                Stmt::If(_, t, e) => {
                    decls(t, out);
                    if let Some(e) = e {
                        decls(e, out);
                    }
                }
                Stmt::While(_, b)
                | Stmt::DoWhile(b, _)
                | Stmt::Switch(_, b)
                | Stmt::Block(b)
                | Stmt::For(_, _, _, b) => decls(b, out),
                _ => {}
            }
        }
    }
    let mut order: Vec<VarId> = Vec::new();
    for f in &sh.script.funcs {
        if f.kind == FuncKind::Globals
            && let Ok(b) = &f.body
        {
            decls(b, &mut order);
        }
    }
    for f in &sh.script.funcs {
        if f.kind == FuncKind::Globals {
            continue;
        }
        order.extend(&f.params);
        if let Ok(b) = &f.body {
            decls(b, &mut order);
        }
    }
    order.extend(0..sh.script.vars.len());
    let mut named = vec![false; sh.script.vars.len()];
    let mut counters: HashMap<(Option<usize>, u8), u32> = HashMap::new();
    let structures = &sh.table.structures;
    for v in order {
        // A slot of a joined variable goes by the whole variable's name.
        let v = sh.script.root(v).0;
        if named[v] {
            continue;
        }
        named[v] = true;
        let var = &sh.script.vars[v];
        let prefix = match var.ty {
            None | Some(Ty::Int) => "n".to_string(),
            Some(Ty::Float) => "f".into(),
            Some(Ty::String) => "s".into(),
            Some(Ty::Object) => "o".into(),
            Some(Ty::Vector) => "v".into(),
            Some(Ty::Engine(n)) => match structures.get(usize::from(n)).map(String::as_str) {
                Some("event") => "ev".into(),
                Some(s) => s.chars().next().map(String::from).unwrap_or("e".into()),
                None => "e".into(),
            },
            Some(Ty::Struct(_)) => "st".into(),
        };
        let (kind, k) = match var.kind {
            VarKind::Global => ("Global", 0u8),
            VarKind::Param => ("Param", 1),
            VarKind::Local => ("Local", 2),
        };
        let c = counters.entry((var.owner, k)).or_insert(0);
        *c += 1;
        sh.script.vars[v].name = format!("{prefix}{kind}{c}");
    }
}

/// `init; while (cond) { ...; step; }` as BioWare writes it, `for (init;
/// cond; step) { ... }`, where the two compile alike: the body declares
/// nothing at its top level (a `for` body's locals are popped before the
/// step) and does not `continue` (which in a `for` goes to the step).
fn fors(body: &mut Vec<Stmt>) {
    for st in body.iter_mut() {
        match st {
            Stmt::If(_, t, e) => {
                fors(t);
                if let Some(e) = e {
                    fors(e);
                }
            }
            Stmt::While(_, b) | Stmt::DoWhile(b, _) | Stmt::Switch(_, b) | Stmt::Block(b) => {
                fors(b)
            }
            _ => {}
        }
    }
    let mut k = 1;
    while k < body.len() {
        let fits = match (&body[k - 1], &body[k]) {
            (Stmt::Expr(init @ (Expr::Assign(..) | Expr::IncDec { .. })), Stmt::While(_, b)) => {
                let var = |e: &Expr| match e {
                    Expr::Assign(p, _) | Expr::IncDec { place: p, .. } => Some(p.clone()),
                    _ => None,
                };
                match b.last() {
                    Some(Stmt::Expr(step @ (Expr::Assign(..) | Expr::IncDec { .. }))) => {
                        var(init) == var(step)
                            && !b.iter().any(|s| matches!(s, Stmt::Decl(_)))
                            && !continues(b)
                    }
                    _ => false,
                }
            }
            _ => false,
        };
        if fits {
            let Stmt::Expr(init) = body.remove(k - 1) else {
                unreachable!()
            };
            let Stmt::While(cond, mut b) = body.remove(k - 1) else {
                unreachable!()
            };
            let Some(Stmt::Expr(step)) = b.pop() else {
                unreachable!()
            };
            body.insert(k - 1, Stmt::For(init, cond, step, b));
        }
        k += 1;
    }
}

/// Whether a loop body continues its loop (not a nested one).
fn continues(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Continue => true,
        Stmt::If(_, t, e) => continues(t) || e.as_deref().is_some_and(continues),
        Stmt::Block(b) | Stmt::Switch(_, b) => continues(b),
        _ => false,
    })
}
