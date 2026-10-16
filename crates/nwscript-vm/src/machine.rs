//! Execution. The stack is a vector of four-byte elements, so byte offsets
//! in instructions are divided by four; a vector is three elements. After
//! `SAVEBP`, BP indexes the element holding the saved BP, so the globals
//! sit just below it.

use crate::value::{Closure, Ty, Value};
use crate::{Call, Engine, Fault, FaultKind, Routine, Run};
use nwscript_ncs::{Opcode, Operands, Program, Type};
use std::collections::{BTreeSet, VecDeque};
use std::rc::Rc;

/// How far a run may go.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Instructions per run, deferred actions included.
    pub steps: u64,
    /// Deferred actions run after the script's body.
    pub deferred: usize,
    /// Keep the last this many executed instructions in [`Run::history`], for debugging.
    pub history: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            steps: 5_000_000,
            deferred: 64,
            history: 0,
        }
    }
}

struct Machine<'a> {
    program: &'a Program,
    routines: &'a [Routine],
    stack: Vec<Value>,
    bp: usize,
    calls: Vec<usize>,
    pending: Option<Rc<Closure>>,
    deferred: VecDeque<Rc<Closure>>,
    next_closure: usize,
    trace: Vec<Call>,
    unmodelled: BTreeSet<String>,
    steps: u64,
    limits: Limits,
    current: Option<usize>,
    history: VecDeque<(u32, String)>,
}

type Step<T> = Result<T, FaultKind>;

fn mismatch(expected: &'static str, v: &Value) -> FaultKind {
    FaultKind::TypeMismatch {
        expected,
        found: v.kind(),
    }
}

