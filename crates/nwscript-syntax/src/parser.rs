//! The recursive-descent parser: tokens to a lossless green tree.
//!
//! Trivia is attached as the parser goes: whitespace and comments before a
//! token land in whichever node is open when that token is consumed, and
//! trivia before a node's first token stays in the parent. Every token the
//! lexer produced ends up in the tree, so the tree prints back to the source.
//!
//! Recovery: a missing token is reported and not invented; a token that
//! starts nothing where something was required is wrapped in an
//! [`ERROR`] node and skipped, unless it is one the enclosing construct can
//! resume at (`;`, `}`, a closing bracket, a keyword that starts a statement
//! or declaration). At most one error is reported per token position, so
//! one mistake yields one diagnostic.
//!
//! Declarations are recognised by this rule: at the start of a statement, a
//! built-in type keyword, `struct`, `const`, or an identifier followed by
//! another identifier begins a declaration. At top level everything but a
//! directive or `;` is a declaration, so its first identifier is a type.
//! Engine structure types such as `effect` are therefore ordinary
//! identifiers to the parser, and whether a type name exists is decided by
//! name resolution, not here.

use crate::SyntaxKind::{self, *};
use crate::lexer::Token;
use crate::text::latin1_str;
use nwscript_diag::{Diagnostic, FileId, Span, codes};
use rowan::{Checkpoint, GreenNode, GreenNodeBuilder};

pub(crate) fn parse_tokens(
    file: FileId,
    src: &[u8],
    tokens: &[Token],
    lex_errors: Vec<usize>,
) -> (GreenNode, Vec<Diagnostic>) {
    let sig: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.kind.is_trivia())
        .map(|(i, _)| i)
        .collect();
    let mut p = Parser {
        file,
        src,
        tokens,
        sig,
        raw: 0,
        cur: 0,
        builder: GreenNodeBuilder::new(),
        diags: Vec::new(),
        last_error: None,
        lex_errors,
    };
    p.source_file();
    (p.builder.finish(), p.diags)
}

/// Tokens a construct can resume at after an error inside it, so they are
/// never swallowed by recovery.
const RECOVERY: &[SyntaxKind] = &[
    SEMICOLON,
    R_CURLY,
    L_CURLY,
    R_PAREN,
    R_BRACK,
    COMMA,
    COLON,
    HASH,
    IF_KW,
    ELSE_KW,
    WHILE_KW,
    DO_KW,
    FOR_KW,
    SWITCH_KW,
    CASE_KW,
    DEFAULT_KW,
    BREAK_KW,
    CONTINUE_KW,
    RETURN_KW,
    STRUCT_KW,
    CONST_KW,
    VOID_KW,
    INT_KW,
    FLOAT_KW,
    STRING_KW,
    OBJECT_KW,
    VECTOR_KW,
    ACTION_KW,
    EOF,
];

struct Parser<'a> {
    file: FileId,
    src: &'a [u8],
    tokens: &'a [Token],
    /// Indices into `tokens` of the non-trivia tokens.
    sig: Vec<usize>,
    /// The next token of `tokens` not yet in the tree.
    raw: usize,
    /// The current non-trivia token, as an index into `sig`.
    cur: usize,
    builder: GreenNodeBuilder<'static>,
    diags: Vec<Diagnostic>,
    /// The value of `cur` when the last error was reported.
    last_error: Option<usize>,
    /// Start offsets of the lexer's diagnostics.
    lex_errors: Vec<usize>,
}

