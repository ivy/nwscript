//! The recovered program: variables, expressions and statements, before
//! printing.

/// A value's type as the bytecode shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Int,
    Float,
    String,
    Object,
    Vector,
    /// Engine structure `n` (`effect`, `location`, ... by `nwscript.nss`'s numbering).
    Engine(u8),
    /// The script struct with this index in [`Script::structs`].
    Struct(usize),
}

impl Ty {
    /// Bytes on the stack, for the types whose size does not depend on a struct.
    pub fn size(self) -> Option<u32> {
        match self {
            Ty::Vector => Some(12),
            Ty::Struct(_) => None,
            _ => Some(4),
        }
    }

    /// The type an `RSADD` type byte reserves.
    pub fn of_rsadd(ty: nwscript_ncs::Type) -> Option<Ty> {
        use nwscript_ncs::Type as T;
        Some(match ty {
            T::INT => Ty::Int,
            T::FLOAT => Ty::Float,
            T::STRING => Ty::String,
            T::OBJECT => Ty::Object,
            t => Ty::Engine(t.engine()?),
        })
    }

    /// The type of an engine routine's parameter or return value; `None`
    /// for `void` and `action`.
    pub fn of_sema(t: &nwscript_sema::Type) -> Option<Ty> {
        use nwscript_sema::Type as S;
        Some(match t {
            S::Int => Ty::Int,
            S::Float => Ty::Float,
            S::String => Ty::String,
            S::Object => Ty::Object,
            S::Vector => Ty::Vector,
            S::Engine(n) => Ty::Engine(*n),
            S::Void | S::Action | S::Struct(_) => return None,
        })
    }
}

pub type VarId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    Global,
    Param,
    Local,
}

/// A variable. Separate stack slots that turn out to be one vector or
/// struct are joined: each slot's variable then names the whole variable
/// and its component.
#[derive(Clone, Debug)]
pub struct Var {
    pub kind: VarKind,
    pub ty: Option<Ty>,
    pub size: u32,
    pub name: String,
    /// The whole variable and component index, when this slot was joined into one.
    pub part_of: Option<(VarId, usize)>,
    /// The function that declares it; `None` for a global.
    pub owner: Option<usize>,
}

/// A struct the program declares, by its fields' types.
#[derive(Clone, Debug, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Ty>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Comp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    LogAnd,
    LogOr,
    BitOr,
    BitXor,
    BitAnd,
    Eq,
    Ne,
    Ge,
    Gt,
    Lt,
    Le,
    Shl,
    Shr,
    UShr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl BinOp {
    pub fn text(self) -> &'static str {
        use BinOp::*;
        match self {
            LogAnd => "&&",
            LogOr => "||",
            BitOr => "|",
            BitXor => "^",
            BitAnd => "&",
            Eq => "==",
            Ne => "!=",
            Ge => ">=",
            Gt => ">",
            Lt => "<",
            Le => "<=",
            Shl => "<<",
            Shr => ">>",
            UShr => ">>>",
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Mod => "%",
        }
    }

    /// Binding strength, higher binds tighter (C's table).
    pub fn prec(self) -> u8 {
        use BinOp::*;
        match self {
            LogOr => 3,
            LogAnd => 4,
            BitOr => 5,
            BitXor => 6,
            BitAnd => 7,
            Eq | Ne => 8,
            Ge | Gt | Lt | Le => 9,
            Shl | Shr | UShr => 10,
            Add | Sub => 11,
            Mul | Div | Mod => 12,
        }
    }
}

/// Something an assignment or increment writes: a variable, or one slot
/// of a vector or struct variable (see [`Var::part_of`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    Var(VarId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Int(i32),
    Float(f32),
    /// Windows-1252 bytes.
    Str(Vec<u8>),
    /// 0 is `OBJECT_SELF`, 1 is `OBJECT_INVALID`.
    Object(i32),
    /// A vector literal; written only as an omitted default.
    Vector([f32; 3]),
    Place(Place),
    Engine {
        routine: u16,
        args: Vec<Expr>,
    },
    /// A call to the script function with this index in [`Script::funcs`].
    User {
        func: usize,
        args: Vec<Expr>,
    },
    /// Component `n` of a vector value that is not a variable: `f().x`.
    Member(Box<Expr>, usize),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Assign(Place, Box<Expr>),
    IncDec {
        place: Place,
        inc: bool,
        post: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Decl(VarId),
    Expr(Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    DoWhile(Vec<Stmt>, Expr),
    /// `for (init; cond; step) body`.
    For(Expr, Expr, Expr, Vec<Stmt>),
    /// The subject and the body, whose top level holds `Case` and `Default` labels.
    Switch(Expr, Vec<Stmt>),
    Case(i32),
    Default,
    Block(Vec<Stmt>),
    Break,
    Continue,
    Return(Option<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuncKind {
    /// The code before the entry point that initialises the globals.
    Globals,
    /// `main` or `StartingConditional`.
    Entry,
    User,
}

/// One function of the recovered program.
#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    pub kind: FuncKind,
    /// The function's byte range in the original file.
    pub start: u32,
    pub end: u32,
    pub params: Vec<VarId>,
    /// Defaults the parameters declare, where a call relies on one.
    pub defaults: Vec<Option<Expr>>,
    pub ret: Option<Ty>,
    pub body: Result<Vec<Stmt>, String>,
}

/// The whole recovered program.
#[derive(Clone, Debug, Default)]
pub struct Script {
    pub vars: Vec<Var>,
    pub structs: Vec<StructDef>,
    pub funcs: Vec<Func>,
}

impl Script {
    /// The variable a slot belongs to and its component, following joins.
    pub fn root(&self, v: VarId) -> (VarId, Option<usize>) {
        match self.vars[v].part_of {
            Some((whole, n)) => (whole, Some(n)),
            None => (v, None),
        }
    }
}
