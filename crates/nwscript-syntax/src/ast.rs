//! Typed views over the syntax tree, in rust-analyzer's style: each node
//! type wraps a [`SyntaxNode`] of one kind, and its accessors find children
//! by kind. A missing child (after a syntax error) is `None`; the accessors
//! never panic.
//!
//! Text accessors return the tree's text, which for identifiers, keywords
//! and include names is the source's ASCII. String literal contents come
//! back as source bytes ([`Literal::value`]).

use crate::SyntaxKind::{self, *};
use crate::text::latin1_bytes;
use crate::{SyntaxNode, SyntaxToken};

/// A typed node.
pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

fn child<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
    parent.children().find_map(N::cast)
}

fn children<N: AstNode>(parent: &SyntaxNode) -> impl Iterator<Item = N> + use<N> {
    parent.children().filter_map(N::cast)
}

fn token(parent: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    parent
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == kind)
}

/// The first non-trivia token directly in `parent`.
fn first_token(parent: &SyntaxNode) -> Option<SyntaxToken> {
    parent
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| !t.kind().is_trivia())
}

/// The first child node of type `N` after the token of kind `after`.
fn child_after<N: AstNode>(parent: &SyntaxNode, after: SyntaxKind) -> Option<N> {
    parent
        .children_with_tokens()
        .skip_while(|e| e.as_token().is_none_or(|t| t.kind() != after))
        .filter_map(|e| e.into_node())
        .find_map(N::cast)
}

macro_rules! ast_node {
    ($(#[$m:meta])* $name:ident, $kind:ident) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name(SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == $kind
            }
            fn cast(node: SyntaxNode) -> Option<Self> {
                Self::can_cast(node.kind()).then(|| $name(node))
            }
            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

macro_rules! ast_enum {
    ($(#[$m:meta])* $name:ident { $( $variant:ident ( $ty:ident ) ),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $variant($ty), )*
        }

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                $( $ty::can_cast(kind) )||*
            }
            fn cast(node: SyntaxNode) -> Option<Self> {
                $( if $ty::can_cast(node.kind()) { return $ty::cast(node).map($name::$variant); } )*
                None
            }
            fn syntax(&self) -> &SyntaxNode {
                match self {
                    $( $name::$variant(n) => n.syntax(), )*
                }
            }
        }
    };
}

// ---- top level ---------------------------------------------------------

ast_node!(
    /// A whole file: directives and declarations.
    SourceFile,
    SOURCE_FILE
);
ast_node!(
    /// `#include "name"`.
    Include,
    INCLUDE
);
ast_node!(
    /// `#define NAME rest-of-line`.
    Define,
    DEFINE
);
ast_node!(
    /// A function prototype (`;`) or definition (a body).
    FunctionDecl,
    FUNCTION_DECL
);
ast_node!(
    /// `( params )` or `(void)`.
    ParamList,
    PARAM_LIST
);
ast_node!(
    /// `[const] type name [= default]`.
    Param,
    PARAM
);
ast_node!(
    /// `[const] type declarator {, declarator};` at top level.
    GlobalDecl,
    GLOBAL_DECL
);
ast_node!(
    /// `[const] type declarator {, declarator};` in a block, or the
    /// declaration in a `for` initialiser (without the `;`).
    LocalDecl,
    LOCAL_DECL
);
ast_node!(
    /// `name [= initialiser]`.
    Declarator,
    DECLARATOR
);
ast_node!(
    /// `struct NAME { members };`.
    StructDecl,
    STRUCT_DECL
);
ast_node!(
    /// One member declaration in a struct: `type name {, name};`.
    FieldDecl,
    FIELD_DECL
);
ast_node!(
    /// A `;` at top level.
    EmptyDecl,
    EMPTY_DECL
);
ast_node!(
    /// A type: a built-in keyword, `struct NAME`, or a name (an engine
    /// structure type such as `effect`, or an unknown name).
    Type,
    TYPE
);
ast_node!(
    /// Tokens skipped while recovering from a syntax error.
    ErrorNode,
    ERROR
);

ast_enum!(
    /// A top-level item.
    Item {
        Include(Include),
        Define(Define),
        Function(FunctionDecl),
        Global(GlobalDecl),
        Struct(StructDecl),
        Empty(EmptyDecl),
    }
);

