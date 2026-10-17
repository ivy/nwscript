//! Bytecode from a checked program, per game profile.
//!
//! [`generate`] is a pure function of a [`hir::Program`] and a [`Profile`]:
//! nothing else reaches it, no lint level and no compatibility mode, so the
//! bytes a program compiles to never depend on how strictly it was checked.
//!
//! ```compile_fail
//! // There is no third argument to pass a lint level or a mode through.
//! let _ = nwscript_codegen::generate(todo!(), nwscript_codegen::Profile::K1, ());
//! ```
//!
//! The encoding is BioWare's, as the vanilla corpus shows it. The program
//! starts with a stub that calls the globals routine when the script has
//! globals, or the entry point directly; `int StartingConditional()` gets a
//! return slot reserved first. Each function's code follows, the entry point
//! first, then each function it calls, depth first in the order the calls
//! are emitted. A function's caller reserves the return slot and pushes the
//! arguments last first; the callee removes its parameters before `RETN`.
//! Variables are addressed from SP, globals from BP (from SP inside the
//! globals routine, before BP is set). The layouts of statements and
//! expressions, including code no path reaches, are documented where they
//! are emitted.

mod emit;

use emit::{Emitter, Label};
use nwscript_diag::{Diagnostic, codes};
use nwscript_ncs::{Opcode, Operands, Type as T};
use nwscript_sema::hir::*;
use nwscript_sema::{ConstValue, Type};
use std::collections::HashMap;

/// The game a program is compiled for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Profile {
    K1,
    K2,
}

/// Compile a checked program. It needs an entry point and no
/// [`ExprKind::Error`]: a program whose only errors are in
/// [`codes::BIOWARE_ACCEPTED`] qualifies.
pub fn generate(program: &Program, profile: Profile) -> Result<nwscript_ncs::Program, Diagnostic> {
    generate_with_layout(program, profile).map(|(p, _)| p)
}

/// [`generate`], and the file offset where each function's code starts, in
/// the order of the bytecode.
pub fn generate_with_layout(
    program: &Program,
    profile: Profile,
) -> Result<(nwscript_ncs::Program, Vec<(FuncId, u32)>), Diagnostic> {
    let _ = profile;
    let Some(entry) = program.entry else {
        return Err(Diagnostic::new(
            &codes::E0401,
            "a script without `void main()` or `int StartingConditional()` has nothing to compile",
        )
        .help("a file without an entry point is a library: include it from a script"));
    };
    let mut g = Gen::new(program);
    g.program(entry)?;
    let labels: Vec<(FuncId, Label)> = g.fn_labels.iter().map(|(f, l)| (*f, *l)).collect();
    let (code, at) = g.e.finish();
    let mut layout: Vec<(FuncId, u32)> = labels
        .into_iter()
        .map(|(f, l)| (f, at[l.0 as usize]))
        .filter(|(_, o)| *o != u32::MAX)
        .collect();
    layout.sort_by_key(|(_, o)| *o);
    Ok((code, layout))
}

/// Where a variable lives.
#[derive(Clone, Copy)]
enum Loc {
    /// At this position of the current frame, addressed from SP.
    Sp(i32),
    /// At this position among the globals, addressed from BP.
    Bp(i32),
}

struct Loop {
    brk: Label,
    cont: Option<Label>,
}

struct Gen<'p> {
    p: &'p Program,
    e: Emitter,
    /// Each global's position from the first.
    globals: Vec<i32>,
    globals_size: i32,
    fn_labels: HashMap<FuncId, Label>,
    emitted: Vec<bool>,
    // The frame being generated.
    sp: i32,
    entry_sp: i32,
    in_globals: bool,
    ret: Option<(i32, i32)>,
    params: Vec<i32>,
    locals: HashMap<LocalId, i32>,
    loops: Vec<Loop>,
    epilogue: Option<Label>,
    callees: Vec<FuncId>,
    current: Option<FuncId>,
}

