//! Engine calls with their arguments followed through the program: what a
//! script's own functions pass on from their parameters, what locals and
//! globals hold, and what earlier calls return.
//!
//! [`facts::calls`](crate::facts::calls) reads an argument only where one
//! `CONST*` instruction pushes it. A script that calls
//! `UT_SetPlotBooleanFlag(OBJECT_SELF, SW_PLOT_BOOLEAN_01, TRUE)` reaches
//! the engine's `SetLocalBoolean` through a function of its own, with the
//! index read from a global, so that call's arguments are all computed
//! there. [`resolved_calls`] runs each subroutine over abstract values (a
//! literal; the subroutine's parameter at a stack offset; `OBJECT_SELF`;
//! what a routine returns, with its arguments; unknown) and, at each `JSR`
//! and each deferred action, puts the caller's values in place of the
//! callee's parameters. A global is a literal when the globals initialiser
//! pushes one and no instruction writes the global again.
//!
//! `OBJECT_SELF` is the object the code runs as. A deferred action given to
//! a routine taking an object before the action (`AssignCommand`) runs as
//! that object, so inside it `OBJECT_SELF` is that argument; given to one
//! taking none (`DelayCommand`, `ActionDoCommand`) it runs as the caller.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::cfg::{self, Cfg, Depths, SubKind};
use crate::facts::{ArgType, Literal};
use crate::{Opcode, Operands, Program, Type};

/// An argument's value, as far as the program shows it.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Literal(Literal),
    /// What a routine returns, with its arguments (`None` where unknown):
    /// `GetObjectByTag("k37_inv_statue", 0)`.
    Call {
        routine: u16,
        args: Vec<Option<Value>>,
    },
}

/// One engine call with its arguments resolved, and the path the program
/// takes to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    /// File offsets from the instruction where the arguments are known to
    /// the `ACTION`: each `JSR` (or the `STORE_STATE` of a deferred action)
    /// on the way, then the `ACTION`. A call whose arguments its own
    /// subroutine settles has the `ACTION` alone.
    pub path: Vec<u32>,
    pub routine: u16,
    /// One entry per argument: the value, or `None` where unknown.
    pub args: Vec<Option<Value>>,
}

/// How deep a routine's return value nests in another's arguments before
/// it is left unknown.
const MAX_NESTING: usize = 3;
/// The open calls one subroutine passes up to its callers, at most.
const MAX_OPEN: usize = 4096;
/// The resolved calls one program yields, at most.
const MAX_RESOLVED: usize = 20_000;
/// Visits of one block before its values are widened to unknown.
const MAX_VISITS: u32 = 32;

/// Every engine call whose arguments following the program resolves
/// further than [`facts::calls`](crate::facts::calls) reads them: a
/// literal where that reads none, or a routine's return value. `param`
/// and `ret` are [`facts::calls`](crate::facts::calls)'s. Calls are in
/// the order the analysis meets them; an `ACTION` reached with the same
/// arguments along several paths is listed once, by the first.
pub fn resolved_calls(
    program: &Program,
    param: impl Fn(u16, usize) -> Option<ArgType>,
    ret: impl Fn(u16) -> Option<ArgType>,
) -> Vec<Resolved> {
    if program.is_empty() {
        return Vec::new();
    }
    let plain: HashMap<u32, Vec<Option<Literal>>> = crate::facts::calls(program, &param, &ret)
        .into_iter()
        .map(|c| (c.offset, c.args))
        .collect();
    let graph = cfg::build(program);
    let size = |r: u16, k: usize| param(r, k).map_or(4, ArgType::size);
    let arg_bytes = |r: u16, argc: u8| -> i32 { (0..usize::from(argc)).map(|k| size(r, k)).sum() };
    let depths = cfg::stack_depths(program, &graph, &|r, argc| {
        ret(r).map(|t| t.size() - arg_bytes(r, argc))
    });
    let sub_at: HashMap<usize, usize> = graph
        .subs
        .iter()
        .enumerate()
        .map(|(i, s)| (graph.blocks[s.entry].start, i))
        .collect();
    let mut a = Analysis {
        program,
        graph: &graph,
        depths: &depths,
        sub_at,
        param: &param,
        ret: &ret,
        globals: HashMap::new(),
        summaries: vec![None; graph.subs.len()],
        busy: vec![false; graph.subs.len()],
        out: Vec::new(),
        seen: HashSet::new(),
        plain: &plain,
    };
    a.globals = a.globals();
    let Some(entry) = graph.subs.iter().position(|s| s.kind == SubKind::Entry) else {
        return Vec::new();
    };
    let summary = a.summary(entry);
    for r in summary.open.iter() {
        let env = |_: i32| V::Unknown;
        let (args, _) = subst_all(&r.args, &env, &V::Lit(Literal::Object(0)));
        a.emit(Rec {
            path: r.path.clone(),
            routine: r.routine,
            args,
        });
    }
    a.out
}

