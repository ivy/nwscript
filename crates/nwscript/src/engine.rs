//! The engine function table, found through a resolver, and its use by the
//! interpreter and the disassembler.

use crate::Resolver;
use nwscript_diag::{Diagnostic, SourceMap};
use nwscript_sema::{EngineTable, Type};
use nwscript_vm::{Routine, Ty};

/// Read `nwscript.nss` from the resolver. `None` when the resolver has none;
/// otherwise the table and what reading it reported, such as KotOR 1's
/// line-5145 typo.
pub fn load_table(
    resolver: &mut dyn Resolver,
    map: &mut SourceMap,
) -> std::io::Result<Option<(EngineTable, Vec<Diagnostic>)>> {
    let Some(r) = resolver.nwscript()? else {
        return Ok(None);
    };
    let file = map.add(r.path.clone(), r.bytes.clone());
    Ok(Some(EngineTable::from_nwscript(file, &r.bytes)))
}

fn ty(t: &Type) -> Ty {
    match t {
        Type::Void => Ty::Void,
        Type::Int => Ty::Int,
        Type::Float => Ty::Float,
        Type::String => Ty::String,
        Type::Object => Ty::Object,
        Type::Vector => Ty::Vector,
        Type::Action => Ty::Action,
        Type::Engine(n) => Ty::Engine(*n),
        // Engine routines take no script structs; a table that declared one
        // would be read as an int-sized value.
        Type::Struct(_) => Ty::Int,
    }
}

/// The interpreter's view of the table: each routine's signature by number.
pub fn routines(table: &EngineTable) -> Vec<Routine> {
    table
        .functions
        .iter()
        .map(|f| Routine {
            name: f.name.clone(),
            params: f.params.iter().map(|p| ty(&p.ty)).collect(),
            ret: ty(&f.ret),
        })
        .collect()
}