fn unsupported(span: nwscript_diag::Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        &codes::E0401,
        format!("code generation cannot compile {what}"),
    )
    .primary(span, "")
}

impl<'p> Gen<'p> {
    fn new(p: &'p Program) -> Self {
        let mut globals = Vec::new();
        let mut at = 0;
        for g in &p.globals {
            globals.push(at);
            at += p.size_of(&g.ty) as i32;
        }
        Gen {
            p,
            e: Emitter::default(),
            globals,
            globals_size: at,
            fn_labels: HashMap::new(),
            emitted: vec![false; p.functions.len()],
            sp: 0,
            entry_sp: 0,
            in_globals: false,
            ret: None,
            params: Vec::new(),
            locals: HashMap::new(),
            loops: Vec::new(),
            epilogue: None,
            callees: Vec::new(),
            current: None,
        }
    }

    fn size(&self, t: &Type) -> i32 {
        self.p.size_of(t) as i32
    }

    fn fn_label(&mut self, id: FuncId) -> Label {
        if let Some(l) = self.fn_labels.get(&id) {
            return *l;
        }
        let l = self.e.label();
        self.fn_labels.insert(id, l);
        l
    }

    fn ins(&mut self, op: Opcode, ty: T, operands: Operands) {
        self.e.ins(op, ty, operands);
    }

    /// `MOVSP -n`, when `n` is not zero.
    fn pop(&mut self, n: i32) {
        if n != 0 {
            self.ins(Opcode::MovSp, T::VOID, Operands::Offset(-n));
            self.sp -= n;
        }
    }

    // ---- the program ---------------------------------------------------------

    fn program(&mut self, entry: Entry) -> Result<(), Diagnostic> {
        let sc = entry.kind == EntryKind::StartingConditional;
        let entry_label = self.fn_label(entry.function);
        if sc {
            self.ins(Opcode::RsAdd, T::INT, Operands::None);
        }
        let mut first = Vec::new();
        // A struct declaration, like a global, makes a globals routine, even
        // an empty one (nwnnsscomp's choice; the corpus declares a struct
        // only beside globals).
        if self.p.globals.is_empty() && self.p.structs.is_empty() {
            self.e.jump(Opcode::Jsr, entry_label);
            self.ins(Opcode::Retn, T::VOID, Operands::None);
        } else {
            let globals = self.e.label();
            self.e.jump(Opcode::Jsr, globals);
            self.ins(Opcode::Retn, T::VOID, Operands::None);
            self.e.place(globals);
            self.globals_routine(sc, entry_label)?;
            first = std::mem::take(&mut self.callees);
        }
        // Functions follow in the order the code calls them: the globals
        // routine's callees first, then the entry point's, depth first.
        first.push(entry.function);
        for id in first {
            self.function_once(id)?;
        }
        Ok(())
    }

    /// The globals routine: each global reserved and initialised in order,
    /// then BP saved and the entry point called. For `StartingConditional`
    /// a return slot is reserved before the call and its value copied to the
    /// stub's slot below the globals after.
    fn globals_routine(&mut self, sc: bool, entry: Label) -> Result<(), Diagnostic> {
        self.in_globals = true;
        self.sp = 0;
        self.entry_sp = 0;
        for (i, g) in self.p.globals.iter().enumerate() {
            let pos = self.sp;
            debug_assert_eq!(pos, self.globals[i]);
            self.rsadd(&g.ty);
            if let Some(init) = &g.init {
                self.expr(init)?;
                let n = self.size(&g.ty);
                self.cpdown(Loc::Sp(pos), 0, n);
                self.pop(n);
            }
        }
        self.ins(Opcode::SaveBp, T::VOID, Operands::None);
        self.sp += 4;
        if sc {
            self.ins(Opcode::RsAdd, T::INT, Operands::None);
            self.sp += 4;
        }
        self.e.jump(Opcode::Jsr, entry);
        if sc {
            // The stub's slot sits just below the globals.
            self.ins(
                Opcode::CpDownSp,
                T::STACK,
                Operands::Stack {
                    offset: -4 - self.sp,
                    size: 4,
                },
            );
            self.pop(4);
        }
        self.ins(Opcode::RestoreBp, T::VOID, Operands::None);
        self.sp -= 4;
        self.pop(self.globals_size);
        self.ins(Opcode::Retn, T::VOID, Operands::None);
        self.in_globals = false;
        Ok(())
    }