/// An abstract value of one stack element.
#[derive(Debug, Clone, PartialEq)]
enum V {
    Unknown,
    Lit(Literal),
    /// The caller's element at this byte offset below the subroutine's
    /// entry: a parameter, or for a deferred action a saved local.
    Param(i32),
    /// `OBJECT_SELF` of the code running: the caller's, or for a deferred
    /// action the object it is assigned to.
    SelfObj,
    /// What `routine` returned.
    Ret {
        routine: u16,
        args: Rc<Vec<V>>,
    },
}

impl V {
    fn is_open(&self) -> bool {
        match self {
            V::Param(_) | V::SelfObj => true,
            V::Ret { args, .. } => args.iter().any(V::is_open),
            _ => false,
        }
    }

    fn nesting(&self) -> usize {
        match self {
            V::Ret { args, .. } => 1 + args.iter().map(V::nesting).max().unwrap_or(0),
            _ => 0,
        }
    }

    fn join(&self, other: &V) -> V {
        if self == other {
            self.clone()
        } else {
            V::Unknown
        }
    }
}

/// `v` with the caller's values for the parameters (`env`) and the object
/// the code runs as (`me`).
fn subst(v: &V, env: &dyn Fn(i32) -> V, me: &V) -> V {
    match v {
        V::Param(o) => env(*o),
        V::SelfObj => me.clone(),
        V::Ret { routine, args } => {
            let args: Vec<V> = args.iter().map(|a| subst(a, env, me)).collect();
            let r = V::Ret {
                routine: *routine,
                args: Rc::new(args),
            };
            if r.nesting() > MAX_NESTING {
                V::Unknown
            } else {
                r
            }
        }
        other => other.clone(),
    }
}

fn subst_all(args: &[V], env: &dyn Fn(i32) -> V, me: &V) -> (Vec<V>, bool) {
    let out: Vec<V> = args.iter().map(|a| subst(a, env, me)).collect();
    let open = out.iter().any(V::is_open);
    (out, open)
}

/// The values of a subroutine's stack: its own elements from its entry
/// up, and those below the entry it has written.
#[derive(Debug, Clone, PartialEq, Default)]
struct State {
    stack: Vec<V>,
    below: BTreeMap<i32, V>,
    /// Deferred actions met and not yet handed to their routine.
    pending: Vec<Rc<Pending>>,
}

impl State {
    fn depth(&self) -> i32 {
        self.stack.len() as i32 * 4
    }

    /// The element at byte offset `at` from the entry.
    fn get(&self, at: i32) -> V {
        if at % 4 != 0 {
            return V::Unknown;
        }
        if at >= 0 {
            self.stack
                .get((at / 4) as usize)
                .cloned()
                .unwrap_or(V::Unknown)
        } else {
            self.below.get(&at).cloned().unwrap_or(V::Param(at))
        }
    }

    fn set(&mut self, at: i32, v: V) {
        if at % 4 != 0 {
            return;
        }
        if at >= 0 {
            if let Some(s) = self.stack.get_mut((at / 4) as usize) {
                *s = v;
            }
        } else {
            self.below.insert(at, v);
        }
    }

