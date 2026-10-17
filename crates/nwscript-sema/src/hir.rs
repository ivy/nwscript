//! The checked program: a typed, resolved form of a script and everything
//! it includes, which code generation walks without re-deriving anything.
//!
//! [`crate::analyze`] builds a [`Program`] from the syntax trees. In it every
//! expression carries its [`Type`] and [`Span`]; every name is resolved to a
//! [`Var`] (a global, a parameter or a local), an engine constant with its
//! folded value, a user function ([`FuncId`]) or an engine routine (its
//! `ACTION` number); every call lists one [`Arg`] per parameter with omitted
//! defaults filled in; `case` labels carry their folded values; structs carry
//! their layouts; and each function says whether the entry point reaches it.
//!
//! Lints have no part in this form: what is reported and at which level is
//! decided over the diagnostics, never by changing the program.
//!
//! A program built from source with errors still has this shape, with
//! [`ExprKind::Error`] where an expression could not be resolved, so an
//! editor can use it. Code generation needs a program whose remaining errors
//! are at most the codes in [`nwscript_diag::codes::BIOWARE_ACCEPTED`], for
//! which the form records what was written:
//!
//! - a call to an engine routine with arguments its declaration does not
//!   take (`E0308`, `E0309`) lists exactly the arguments written, then any
//!   defaults that follow them;
//! - a definition whose return type disagrees with its prototype (`E0311`)
//!   has the definition's signature.

use crate::types::{ConstValue, Type};
use nwscript_diag::{FileId, Span};
use nwscript_syntax::Parse;
pub use nwscript_syntax::ast::{AssignOp, BinaryOp, IncDec};

/// A user function, by its index in [`Program::functions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(pub u32);

/// A script global, by its index in [`Program::globals`]: its position in
/// declaration order across the script and its includes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalId(pub u32);

/// A struct type, by its index in [`Program::structs`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StructId(pub u32);

/// A local variable, by its index in its function's [`Function::locals`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

/// A lexical scope, by its index in its function's [`Function::scopes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(pub u32);

/// A script and everything it includes, checked.
#[derive(Clone, Debug)]
pub struct Program {
    /// The parsed files: the script first, then each include in the order it
    /// was first reached. An include reached a second time is not read again.
    pub files: Vec<SourceUnit>,
    /// Struct types in declaration order.
    pub structs: Vec<StructDef>,
    /// Script globals in declaration order, includes expanded where their
    /// `#include` stands. `nwscript.nss` constants are not globals; a use of
    /// one is [`ExprKind::Constant`].
    pub globals: Vec<Global>,
    /// User functions in the order of their first declaration (prototype or
    /// definition).
    pub functions: Vec<Function>,
    /// `void main()` or `int StartingConditional()`; `None` for a library
    /// such as an include file.
    pub entry: Option<Entry>,
}

/// One parsed file of the program.
#[derive(Clone, Debug)]
pub struct SourceUnit {
    /// The script's own name, or the include's name as the resolver gave it.
    pub name: String,
    pub parse: Parse,
    /// The file and `#include` directive that first reached this file;
    /// `None` for the script itself.
    pub included_from: Option<(FileId, Span)>,
}

/// The entry point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub kind: EntryKind,
    pub function: FuncId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// `void main()`, run by the engine.
    Main,
    /// `int StartingConditional()`, a dialogue condition.
    StartingConditional,
}

impl EntryKind {
    pub fn name(self) -> &'static str {
        match self {
            EntryKind::Main => "main",
            EntryKind::StartingConditional => "StartingConditional",
        }
    }
}

/// A struct type and its layout on the stack.
#[derive(Clone, Debug, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    /// Bytes on the stack: the sum of the fields' sizes.
    pub size: u32,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    /// Bytes from the start of the struct.
    pub offset: u32,
    pub span: Span,
}

/// A script global variable.
#[derive(Clone, Debug)]
pub struct Global {
    pub name: String,
    pub ty: Type,
    /// The initialiser, run before the entry point.
    pub init: Option<Expr>,
    /// The declarator's span.
    pub span: Span,
    /// Position in source order across the script and its includes (shared
    /// with [`Function::decl_seq`]).
    pub seq: u32,
    /// True when a reachable function or another referenced global's
    /// initialiser names it.
    pub referenced: bool,
}