    fn function_once(&mut self, id: FuncId) -> Result<(), Diagnostic> {
        if !self.emitted[id.0 as usize] {
            self.emitted[id.0 as usize] = true;
            self.function(id)?;
        }
        Ok(())
    }

    /// A function: its return slot and parameters are below SP on entry,
    /// the slot deepest and the first parameter on top. The body's own
    /// cleanup ends it; a `return` jumps to the epilogue, which removes the
    /// parameters.
    fn function(&mut self, id: FuncId) -> Result<(), Diagnostic> {
        let f = self.p.function(id);
        let Some(body) = &f.body else {
            let at = f.prototypes.first().copied().unwrap_or_else(|| {
                f.definition
                    .unwrap_or(nwscript_diag::Span::new(f.file, 0, 0))
            });
            return Err(unsupported(
                at,
                &format!("a call to `{}`, which has no body", f.name),
            ));
        };
        let label = self.fn_label(id);
        self.e.place(label);
        let ret = self.size(&f.ret);
        self.ret = (ret > 0).then_some((0, ret));
        let sizes: Vec<i32> = f.params.iter().map(|p| self.size(&p.ty)).collect();
        let total: i32 = sizes.iter().sum();
        self.params = (0..sizes.len())
            .map(|i| ret + sizes[i + 1..].iter().sum::<i32>())
            .collect();
        self.sp = ret + total;
        self.entry_sp = self.sp;
        self.locals.clear();
        self.loops.clear();
        let epilogue = self.e.label();
        self.epilogue = Some(epilogue);
        self.callees.clear();
        self.current = Some(id);
        self.block(body)?;
        self.e.place(epilogue);
        self.pop(total);
        self.ins(Opcode::Retn, T::VOID, Operands::None);
        let callees = std::mem::take(&mut self.callees);
        for c in callees {
            self.function_once(c)?;
        }
        Ok(())
    }

    // ---- storage ----------------------------------------------------------------

    /// Reserve a value of type `t` on the stack: one `RSADD` per scalar.
    fn rsadd(&mut self, t: &Type) {
        match t {
            Type::Vector => {
                for _ in 0..3 {
                    self.ins(Opcode::RsAdd, T::FLOAT, Operands::None);
                }
                self.sp += 12;
            }
            Type::Struct(n) => {
                if let Some((_, s)) = self.p.struct_by_name(n) {
                    for f in &s.fields {
                        self.rsadd(&f.ty);
                    }
                }
            }
            Type::Void | Type::Action => {}
            t => {
                self.ins(Opcode::RsAdd, scalar(t), Operands::None);
                self.sp += 4;
            }
        }
    }

    fn var_loc(&self, v: Var) -> Loc {
        match v {
            Var::Local(l) => Loc::Sp(self.locals[&l]),
            Var::Param(i) => Loc::Sp(self.params[i as usize]),
            Var::Global(g) => {
                let pos = self.globals[g.0 as usize];
                if self.in_globals {
                    Loc::Sp(pos)
                } else {
                    Loc::Bp(pos)
                }
            }
        }
    }

    /// The variable a place expression names, and the byte offset in it.
    fn place(&self, e: &Expr) -> Result<(Loc, i32), Diagnostic> {
        match &e.kind {
            ExprKind::Var(v) => Ok((self.var_loc(*v), 0)),
            ExprKind::Member { base, member } => {
                let (loc, off) = self.place(base)?;
                Ok((loc, off + self.member_offset(*member)))
            }
            _ => Err(unsupported(
                e.span,
                "a store to something that is not a variable",
            )),
        }
    }