    fn pop(&mut self, bytes: i32) -> Option<()> {
        let n = usize::try_from(bytes / 4).ok()?;
        let len = self.stack.len().checked_sub(n)?;
        self.stack.truncate(len);
        Some(())
    }

    fn join(&self, other: &State) -> Option<State> {
        if self.stack.len() != other.stack.len() {
            return None;
        }
        let stack = self
            .stack
            .iter()
            .zip(&other.stack)
            .map(|(a, b)| a.join(b))
            .collect();
        let mut below = BTreeMap::new();
        for k in self.below.keys().chain(other.below.keys()) {
            below.insert(*k, self.get(*k).join(&other.get(*k)));
        }
        let pending = if self.pending == other.pending {
            self.pending.clone()
        } else {
            Vec::new()
        };
        Some(State {
            stack,
            below,
            pending,
        })
    }

    fn widen(&mut self) {
        self.stack.fill(V::Unknown);
        for v in self.below.values_mut() {
            *v = V::Unknown;
        }
    }
}

/// A call met in a subroutine, with values that may still name its
/// parameters or `OBJECT_SELF`.
#[derive(Debug, Clone)]
struct Rec {
    path: Vec<u32>,
    routine: u16,
    args: Vec<V>,
}

/// What a subroutine's callers need of it: the calls whose arguments
/// depend on how it is called, and what it leaves below its entry (its
/// return value) when it returns.
#[derive(Debug, Clone, Default)]
struct Summary {
    open: Rc<Vec<Rec>>,
    writes: Rc<BTreeMap<i32, V>>,
}

/// A deferred action met and not yet handed to its routine: its
/// subroutine, the `STORE_STATE`'s offset, and the values it saves.
#[derive(Debug, Clone, PartialEq)]
struct Pending {
    sub: usize,
    at: u32,
    saved: State,
}

struct Analysis<'a> {
    program: &'a Program,
    graph: &'a Cfg,
    depths: &'a Depths,
    sub_at: HashMap<usize, usize>,
    param: &'a dyn Fn(u16, usize) -> Option<ArgType>,
    ret: &'a dyn Fn(u16) -> Option<ArgType>,
    /// The globals that keep their initial value, by offset from BP.
    globals: HashMap<i32, V>,
    summaries: Vec<Option<Summary>>,
    busy: Vec<bool>,
    out: Vec<Resolved>,
    seen: HashSet<(u32, String)>,
    plain: &'a HashMap<u32, Vec<Option<Literal>>>,
}