impl SourceFile {
    /// The root of a parsed file. The parser always produces a
    /// [`SOURCE_FILE`] root.
    pub(crate) fn cast_root(node: SyntaxNode) -> SourceFile {
        debug_assert_eq!(node.kind(), SOURCE_FILE);
        SourceFile(node)
    }

    /// Items in source order.
    pub fn items(&self) -> impl Iterator<Item = Item> + use<> {
        children(&self.0)
    }

    pub fn includes(&self) -> impl Iterator<Item = Include> + use<> {
        children(&self.0)
    }

    pub fn defines(&self) -> impl Iterator<Item = Define> + use<> {
        children(&self.0)
    }

    /// Function prototypes and definitions, in source order. For
    /// `nwscript.nss`, a prototype's index among these is its engine
    /// function number.
    pub fn functions(&self) -> impl Iterator<Item = FunctionDecl> + use<> {
        children(&self.0)
    }

    pub fn globals(&self) -> impl Iterator<Item = GlobalDecl> + use<> {
        children(&self.0)
    }

    pub fn structs(&self) -> impl Iterator<Item = StructDecl> + use<> {
        children(&self.0)
    }
}

impl Include {
    /// The string literal token, quotes included.
    pub fn path_token(&self) -> Option<SyntaxToken> {
        token(&self.0, STRING)
    }

    /// The name between the quotes, as written (resource names compare
    /// case-insensitively).
    pub fn path(&self) -> Option<String> {
        let t = self.path_token()?;
        Some(string_contents(t.text()).to_string())
    }
}

impl Define {
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }

    /// The tokens after the name, up to the line end.
    pub fn value_tokens(&self) -> Vec<SyntaxToken> {
        self.0
            .children()
            .find(|n| n.kind() == DEFINE_BODY)
            .map(|b| {
                b.children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .filter(|t| !t.kind().is_trivia())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The text after the name up to the line end, trimmed; empty when
    /// there is none.
    pub fn value(&self) -> String {
        self.0
            .children()
            .find(|n| n.kind() == DEFINE_BODY)
            .map(|b| b.text().to_string().trim().to_string())
            .unwrap_or_default()
    }

    /// For `#define ENGINE_STRUCTURE_n name`: `n` and the type name.
    pub fn engine_structure(&self) -> Option<(u32, String)> {
        let name = self.name()?;
        let n = name
            .text()
            .strip_prefix("ENGINE_STRUCTURE_")?
            .parse()
            .ok()?;
        match self.value_tokens().as_slice() {
            [t] if t.kind() == IDENT => Some((n, t.text().to_string())),
            _ => None,
        }
    }

    /// For `#define ENGINE_NUM_STRUCTURES n`: `n`.
    pub fn engine_num_structures(&self) -> Option<u32> {
        if self.name()?.text() != "ENGINE_NUM_STRUCTURES" {
            return None;
        }
        match self.value_tokens().as_slice() {
            [t] if t.kind() == INT_NUMBER => t.text().parse().ok(),
            _ => None,
        }
    }
}

impl FunctionDecl {
    /// The return type.
    pub fn ret_type(&self) -> Option<Type> {
        child(&self.0)
    }

    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }

    pub fn param_list(&self) -> Option<ParamList> {
        child(&self.0)
    }

    /// The parameters in order; none for `()` and `(void)`.
    pub fn params(&self) -> impl Iterator<Item = Param> + use<> {
        self.param_list()
            .into_iter()
            .flat_map(|l| children::<Param>(&l.0))
    }

    /// The body of a definition; `None` for a prototype.
    pub fn body(&self) -> Option<Block> {
        child(&self.0)
    }

    /// True for a prototype: no body.
    pub fn is_prototype(&self) -> bool {
        self.body().is_none()
    }
}

impl ParamList {
    pub fn params(&self) -> impl Iterator<Item = Param> + use<> {
        children(&self.0)
    }

    /// True for `(void)`.
    pub fn is_void(&self) -> bool {
        token(&self.0, VOID_KW).is_some()
    }
}

impl Param {
    pub fn is_const(&self) -> bool {
        token(&self.0, CONST_KW).is_some()
    }

    pub fn ty(&self) -> Option<Type> {
        child(&self.0)
    }

    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }

    /// The default value after `=`, an expression as written.
    pub fn default_value(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl GlobalDecl {
    pub fn is_const(&self) -> bool {
        token(&self.0, CONST_KW).is_some()
    }

    pub fn ty(&self) -> Option<Type> {
        child(&self.0)
    }

    pub fn declarators(&self) -> impl Iterator<Item = Declarator> + use<> {
        children(&self.0)
    }
}

impl LocalDecl {
    pub fn is_const(&self) -> bool {
        token(&self.0, CONST_KW).is_some()
    }

    pub fn ty(&self) -> Option<Type> {
        child(&self.0)
    }

    pub fn declarators(&self) -> impl Iterator<Item = Declarator> + use<> {
        children(&self.0)
    }
}

impl Declarator {
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }

    /// The initialiser after `=`.
    pub fn init(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl StructDecl {
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }

    pub fn fields(&self) -> impl Iterator<Item = FieldDecl> + use<> {
        children(&self.0)
    }
}

impl FieldDecl {
    pub fn ty(&self) -> Option<Type> {
        child(&self.0)
    }

    /// The member names; an initialiser on one is a semantic error.
    pub fn declarators(&self) -> impl Iterator<Item = Declarator> + use<> {
        children(&self.0)
    }
}

/// What a [`Type`] names.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Void,
    Int,
    Float,
    String,
    Object,
    Vector,
    Action,
    /// `struct NAME`; the name token is `None` after a syntax error.
    Struct(Option<SyntaxToken>),
    /// A type written as a name: an engine structure type or an unknown name.
    Named(SyntaxToken),
}

impl Type {
    /// `None` only when the type is missing after a syntax error.
    pub fn kind(&self) -> Option<TypeKind> {
        let t = first_token(&self.0)?;
        Some(match t.kind() {
            VOID_KW => TypeKind::Void,
            INT_KW => TypeKind::Int,
            FLOAT_KW => TypeKind::Float,
            STRING_KW => TypeKind::String,
            OBJECT_KW => TypeKind::Object,
            VECTOR_KW => TypeKind::Vector,
            ACTION_KW => TypeKind::Action,
            STRUCT_KW => TypeKind::Struct(token(&self.0, IDENT)),
            IDENT => TypeKind::Named(t),
            _ => return None,
        })
    }

    /// The type's name as written: the keyword, `struct NAME`, or the name.
    pub fn text(&self) -> String {
        let words: Vec<String> = self
            .0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| !t.kind().is_trivia())
            .map(|t| t.text().to_string())
            .collect();
        words.join(" ")
    }
}

