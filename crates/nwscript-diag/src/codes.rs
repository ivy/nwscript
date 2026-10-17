//! The registry of diagnostic codes. Each code is declared once here, with
//! its default severity, a one-line summary, and a long explanation kept in
//! `explain/CODE.md` beside this crate, which `nws explain CODE` prints.
//!
//! Ranges: `E00xx` inputs and invocation, `E01xx` bytecode, `E02xx` syntax,
//! `E03xx` semantic analysis, `E04xx` code generation, `E05xx` the
//! interpreter, `E06xx` decompilation, `W0xxx` lints.

use crate::Severity;

/// A registered diagnostic code.
#[derive(Debug, PartialEq, Eq)]
pub struct Code {
    pub id: &'static str,
    pub severity: Severity,
    pub summary: &'static str,
    pub explanation: &'static str,
}

macro_rules! codes {
    ($( $id:ident $sev:ident $summary:literal; )*) => {
        $(
            #[doc = $summary]
            pub static $id: Code = Code {
                id: stringify!($id),
                severity: Severity::$sev,
                summary: $summary,
                explanation: include_str!(concat!("../explain/", stringify!($id), ".md")),
            };
        )*
        /// Every registered code, in declaration order.
        pub static ALL: &[&Code] = &[$(&$id),*];
    };
}

codes! {
    E0001 Error "an input file could not be read";
    E0101 Error "not an NCS V1.0 file";
    E0102 Error "the NCS program size record is missing or wrong";
    E0103 Error "unknown NCS opcode";
    E0104 Error "NCS opcode with a type byte it does not take";
    E0105 Error "NCS file is truncated";
    E0110 Error "unknown instruction in NCS assembly";
    E0111 Error "bad operand in NCS assembly";
    E0112 Error "jump to an undefined label in NCS assembly";
    E0113 Error "label defined twice in NCS assembly";
    E0201 Error "a character that begins no token";
    E0202 Error "a block comment without its closing `*/`";
    E0203 Error "a string literal without its closing quote";
    E0204 Error "a number literal in no form NWScript reads";
    E0205 Error "a syntax error: the parser expected something else";
    E0206 Error "a malformed or misplaced directive";
    E0207 Error "statements or expressions nested beyond the parser's limit";
    E0301 Error "a type name that names no type";
    E0302 Error "an initialiser or default that is not a constant";
    E0501 Error "the script faulted in the interpreter";
    E0502 Warning "the script stopped on an answer only the engine can give";
}

/// The code with this id, if registered.
pub fn lookup(id: &str) -> Option<&'static Code> {
    ALL.iter().copied().find(|c| c.id.eq_ignore_ascii_case(id))
}