    fn member_offset(&self, m: Member) -> i32 {
        match m {
            Member::Field { ty, index } => {
                self.p.struct_def(ty).fields[index as usize].offset as i32
            }
            Member::Component(c) => 4 * i32::from(c),
        }
    }

    fn cptop(&mut self, loc: Loc, off: i32, size: i32) {
        let (op, offset) = match loc {
            Loc::Sp(pos) => (Opcode::CpTopSp, pos + off - self.sp),
            Loc::Bp(pos) => (Opcode::CpTopBp, pos + off - self.globals_size),
        };
        self.ins(
            op,
            T::STACK,
            Operands::Stack {
                offset,
                size: size as u16,
            },
        );
        self.sp += size;
    }

    fn cpdown(&mut self, loc: Loc, off: i32, size: i32) {
        let (op, offset) = match loc {
            Loc::Sp(pos) => (Opcode::CpDownSp, pos + off - self.sp),
            Loc::Bp(pos) => (Opcode::CpDownBp, pos + off - self.globals_size),
        };
        self.ins(
            op,
            T::STACK,
            Operands::Stack {
                offset,
                size: size as u16,
            },
        );
    }

    fn konst(&mut self, v: &ConstValue) {
        match v {
            ConstValue::Int(i) => self.ins(Opcode::Const, T::INT, Operands::Int(*i)),
            ConstValue::Float(f) => self.ins(Opcode::Const, T::FLOAT, Operands::Float(*f)),
            ConstValue::String(s) => {
                self.ins(Opcode::Const, T::STRING, Operands::String(s.clone()))
            }
            ConstValue::Object(o) => self.ins(Opcode::Const, T::OBJECT, Operands::Object(*o)),
            ConstValue::Vector(xyz) => {
                for c in xyz {
                    self.ins(Opcode::Const, T::FLOAT, Operands::Float(*c));
                }
                self.sp += 8;
            }
        }
        self.sp += 4;
    }

    // ---- statements -------------------------------------------------------------

    /// A block, then one `MOVSP` removing the locals it declared.
    fn block(&mut self, b: &Block) -> Result<(), Diagnostic> {
        let start = self.sp;
        for s in &b.stmts {
            self.stmt(s)?;
        }
        self.pop(self.sp - start);
        Ok(())
    }