impl Parser<'_> {
    // ---- the token cursor -------------------------------------------------

    fn nth(&self, n: usize) -> SyntaxKind {
        self.sig
            .get(self.cur + n)
            .map_or(EOF, |&i| self.tokens[i].kind)
    }

    fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        kinds.contains(&self.current())
    }

    fn current_text(&self) -> &[u8] {
        match self.sig.get(self.cur) {
            Some(&i) => &self.src[self.tokens[i].range()],
            None => b"",
        }
    }

    /// The current token's byte span, or an empty span at the end of input.
    fn current_span(&self) -> Span {
        match self.sig.get(self.cur) {
            Some(&i) => {
                let t = self.tokens[i];
                Span::new(self.file, t.start as usize, t.end() as usize)
            }
            None => Span::new(self.file, self.src.len(), self.src.len()),
        }
    }

    /// Put trivia up to the current token into the open node.
    fn flush_trivia(&mut self) {
        let upto = self.sig.get(self.cur).copied().unwrap_or(self.tokens.len());
        while self.raw < upto {
            self.emit(self.raw, None);
            self.raw += 1;
        }
    }

    fn emit(&mut self, index: usize, remap: Option<SyntaxKind>) {
        let t = self.tokens[index];
        let text = latin1_str(&self.src[t.range()]);
        self.builder
            .token(remap.unwrap_or(t.kind).into(), text.as_ref());
    }

    fn bump(&mut self) {
        self.bump_as(None);
    }

    fn bump_as(&mut self, remap: Option<SyntaxKind>) {
        if self.cur >= self.sig.len() {
            return;
        }
        self.flush_trivia();
        self.emit(self.raw, remap);
        self.raw += 1;
        self.cur += 1;
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn start(&mut self, kind: SyntaxKind) {
        self.flush_trivia();
        self.builder.start_node(kind.into());
    }

    fn finish(&mut self) {
        self.builder.finish_node();
    }

    fn checkpoint(&mut self) -> Checkpoint {
        self.flush_trivia();
        self.builder.checkpoint()
    }

    fn start_at(&mut self, cp: Checkpoint, kind: SyntaxKind) {
        self.builder.start_node_at(cp, kind.into());
    }

    /// Whether a line end (or the start of the file) comes between the
    /// previous token and the current one.
    fn line_break_before(&self) -> bool {
        let to = self.sig.get(self.cur).copied().unwrap_or(self.tokens.len());
        let from = if self.cur == 0 {
            return true;
        } else {
            self.sig[self.cur - 1] + 1
        };
        self.tokens[from..to].iter().any(|t| t.kind == NEWLINE)
    }

    /// Whether only spaces and tabs come between the current token and the
    /// start of its line.
    fn at_line_start(&self) -> bool {
        let to = self.sig.get(self.cur).copied().unwrap_or(self.tokens.len());
        for t in self.tokens[..to].iter().rev() {
            match t.kind {
                WHITESPACE => continue,
                NEWLINE => return true,
                _ => return false,
            }
        }
        true
    }

    // ---- errors ----------------------------------------------------------

    /// Report at the current token, once per token position.
    fn error_with(&mut self, d: impl FnOnce(Span) -> Diagnostic) {
        if self.last_error == Some(self.cur) || self.after_lex_error() {
            return;
        }
        self.last_error = Some(self.cur);
        let span = self.current_span();
        self.diags.push(d(span));
    }

    /// Whether the lexer reported an error in the previous token or the
    /// trivia after it: the parser's complaint about what follows would be
    /// a consequence of that one (an unterminated string that took the
    /// closing `)` with it, a stray byte where an operand was).
    fn after_lex_error(&self) -> bool {
        let from = match self.cur.checked_sub(1) {
            Some(prev) => self.tokens[self.sig[prev]].start as usize,
            None => 0,
        };
        let to = self.current_span().start;
        self.lex_errors
            .iter()
            .any(|&s| s >= from && (s < to || (s == to && self.at(EOF))))
    }

    fn found(&self) -> String {
        match self.current() {
            IDENT => format!("identifier `{}`", latin1_str(self.current_text()).as_ref()),
            k => k.describe().to_string(),
        }
    }

    fn error_expected(&mut self, what: &str) {
        let msg = format!("expected {what}, found {}", self.found());
        let label = format!("expected {what}");
        self.error_with(|span| Diagnostic::new(&codes::E0205, msg).primary(span, label));
    }

    fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.error_expected(kind.describe());
        false
    }

    /// Report and, unless the token is one to resume at, skip it.
    fn error_recover(&mut self, what: &str) {
        self.error_expected(what);
        if !self.at_any(RECOVERY) {
            self.start(ERROR);
            self.bump();
            self.finish();
        }
    }

    /// Report and skip the current token whatever it is; for loops that
    /// must make progress.
    fn error_bump(&mut self, what: &str) {
        self.error_expected(what);
        self.start(ERROR);
        self.bump();
        self.finish();
    }

    // ---- top level ------------------------------------------------------

    fn source_file(&mut self) {
        self.builder.start_node(SOURCE_FILE.into());
        while !self.at(EOF) {
            self.top_level();
        }
        self.flush_trivia();
        self.finish();
    }

    fn top_level(&mut self) {
        match self.current() {
            HASH => self.directive(),
            SEMICOLON => {
                self.start(EMPTY_DECL);
                self.bump();
                self.finish();
            }
            STRUCT_KW if self.nth(1) == IDENT && self.nth(2) == L_CURLY => self.struct_decl(),
            CONST_KW => {
                self.start(GLOBAL_DECL);
                self.bump();
                self.ty();
                self.declarators();
                self.expect(SEMICOLON);
                self.finish();
            }
            _ if self.at_type_start() => {
                let cp = self.checkpoint();
                self.ty();
                if self.at(IDENT) && self.nth(1) == L_PAREN {
                    self.start_at(cp, FUNCTION_DECL);
                    self.bump();
                    self.function_rest();
                } else {
                    self.start_at(cp, GLOBAL_DECL);
                    self.declarators();
                    self.expect(SEMICOLON);
                }
                self.finish();
            }
            _ => self.error_bump("a declaration"),
        }
    }

    fn at_type_start(&self) -> bool {
        let k = self.current();
        k.is_type_keyword() || k == IDENT || k == STRUCT_KW
    }

    /// A function's parameters and its body or `;`, after its name.
    fn function_rest(&mut self) {
        self.start(PARAM_LIST);
        self.bump(); // (
        if self.at(VOID_KW) && self.nth(1) == R_PAREN {
            self.bump();
        } else {
            if !self.at(R_PAREN) {
                loop {
                    if !(self.at_type_start() || self.at(CONST_KW)) {
                        self.error_recover("a parameter");
                        break;
                    }
                    self.param();
                    if !self.eat(COMMA) {
                        break;
                    }
                }
            }
        }
        self.expect(R_PAREN);
        self.finish();
        match self.current() {
            L_CURLY => self.block(),
            SEMICOLON => self.bump(),
            _ => self.error_expected("`{` or `;`"),
        }
    }

    fn param(&mut self) {
        self.start(PARAM);
        self.eat(CONST_KW);
        self.ty();
        self.expect(IDENT);
        if self.eat(EQ) {
            self.expr();
        }
        self.finish();
    }

    fn ty(&mut self) {
        self.start(TYPE);
        match self.current() {
            k if k.is_type_keyword() => self.bump(),
            IDENT => self.bump(),
            STRUCT_KW => {
                self.bump();
                self.expect(IDENT);
            }
            _ => self.error_expected("a type"),
        }
        self.finish();
    }

    /// `declarator { "," declarator }`.
    fn declarators(&mut self) {
        loop {
            self.start(DECLARATOR);
            self.expect(IDENT);
            if self.eat(EQ) {
                self.expr();
            }
            self.finish();
            if !self.eat(COMMA) {
                break;
            }
        }
    }

    fn struct_decl(&mut self) {
        self.start(STRUCT_DECL);
        self.bump(); // struct
        self.bump(); // name
        self.bump(); // {
        while !self.at(R_CURLY) && !self.at(EOF) && !self.at_function_start() {
            if self.at_type_start() || self.at(CONST_KW) {
                self.start(FIELD_DECL);
                self.eat(CONST_KW);
                self.ty();
                self.declarators();
                self.expect(SEMICOLON);
                self.finish();
            } else {
                self.error_bump("a member declaration");
            }
        }
        self.expect(R_CURLY);
        self.expect(SEMICOLON);
        self.finish();
    }

    // ---- directives -----------------------------------------------------

    fn directive(&mut self) {
        let at_line_start = self.at_line_start();
        let hash = self.current_span();
        let kind = match self.nth(1) {
            IDENT => match self.text_at(1) {
                b"include" => Some(INCLUDE),
                b"define" => Some(DEFINE),
                _ => None,
            },
            _ => None,
        };
        let Some(kind) = kind else {
            self.start(ERROR);
            self.bump();
            let span = self.current_span();
            self.report_directive(
                hash.to(span),
                "unknown directive",
                "only `#include` and `#define` exist",
            );
            self.rest_of_line();
            self.finish();
            return;
        };
        if !at_line_start {
            self.report_directive(
                hash,
                "a directive must begin its line",
                "start a new line before the `#`",
            );
        }
        self.start(kind);
        self.bump(); // #
        if kind == INCLUDE {
            self.bump_as(Some(INCLUDE_KW));
            if self.at(STRING) && !self.line_break_before() {
                self.bump();
            } else {
                self.error_expected("a quoted file name");
            }
            if !self.at(EOF) && !self.line_break_before() {
                self.error_expected("a line end");
                self.start(ERROR);
                self.rest_of_line();
                self.finish();
            }
        } else {
            self.bump_as(Some(DEFINE_KW));
            if self.at(IDENT) && !self.line_break_before() {
                self.bump();
            } else {
                self.error_expected("a name");
            }
            if !self.at(EOF) && !self.line_break_before() {
                self.start(DEFINE_BODY);
                self.rest_of_line();
                self.finish();
            }
        }
        self.finish();
    }

    fn text_at(&self, n: usize) -> &[u8] {
        match self.sig.get(self.cur + n) {
            Some(&i) => &self.src[self.tokens[i].range()],
            None => b"",
        }
    }

    /// Consume tokens up to the end of the current line.
    fn rest_of_line(&mut self) {
        while !self.at(EOF) && !self.line_break_before() {
            self.bump();
        }
    }

    fn report_directive(&mut self, span: Span, msg: &str, help: &str) {
        if self.last_error == Some(self.cur) {
            return;
        }
        self.last_error = Some(self.cur);
        self.diags.push(
            Diagnostic::new(&codes::E0206, msg)
                .primary(span, "")
                .help(help),
        );
    }

    // ---- statements -----------------------------------------------------

    fn block(&mut self) {
        self.start(BLOCK);
        self.bump(); // {
        while !self.at(R_CURLY) && !self.at(EOF) {
            if self.at_function_start() {
                break;
            }
            self.statement();
        }
        self.expect(R_CURLY);
        self.finish();
    }

    /// `type name (`: a function, which cannot begin a statement; a block
    /// that meets one is missing its `}`.
    fn at_function_start(&self) -> bool {
        let k = self.current();
        let after_type = if k.is_type_keyword() || k == IDENT {
            1
        } else if k == STRUCT_KW && self.nth(1) == IDENT {
            2
        } else {
            return false;
        };
        self.nth(after_type) == IDENT && self.nth(after_type + 1) == L_PAREN
    }

    fn at_local_decl(&self) -> bool {
        match self.current() {
            k if k.is_type_keyword() => true,
            CONST_KW | STRUCT_KW => true,
            IDENT => self.nth(1) == IDENT,
            _ => false,
        }
    }

    fn statement(&mut self) {
        match self.current() {
            L_CURLY => self.block(),
            SEMICOLON => {
                self.start(EMPTY_STMT);
                self.bump();
                self.finish();
            }
            HASH => {
                self.report_directive(
                    self.current_span(),
                    "a directive must be at top level",
                    "move it before the declarations",
                );
                self.directive();
            }
            STRUCT_KW if self.nth(1) == IDENT && self.nth(2) == L_CURLY => self.struct_decl(),
            _ if self.at_local_decl() => {
                self.local_decl_body();
                self.expect(SEMICOLON);
                self.finish();
            }
            IF_KW => {
                self.start(IF_STMT);
                self.bump();
                self.paren_condition();
                self.body();
                if self.eat(ELSE_KW) {
                    self.body();
                }
                self.finish();
            }
            WHILE_KW => {
                self.start(WHILE_STMT);
                self.bump();
                self.paren_condition();
                self.body();
                self.finish();
            }
            DO_KW => {
                self.start(DO_STMT);
                self.bump();
                self.body();
                self.expect(WHILE_KW);
                self.paren_condition();
                self.expect(SEMICOLON);
                self.finish();
            }
            FOR_KW => self.for_stmt(),
            SWITCH_KW => {
                self.start(SWITCH_STMT);
                self.bump();
                self.paren_condition();
                self.body();
                self.finish();
            }
            CASE_KW => {
                self.start(CASE_LABEL);
                self.bump();
                self.expr();
                self.expect(COLON);
                self.finish();
            }
            DEFAULT_KW => {
                self.start(DEFAULT_LABEL);
                self.bump();
                self.expect(COLON);
                self.finish();
            }
            BREAK_KW | CONTINUE_KW => {
                self.start(if self.at(BREAK_KW) {
                    BREAK_STMT
                } else {
                    CONTINUE_STMT
                });
                self.bump();
                self.expect(SEMICOLON);
                self.finish();
            }
            RETURN_KW => {
                self.start(RETURN_STMT);
                self.bump();
                if !self.at(SEMICOLON) && self.at_expr_start() {
                    self.expr();
                }
                self.expect(SEMICOLON);
                self.finish();
            }
            _ if self.at_expr_start() => {
                self.start(EXPR_STMT);
                self.expr();
                self.expect(SEMICOLON);
                self.finish();
            }
            _ => self.error_bump("a statement"),
        }
    }

    /// `[const] type declarator {, declarator}`, left open in a
    /// [`LOCAL_DECL`] node for the caller to finish.
    fn local_decl_body(&mut self) {
        self.start(LOCAL_DECL);
        self.eat(CONST_KW);
        self.ty();
        self.declarators();
    }

    /// A statement as the body of `if`, `else`, a loop or `switch`.
    fn body(&mut self) {
        if self.at(EOF) || self.at(R_CURLY) || self.at_function_start() {
            self.error_expected("a statement");
        } else {
            self.statement();
        }
    }

    /// `( expression )` after `if`, `while`, `switch`.
    fn paren_condition(&mut self) {
        self.expect(L_PAREN);
        self.expr();
        self.expect(R_PAREN);
    }

    fn for_stmt(&mut self) {
        self.start(FOR_STMT);
        self.bump(); // for
        self.expect(L_PAREN);
        if !self.at(SEMICOLON) {
            self.start(FOR_INIT);
            if self.at_local_decl() {
                self.local_decl_body();
                self.finish();
            } else {
                self.comma_separated();
            }
            self.finish();
        }
        self.expect(SEMICOLON);
        if !self.at(SEMICOLON) {
            self.start(FOR_COND);
            self.expr();
            self.finish();
        }
        self.expect(SEMICOLON);
        if !self.at(R_PAREN) {
            self.start(FOR_STEP);
            self.comma_separated();
            self.finish();
        }
        self.expect(R_PAREN);
        self.body();
        self.finish();
    }

    /// `expression { "," expression }`, stopping at the first expression
    /// that is missing.
    fn comma_separated(&mut self) {
        while self.expr_bp(0) && self.eat(COMMA) {}
    }

    // ---- expressions ----------------------------------------------------

    fn at_expr_start(&self) -> bool {
        matches!(
            self.current(),
            IDENT
                | INT_NUMBER
                | FLOAT_NUMBER
                | STRING
                | OBJECT_SELF_KW
                | OBJECT_INVALID_KW
                | L_PAREN
                | L_BRACK
                | MINUS
                | PLUS
                | BANG
                | TILDE
                | PLUS2
                | MINUS2
        )
    }

    fn expr(&mut self) {
        self.expr_bp(0);
    }

    /// Pratt loop over the binary, conditional and assignment operators.
    /// Returns false when no expression was found (and that was reported).
    fn expr_bp(&mut self, min_bp: u8) -> bool {
        let cp = self.checkpoint();
        if !self.unary() {
            return false;
        }
        loop {
            let op = self.current();
            if let Some(l_bp) = assign_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                self.start_at(cp, ASSIGN_EXPR);
                self.bump();
                self.expr_bp(l_bp - 1);
                self.finish();
            } else if op == QUESTION {
                if COND_BP < min_bp {
                    break;
                }
                self.start_at(cp, COND_EXPR);
                self.bump();
                self.expr();
                self.expect(COLON);
                self.expr_bp(COND_BP - 1);
                self.finish();
            } else if let Some(level) = binary_level(op) {
                let l_bp = 2 * level;
                if l_bp < min_bp {
                    break;
                }
                self.start_at(cp, BIN_EXPR);
                self.bump();
                self.expr_bp(l_bp + 1);
                self.finish();
            } else {
                break;
            }
        }
        true
    }

    /// Prefix operators, then an operand with its postfix operators.
    fn unary(&mut self) -> bool {
        if self.at_any(&[MINUS, PLUS, BANG, TILDE, PLUS2, MINUS2]) {
            self.start(PREFIX_EXPR);
            self.bump();
            self.expr_bp(PREFIX_BP);
            self.finish();
            return true;
        }
        let cp = self.checkpoint();
        if !self.primary() {
            return false;
        }
        loop {
            match self.current() {
                DOT => {
                    self.start_at(cp, MEMBER_EXPR);
                    self.bump();
                    self.expect(IDENT);
                    self.finish();
                }
                PLUS2 | MINUS2 => {
                    self.start_at(cp, POSTFIX_EXPR);
                    self.bump();
                    self.finish();
                }
                _ => break,
            }
        }
        true
    }

    fn primary(&mut self) -> bool {
        match self.current() {
            INT_NUMBER | FLOAT_NUMBER | STRING | OBJECT_SELF_KW | OBJECT_INVALID_KW => {
                self.start(LITERAL);
                self.bump();
                self.finish();
            }
            IDENT if self.nth(1) == L_PAREN => {
                self.start(CALL_EXPR);
                self.start(NAME_REF);
                self.bump();
                self.finish();
                self.start(ARG_LIST);
                self.bump(); // (
                if !self.at(R_PAREN) {
                    self.comma_separated();
                }
                self.expect(R_PAREN);
                self.finish();
                self.finish();
            }
            IDENT => {
                self.start(NAME_REF);
                self.bump();
                self.finish();
            }
            L_PAREN => {
                self.start(PAREN_EXPR);
                self.bump();
                self.expr();
                self.expect(R_PAREN);
                self.finish();
            }
            L_BRACK => {
                self.start(VECTOR_EXPR);
                self.bump();
                if !self.at(R_BRACK) {
                    self.comma_separated();
                }
                self.expect(R_BRACK);
                self.finish();
            }
            _ => {
                let before = self.cur;
                self.error_recover("an expression");
                // A skipped stray token: the expression may follow it.
                return self.cur != before && self.at_expr_start() && self.primary();
            }
        }
        true
    }
}

