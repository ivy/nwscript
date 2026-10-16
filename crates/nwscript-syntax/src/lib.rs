//! Lexer, error-recovering parser and lossless syntax tree for NWScript.
//!
//! [`parse`] turns a file's bytes into a [`Parse`]: a rowan syntax tree that
//! holds every byte of the input, whitespace and comments included, and the
//! diagnostics for whatever did not parse. The tree prints back to the exact
//! input ([`Parse::to_bytes`]) whatever the input was. The grammar is
//! `docs/grammar.md`; the typed view over the tree is [`ast`].
//!
//! The tree's text is the source's bytes read as Latin-1 (see [`text`]), so
//! a byte above 0x7F is two bytes of tree text. Diagnostics carry byte spans;
//! [`Parse::span`] converts a tree range to one.
//!
//! ```
//! use nwscript_diag::FileId;
//! use nwscript_syntax::{ast, parse};
//!
//! let p = parse(FileId(0), b"int TRUE = 1;\r\nvoid main() { PrintInteger(TRUE); }\r\n");
//! assert!(p.diagnostics().is_empty());
//! let file = p.tree();
//! let names: Vec<String> = file
//!     .functions()
//!     .filter_map(|f| f.name())
//!     .map(|t| t.text().to_string())
//!     .collect();
//! assert_eq!(names, ["main"]);
//! ```

pub mod ast;
mod dump;
mod kind;
pub mod lexer;
mod parser;
pub mod text;

pub use kind::{Nwscript, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
pub use rowan::{GreenNode, TextRange, TextSize};

use nwscript_diag::{Diagnostic, FileId, Span};
use text::OffsetMap;

/// A parsed file: its tree and the diagnostics from lexing and parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parse {
    file: FileId,
    green: GreenNode,
    offsets: OffsetMap,
    diagnostics: Vec<Diagnostic>,
}

/// Parse a file's bytes. `file` is the id the caller registered the bytes
/// under in its `SourceMap`; every diagnostic's span refers to it.
pub fn parse(file: FileId, bytes: &[u8]) -> Parse {
    let (tokens, mut diagnostics) = lexer::lex(file, bytes);
    let lex_errors = diagnostics
        .iter()
        .filter_map(|d| d.primary_span().map(|s| s.start))
        .collect();
    let (green, parse_diags) = parser::parse_tokens(file, bytes, &tokens, lex_errors);
    diagnostics.extend(parse_diags);
    diagnostics.sort_by_key(|d| d.primary_span().map(|s| (s.start, s.end)));
    Parse {
        file,
        green,
        offsets: OffsetMap::new(bytes),
        diagnostics,
    }
}

impl Parse {
    pub fn file(&self) -> FileId {
        self.file
    }

    /// The root of the tree. Red nodes are cheap and not `Send`; make them
    /// where they are used.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// The typed root.
    pub fn tree(&self) -> ast::SourceFile {
        ast::SourceFile::cast_root(self.syntax())
    }

    pub fn green(&self) -> &GreenNode {
        &self.green
    }

    /// Lexing and parsing diagnostics, in source order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// The source bytes, printed from the tree.
    pub fn to_bytes(&self) -> Vec<u8> {
        text::latin1_bytes(&self.syntax().to_string())
    }

    /// The byte span of a range of this tree's text.
    pub fn span(&self, range: TextRange) -> Span {
        let r = self.offsets.to_byte_range(range);
        Span::new(self.file, r.start, r.end)
    }

    /// The byte span of a node or token of this tree, trivia inside it
    /// included.
    pub fn span_of(&self, element: impl Into<SyntaxElement>) -> Span {
        self.span(element.into().text_range())
    }

    /// The text offset in this tree of a byte offset in the source.
    pub fn text_offset(&self, byte: usize) -> TextSize {
        self.offsets.to_text(byte)
    }

    /// The tree as indented text, one node or token per line with byte
    /// ranges, followed by the diagnostics. For snapshot tests and
    /// debugging.
    pub fn debug_dump(&self) -> String {
        dump::dump(self)
    }
}