    /// Each local in turn: reserved, then initialised by copying the value
    /// down into it and removing the value.
    fn declare(&mut self, ds: &[Declared]) -> Result<(), Diagnostic> {
        let f = self
            .p
            .function(self.current.expect("locals are in a function"));
        for d in ds {
            let pos = self.sp;
            let ty = f.local(d.local).ty.clone();
            self.rsadd(&ty);
            self.locals.insert(d.local, pos);
            if let Some(init) = &d.init {
                self.expr(init)?;
                let n = self.size(&ty);
                self.cpdown(Loc::Sp(pos), 0, n);
                self.pop(n);
            }
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), Diagnostic> {
        match &s.kind {
            StmtKind::Block(b) => self.block(b)?,
            StmtKind::Decl(ds) => self.declare(ds)?,
            StmtKind::Expr(e) => {
                self.expr(e)?;
                let n = self.size(&e.ty);
                self.pop(n);
            }
            // `JZ` past the branch, which ends with a `JMP` to the end even
            // when there is no `else`.
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond)?;
                let end = self.e.label();
                let other = if otherwise.is_some() {
                    self.e.label()
                } else {
                    end
                };
                self.e.jump(Opcode::Jz, other);
                self.sp -= 4;
                self.stmt(then)?;
                self.e.jump(Opcode::Jmp, end);
                if let Some(o) = otherwise {
                    self.e.place(other);
                    self.stmt(o)?;
                }
                self.e.place(end);
            }
            StmtKind::While { cond, body } => {
                let top = self.e.label();
                let end = self.e.label();
                self.e.place(top);
                self.expr(cond)?;
                self.e.jump(Opcode::Jz, end);
                self.sp -= 4;
                // `continue` goes to the jump back, past the body's cleanup.
                let next = self.e.label();
                self.in_loop(end, Some(next), body)?;
                self.e.place(next);
                self.e.jump(Opcode::Jmp, top);
                self.e.place(end);
            }
            StmtKind::DoWhile { body, cond } => {
                let top = self.e.label();
                let test = self.e.label();
                let end = self.e.label();
                self.e.place(top);
                self.in_loop(end, Some(test), body)?;
                self.e.place(test);
                self.expr(cond)?;
                self.e.jump(Opcode::Jz, end);
                self.sp -= 4;
                self.e.jump(Opcode::Jmp, top);
                self.e.place(end);
            }
            StmtKind::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                let start = self.sp;
                match init {
                    ForInit::Exprs(es) => {
                        for e in es {
                            self.expr(e)?;
                            let n = self.size(&e.ty);
                            self.pop(n);
                        }
                    }
                    ForInit::Decl(ds) => self.declare(ds)?,
                }
                let top = self.e.label();
                let next = self.e.label();
                let end = self.e.label();
                self.e.place(top);
                // A missing condition is a constant 1, tested like any other.
                match cond {
                    Some(c) => self.expr(c)?,
                    None => self.konst(&ConstValue::Int(1)),
                }
                self.e.jump(Opcode::Jz, end);
                self.sp -= 4;
                self.in_loop(end, Some(next), body)?;
                self.e.place(next);
                for e in step {
                    self.expr(e)?;
                    let n = self.size(&e.ty);
                    self.pop(n);
                }
                self.e.jump(Opcode::Jmp, top);
                self.e.place(end);
                self.pop(self.sp - start);
            }
            StmtKind::Switch {
                value,
                body,
                labels,
            } => self.switch(value, body, labels)?,
            StmtKind::Case(_) | StmtKind::Default | StmtKind::Empty => {}
            // A jump straight to the target: the locals of the blocks it
            // leaves stay on the stack, as in BioWare's bytecode.
            StmtKind::Break => {
                let to = self.loops.last().map(|l| l.brk);
                if let Some(to) = to {
                    self.e.jump(Opcode::Jmp, to);
                }
            }
            StmtKind::Continue => {
                let to = self.loops.iter().rev().find_map(|l| l.cont);
                if let Some(to) = to {
                    self.e.jump(Opcode::Jmp, to);
                }
            }
            // The value is copied to the return slot, everything above the
            // parameters removed, and the epilogue jumped to. The `MOVSP`
            // that would remove the value as a statement's follows the jump,
            // where nothing reaches it.
            StmtKind::Return(value) => {
                let epilogue = self.epilogue.expect("a return is in a function");
                match value {
                    Some(v) => {
                        let n = self.size(&v.ty);
                        self.expr(v)?;
                        if let Some((pos, size)) = self.ret {
                            self.cpdown(Loc::Sp(pos), 0, size);
                        }
                        let above = self.sp - self.entry_sp;
                        self.ins(Opcode::MovSp, T::VOID, Operands::Offset(-above));
                        self.e.jump(Opcode::Jmp, epilogue);
                        self.pop(n);
                    }
                    None => {
                        let above = self.sp - self.entry_sp;
                        if above != 0 {
                            self.ins(Opcode::MovSp, T::VOID, Operands::Offset(-above));
                        }
                        self.e.jump(Opcode::Jmp, epilogue);
                    }
                }
            }
        }
        Ok(())
    }

    fn in_loop(&mut self, brk: Label, cont: Option<Label>, body: &Stmt) -> Result<(), Diagnostic> {
        self.loops.push(Loop { brk, cont });
        let r = self.stmt(body);
        self.loops.pop();
        r
    }

    /// The value stays on the stack while each `case` compares a copy of it
    /// and jumps to its label; then a jump to `default`, or to the end. The
    /// end, where `break` goes, removes the value.
    fn switch(
        &mut self,
        value: &Expr,
        body: &Block,
        labels: &[SwitchLabel],
    ) -> Result<(), Diagnostic> {
        self.expr(value)?;
        let end = self.e.label();
        let mut at: HashMap<usize, Label> = HashMap::new();
        for l in labels {
            let label = self.e.label();
            at.insert(l.index, label);
            if let Some(v) = l.value {
                self.ins(
                    Opcode::CpTopSp,
                    T::STACK,
                    Operands::Stack {
                        offset: -4,
                        size: 4,
                    },
                );
                self.ins(Opcode::Const, T::INT, Operands::Int(v));
                self.ins(Opcode::Equal, T::INT_INT, Operands::None);
                self.e.jump(Opcode::Jnz, label);
            }
        }
        let default = labels
            .iter()
            .find(|l| l.value.is_none())
            .map(|l| at[&l.index]);
        self.e.jump(Opcode::Jmp, default.unwrap_or(end));
        self.loops.push(Loop {
            brk: end,
            cont: None,
        });
        let start = self.sp;
        let mut r = Ok(());
        for (i, s) in body.stmts.iter().enumerate() {
            if let Some(l) = at.get(&i) {
                self.e.place(*l);
            }
            r = self.stmt(s);
            if r.is_err() {
                break;
            }
        }
        self.loops.pop();
        r?;
        self.pop(self.sp - start);
        self.e.place(end);
        self.pop(4);
        Ok(())
    }

    // ---- expressions -----------------------------------------------------------

    fn expr(&mut self, e: &Expr) -> Result<(), Diagnostic> {
        match &e.kind {
            ExprKind::Literal(v) | ExprKind::Constant { value: v, .. } => self.konst(v),
            ExprKind::Var(v) => {
                let n = self.size(&e.ty);
                self.cptop(self.var_loc(*v), 0, n);
            }
            // The whole base, then a `DESTRUCT` that keeps the member.
            ExprKind::Member { base, member } => {
                self.expr(base)?;
                let whole = self.size(&base.ty);
                let n = self.size(&e.ty);
                let off = self.member_offset(*member);
                self.ins(
                    Opcode::Destruct,
                    T::STACK,
                    Operands::Destruct {
                        size: whole as u16,
                        keep_offset: off as u16,
                        keep_size: n as u16,
                    },
                );
                self.sp += n - whole;
            }
            ExprKind::Call(c) => self.call(c, &e.ty)?,
            ExprKind::Unary { op, operand } => {
                self.expr(operand)?;
                let ty = scalar(&operand.ty);
                match op {
                    UnaryOp::Neg => self.ins(Opcode::Neg, ty, Operands::None),
                    UnaryOp::Not => self.ins(Opcode::Not, T::INT, Operands::None),
                    UnaryOp::BitNot => self.ins(Opcode::Comp, T::INT, Operands::None),
                    UnaryOp::Plus => {}
                }
            }
            ExprKind::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs, &e.ty)?,
            ExprKind::Assign { op, target, value } => {
                let n = self.size(&target.ty);
                let (loc, off) = self.place(target)?;
                match op.binary_op() {
                    None => self.expr(value)?,
                    Some(bop) => self.binary(bop, target, value, &target.ty)?,
                }
                self.cpdown(loc, off, n);
            }
            // Prefix: change the variable, then read it. Postfix: read, then
            // change it below the value read. A member is read as any member
            // is, the whole variable then a `DESTRUCT`.
            ExprKind::IncDec { op, prefix, target } => {
                let (loc, off) = self.place(target)?;
                if !*prefix {
                    self.expr(target)?;
                }
                let (opcode, offset) = match (loc, op) {
                    (Loc::Sp(pos), IncDec::Inc) => (Opcode::IncISp, pos + off - self.sp),
                    (Loc::Sp(pos), IncDec::Dec) => (Opcode::DecISp, pos + off - self.sp),
                    (Loc::Bp(pos), IncDec::Inc) => (Opcode::IncIBp, pos + off - self.globals_size),
                    (Loc::Bp(pos), IncDec::Dec) => (Opcode::DecIBp, pos + off - self.globals_size),
                };
                self.ins(opcode, T::INT, Operands::Offset(offset));
                if *prefix {
                    self.expr(target)?;
                }
            }
            ExprKind::Cond {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond)?;
                let other = self.e.label();
                let end = self.e.label();
                self.e.jump(Opcode::Jz, other);
                self.sp -= 4;
                let start = self.sp;
                self.expr(then)?;
                self.e.jump(Opcode::Jmp, end);
                self.sp = start;
                self.e.place(other);
                self.expr(otherwise)?;
                self.e.place(end);
            }
            ExprKind::Vector(parts) => {
                for p in parts.iter() {
                    self.expr(p)?;
                }
            }
            ExprKind::Error => return Err(unsupported(e.span, "an expression that did not check")),
        }
        Ok(())
    }

    fn binary(
        &mut self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        ty: &Type,
    ) -> Result<(), Diagnostic> {
        match op {
            // A copy of the left value decides; zero skips the right operand
            // and the `LOGAND`, leaving the zero.
            BinaryOp::LogAnd => {
                self.expr(lhs)?;
                let end = self.e.label();
                self.ins(
                    Opcode::CpTopSp,
                    T::STACK,
                    Operands::Stack {
                        offset: -4,
                        size: 4,
                    },
                );
                self.e.jump(Opcode::Jz, end);
                self.expr(rhs)?;
                self.ins(Opcode::LogAnd, T::INT_INT, Operands::None);
                self.sp -= 4;
                self.e.place(end);
            }
            // Both operands always run: zero goes to the right operand, and
            // non-zero tests a second copy whose jump to the `LOGOR` is never
            // taken (docs/research/bytecode-quirks.md).
            BinaryOp::LogOr => {
                self.expr(lhs)?;
                let right = self.e.label();
                let end = self.e.label();
                self.ins(
                    Opcode::CpTopSp,
                    T::STACK,
                    Operands::Stack {
                        offset: -4,
                        size: 4,
                    },
                );
                self.e.jump(Opcode::Jz, right);
                self.ins(
                    Opcode::CpTopSp,
                    T::STACK,
                    Operands::Stack {
                        offset: -4,
                        size: 4,
                    },
                );
                self.e.jump(Opcode::Jz, end);
                self.e.place(right);
                self.expr(rhs)?;
                self.e.place(end);
                self.ins(Opcode::LogOr, T::INT_INT, Operands::None);
                self.sp -= 4;
            }
            _ => {
                self.expr(lhs)?;
                self.expr(rhs)?;
                let opcode = match op {
                    BinaryOp::BitOr => Opcode::IncOr,
                    BinaryOp::BitXor => Opcode::ExcOr,
                    BinaryOp::BitAnd => Opcode::BoolAnd,
                    BinaryOp::Eq => Opcode::Equal,
                    BinaryOp::Ne => Opcode::NEqual,
                    BinaryOp::Lt => Opcode::Lt,
                    BinaryOp::Gt => Opcode::Gt,
                    BinaryOp::Le => Opcode::Leq,
                    BinaryOp::Ge => Opcode::Geq,
                    BinaryOp::Shl => Opcode::ShLeft,
                    BinaryOp::Shr => Opcode::ShRight,
                    BinaryOp::UShr => Opcode::UShRight,
                    BinaryOp::Add => Opcode::Add,
                    BinaryOp::Sub => Opcode::Sub,
                    BinaryOp::Mul => Opcode::Mul,
                    BinaryOp::Div => Opcode::Div,
                    BinaryOp::Mod => Opcode::Mod,
                    BinaryOp::LogAnd | BinaryOp::LogOr => unreachable!(),
                };
                let (l, r) = (self.size(&lhs.ty), self.size(&rhs.ty));
                let (pair, operands) = match (&lhs.ty, &rhs.ty) {
                    (Type::Struct(_), _) | (Type::Vector, Type::Vector)
                        if matches!(op, BinaryOp::Eq | BinaryOp::Ne) =>
                    {
                        (T::STRUCT_STRUCT, Operands::Size(l as u16))
                    }
                    (a, b) => (pair(a, b), Operands::None),
                };
                self.ins(opcode, pair, operands);
                self.sp += self.size(ty) - l - r;
            }
        }
        Ok(())
    }

    /// A call: for a script function, a return slot, the arguments last
    /// first, `JSR`; for an engine routine, the arguments last first and
    /// `ACTION`, which leaves the result.
    fn call(&mut self, c: &Call, ret: &Type) -> Result<(), Diagnostic> {
        let base = self.sp;
        if let Callee::User(_) = c.callee {
            self.rsadd(ret);
        }
        for a in c.args.iter().rev() {
            match a {
                Arg::Value(e) => self.expr(e)?,
                Arg::Default(v) => self.konst(v),
                Arg::Action(e) => self.action(e)?,
            }
        }
        match c.callee {
            Callee::User(id) => {
                let l = self.fn_label(id);
                self.e.jump(Opcode::Jsr, l);
                if !self.callees.contains(&id) {
                    self.callees.push(id);
                }
            }
            Callee::Engine(n) => self.ins(
                Opcode::Action,
                T::VOID,
                Operands::Action {
                    routine: n,
                    argc: c.args.len() as u8,
                },
            ),
        }
        self.sp = base + self.size(ret);
        Ok(())
    }

    /// An `action` argument: `STORE_STATE` with the globals and the current
    /// frame, a `JMP` over the deferred code, and the deferred code: the
    /// call, then `RETN`. It pushes nothing.
    fn action(&mut self, e: &Expr) -> Result<(), Diagnostic> {
        let globals = if self.p.globals.is_empty() {
            0
        } else {
            self.globals_size
        };
        self.ins(
            Opcode::StoreState,
            T::STORE_STATE,
            Operands::StoreState {
                globals: globals as u32,
                locals: self.sp as u32,
            },
        );
        let over = self.e.label();
        self.e.jump(Opcode::Jmp, over);
        let start = self.sp;
        self.expr(e)?;
        self.ins(Opcode::Retn, T::VOID, Operands::None);
        self.sp = start;
        self.e.place(over);
        Ok(())
    }
}

/// The type byte of a scalar value.
fn scalar(t: &Type) -> T {
    match t {
        Type::Int => T::INT,
        Type::Float => T::FLOAT,
        Type::String => T::STRING,
        Type::Object => T::OBJECT,
        Type::Engine(n) => T(T::ENGINE_0.0 + n),
        _ => T::VOID,
    }
}

/// The type byte of a pair of operands.
fn pair(a: &Type, b: &Type) -> T {
    match (a, b) {
        (Type::Int, Type::Int) => T::INT_INT,
        (Type::Float, Type::Float) => T::FLOAT_FLOAT,
        (Type::Int, Type::Float) => T::INT_FLOAT,
        (Type::Float, Type::Int) => T::FLOAT_INT,
        (Type::String, Type::String) => T::STRING_STRING,
        (Type::Object, Type::Object) => T::OBJECT_OBJECT,
        (Type::Vector, Type::Vector) => T::VECTOR_VECTOR,
        (Type::Vector, Type::Float) => T::VECTOR_FLOAT,
        (Type::Float, Type::Vector) => T::FLOAT_VECTOR,
        (Type::Engine(n), Type::Engine(_)) => T(T::ENGINE_ENGINE_0.0 + n),
        _ => T::STRUCT_STRUCT,
    }
}
