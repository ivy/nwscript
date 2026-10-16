//! The lexer: bytes to tokens, every byte in exactly one token.
//!
//! Source is Windows-1252 bytes, not UTF-8. Line ends are tokens of their
//! own ([`NEWLINE`]) so the parser can see where a `#define` line ends.
//! Bytes that start no token become [`UNKNOWN`] trivia with a diagnostic;
//! an unterminated string or block comment ends at its line end, so one
//! slip does not swallow the rest of the file.

use crate::SyntaxKind::{self, *};
use nwscript_diag::{Diagnostic, FileId, Span, codes};

/// A token: its kind and its byte range in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: SyntaxKind,
    pub start: u32,
    pub len: u32,
}

impl Token {
    pub fn end(&self) -> u32 {
        self.start + self.len
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end() as usize
    }
}

/// Longest first within each first byte, so the first match is the longest.
const PUNCT: &[(&[u8], SyntaxKind)] = &[
    (b">>>=", USHR_EQ),
    (b">>>", USHR),
    (b">>=", SHR_EQ),
    (b">>", SHR),
    (b">=", GTEQ),
    (b">", GT),
    (b"<<=", SHL_EQ),
    (b"<<", SHL),
    (b"<=", LTEQ),
    (b"<", LT),
    (b"++", PLUS2),
    (b"+=", PLUS_EQ),
    (b"+", PLUS),
    (b"--", MINUS2),
    (b"-=", MINUS_EQ),
    (b"-", MINUS),
    (b"&&", AMP2),
    (b"&=", AMP_EQ),
    (b"&", AMP),
    (b"||", PIPE2),
    (b"|=", PIPE_EQ),
    (b"|", PIPE),
    (b"==", EQ2),
    (b"=", EQ),
    (b"!=", NEQ),
    (b"!", BANG),
    (b"*=", STAR_EQ),
    (b"*", STAR),
    (b"/=", SLASH_EQ),
    (b"/", SLASH),
    (b"%=", PERCENT_EQ),
    (b"%", PERCENT),
    (b"^=", CARET_EQ),
    (b"^", CARET),
    (b"~", TILDE),
    (b"?", QUESTION),
    (b":", COLON),
    (b";", SEMICOLON),
    (b",", COMMA),
    (b".", DOT),
    (b"(", L_PAREN),
    (b")", R_PAREN),
    (b"{", L_CURLY),
    (b"}", R_CURLY),
    (b"[", L_BRACK),
    (b"]", R_BRACK),
    (b"#", HASH),
];

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Bytes that can begin some token or trivia.
fn starts_token(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'"')
        || is_ident_continue(b)
        || PUNCT.iter().any(|(p, _)| p[0] == b)
}

/// Split `src` into tokens. Diagnostics for unreadable input carry `file`.
pub fn lex(file: FileId, src: &[u8]) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lx = Lexer {
        src,
        file,
        pos: 0,
        tokens: Vec::new(),
        diags: Vec::new(),
    };
    lx.run();
    (lx.tokens, lx.diags)
}

struct Lexer<'a> {
    src: &'a [u8],
    file: FileId,
    pos: usize,
    tokens: Vec<Token>,
    diags: Vec<Diagnostic>,
}