// ---- statements ----------------------------------------------------------

ast_node!(
    /// `{ statements }`.
    Block,
    BLOCK
);
ast_node!(
    /// `expression;`.
    ExprStmt,
    EXPR_STMT
);
ast_node!(
    /// `;`.
    EmptyStmt,
    EMPTY_STMT
);
ast_node!(
    /// `if (condition) then [else otherwise]`.
    IfStmt,
    IF_STMT
);
ast_node!(
    /// `while (condition) body`.
    WhileStmt,
    WHILE_STMT
);
ast_node!(
    /// `do body while (condition);`.
    DoStmt,
    DO_STMT
);
ast_node!(
    /// `for (init; condition; step) body`.
    ForStmt,
    FOR_STMT
);
ast_node!(
    /// The initialiser of a `for`: a declaration or expressions.
    ForInit,
    FOR_INIT
);
ast_node!(
    /// `switch (value) body`.
    SwitchStmt,
    SWITCH_STMT
);
ast_node!(
    /// `case value:`. Labels are statements of the block they sit in;
    /// that they sit in a switch is a semantic check.
    CaseLabel,
    CASE_LABEL
);
ast_node!(
    /// `default:`.
    DefaultLabel,
    DEFAULT_LABEL
);
ast_node!(
    /// `break;`.
    BreakStmt,
    BREAK_STMT
);
ast_node!(
    /// `continue;`.
    ContinueStmt,
    CONTINUE_STMT
);
ast_node!(
    /// `return [value];`.
    ReturnStmt,
    RETURN_STMT
);

