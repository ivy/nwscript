//! `[CompileList]`: a script's source with its `#2DAMEMORY#` and `#StrRef#`
//! tokens filled, compiled through `nwscript` as `nws compile` compiles it,
//! the includes and `nwscript.nss` found beside the script in the
//! installer's folders first and then in the install. No compiler a mod
//! ships is ever run.

use super::{Failure, Tokens};
use crate::issue::codes;
use nwscript::{FsResolver, Resolved, Resolver};
use nwscript_diag::{Levels, SourceMap};
use rakata_game::Install;
use std::io;
use std::path::PathBuf;

/// The installer's folders, then the install.
struct Chain<'a> {
    fs: FsResolver,
    install: &'a mut Install,
    inputs: Vec<nwscript::InputRecord>,
}

impl Resolver for Chain<'_> {
    fn include(&mut self, name: &str) -> io::Result<Option<Resolved>> {
        if let Some(r) = self.fs.include(name)? {
            return Ok(Some(r));
        }
        self.install.include(name)
    }

    /// A copy the mod ships beside its scripts (for the compiler it bundles),
    /// then the install's, as `nws compile --game-dir` finds it.
    fn nwscript(&mut self) -> io::Result<Option<Resolved>> {
        if let Some(r) = self.fs.nwscript()? {
            return Ok(Some(r));
        }
        self.install.nwscript()
    }

    fn inputs(&self) -> &[nwscript::InputRecord] {
        &self.inputs
    }
}

/// `#2DAMEMORY#` and `#StrRef#` in the source, without regard to case,
/// replaced by the tokens' values. A token with no value is an error.
pub fn fill_tokens(source: &[u8], tokens: &Tokens) -> Result<Vec<u8>, Failure> {
    let mut out = Vec::with_capacity(source.len());
    let mut i = 0;
    while i < source.len() {
        if source[i] == b'#'
            && let Some((len, value)) = token_at(&source[i + 1..], tokens)?
        {
            out.extend_from_slice(value.as_bytes());
            i += 1 + len + 1;
            continue;
        }
        out.push(source[i]);
        i += 1;
    }
    Ok(out)
}

/// At `rest` (just after a `#`), a token name and its closing `#`: the
/// name's length and the token's value.
fn token_at(rest: &[u8], tokens: &Tokens) -> Result<Option<(usize, String)>, Failure> {
    let Some(end) = rest.iter().take(32).position(|&b| b == b'#') else {
        return Ok(None);
    };
    let Ok(name) = std::str::from_utf8(&rest[..end]) else {
        return Ok(None);
    };
    match crate::value::Value::parse(name) {
        v @ (crate::value::Value::Memory(_) | crate::value::Value::StrRef(_)) => {
            Ok(Some((end, tokens.resolve(&v)?)))
        }
        _ => Ok(None),
    }
}

/// Compile `source`, named `name`, to bytecode.
pub fn compile(
    name: &str,
    source: &[u8],
    data_dirs: &[PathBuf],
    install: &mut Install,
    game: nwscript::Game,
) -> Result<Vec<u8>, Failure> {
    let mut map = SourceMap::new();
    let mut checker = nwscript::Checker::with_declarations(nwscript::Declarations::Shipped, game);
    let mut resolver = Chain {
        fs: FsResolver::new(data_dirs.to_vec(), None),
        install,
        inputs: Vec::new(),
    };
    let out = checker.check(&mut resolver, &mut map, name, source);
    let diags = nwscript::modes::finish(
        out.analysis.as_ref(),
        out.diagnostics,
        &map,
        &Levels::default(),
    );
    let first_error = diags.iter().find(|d| d.is_error());
    if let Some(d) = first_error {
        let errors = diags.iter().filter(|d| d.is_error()).count();
        return Err(Failure::new(
            codes::COMPILE_FAILED,
            format!(
                "{name} does not compile ({errors} error{}): error[{}]: {}",
                if errors == 1 { "" } else { "s" },
                d.code.id,
                d.message
            ),
        ));
    }
    let Some(analysis) = out.analysis else {
        return Err(Failure::new(
            codes::COMPILE_FAILED,
            format!("{name} does not compile: no nwscript.nss in the install or tslpatchdata"),
        ));
    };
    let program = nwscript::compile::compile(&analysis.program, game).map_err(|d| {
        Failure::new(
            codes::COMPILE_FAILED,
            format!(
                "{name} does not compile: error[{}]: {}",
                d.code.id, d.message
            ),
        )
    })?;
    Ok(nwscript::ncs::encode(&program))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_in_source_are_filled_and_other_hashes_kept() {
        let mut t = Tokens::default();
        t.memory.insert(1, "42".into());
        t.strref.insert(0, 12345);
        let src = b"#include \"x\"\nint a = #2DAMEMORY1#; int b = #strref0#; // #nope#";
        let out = fill_tokens(src, &t).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "#include \"x\"\nint a = 42; int b = 12345; // #nope#"
        );
        let e = fill_tokens(b"#2DAMEMORY9#", &t).unwrap_err();
        assert_eq!(e.code, codes::TOKEN_NO_VALUE);
    }
}