impl Machine<'_> {
    fn cells(off: i32) -> Step<isize> {
        if off % 4 != 0 {
            return Err(FaultKind::BadStackOffset(off));
        }
        Ok((off / 4) as isize)
    }

    /// The index of the element `off` bytes from `base`.
    fn at(&self, base: usize, off: i32) -> Step<usize> {
        let i = base as isize + Self::cells(off)?;
        if i < 0 || i as usize >= self.stack.len() {
            return Err(FaultKind::BadStackOffset(off));
        }
        Ok(i as usize)
    }

    fn pop(&mut self) -> Step<Value> {
        self.stack.pop().ok_or(FaultKind::StackUnderflow)
    }

    fn pop_int(&mut self) -> Step<i32> {
        match self.pop()? {
            Value::Int(i) => Ok(i),
            v => Err(mismatch("int", &v)),
        }
    }

    fn pop_float(&mut self) -> Step<f32> {
        match self.pop()? {
            Value::Float(f) => Ok(f),
            v => Err(mismatch("float", &v)),
        }
    }

    fn pop_vector(&mut self) -> Step<[f32; 3]> {
        let z = self.pop_float()?;
        let y = self.pop_float()?;
        let x = self.pop_float()?;
        Ok([x, y, z])
    }

    fn push_value(&mut self, v: Value) {
        match v {
            Value::Void | Value::Action(_) => {}
            Value::Vector([x, y, z]) => {
                self.stack
                    .extend([Value::Float(x), Value::Float(y), Value::Float(z)]);
            }
            v => self.stack.push(v),
        }
    }

    fn pop_typed(&mut self, t: Ty) -> Step<Value> {
        Ok(match t {
            Ty::Vector => Value::Vector(self.pop_vector()?),
            Ty::Int => Value::Int(self.pop_int()?),
            Ty::Float => Value::Float(self.pop_float()?),
            Ty::String => match self.pop()? {
                v @ Value::String(_) => v,
                v => return Err(mismatch("string", &v)),
            },
            Ty::Object => match self.pop()? {
                v @ Value::Object(_) => v,
                v => return Err(mismatch("object", &v)),
            },
            Ty::Engine(_) => match self.pop()? {
                v @ Value::Engine(..) => v,
                v => return Err(mismatch("engine structure", &v)),
            },
            Ty::Void | Ty::Action => Value::Void,
        })
    }

    fn binary(&mut self, op: Opcode, ty: Type) -> Step<()> {
        use Opcode::*;
        let r = match ty {
            Type::INT_INT => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                Value::Int(match op {
                    LogAnd => i32::from(a != 0 && b != 0),
                    LogOr => i32::from(a != 0 || b != 0),
                    IncOr => a | b,
                    ExcOr => a ^ b,
                    BoolAnd => a & b,
                    Equal => i32::from(a == b),
                    NEqual => i32::from(a != b),
                    Geq => i32::from(a >= b),
                    Gt => i32::from(a > b),
                    Lt => i32::from(a < b),
                    Leq => i32::from(a <= b),
                    ShLeft => a.wrapping_shl(b as u32),
                    ShRight => a.wrapping_shr(b as u32),
                    UShRight => ((a as u32).wrapping_shr(b as u32)) as i32,
                    Add => a.wrapping_add(b),
                    Sub => a.wrapping_sub(b),
                    Mul => a.wrapping_mul(b),
                    Div | Mod if b == 0 => return Err(FaultKind::DivisionByZero),
                    Div => a.wrapping_div(b),
                    Mod => a.wrapping_rem(b),
                    _ => return Err(FaultKind::Unsupported("this integer operation")),
                })
            }
            Type::FLOAT_FLOAT | Type::INT_FLOAT | Type::FLOAT_INT => {
                let b = match ty {
                    Type::FLOAT_INT => self.pop_int()? as f32,
                    _ => self.pop_float()?,
                };
                let a = match ty {
                    Type::INT_FLOAT => self.pop_int()? as f32,
                    _ => self.pop_float()?,
                };
                match op {
                    Equal => Value::Int(i32::from(a == b)),
                    NEqual => Value::Int(i32::from(a != b)),
                    Geq => Value::Int(i32::from(a >= b)),
                    Gt => Value::Int(i32::from(a > b)),
                    Lt => Value::Int(i32::from(a < b)),
                    Leq => Value::Int(i32::from(a <= b)),
                    Add => Value::Float(a + b),
                    Sub => Value::Float(a - b),
                    Mul => Value::Float(a * b),
                    Div if b == 0.0 => return Err(FaultKind::DivisionByZero),
                    Div => Value::Float(a / b),
                    _ => return Err(FaultKind::Unsupported("this float operation")),
                }
            }
            Type::STRING_STRING => {
                let b = self.pop_typed(Ty::String)?;
                let a = self.pop_typed(Ty::String)?;
                let (Value::String(a), Value::String(b)) = (a, b) else {
                    unreachable!()
                };
                match op {
                    Add => Value::String(Rc::from([&a[..], &b[..]].concat())),
                    Equal => Value::Int(i32::from(a == b)),
                    NEqual => Value::Int(i32::from(a != b)),
                    _ => return Err(FaultKind::Unsupported("this string operation")),
                }
            }
            Type::OBJECT_OBJECT => {
                let b = self.pop_typed(Ty::Object)?;
                let a = self.pop_typed(Ty::Object)?;
                match op {
                    Equal => Value::Int(i32::from(a == b)),
                    NEqual => Value::Int(i32::from(a != b)),
                    _ => return Err(FaultKind::Unsupported("this object operation")),
                }
            }
            Type::VECTOR_VECTOR => {
                let b = self.pop_vector()?;
                let a = self.pop_vector()?;
                match op {
                    Add => Value::Vector([a[0] + b[0], a[1] + b[1], a[2] + b[2]]),
                    Sub => Value::Vector([a[0] - b[0], a[1] - b[1], a[2] - b[2]]),
                    Equal => Value::Int(i32::from(a == b)),
                    NEqual => Value::Int(i32::from(a != b)),
                    _ => return Err(FaultKind::Unsupported("this vector operation")),
                }
            }
            Type::VECTOR_FLOAT => {
                let f = self.pop_float()?;
                let v = self.pop_vector()?;
                match op {
                    Mul => Value::Vector(v.map(|c| c * f)),
                    Div if f == 0.0 => return Err(FaultKind::DivisionByZero),
                    Div => Value::Vector(v.map(|c| c / f)),
                    _ => return Err(FaultKind::Unsupported("this vector operation")),
                }
            }
            Type::FLOAT_VECTOR => {
                let v = self.pop_vector()?;
                let f = self.pop_float()?;
                match op {
                    Mul => Value::Vector(v.map(|c| c * f)),
                    _ => return Err(FaultKind::Unsupported("this vector operation")),
                }
            }
            t if t.engine_pair().is_some() => {
                let b = self.pop()?;
                let a = self.pop()?;
                match op {
                    Equal => Value::Int(i32::from(a == b)),
                    NEqual => Value::Int(i32::from(a != b)),
                    _ => return Err(FaultKind::Unsupported("this engine-structure operation")),
                }
            }
            _ => return Err(FaultKind::Unsupported("this operand type")),
        };
        self.push_value(r);
        Ok(())
    }

    fn action(&mut self, routine: u16, argc: u8) -> Step<()> {
        let sig = self
            .routines
            .get(usize::from(routine))
            .ok_or(FaultKind::UnknownRoutine(routine))?;
        let mut args = Vec::with_capacity(usize::from(argc));
        for t in sig.params.iter().take(usize::from(argc)) {
            if *t == Ty::Action {
                let c = self.pending.take().ok_or(FaultKind::NoStoredState)?;
                self.deferred.push_back(c.clone());
                args.push(Value::Action(c));
            } else {
                let v = self.pop_typed(*t).map_err(|e| match e {
                    FaultKind::TypeMismatch { expected, found } => FaultKind::ArgumentMismatch {
                        routine,
                        expected,
                        found,
                    },
                    e => e,
                })?;
                args.push(v);
            }
        }
        self.record(routine, sig, args);
        Ok(())
    }

    fn record(&mut self, routine: u16, sig: &Routine, args: Vec<Value>) {
        // `exec` fills in the reply: the engine is not reachable from here.
        self.trace.push(Call {
            routine,
            name: sig.name.clone(),
            args,
            ret: Value::Void,
            modelled: true,
            deferred: self.current,
        });
    }

    fn exec(&mut self, engine: &mut dyn Engine, start: usize) -> Result<(), Fault> {
        let ins = self.program.instructions();
        let offs = self.program.offsets();
        let mut pc = start;
        loop {
            let Some(i) = ins.get(pc) else {
                return Err(Fault {
                    offset: self.program.file_len(),
                    kind: FaultKind::BadJump(self.program.file_len()),
                });
            };
            let at = offs[pc];
            self.steps += 1;
            if self.steps > self.limits.steps {
                return Err(Fault {
                    offset: at,
                    kind: FaultKind::Budget,
                });
            }
            let fault = |kind| Fault { offset: at, kind };
            let jump = |m: &Self, rel: i32| -> Result<usize, Fault> {
                let t = at.wrapping_add_signed(rel);
                m.program.index_at(t).ok_or(Fault {
                    offset: at,
                    kind: FaultKind::BadJump(t),
                })
            };
            if self.limits.history > 0 {
                if self.history.len() == self.limits.history {
                    self.history.pop_front();
                }
                let top: Vec<String> = self
                    .stack
                    .iter()
                    .rev()
                    .take(3)
                    .map(|v| v.to_string())
                    .collect();
                self.history.push_back((
                    at,
                    format!(
                        "{i}    [sp={} bp={} top: {}]",
                        self.stack.len() * 4,
                        self.bp * 4,
                        top.join(", ")
                    ),
                ));
            }
            let mut next = pc + 1;
            match (i.opcode, &i.operands) {
                (Opcode::CpDownSp, Operands::Stack { offset, size })
                | (Opcode::CpDownBp, Operands::Stack { offset, size }) => {
                    let n = usize::from(*size) / 4;
                    let base = if i.opcode == Opcode::CpDownSp {
                        self.stack.len()
                    } else {
                        self.bp
                    };
                    let dst = self.at(base, *offset).map_err(fault)?;
                    let src = self
                        .stack
                        .len()
                        .checked_sub(n)
                        .ok_or(fault(FaultKind::StackUnderflow))?;
                    if dst + n > self.stack.len() {
                        return Err(fault(FaultKind::BadStackOffset(*offset)));
                    }
                    for k in 0..n {
                        self.stack[dst + k] = self.stack[src + k].clone();
                    }
                }
                (Opcode::CpTopSp, Operands::Stack { offset, size })
                | (Opcode::CpTopBp, Operands::Stack { offset, size }) => {
                    let n = usize::from(*size) / 4;
                    let base = if i.opcode == Opcode::CpTopSp {
                        self.stack.len()
                    } else {
                        self.bp
                    };
                    let s = self.at(base, *offset).map_err(fault)?;
                    if s + n > self.stack.len() {
                        return Err(fault(FaultKind::BadStackOffset(*offset)));
                    }
                    let copy: Vec<Value> = self.stack[s..s + n].to_vec();
                    self.stack.extend(copy);
                }
                (Opcode::RsAdd, _) => {
                    let v = match i.ty {
                        Type::INT => Value::Int(0),
                        Type::FLOAT => Value::Float(0.0),
                        Type::STRING => Value::string(""),
                        Type::OBJECT => Value::Object(crate::value::OBJECT_INVALID),
                        t => Value::Engine(t.engine().unwrap_or(0), 0),
                    };
                    self.stack.push(v);
                }
                (Opcode::Const, o) => self.stack.push(match o {
                    Operands::Int(v) => Value::Int(*v),
                    Operands::Float(v) => Value::Float(*v),
                    Operands::String(s) => Value::String(Rc::from(&s[..])),
                    Operands::Object(o) => Value::Object(*o),
                    _ => return Err(fault(FaultKind::Unsupported("this constant"))),
                }),
                (Opcode::Action, Operands::Action { routine, argc }) => {
                    self.action(*routine, *argc).map_err(fault)?;
                    let call = self.trace.last_mut().expect("action recorded a call");
                    let sig = &self.routines[usize::from(*routine)];
                    let reply = engine.call(*routine, sig, &call.args);
                    if !reply.modelled {
                        self.unmodelled.insert(sig.name.clone());
                    }
                    call.ret = reply.value.clone();
                    call.modelled = reply.modelled;
                    self.push_value(reply.value);
                }
                (Opcode::Equal | Opcode::NEqual, Operands::Size(size)) => {
                    let n = usize::from(*size) / 4;
                    if self.stack.len() < 2 * n {
                        return Err(fault(FaultKind::StackUnderflow));
                    }
                    let b = self.stack.split_off(self.stack.len() - n);
                    let a = self.stack.split_off(self.stack.len() - n);
                    let eq = a == b;
                    self.stack
                        .push(Value::Int(i32::from(if i.opcode == Opcode::Equal {
                            eq
                        } else {
                            !eq
                        })));
                }
                (
                    Opcode::LogAnd
                    | Opcode::LogOr
                    | Opcode::IncOr
                    | Opcode::ExcOr
                    | Opcode::BoolAnd
                    | Opcode::Equal
                    | Opcode::NEqual
                    | Opcode::Geq
                    | Opcode::Gt
                    | Opcode::Lt
                    | Opcode::Leq
                    | Opcode::ShLeft
                    | Opcode::ShRight
                    | Opcode::UShRight
                    | Opcode::Add
                    | Opcode::Sub
                    | Opcode::Mul
                    | Opcode::Div
                    | Opcode::Mod,
                    _,
                ) => self.binary(i.opcode, i.ty).map_err(fault)?,
                (Opcode::Neg, _) => {
                    let v = match self.pop().map_err(fault)? {
                        Value::Int(a) => Value::Int(a.wrapping_neg()),
                        Value::Float(a) => Value::Float(-a),
                        v => return Err(fault(mismatch("int or float", &v))),
                    };
                    self.stack.push(v);
                }
                (Opcode::Comp, _) => {
                    let a = self.pop_int().map_err(fault)?;
                    self.stack.push(Value::Int(!a));
                }
                (Opcode::Not, _) => {
                    let a = self.pop_int().map_err(fault)?;
                    self.stack.push(Value::Int(i32::from(a == 0)));
                }
                (Opcode::MovSp, Operands::Offset(o)) => {
                    let n = Self::cells(*o).map_err(fault)?;
                    let len = self.stack.len() as isize + n;
                    if n > 0 || len < 0 {
                        return Err(fault(FaultKind::BadStackOffset(*o)));
                    }
                    self.stack.truncate(len as usize);
                }
                (Opcode::Jmp, Operands::Jump(r)) => next = jump(self, *r)?,
                (Opcode::Jsr, Operands::Jump(r)) => {
                    self.calls.push(pc + 1);
                    next = jump(self, *r)?;
                }
                (Opcode::Jz | Opcode::Jnz, Operands::Jump(r)) => {
                    let v = self.pop_int().map_err(fault)?;
                    if (v == 0) == (i.opcode == Opcode::Jz) {
                        next = jump(self, *r)?;
                    }
                }
                (Opcode::Retn, _) => match self.calls.pop() {
                    Some(r) => next = r,
                    None => return Ok(()),
                },
                (
                    Opcode::Destruct,
                    Operands::Destruct {
                        size,
                        keep_offset,
                        keep_size,
                    },
                ) => {
                    let n = usize::from(*size) / 4;
                    let region = self
                        .stack
                        .len()
                        .checked_sub(n)
                        .ok_or(fault(FaultKind::StackUnderflow))?;
                    let ks = region + usize::from(*keep_offset) / 4;
                    let ke = ks + usize::from(*keep_size) / 4;
                    if ke > self.stack.len() {
                        return Err(fault(FaultKind::BadStackOffset(i32::from(*keep_offset))));
                    }
                    let kept: Vec<Value> = self.stack[ks..ke].to_vec();
                    self.stack.truncate(region);
                    self.stack.extend(kept);
                }
                (
                    Opcode::DecISp | Opcode::IncISp | Opcode::DecIBp | Opcode::IncIBp,
                    Operands::Offset(o),
                ) => {
                    let base = if matches!(i.opcode, Opcode::DecISp | Opcode::IncISp) {
                        self.stack.len()
                    } else {
                        self.bp
                    };
                    let k = self.at(base, *o).map_err(fault)?;
                    let d = if matches!(i.opcode, Opcode::DecISp | Opcode::DecIBp) {
                        -1
                    } else {
                        1
                    };
                    match &mut self.stack[k] {
                        Value::Int(v) => *v = v.wrapping_add(d),
                        v => return Err(fault(mismatch("int", v))),
                    }
                }
                (Opcode::SaveBp, _) => {
                    // BP is set to the slot the old BP is saved in, so the
                    // last global is at BP-4.
                    let old = self.bp;
                    self.bp = self.stack.len();
                    self.stack.push(Value::Int(old as i32));
                }
                (Opcode::RestoreBp, _) => {
                    let v = self.pop_int().map_err(fault)?;
                    self.bp =
                        usize::try_from(v).map_err(|_| fault(FaultKind::BadStackOffset(v)))?;
                }
                (Opcode::StoreState, Operands::StoreState { globals, locals }) => {
                    let g = *globals as usize / 4;
                    let l = *locals as usize / 4;
                    if g > self.bp || l > self.stack.len() {
                        return Err(fault(FaultKind::StackUnderflow));
                    }
                    let id = self.next_closure;
                    self.next_closure += 1;
                    self.pending = Some(Rc::new(Closure {
                        id,
                        resume: at + u32::from(i.ty.0),
                        globals: self.stack[self.bp - g..self.bp].to_vec(),
                        locals: self.stack[self.stack.len() - l..].to_vec(),
                    }));
                }
                (Opcode::StoreStateAll, _) => {
                    return Err(fault(FaultKind::Unsupported("STORE_STATEALL")));
                }
                (Opcode::Nop, _) => {}
                _ => return Err(fault(FaultKind::Unsupported("this instruction form"))),
            }
            pc = next;
        }
    }
}