impl Lexer<'_> {
    fn at(&self, i: usize) -> u8 {
        self.src.get(i).copied().unwrap_or(0)
    }

    fn push(&mut self, kind: SyntaxKind, start: usize) {
        self.tokens.push(Token {
            kind,
            start: start as u32,
            len: (self.pos - start) as u32,
        });
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.file, start, end)
    }

    /// The offset of the next CR or LF at or after `from`, or the end.
    fn line_end_from(&self, from: usize) -> usize {
        self.src[from..]
            .iter()
            .position(|&b| b == b'\r' || b == b'\n')
            .map_or(self.src.len(), |i| from + i)
    }

    fn run(&mut self) {
        while self.pos < self.src.len() {
            let start = self.pos;
            let b = self.src[start];
            let kind = match b {
                b'\r' => {
                    self.pos += if self.at(start + 1) == b'\n' { 2 } else { 1 };
                    NEWLINE
                }
                b'\n' => {
                    self.pos += 1;
                    NEWLINE
                }
                b' ' | b'\t' => {
                    while matches!(self.at(self.pos), b' ' | b'\t') && self.pos < self.src.len() {
                        self.pos += 1;
                    }
                    WHITESPACE
                }
                b'/' if self.at(start + 1) == b'/' => {
                    self.pos = self.line_end_from(start);
                    LINE_COMMENT
                }
                b'/' if self.at(start + 1) == b'*' => self.block_comment(start),
                b'"' => self.string(start),
                b'0'..=b'9' => self.number(start),
                _ if is_ident_start(b) => {
                    while self.pos < self.src.len() && is_ident_continue(self.src[self.pos]) {
                        self.pos += 1;
                    }
                    SyntaxKind::from_keyword(&self.src[start..self.pos]).unwrap_or(IDENT)
                }
                _ => {
                    let rest = &self.src[start..];
                    if let Some((p, k)) = PUNCT.iter().find(|(p, _)| rest.starts_with(p)) {
                        self.pos += p.len();
                        *k
                    } else {
                        self.unknown(start)
                    }
                }
            };
            self.push(kind, start);
        }
    }

    fn block_comment(&mut self, start: usize) -> SyntaxKind {
        let body = start + 2;
        match self.src[body..].windows(2).position(|w| w == b"*/") {
            Some(i) => self.pos = body + i + 2,
            None => {
                self.pos = self.line_end_from(body);
                self.diags.push(
                    Diagnostic::new(&codes::E0202, "unterminated block comment")
                        .primary(self.span(start, start + 2), "this `/*` has no `*/`")
                        .help("close the comment with `*/`, or use `//` for a comment that ends at the line end"),
                );
            }
        }
        BLOCK_COMMENT
    }

    fn string(&mut self, start: usize) -> SyntaxKind {
        let mut i = start + 1;
        loop {
            match self.src.get(i) {
                Some(b'"') => {
                    self.pos = i + 1;
                    return STRING;
                }
                Some(b'\\')
                    if !matches!(self.at(i + 1), b'\r' | b'\n') && i + 1 < self.src.len() =>
                {
                    i += 2
                }
                Some(b'\r' | b'\n') | None => break,
                Some(_) => i += 1,
            }
        }
        self.pos = i;
        self.diags.push(
            Diagnostic::new(&codes::E0203, "unterminated string literal")
                .primary(self.span(start, i), "this string has no closing `\"`")
                .help("a string literal ends on the line it starts; add the closing `\"`"),
        );
        STRING
    }

    /// Digits, an optional fraction, then any letters, digits and
    /// underscores run on, checked as a whole against the literal forms.
    fn number(&mut self, start: usize) -> SyntaxKind {
        let mut i = start;
        while self.at(i).is_ascii_digit() && i < self.src.len() {
            i += 1;
        }
        let mut kind = INT_NUMBER;
        if self.at(i) == b'.' && i < self.src.len() {
            kind = FLOAT_NUMBER;
            i += 1;
            while self.at(i).is_ascii_digit() && i < self.src.len() {
                i += 1;
            }
        }
        while i < self.src.len() && is_ident_continue(self.src[i]) {
            i += 1;
        }
        self.pos = i;
        let text = &self.src[start..i];
        let ok = match kind {
            INT_NUMBER => valid_int(text),
            _ => valid_float(text),
        };
        if !ok {
            self.diags.push(
                Diagnostic::new(&codes::E0204, "invalid number literal")
                    .primary(self.span(start, i), "not a number NWScript can read")
                    .help("integers are decimal like `42` or hexadecimal like `0x2a`; floats have digits on both sides of the point, like `1.0` or `1.0f`"),
            );
        }
        kind
    }

    fn unknown(&mut self, start: usize) -> SyntaxKind {
        let mut i = start + 1;
        while i < self.src.len() && !starts_token(self.src[i]) {
            i += 1;
        }
        self.pos = i;
        let what = if self.src[start] >= 0x80 {
            "a byte above 0x7F may appear only in a comment".to_string()
        } else {
            format!("byte 0x{:02X} begins no token", self.src[start])
        };
        self.diags.push(
            Diagnostic::new(&codes::E0201, "unexpected character")
                .primary(self.span(start, i), what)
                .help("remove it, or move it into a comment"),
        );
        UNKNOWN
    }
}

/// `digit {digit}` or `0x hex {hex}`.
pub(crate) fn valid_int(t: &[u8]) -> bool {
    if let Some(h) = t.strip_prefix(b"0x") {
        !h.is_empty() && h.iter().all(u8::is_ascii_hexdigit)
    } else {
        !t.is_empty() && t.iter().all(u8::is_ascii_digit)
    }
}

/// `digit {digit} "." digit {digit} ["f"]`.
pub(crate) fn valid_float(t: &[u8]) -> bool {
    let t = t.strip_suffix(b"f").unwrap_or(t);
    let Some(dot) = t.iter().position(|&b| b == b'.') else {
        return false;
    };
    let (a, b) = (&t[..dot], &t[dot + 1..]);
    !a.is_empty() && !b.is_empty() && a.iter().chain(b).all(u8::is_ascii_digit)
}