/// A user function.
#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub ret: Type,
    pub params: Vec<Param>,
    /// Every prototype's span, in order.
    pub prototypes: Vec<Span>,
    /// The definition's span; `None` when only prototypes exist.
    pub definition: Option<Span>,
    /// Position in source order of the first declaration.
    pub decl_seq: u32,
    /// Position in source order of the definition.
    pub def_seq: Option<u32>,
    /// The file holding the definition, or the first prototype.
    pub file: FileId,
    /// The body of the definition.
    pub body: Option<Block>,
    pub locals: Vec<Local>,
    /// Scopes of the body: the body block's scope is the first. Parameters
    /// sit outside all of them.
    pub scopes: Vec<Scope>,
    /// True when the entry point reaches this function by calls, `action`
    /// arguments included, or a global initialiser calls it. Always false in
    /// a program without an entry point.
    pub reachable: bool,
    /// The user functions this one's body calls, each once, in order of
    /// first call.
    pub callees: Vec<FuncId>,
}

/// A parameter of a user function.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    /// The default a call that omits this argument passes.
    pub default: Option<ConstValue>,
    pub span: Span,
}

/// A local variable.
#[derive(Clone, Debug, PartialEq)]
pub struct Local {
    pub name: String,
    pub ty: Type,
    /// The scope it is declared in.
    pub scope: ScopeId,
    /// The declarator's span.
    pub span: Span,
}

/// A lexical scope: a block, a `for` with a declaration in its initialiser.
#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    /// Locals declared directly in this scope, in declaration order.
    pub locals: Vec<LocalId>,
    pub span: Span,
}

/// `{ statements }` with its own scope.
#[derive(Clone, Debug)]
pub struct Block {
    pub scope: ScopeId,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Block(Block),
    /// A declaration of one or more locals, each with its initialiser, in
    /// order.
    Decl(Vec<Declared>),
    /// An expression evaluated for its effect; its value is discarded.
    Expr(Expr),
    If {
        cond: Expr,
        then: Box<Stmt>,
        otherwise: Option<Box<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    DoWhile {
        body: Box<Stmt>,
        cond: Expr,
    },
    For {
        /// The scope of a declaration in the initialiser; `None` when the
        /// initialiser is expressions.
        scope: Option<ScopeId>,
        init: ForInit,
        cond: Option<Expr>,
        step: Vec<Expr>,
        body: Box<Stmt>,
    },
    /// `switch (value) { ... }`. The labels are statements of the body, in
    /// place; `labels` lists them in order for building the jump table.
    Switch {
        value: Expr,
        body: Block,
        labels: Vec<SwitchLabel>,
    },
    /// `case value:`, directly in a switch body.
    Case(i32),
    /// `default:`, directly in a switch body.
    Default,
    Break,
    Continue,
    Return(Option<Expr>),
    /// `;`.
    Empty,
}

/// One local of a declaration statement.
#[derive(Clone, Debug)]
pub struct Declared {
    pub local: LocalId,
    pub init: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum ForInit {
    Exprs(Vec<Expr>),
    Decl(Vec<Declared>),
}

/// A label of a switch: its value (`None` for `default`), and the index of
/// its statement in the switch body's `stmts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwitchLabel {
    pub value: Option<i32>,
    pub index: usize,
    pub span: Span,
}

/// An expression with its type.
#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Type,
    /// The expression's span; for a parenthesised expression, the span of
    /// what is inside the parentheses.
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    /// A literal as written: an int, float or string literal, `OBJECT_SELF`
    /// (`Object(0)`) or `OBJECT_INVALID` (`Object(1)`). A negative number is
    /// [`UnaryOp::Neg`] of a literal, as written.
    Literal(ConstValue),
    /// A constant `nwscript.nss` declares, by name, with its folded value.
    Constant {
        name: String,
        value: ConstValue,
    },
    /// A variable's value.
    Var(Var),
    /// A struct field or vector component of `base`.
    Member {
        base: Box<Expr>,
        member: Member,
    },
    Call(Call),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// The operands' types pick the instruction: `int + float` is `ADDIF`.
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `target op= value`; the value of the expression is the stored value.
    /// `target` is a place: [`ExprKind::Var`] or a [`ExprKind::Member`]
    /// chain over one.
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    /// `++x`, `x--` and the like, on an `int` place.
    IncDec {
        op: IncDec,
        prefix: bool,
        target: Box<Expr>,
    },
    /// `cond ? then : otherwise`.
    Cond {
        cond: Box<Expr>,
        then: Box<Expr>,
        otherwise: Box<Expr>,
    },
    /// `[x, y, z]`.
    Vector(Box<[Expr; 3]>),
    /// Something that did not check; its type is [`Type::Void`] unless the
    /// checker knew better. Only a program with errors has these.
    Error,
}