ast_enum!(
    /// A statement.
    Stmt {
        Block(Block),
        Local(LocalDecl),
        Struct(StructDecl),
        Expr(ExprStmt),
        Empty(EmptyStmt),
        If(IfStmt),
        While(WhileStmt),
        Do(DoStmt),
        For(ForStmt),
        Switch(SwitchStmt),
        Case(CaseLabel),
        Default(DefaultLabel),
        Break(BreakStmt),
        Continue(ContinueStmt),
        Return(ReturnStmt),
    }
);

impl Block {
    pub fn statements(&self) -> impl Iterator<Item = Stmt> + use<> {
        children(&self.0)
    }
}

impl ExprStmt {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl IfStmt {
    pub fn condition(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn then_branch(&self) -> Option<Stmt> {
        child(&self.0)
    }

    pub fn else_branch(&self) -> Option<Stmt> {
        child_after(&self.0, ELSE_KW)
    }
}

impl WhileStmt {
    pub fn condition(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn body(&self) -> Option<Stmt> {
        child(&self.0)
    }
}

impl DoStmt {
    pub fn body(&self) -> Option<Stmt> {
        child(&self.0)
    }

    pub fn condition(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl ForStmt {
    pub fn init(&self) -> Option<ForInit> {
        child(&self.0)
    }

    pub fn condition(&self) -> Option<Expr> {
        self.0
            .children()
            .find(|n| n.kind() == FOR_COND)
            .and_then(|n| child(&n))
    }

    /// The step expressions, in order.
    pub fn step(&self) -> Vec<Expr> {
        self.0
            .children()
            .find(|n| n.kind() == FOR_STEP)
            .map(|n| children(&n).collect())
            .unwrap_or_default()
    }

    pub fn body(&self) -> Option<Stmt> {
        child(&self.0)
    }
}

impl ForInit {
    /// The declaration, when the initialiser is one.
    pub fn decl(&self) -> Option<LocalDecl> {
        child(&self.0)
    }

    /// The expressions, when the initialiser is not a declaration.
    pub fn exprs(&self) -> impl Iterator<Item = Expr> + use<> {
        children(&self.0)
    }
}

impl SwitchStmt {
    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn body(&self) -> Option<Stmt> {
        child(&self.0)
    }
}

impl CaseLabel {
    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl ReturnStmt {
    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }
}

// ---- expressions ---------------------------------------------------------

ast_node!(
    /// An integer, float or string literal, `OBJECT_SELF` or
    /// `OBJECT_INVALID`.
    Literal,
    LITERAL
);
ast_node!(
    /// A name used as a value.
    NameRef,
    NAME_REF
);
ast_node!(
    /// `( expression )`.
    ParenExpr,
    PAREN_EXPR
);
ast_node!(
    /// `name ( arguments )`.
    CallExpr,
    CALL_EXPR
);
ast_node!(
    /// `( arguments )` of a call.
    ArgList,
    ARG_LIST
);
ast_node!(
    /// `base . field`.
    MemberExpr,
    MEMBER_EXPR
);
ast_node!(
    /// `operand ++` or `operand --`.
    PostfixExpr,
    POSTFIX_EXPR
);
ast_node!(
    /// `- + ! ~ ++ --` applied to an operand.
    PrefixExpr,
    PREFIX_EXPR
);
ast_node!(
    /// `lhs op rhs` for the binary operators.
    BinExpr,
    BIN_EXPR
);
ast_node!(
    /// `target op value` for `=` and the compound assignments.
    AssignExpr,
    ASSIGN_EXPR
);
ast_node!(
    /// `condition ? then : otherwise`.
    CondExpr,
    COND_EXPR
);
ast_node!(
    /// `[x, y, z]`. The element count is not checked by the parser.
    VectorExpr,
    VECTOR_EXPR
);

ast_enum!(
    /// An expression.
    Expr {
        Literal(Literal),
        Name(NameRef),
        Paren(ParenExpr),
        Call(CallExpr),
        Member(MemberExpr),
        Postfix(PostfixExpr),
        Prefix(PrefixExpr),
        Binary(BinExpr),
        Assign(AssignExpr),
        Cond(CondExpr),
        Vector(VectorExpr),
    }
);

/// A literal's value.
#[derive(Clone, Debug, PartialEq)]
pub enum LiteralValue {
    /// An integer literal's value. Literals carry no sign; values above
    /// `i32::MAX` are kept for name resolution to judge.
    Int(i64),
    Float(f32),
    /// The bytes between the quotes, escapes not interpreted.
    String(Vec<u8>),
    ObjectSelf,
    ObjectInvalid,
}

impl Literal {
    pub fn token(&self) -> Option<SyntaxToken> {
        first_token(&self.0)
    }

    /// The value; `None` for a malformed number (already diagnosed) or an
    /// integer beyond `i64`.
    pub fn value(&self) -> Option<LiteralValue> {
        let t = self.token()?;
        let text = t.text();
        Some(match t.kind() {
            INT_NUMBER => LiteralValue::Int(parse_int(text)?),
            FLOAT_NUMBER => {
                if !crate::lexer::valid_float(text.as_bytes()) {
                    return None;
                }
                LiteralValue::Float(text.trim_end_matches('f').parse().ok()?)
            }
            STRING => LiteralValue::String(latin1_bytes(string_contents(text))),
            OBJECT_SELF_KW => LiteralValue::ObjectSelf,
            OBJECT_INVALID_KW => LiteralValue::ObjectInvalid,
            _ => return None,
        })
    }
}

fn parse_int(text: &str) -> Option<i64> {
    if !crate::lexer::valid_int(text.as_bytes()) {
        return None;
    }
    match text.strip_prefix("0x") {
        Some(h) => i64::from_str_radix(h, 16).ok(),
        None => text.parse().ok(),
    }
}

/// A string token's text without its quotes; an unterminated one has only
/// the opening quote.
fn string_contents(text: &str) -> &str {
    let t = text.strip_prefix('"').unwrap_or(text);
    if t.ends_with('"') && !t.is_empty() {
        &t[..t.len() - 1]
    } else {
        t
    }
}

impl NameRef {
    pub fn ident(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }

    pub fn name(&self) -> String {
        self.ident()
            .map(|t| t.text().to_string())
            .unwrap_or_default()
    }
}

impl ParenExpr {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl CallExpr {
    pub fn callee(&self) -> Option<NameRef> {
        child(&self.0)
    }

    /// The called function's name token.
    pub fn name(&self) -> Option<SyntaxToken> {
        self.callee()?.ident()
    }

    pub fn arg_list(&self) -> Option<ArgList> {
        child(&self.0)
    }

    pub fn args(&self) -> impl Iterator<Item = Expr> + use<> {
        self.arg_list()
            .into_iter()
            .flat_map(|l| children::<Expr>(&l.0))
    }
}

impl ArgList {
    pub fn args(&self) -> impl Iterator<Item = Expr> + use<> {
        children(&self.0)
    }
}

impl MemberExpr {
    pub fn base(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn field(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }
}

/// `++` or `--`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IncDec {
    Inc,
    Dec,
}

impl PostfixExpr {
    pub fn operand(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn op_token(&self) -> Option<SyntaxToken> {
        token(&self.0, PLUS2).or_else(|| token(&self.0, MINUS2))
    }

    pub fn op(&self) -> Option<IncDec> {
        Some(match self.op_token()?.kind() {
            PLUS2 => IncDec::Inc,
            _ => IncDec::Dec,
        })
    }
}

/// A prefix operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrefixOp {
    /// `-`
    Neg,
    /// `+`
    Plus,
    /// `!`
    Not,
    /// `~`
    BitNot,
    /// `++`
    Inc,
    /// `--`
    Dec,
}

impl PrefixExpr {
    pub fn op_token(&self) -> Option<SyntaxToken> {
        first_token(&self.0)
    }

    pub fn op(&self) -> Option<PrefixOp> {
        Some(match self.op_token()?.kind() {
            MINUS => PrefixOp::Neg,
            PLUS => PrefixOp::Plus,
            BANG => PrefixOp::Not,
            TILDE => PrefixOp::BitNot,
            PLUS2 => PrefixOp::Inc,
            MINUS2 => PrefixOp::Dec,
            _ => return None,
        })
    }

    pub fn operand(&self) -> Option<Expr> {
        child(&self.0)
    }
}

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    /// `||`
    LogOr,
    /// `&&`
    LogAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `&`
    BitAnd,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `>>>`
    UShr,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
}

impl BinaryOp {
    fn from_kind(k: SyntaxKind) -> Option<BinaryOp> {
        use BinaryOp::*;
        Some(match k {
            PIPE2 => LogOr,
            AMP2 => LogAnd,
            PIPE => BitOr,
            CARET => BitXor,
            AMP => BitAnd,
            EQ2 => Eq,
            NEQ => Ne,
            LT => Lt,
            GT => Gt,
            LTEQ => Le,
            GTEQ => Ge,
            SHL => Shl,
            SHR => Shr,
            USHR => UShr,
            PLUS => Add,
            MINUS => Sub,
            STAR => Mul,
            SLASH => Div,
            PERCENT => Mod,
            _ => return None,
        })
    }
}

/// The operator token directly in `node` that `f` recognises.
fn op_token_of<T>(node: &SyntaxNode, f: impl Fn(SyntaxKind) -> Option<T>) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| f(t.kind()).is_some())
}

impl BinExpr {
    pub fn lhs(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn op_token(&self) -> Option<SyntaxToken> {
        op_token_of(&self.0, BinaryOp::from_kind)
    }

    pub fn op(&self) -> Option<BinaryOp> {
        BinaryOp::from_kind(self.op_token()?.kind())
    }

    pub fn rhs(&self) -> Option<Expr> {
        let op = self.op_token()?.kind();
        child_after(&self.0, op)
    }
}

/// `=` or a compound assignment operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssignOp {
    /// `=`
    Assign,
    /// `+=`
    Add,
    /// `-=`
    Sub,
    /// `*=`
    Mul,
    /// `/=`
    Div,
    /// `%=`
    Mod,
    /// `&=`
    BitAnd,
    /// `|=`
    BitOr,
    /// `^=`
    BitXor,
    /// `<<=`
    Shl,
    /// `>>=`
    Shr,
    /// `>>>=`
    UShr,
}

impl AssignOp {
    fn from_kind(k: SyntaxKind) -> Option<AssignOp> {
        use AssignOp::*;
        Some(match k {
            EQ => Assign,
            PLUS_EQ => Add,
            MINUS_EQ => Sub,
            STAR_EQ => Mul,
            SLASH_EQ => Div,
            PERCENT_EQ => Mod,
            AMP_EQ => BitAnd,
            PIPE_EQ => BitOr,
            CARET_EQ => BitXor,
            SHL_EQ => Shl,
            SHR_EQ => Shr,
            USHR_EQ => UShr,
            _ => return None,
        })
    }

