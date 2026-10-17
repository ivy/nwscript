//! Name resolution, types, constants, the engine function table from
//! `nwscript.nss`, and lints.

pub mod table;
pub mod types;

pub use table::{Constant, EngineTable, Function, Param};
pub use types::{ConstValue, Type};
