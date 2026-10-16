//! The kinds of tokens and nodes in the syntax tree, and the rowan language
//! that ties them to it.

macro_rules! kinds {
    ($( $(#[$m:meta])* $name:ident ),* $(,)?) => {
        /// A token or node kind. Tokens come first, nodes after
        /// [`SyntaxKind::SOURCE_FILE`].
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum SyntaxKind {
            $( $(#[$m])* $name, )*
        }

        const ALL_KINDS: &[SyntaxKind] = &[$(SyntaxKind::$name),*];
    };
}

kinds! {
    // Trivia.
    /// A run of spaces and tabs.
    WHITESPACE,
    /// One line end: CR LF, LF or CR.
    NEWLINE,
    LINE_COMMENT,
    BLOCK_COMMENT,
    /// Bytes the lexer could not read as any token; it reported them.
    UNKNOWN,

    // Literals and names.
    IDENT,
    INT_NUMBER,
    FLOAT_NUMBER,
    STRING,

    // Keywords.
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
    OBJECT_SELF_KW,
    OBJECT_INVALID_KW,
    /// `include` after `#`; elsewhere it is an identifier.
    INCLUDE_KW,
    /// `define` after `#`; elsewhere it is an identifier.
    DEFINE_KW,

    // Punctuators.
    L_CURLY,
    R_CURLY,
    L_PAREN,
    R_PAREN,
    L_BRACK,
    R_BRACK,
    SEMICOLON,
    COMMA,
    DOT,
    COLON,
    QUESTION,
    HASH,
    EQ,
    PLUS_EQ,
    MINUS_EQ,
    STAR_EQ,
    SLASH_EQ,
    PERCENT_EQ,
    AMP_EQ,
    PIPE_EQ,
    CARET_EQ,
    SHL_EQ,
    SHR_EQ,
    USHR_EQ,
    PIPE2,
    AMP2,
    PIPE,
    CARET,
    AMP,
    EQ2,
    NEQ,
    LT,
    GT,
    LTEQ,
    GTEQ,
    SHL,
    SHR,
    USHR,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    PERCENT,
    BANG,
    TILDE,
    PLUS2,
    MINUS2,
    /// End of input; never in a tree.
    EOF,

    // Nodes.
    SOURCE_FILE,
    INCLUDE,
    DEFINE,
    /// The rest of a `#define` line after its name.
    DEFINE_BODY,
    FUNCTION_DECL,
    PARAM_LIST,
    PARAM,
    GLOBAL_DECL,
    LOCAL_DECL,
    DECLARATOR,
    STRUCT_DECL,
    FIELD_DECL,
    /// A `;` at top level.
    EMPTY_DECL,
    TYPE,
    BLOCK,
    EXPR_STMT,
    EMPTY_STMT,
    IF_STMT,
    WHILE_STMT,
    DO_STMT,
    FOR_STMT,
    FOR_INIT,
    FOR_COND,
    FOR_STEP,
    SWITCH_STMT,
    CASE_LABEL,
    DEFAULT_LABEL,
    BREAK_STMT,
    CONTINUE_STMT,
    RETURN_STMT,
    LITERAL,
    NAME_REF,
    PAREN_EXPR,
    CALL_EXPR,
    ARG_LIST,
    MEMBER_EXPR,
    POSTFIX_EXPR,
    PREFIX_EXPR,
    BIN_EXPR,
    ASSIGN_EXPR,
    COND_EXPR,
    VECTOR_EXPR,
    /// Tokens the parser skipped while recovering from an error.
    ERROR,
}

use SyntaxKind::*;

impl SyntaxKind {
    /// Whitespace, line ends, comments and unreadable bytes: kept in the
    /// tree, skipped by the parser.
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            WHITESPACE | NEWLINE | LINE_COMMENT | BLOCK_COMMENT | UNKNOWN
        )
    }

    pub fn is_keyword(self) -> bool {
        (IF_KW as u16..=DEFINE_KW as u16).contains(&(self as u16))
    }

    pub fn is_punct(self) -> bool {
        (L_CURLY as u16..=MINUS2 as u16).contains(&(self as u16))
    }

    /// The keyword spelled exactly `text`, if any. `include` and `define`
    /// are keywords only after `#`, so they are not here.
    pub fn from_keyword(text: &[u8]) -> Option<SyntaxKind> {
        Some(match text {
            b"if" => IF_KW,
            b"else" => ELSE_KW,
            b"while" => WHILE_KW,
            b"do" => DO_KW,
            b"for" => FOR_KW,
            b"switch" => SWITCH_KW,
            b"case" => CASE_KW,
            b"default" => DEFAULT_KW,
            b"break" => BREAK_KW,
            b"continue" => CONTINUE_KW,
            b"return" => RETURN_KW,
            b"struct" => STRUCT_KW,
            b"const" => CONST_KW,
            b"void" => VOID_KW,
            b"int" => INT_KW,
            b"float" => FLOAT_KW,
            b"string" => STRING_KW,
            b"object" => OBJECT_KW,
            b"vector" => VECTOR_KW,
            b"action" => ACTION_KW,
            b"OBJECT_SELF" => OBJECT_SELF_KW,
            b"OBJECT_INVALID" => OBJECT_INVALID_KW,
            _ => return None,
        })
    }

    /// The built-in type keywords. Engine structure types such as `effect`
    /// are identifiers, named by `nwscript.nss`'s `#define` lines.
    pub fn is_type_keyword(self) -> bool {
        matches!(
            self,
            VOID_KW | INT_KW | FLOAT_KW | STRING_KW | OBJECT_KW | VECTOR_KW | ACTION_KW
        )
    }

    /// How the kind is written in a message: the token's text for keywords
    /// and punctuators, a description otherwise.
    pub fn describe(self) -> &'static str {
        match self {
            WHITESPACE => "whitespace",
            NEWLINE => "line end",
            LINE_COMMENT | BLOCK_COMMENT => "comment",
            UNKNOWN => "unknown character",
            IDENT => "identifier",
            INT_NUMBER => "integer literal",
            FLOAT_NUMBER => "float literal",
            STRING => "string literal",
            IF_KW => "`if`",
            ELSE_KW => "`else`",
            WHILE_KW => "`while`",
            DO_KW => "`do`",
            FOR_KW => "`for`",
            SWITCH_KW => "`switch`",
            CASE_KW => "`case`",
            DEFAULT_KW => "`default`",
            BREAK_KW => "`break`",
            CONTINUE_KW => "`continue`",
            RETURN_KW => "`return`",
            STRUCT_KW => "`struct`",
            CONST_KW => "`const`",
            VOID_KW => "`void`",
            INT_KW => "`int`",
            FLOAT_KW => "`float`",
            STRING_KW => "`string`",
            OBJECT_KW => "`object`",
            VECTOR_KW => "`vector`",
            ACTION_KW => "`action`",
            OBJECT_SELF_KW => "`OBJECT_SELF`",
            OBJECT_INVALID_KW => "`OBJECT_INVALID`",
            INCLUDE_KW => "`include`",
            DEFINE_KW => "`define`",
            L_CURLY => "`{`",
            R_CURLY => "`}`",
            L_PAREN => "`(`",
            R_PAREN => "`)`",
            L_BRACK => "`[`",
            R_BRACK => "`]`",
            SEMICOLON => "`;`",
            COMMA => "`,`",
            DOT => "`.`",
            COLON => "`:`",
            QUESTION => "`?`",
            HASH => "`#`",
            EQ => "`=`",
            PLUS_EQ => "`+=`",
            MINUS_EQ => "`-=`",
            STAR_EQ => "`*=`",
            SLASH_EQ => "`/=`",
            PERCENT_EQ => "`%=`",
            AMP_EQ => "`&=`",
            PIPE_EQ => "`|=`",
            CARET_EQ => "`^=`",
            SHL_EQ => "`<<=`",
            SHR_EQ => "`>>=`",
            USHR_EQ => "`>>>=`",
            PIPE2 => "`||`",
            AMP2 => "`&&`",
            PIPE => "`|`",
            CARET => "`^`",
            AMP => "`&`",
            EQ2 => "`==`",
            NEQ => "`!=`",
            LT => "`<`",
            GT => "`>`",
            LTEQ => "`<=`",
            GTEQ => "`>=`",
            SHL => "`<<`",
            SHR => "`>>`",
            USHR => "`>>>`",
            PLUS => "`+`",
            MINUS => "`-`",
            STAR => "`*`",
            SLASH => "`/`",
            PERCENT => "`%`",
            BANG => "`!`",
            TILDE => "`~`",
            PLUS2 => "`++`",
            MINUS2 => "`--`",
            EOF => "end of file",
            _ => "syntax node",
        }
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        rowan::SyntaxKind(kind as u16)
    }
}

/// NWScript as a rowan language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nwscript {}

impl rowan::Language for Nwscript {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        ALL_KINDS[raw.0 as usize]
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<Nwscript>;
pub type SyntaxToken = rowan::SyntaxToken<Nwscript>;
pub type SyntaxElement = rowan::SyntaxElement<Nwscript>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_round_trip() {
        for (i, k) in ALL_KINDS.iter().enumerate() {
            assert_eq!(*k as usize, i);
        }
    }
}