    /// The binary operator a compound assignment applies; `None` for `=`.
    pub fn binary_op(self) -> Option<BinaryOp> {
        Some(match self {
            AssignOp::Assign => return None,
            AssignOp::Add => BinaryOp::Add,
            AssignOp::Sub => BinaryOp::Sub,
            AssignOp::Mul => BinaryOp::Mul,
            AssignOp::Div => BinaryOp::Div,
            AssignOp::Mod => BinaryOp::Mod,
            AssignOp::BitAnd => BinaryOp::BitAnd,
            AssignOp::BitOr => BinaryOp::BitOr,
            AssignOp::BitXor => BinaryOp::BitXor,
            AssignOp::Shl => BinaryOp::Shl,
            AssignOp::Shr => BinaryOp::Shr,
            AssignOp::UShr => BinaryOp::UShr,
        })
    }
}

impl AssignExpr {
    pub fn target(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn op_token(&self) -> Option<SyntaxToken> {
        op_token_of(&self.0, AssignOp::from_kind)
    }

    pub fn op(&self) -> Option<AssignOp> {
        AssignOp::from_kind(self.op_token()?.kind())
    }

    pub fn value(&self) -> Option<Expr> {
        let op = self.op_token()?.kind();
        child_after(&self.0, op)
    }
}

impl CondExpr {
    pub fn condition(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn then_expr(&self) -> Option<Expr> {
        child_after(&self.0, QUESTION)
    }

    pub fn else_expr(&self) -> Option<Expr> {
        child_after(&self.0, COLON)
    }
}

impl VectorExpr {
    pub fn elements(&self) -> impl Iterator<Item = Expr> + use<> {
        children(&self.0)
    }
}