/// Binding power of the operand of a prefix operator: tighter than every
/// binary operator.
const PREFIX_BP: u8 = 30;
/// `?:` sits between `||` and assignment, and is right-associative.
const COND_BP: u8 = 5;

/// Left binding power of an assignment operator; right-associative, the
/// loosest level.
fn assign_bp(k: SyntaxKind) -> Option<u8> {
    matches!(
        k,
        EQ | PLUS_EQ
            | MINUS_EQ
            | STAR_EQ
            | SLASH_EQ
            | PERCENT_EQ
            | AMP_EQ
            | PIPE_EQ
            | CARET_EQ
            | SHL_EQ
            | SHR_EQ
            | USHR_EQ
    )
    .then_some(3)
}

/// The precedence level of a left-associative binary operator, loosest
/// first; `docs/grammar.md`'s table read bottom up.
fn binary_level(k: SyntaxKind) -> Option<u8> {
    Some(match k {
        PIPE2 => 3,
        AMP2 => 4,
        PIPE => 5,
        CARET => 6,
        AMP => 7,
        EQ2 | NEQ => 8,
        LT | GT | LTEQ | GTEQ => 9,
        SHL | SHR | USHR => 10,
        PLUS | MINUS => 11,
        STAR | SLASH | PERCENT => 12,
        _ => return None,
    })
}