/// What a name resolved to as a variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Var {
    Global(GlobalId),
    /// The function's parameter at this position.
    Param(u32),
    Local(LocalId),
}

/// A member access.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Member {
    /// Field `index` of struct `ty`; its offset is in [`StructDef::fields`].
    Field { ty: StructId, index: u32 },
    /// A vector's `.x` (0), `.y` (1) or `.z` (2), each a float at 4 bytes per
    /// component.
    Component(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// `-`
    Neg,
    /// `+`
    Plus,
    /// `!`
    Not,
    /// `~`
    BitNot,
}

/// A call, with one argument per parameter of the callee.
#[derive(Clone, Debug)]
pub struct Call {
    pub callee: Callee,
    /// In parameter order: what was written, then the defaults of the
    /// parameters the call omits. Shorter or longer than the parameter list
    /// only where `E0308` was reported.
    pub args: Vec<Arg>,
    /// The span of the callee's name.
    pub name_span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Callee {
    /// An engine routine by its `ACTION` number.
    Engine(u16),
    User(FuncId),
}

#[derive(Clone, Debug)]
pub enum Arg {
    /// A value computed before the call.
    Value(Expr),
    /// An omitted argument: the parameter's default.
    Default(ConstValue),
    /// An `action` argument: a call to a `void` function, stored with the
    /// state of the stack and run later by the engine.
    Action(Expr),
}

impl Program {
    /// The script itself.
    pub fn main(&self) -> &SourceUnit {
        &self.files[0]
    }

    pub fn function(&self, id: FuncId) -> &Function {
        &self.functions[id.0 as usize]
    }

    pub fn global(&self, id: GlobalId) -> &Global {
        &self.globals[id.0 as usize]
    }

    pub fn struct_def(&self, id: StructId) -> &StructDef {
        &self.structs[id.0 as usize]
    }

    pub fn struct_by_name(&self, name: &str) -> Option<(StructId, &StructDef)> {
        self.structs
            .iter()
            .enumerate()
            .find(|(_, s)| s.name == name)
            .map(|(i, s)| (StructId(i as u32), s))
    }

    /// Bytes a value of this type takes on the stack. A struct the program
    /// does not declare takes 0.
    pub fn size_of(&self, ty: &Type) -> u32 {
        match ty {
            Type::Struct(n) => self.struct_by_name(n).map_or(0, |(_, s)| s.size),
            t => t.size().unwrap_or(0),
        }
    }

    /// The function with this name.
    pub fn function_by_name(&self, name: &str) -> Option<(FuncId, &Function)> {
        self.functions
            .iter()
            .enumerate()
            .find(|(_, f)| f.name == name)
            .map(|(i, f)| (FuncId(i as u32), f))
    }

    /// The functions the entry point reaches, in order of definition.
    pub fn reachable_functions(&self) -> Vec<FuncId> {
        let mut ids: Vec<FuncId> = (0..self.functions.len() as u32)
            .map(FuncId)
            .filter(|&id| self.function(id).reachable)
            .collect();
        ids.sort_by_key(|&id| self.function(id).def_seq);
        ids
    }
}

impl Function {
    pub fn local(&self, id: LocalId) -> &Local {
        &self.locals[id.0 as usize]
    }

    pub fn scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0 as usize]
    }
}

impl Expr {
    /// True for a place an assignment can store to.
    pub fn is_place(&self) -> bool {
        match &self.kind {
            ExprKind::Var(_) => true,
            ExprKind::Member { base, .. } => base.is_place(),
            _ => false,
        }
    }
}
