//! Name resolution, types, constants, the engine function table from
//! `nwscript.nss`, and lints.
//!
//! [`EngineTable::from_nwscript`] reads the engine's routines and constants.
//! [`analyze`] checks a script against it, with its includes, and returns a
//! [`hir::Program`]: the typed, resolved form code generation walks.

mod check;
pub mod hir;
pub mod lint;
pub mod table;
pub mod types;
pub mod visit;

pub use check::{Analysis, Includes, ParsedIncludes, analyze};
pub use table::{Constant, EngineTable, Function, Param};
pub use types::{ConstValue, Type};