/// Run a program from its first instruction against `engine`, then run the
/// deferred actions it created, in order, up to the limit. `routines` is
/// the engine routine table, indexed by `ACTION` number.
pub fn run(
    program: &Program,
    routines: &[Routine],
    engine: &mut dyn Engine,
    limits: Limits,
) -> Run {
    let mut m = Machine {
        program,
        routines,
        stack: Vec::new(),
        bp: 0,
        calls: Vec::new(),
        pending: None,
        deferred: VecDeque::new(),
        next_closure: 0,
        trace: Vec::new(),
        unmodelled: BTreeSet::new(),
        steps: 0,
        limits,
        current: None,
        history: VecDeque::new(),
    };
    let returns_int = program
        .instructions()
        .first()
        .is_some_and(|i| i.opcode == Opcode::RsAdd && i.ty == Type::INT);
    let mut fault = m.exec(engine, 0).err();
    let result = if returns_int && fault.is_none() {
        m.stack.first().cloned()
    } else {
        None
    };
    let mut ran = 0;
    while fault.is_none() && ran < limits.deferred {
        let Some(c) = m.deferred.pop_front() else {
            break;
        };
        ran += 1;
        m.stack = c
            .globals
            .iter()
            .cloned()
            .chain(c.locals.iter().cloned())
            .collect();
        m.bp = c.globals.len();
        m.calls.clear();
        m.current = Some(c.id);
        match program.index_at(c.resume) {
            Some(start) => fault = m.exec(engine, start).err(),
            None => {
                fault = Some(Fault {
                    offset: c.resume,
                    kind: FaultKind::BadJump(c.resume),
                })
            }
        }
    }
    Run {
        result,
        trace: m.trace,
        unmodelled: m.unmodelled.into_iter().collect(),
        fault,
        steps: m.steps,
        deferred_skipped: m.deferred.len(),
        history: m
            .history
            .into_iter()
            .map(|(o, s)| format!("{o:>6}: {s}"))
            .collect(),
    }
}