impl Analysis<'_> {
    fn size(&self, r: u16, k: usize) -> i32 {
        (self.param)(r, k).map_or(4, ArgType::size)
    }

    /// `OBJECT_SELF` at instruction `i`: the script's own object, or in a
    /// deferred action's body the object it runs as, which its routine
    /// settles.
    fn me(&self, i: usize) -> V {
        let sub = self.graph.blocks[self.graph.block_of[i]].sub;
        if sub.is_some_and(|s| self.graph.subs[s].kind == SubKind::Closure) {
            V::SelfObj
        } else {
            V::Lit(Literal::Object(0))
        }
    }

    /// The subroutine a `JSR` or `STORE_STATE` at instruction `i` names.
    fn target(&self, i: usize) -> Option<usize> {
        let x = &self.program.instructions()[i];
        let at = self.program.offsets()[i];
        let t = match x.opcode {
            Opcode::Jsr => x.jump_target(at)?,
            Opcode::StoreState => at + u32::from(x.ty.0),
            _ => return None,
        };
        self.sub_at.get(&self.program.index_at(t)?).copied()
    }

    /// The globals: the values the initialiser leaves when it saves BP,
    /// less every global an instruction writes.
    fn globals(&self) -> HashMap<i32, V> {
        let ins = self.program.instructions();
        let saves: Vec<usize> = (0..ins.len())
            .filter(|&i| ins[i].opcode == Opcode::SaveBp)
            .collect();
        let [save] = saves[..] else {
            return HashMap::new();
        };
        let Some(sub) = self.graph.blocks[self.graph.block_of[save]].sub else {
            return HashMap::new();
        };
        // Straight-line code from the initialiser's entry to SAVEBP.
        let mut st = State::default();
        let mut i = self.graph.blocks[self.graph.subs[sub].entry].start;
        while i < save {
            let x = &ins[i];
            if x.opcode.is_jump() || x.opcode == Opcode::Retn {
                return HashMap::new();
            }
            if self.step(i, &mut st, None, &HashMap::new()).is_none() {
                return HashMap::new();
            }
            i += 1;
        }
        let mut written = HashSet::new();
        for x in ins {
            match (&x.opcode, &x.operands) {
                (Opcode::CpDownBp, Operands::Stack { offset, size }) => {
                    for k in (0..i32::from(*size)).step_by(4) {
                        written.insert(offset + k);
                    }
                }
                (Opcode::IncIBp | Opcode::DecIBp, Operands::Offset(o)) => {
                    written.insert(*o);
                }
                _ => {}
            }
        }
        let d = st.depth();
        let mut out = HashMap::new();
        for (k, v) in st.stack.iter().enumerate() {
            let o = k as i32 * 4 - d;
            if written.contains(&o) {
                continue;
            }
            let v = match v {
                V::SelfObj => V::Lit(Literal::Object(0)),
                v if v.is_open() => continue,
                v => v.clone(),
            };
            out.insert(o, v);
        }
        out
    }

    /// The summary of subroutine `sid`, computed once.
    fn summary(&mut self, sid: usize) -> Summary {
        if let Some(s) = &self.summaries[sid] {
            return s.clone();
        }
        if self.busy[sid] {
            return Summary::default();
        }
        self.busy[sid] = true;
        let s = self.analyse(sid);
        self.busy[sid] = false;
        self.summaries[sid] = Some(s.clone());
        s
    }

    /// Whether the edge into block `s` stays in subroutine `sid`.
    fn follows(&self, sid: usize, s: usize) -> bool {
        self.graph.blocks[s].sub == Some(sid)
            || !self.sub_at.contains_key(&self.graph.blocks[s].start)
    }

    fn analyse(&mut self, sid: usize) -> Summary {
        let graph = self.graph;
        let n = graph.blocks.len();
        let entry = graph.subs[sid].entry;
        let mut at_entry: Vec<Option<State>> = vec![None; n];
        let mut broken = vec![false; n];
        let mut visits = vec![0u32; n];
        at_entry[entry] = Some(State::default());
        let mut queue = std::collections::VecDeque::from([entry]);
        let mut callees: HashMap<usize, Summary> = HashMap::new();
        // Values at each block's entry, to a fixed point.
        while let Some(b) = queue.pop_front() {
            let Some(mut st) = at_entry[b].clone() else {
                continue;
            };
            visits[b] += 1;
            let blk = &graph.blocks[b];
            let mut ok = true;
            for i in blk.start..blk.end {
                self.prepare(i, &mut callees);
                if self.step(i, &mut st, None, &callees).is_none() {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            for &s in &blk.succs {
                if !self.follows(sid, s) || broken[s] {
                    continue;
                }
                let next = match &at_entry[s] {
                    None => Some(st.clone()),
                    Some(old) => match old.join(&st) {
                        Some(mut j) => {
                            if visits[s] > MAX_VISITS {
                                j.widen();
                            }
                            (j != *old).then_some(j)
                        }
                        None => {
                            broken[s] = true;
                            at_entry[s] = None;
                            None
                        }
                    },
                };
                if let Some(j) = next {
                    at_entry[s] = Some(j);
                    queue.push_back(s);
                }
            }
        }
        // One pass over each block with its settled values, collecting.
        let mut open = Vec::new();
        let mut writes: Option<BTreeMap<i32, V>> = None;
        for b in 0..n {
            let Some(mut st) = at_entry[b].clone() else {
                continue;
            };
            if broken[b] {
                continue;
            }
            let blk = &graph.blocks[b];
            for i in blk.start..blk.end {
                if self.program.instructions()[i].opcode == Opcode::Retn {
                    match &mut writes {
                        None => writes = Some(st.below.clone()),
                        Some(w) => {
                            let mut j = BTreeMap::new();
                            for k in w.keys().chain(st.below.keys()) {
                                let a = w.get(k).cloned().unwrap_or(V::Param(*k));
                                j.insert(*k, a.join(&st.get(*k)));
                            }
                            *w = j;
                        }
                    }
                }
                let mut found = Vec::new();
                if self.step(i, &mut st, Some(&mut found), &callees).is_none() {
                    break;
                }
                for r in found {
                    if r.args.iter().any(V::is_open) {
                        if open.len() < MAX_OPEN {
                            open.push(r);
                        }
                    } else {
                        self.emit(r);
                    }
                }
            }
        }
        Summary {
            open: Rc::new(open),
            writes: Rc::new(writes.unwrap_or_default()),
        }
    }

    /// The summary of the subroutine the instruction at `i` calls, before
    /// the instruction runs.
    fn prepare(&mut self, i: usize, callees: &mut HashMap<usize, Summary>) {
        let op = self.program.instructions()[i].opcode;
        if matches!(op, Opcode::Jsr | Opcode::StoreState)
            && let Some(t) = self.target(i)
            && !callees.contains_key(&t)
        {
            let s = self.summary(t);
            callees.insert(t, s);
        }
    }

    /// A call met with all its values known: kept when it says more than
    /// the plain reading of its `ACTION`.
    fn emit(&mut self, r: Rec) {
        if self.out.len() >= MAX_RESOLVED {
            return;
        }
        let action = *r.path.last().expect("a path ends at its ACTION");
        let args: Vec<Option<Value>> = r.args.iter().map(value).collect();
        let plain: Vec<Option<Literal>> = args
            .iter()
            .map(|a| match a {
                Some(Value::Literal(l)) => Some(l.clone()),
                _ => None,
            })
            .collect();
        let more = args.iter().any(|a| matches!(a, Some(Value::Call { .. })))
            || self.plain.get(&action).is_none_or(|p| *p != plain);
        if !more {
            return;
        }
        let key = (action, format!("{args:?}"));
        if !self.seen.insert(key) {
            return;
        }
        self.out.push(Resolved {
            path: r.path,
            routine: r.routine,
            args,
        });
    }

    /// Run instruction `i` over `st`. Calls met go to `found` when given;
    /// `None` where the values cannot be followed past it.
    fn step(
        &self,
        i: usize,
        st: &mut State,
        mut found: Option<&mut Vec<Rec>>,
        callees: &HashMap<usize, Summary>,
    ) -> Option<()> {
        use Opcode::*;
        let x = &self.program.instructions()[i];
        let at = self.program.offsets()[i];
        let d = st.depth();
        match (x.opcode, &x.operands) {
            (Const, o) => st.stack.push(match o {
                Operands::Int(v) => V::Lit(Literal::Int(*v)),
                Operands::Float(v) => V::Lit(Literal::Float(*v)),
                Operands::String(s) => V::Lit(Literal::String(s.clone())),
                Operands::Object(0) => self.me(i),
                Operands::Object(v) => V::Lit(Literal::Object(*v)),
                _ => V::Unknown,
            }),
            (RsAdd, _) => st.stack.push(V::Unknown),
            (CpTopSp, Operands::Stack { offset, size }) => {
                for k in (0..i32::from(*size)).step_by(4) {
                    let v = st.get(d + offset + k);
                    st.stack.push(v);
                }
            }
            (CpDownSp, Operands::Stack { offset, size }) => {
                let n = i32::from(*size);
                for k in (0..n).step_by(4) {
                    let v = st.get(d - n + k);
                    st.set(d + offset + k, v);
                }
            }
            (CpTopBp, Operands::Stack { offset, size }) => {
                for k in (0..i32::from(*size)).step_by(4) {
                    let v = self
                        .globals
                        .get(&(offset + k))
                        .cloned()
                        .unwrap_or(V::Unknown);
                    st.stack.push(v);
                }
            }
            (CpDownBp | Nop | Jmp | Retn | StoreStateAll | IncIBp | DecIBp, _) => {}
            (MovSp, Operands::Offset(o)) => {
                if *o <= 0 {
                    st.pop(-o)?;
                } else {
                    for _ in 0..o / 4 {
                        st.stack.push(V::Unknown);
                    }
                }
            }
            (Jz | Jnz | RestoreBp, _) => st.pop(4)?,
            (SaveBp, _) => st.stack.push(V::Unknown),
            (IncISp | DecISp, Operands::Offset(o)) => st.set(d + o, V::Unknown),
            (Neg, _) => {
                let v = st.stack.pop()?;
                st.stack.push(match v {
                    V::Lit(Literal::Int(n)) => V::Lit(Literal::Int(n.wrapping_neg())),
                    V::Lit(Literal::Float(f)) => V::Lit(Literal::Float(-f)),
                    _ => V::Unknown,
                });
            }
            (Comp | Not, _) => {
                st.pop(4)?;
                st.stack.push(V::Unknown);
            }
            (
                Destruct,
                Operands::Destruct {
                    size,
                    keep_offset,
                    keep_size,
                },
            ) => {
                let n = usize::from(*size) / 4;
                let base = st.stack.len().checked_sub(n)?;
                let ks = base + usize::from(*keep_offset) / 4;
                let ke = ks + usize::from(*keep_size) / 4;
                let kept: Vec<V> = st.stack.get(ks..ke)?.to_vec();
                st.stack.truncate(base);
                st.stack.extend(kept);
            }
            (Action, Operands::Action { routine, argc }) => {
                let routine = *routine;
                let mut args = Vec::with_capacity(usize::from(*argc));
                let mut hi = d;
                let mut action_param = None;
                for k in 0..usize::from(*argc) {
                    let ty = (self.param)(routine, k);
                    let lo = hi - self.size(routine, k);
                    args.push(match ty {
                        Some(ArgType::Action) => {
                            action_param.get_or_insert(k);
                            V::Unknown
                        }
                        Some(ArgType::Vector) => {
                            let c: Vec<V> = (0..3).map(|c| st.get(lo + 4 * c)).collect();
                            match &c[..] {
                                [
                                    V::Lit(Literal::Float(x)),
                                    V::Lit(Literal::Float(y)),
                                    V::Lit(Literal::Float(z)),
                                ] => V::Lit(Literal::Vector([*x, *y, *z])),
                                _ => V::Unknown,
                            }
                        }
                        _ if lo < hi => st.get(lo),
                        _ => V::Unknown,
                    });
                    hi = lo;
                }
                if let Some(k) = action_param
                    && let Some(p) = st.pending.pop()
                {
                    // The deferred action runs as the object argument
                    // before it, when the routine takes one.
                    let me = (0..k)
                        .find(|&j| (self.param)(routine, j) == Some(ArgType::Object))
                        .map_or_else(|| self.me(i), |j| args[j].clone());
                    if let (Some(f), Some(s)) = (found.as_deref_mut(), callees.get(&p.sub)) {
                        let saved = &p.saved;
                        let env = |o: i32| saved.get(saved.depth() + o);
                        for r in s.open.iter() {
                            let (a, _) = subst_all(&r.args, &env, &me);
                            let mut path = vec![p.at];
                            path.extend(&r.path);
                            f.push(Rec {
                                path,
                                routine: r.routine,
                                args: a,
                            });
                        }
                    }
                }
                st.pop(d - hi)?;
                let r = (self.ret)(routine)?;
                match r.size() {
                    0 => {}
                    4 => {
                        let v = V::Ret {
                            routine,
                            args: Rc::new(args.clone()),
                        };
                        st.stack.push(if v.nesting() > MAX_NESTING {
                            V::Unknown
                        } else {
                            v
                        });
                    }
                    n => {
                        for _ in 0..n / 4 {
                            st.stack.push(V::Unknown);
                        }
                    }
                }
                if let Some(f) = found {
                    f.push(Rec {
                        path: vec![at],
                        routine,
                        args,
                    });
                }
            }
            (Jsr, _) => {
                let t = self.target(i)?;
                let net = self.depths.net[t]?;
                if net > 0 {
                    return None;
                }
                let s = callees.get(&t)?;
                let before = st.clone();
                let env = |o: i32| before.get(d + o);
                if let Some(f) = found {
                    for r in s.open.iter() {
                        let (a, _) = subst_all(&r.args, &env, &V::SelfObj);
                        let mut path = vec![at];
                        path.extend(&r.path);
                        f.push(Rec {
                            path,
                            routine: r.routine,
                            args: a,
                        });
                    }
                }
                for (o, v) in s.writes.iter() {
                    let v = subst(v, &env, &V::SelfObj);
                    st.set(d + o, v);
                }
                st.pop(-net)?;
            }
            (StoreState, _) => {
                let sub = self.target(i)?;
                let mut saved = st.clone();
                saved.pending.clear();
                st.pending.push(Rc::new(Pending { sub, at, saved }));
            }
            (op, o) => {
                let pops = binary_pops(op, x.ty, o)?;
                let e = cfg::effect(op, x.ty, o)?;
                st.pop(pops)?;
                for _ in 0..(pops + e) / 4 {
                    st.stack.push(V::Unknown);
                }
            }
        }
        Some(())
    }
}

/// An abstract value as an argument's value.
fn value(v: &V) -> Option<Value> {
    match v {
        V::Lit(l) => Some(Value::Literal(l.clone())),
        V::Ret { routine, args } => Some(Value::Call {
            routine: *routine,
            args: args.iter().map(value).collect(),
        }),
        _ => None,
    }
}

/// The bytes an arithmetic, comparison or logical instruction takes off
/// the stack.
fn binary_pops(op: Opcode, ty: Type, o: &Operands) -> Option<i32> {
    use Opcode::*;
    Some(match (op, o) {
        (Equal | NEqual, Operands::Size(s)) => 2 * i32::from(*s),
        (Add | Sub | Mul | Div, _) => match ty {
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

#[cfg(test)]
mod tests {
    use super::*;
    use nwscript_diag::FileId;

    fn program(asm: &str) -> Program {
        crate::asm::parse(FileId(1), asm.as_bytes()).unwrap()
    }

    /// Routine 0 `SetLocalBoolean(object, int, int)`, 1
    /// `GetObjectByTag(string, int)` returning `object`, 2
    /// `AssignCommand(object, action)`, 3 `DelayCommand(float, action)`.
    fn param(r: u16, k: usize) -> Option<ArgType> {
        use ArgType::*;
        match (r, k) {
            (0, 0) | (1, 1) | (2, 0) => Some(if (r, k) == (1, 1) { Int } else { Object }),
            (0, 1) | (0, 2) => Some(Int),
            (1, 0) => Some(String),
            (2, 1) | (3, 1) => Some(Action),
            (3, 0) => Some(Float),
            _ => None,
        }
    }

    fn ret(r: u16) -> Option<ArgType> {
        match r {
            0 | 2 | 3 => Some(ArgType::Void),
            1 => Some(ArgType::Object),
            _ => None,
        }
    }

    fn lit(l: Literal) -> Option<Value> {
        Some(Value::Literal(l))
    }

    const SELF: Literal = Literal::Object(0);

    /// `int G = 0;` and `void W(object o, int n, int v) {
    /// SetLocalBoolean(o, n, v); }`, the wrapper's shape in
    /// `k_inc_utility`, with `main` below.
    fn with_wrapper(main: &str) -> Program {
        program(&format!(
            "JSR globals\nRETN\nglobals:\nRSADDI\nCONSTI 0\nCPDOWNSP -8, 4\nMOVSP -4\nSAVEBP\nJSR main\nRESTOREBP\nMOVSP -4\nRETN\nmain:\n{main}RETN\nwrap:\nCPTOPSP -12, 4\nCPTOPSP -12, 4\nCPTOPSP -12, 4\nACTION 0, 3\nMOVSP -12\nRETN\n"
        ))
    }

    #[test]
    fn a_wrapper_gets_its_callers_literals_and_globals() {
        // W(OBJECT_SELF, G, TRUE);
        let p = with_wrapper("CONSTI 1\nCPTOPBP -4, 4\nCONSTO 0\nJSR wrap\n");
        let r = resolved_calls(&p, param, ret);
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].routine, 0);
        assert_eq!(r[0].path.len(), 2);
        assert_eq!(
            r[0].args,
            [lit(SELF), lit(Literal::Int(0)), lit(Literal::Int(1))]
        );
    }

    #[test]
    fn a_local_holding_a_call_and_a_written_global_are_followed() {
        // object o = GetObjectByTag("statue", 0); G = 5; W(o, G, FALSE);
        let p = with_wrapper(
            "RSADDO\nCONSTI 0\nCONSTS \"statue\"\nACTION 1, 2\nCPDOWNSP -8, 4\nMOVSP -4\nCONSTI 5\nCPDOWNBP -4, 4\nMOVSP -4\nCONSTI 0\nCPTOPBP -4, 4\nCPTOPSP -12, 4\nJSR wrap\nMOVSP -4\n",
        );
        let r = resolved_calls(&p, param, ret);
        let set: Vec<&Resolved> = r.iter().filter(|c| c.routine == 0).collect();
        assert_eq!(set.len(), 1, "{r:?}");
        assert_eq!(
            set[0].args,
            [
                Some(Value::Call {
                    routine: 1,
                    args: vec![
                        lit(Literal::String(b"statue".to_vec())),
                        lit(Literal::Int(0))
                    ],
                }),
                None,
                lit(Literal::Int(0)),
            ]
        );
    }

    #[test]
    fn a_call_resolved_by_its_own_literals_is_left_to_calls() {
        let p = with_wrapper("CONSTI 1\nCONSTI 3\nCONSTO 0\nACTION 0, 3\n");
        assert!(resolved_calls(&p, param, ret).is_empty());
    }

    #[test]
    fn a_deferred_action_runs_as_the_object_it_is_assigned_to() {
        // AssignCommand(GetObjectByTag("door", 0), SetLocalBoolean(OBJECT_SELF, 3, 1));
        // DelayCommand(1.0, SetLocalBoolean(OBJECT_SELF, 4, 1));
        let p = program(
            "JSR main\nRETN\nmain:\nSTORE_STATE 0, 0\nJMP a\nCONSTI 1\nCONSTI 3\nCONSTO 0\nACTION 0, 3\nRETN\na:\nCONSTI 0\nCONSTS \"door\"\nACTION 1, 2\nACTION 2, 2\nSTORE_STATE 0, 0\nJMP b\nCONSTI 1\nCONSTI 4\nCONSTO 0\nACTION 0, 3\nRETN\nb:\nCONSTF 1.0\nACTION 3, 2\nRETN\n",
        );
        let r = resolved_calls(&p, param, ret);
        let set: Vec<&Resolved> = r.iter().filter(|c| c.routine == 0).collect();
        // The delayed call runs as the caller: its OBJECT_SELF literal is
        // what `calls` reads already.
        assert_eq!(set.len(), 1, "{r:?}");
        assert_eq!(
            set[0].args[0],
            Some(Value::Call {
                routine: 1,
                args: vec![lit(Literal::String(b"door".to_vec())), lit(Literal::Int(0))],
            })
        );
        assert_eq!(set[0].args[1], lit(Literal::Int(3)));
        assert_eq!(set[0].path.len(), 2);
    }
}
