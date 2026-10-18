//! The registry of diagnostic codes. Each code is declared once here, with
//! its default severity, a one-line summary, and a long explanation kept in
//! `explain/CODE.md` beside this crate, which `nws explain CODE` prints.
//!
//! Ranges: `E00xx` inputs and invocation, `E01xx` bytecode, `E02xx` syntax,
//! `E03xx` semantic analysis, `E04xx` code generation, `E05xx` the
//! interpreter, `E06xx` decompilation, `W0xxx` lints.
//!
//! The [`codes!`](crate::codes!) macro that declares them is public: another
//! toolset registers its own codes, under its own prefix and with its own
//! explanation directory, the same way, and looks them up with
//! [`lookup_in`].

use crate::Severity;

/// A registered diagnostic code.
#[derive(Debug, PartialEq, Eq)]
pub struct Code {
    pub id: &'static str,
    pub severity: Severity,
    pub summary: &'static str,
    pub explanation: &'static str,
}

/// Declare a registry of codes: one `pub static` per code, named by its id,
/// and `ALL`, every code in declaration order. `dir` is the directory of
/// the explanations relative to the invoking crate's manifest; each code's
/// explanation is `dir/ID.md`, read at compile time.
///
/// ```ignore
/// nwscript_diag::codes! {
///     dir "explain";
///     RE0001 Error "an input file could not be read";
/// }
/// ```
#[macro_export]
macro_rules! codes {
    (dir $dir:literal; $( $id:ident $sev:ident $summary:literal; )*) => {
        $(
            #[doc = $summary]
            pub static $id: $crate::codes::Code = $crate::codes::Code {
                id: stringify!($id),
                severity: $crate::Severity::$sev,
                summary: $summary,
                explanation: include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"), "/", $dir, "/", stringify!($id), ".md"
                )),
            };
        )*
        /// Every registered code, in declaration order.
        pub static ALL: &[&$crate::codes::Code] = &[$(&$id),*];
    };
}

codes! {
    dir "explain";
    E0001 Error "an input file could not be read";
    E0002 Error "no nwscript.nss to read the engine's declarations from";
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
    E0303 Error "a name that is not declared";
    E0304 Error "a call to a function that is not declared";
    E0305 Error "a value of the wrong type";
    E0306 Error "a call to a script function with the wrong number of arguments";
    E0307 Error "a name used before its declaration";
    E0308 Error "a call to an engine routine with the wrong number of arguments";
    E0309 Error "an argument of the wrong type to an engine routine";
    E0310 Error "an assignment or increment of something that is not a variable";
    E0311 Error "a function definition whose return type disagrees with its prototype";
    E0312 Error "a function definition whose parameters disagree with its prototype";
    E0313 Error "a name declared twice";
    E0314 Error "`break` or `continue` outside a loop or switch";
    E0315 Error "a malformed `case` or `default` label";
    E0316 Error "a function that can end without returning its value";
    E0317 Error "a `return` with a value in a `void` function, or without one in another";
    E0318 Error "a malformed entry point";
    E0319 Error "an include file that cannot be found";
    E0320 Error "an include file that includes itself";
    E0321 Error "a member that does not exist";
    E0322 Error "a call to a function that is declared but never defined";
    E0323 Error "a type that is not allowed here";
    E0324 Error "a parameter without a default after one with a default";
    E0325 Error "a declaration where the language does not allow one";
    E0326 Error "an `action` argument that is not a call to a `void` function";
    E0327 Error "a struct used above its declaration";
    E0328 Error "two `case` labels with the same value";
    E0401 Error "a program code generation cannot compile";
    E0501 Error "the script faulted in the interpreter";
    E0502 Warning "the script stopped on an answer only the engine can give";
    E0601 Error "bytecode without an entry point the decompiler recognises";
    E0602 Warning "a decompiled function is not verified";
    W0001 Warning "a local variable that is never read";
    W0003 Warning "a function that is never called";
    W0004 Warning "a local that hides a parameter, an outer local or a global";
    W0005 Warning "a statement that is never reached";
    W0006 Warning "arguments whose result depends on the order they are evaluated in";
    W0007 Warning "a statement that computes a value and discards it";
    W0008 Warning "an int converted to float in arithmetic";
}

/// Errors BioWare's own compiler accepted: the vanilla scripts were compiled
/// against declarations that did not ship, so a script that needs one of
/// these is checked against the shipped declarations and fails. `--compat
/// bioware` demotes exactly these; the checked program records what was
/// written for each, so code generation needs nothing more.
/// Lints that stay warnings in strict mode: calibration over the vanilla
/// corpus and mods' scripts found them noise as errors (`docs/lints.md`).
pub static STRICT_WARNINGS: &[&Code] = &[];

/// Errors the Windows builds of nwnnsscomp that mods ship accept, found by
/// compiling every mod script of the modshop build that nws rejected with
/// it as a black box (`docs/lints.md`). `--compat nwnnsscomp` and
/// `--compat bioware` accept them as warnings.
pub static NWNNSSCOMP_ACCEPTED: &[&Code] = &[&E0327, &E0328];

pub static BIOWARE_ACCEPTED: &[&Code] = &[&E0308, &E0309, &E0311];

/// The nwscript code with this id, if registered.
pub fn lookup(id: &str) -> Option<&'static Code> {
    lookup_in(ALL, id)
}

/// The code with this id in a registry, if registered; case does not matter.
pub fn lookup_in(registry: &[&'static Code], id: &str) -> Option<&'static Code> {
    registry
        .iter()
        .copied()
        .find(|c| c.id.eq_ignore_ascii_case(id))
}
