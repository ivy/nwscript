//! TSLPatcher's `changes.ini` and `namespaces.ini`, read with the patcher's
//! semantics into a typed model, and the plan an install of them would run.
//!
//! The dialect and the semantics are written down in
//! `docs/formats/changes-ini.md`; this crate is that document as code. The
//! [`Ini`] reader keeps every line's position, [`Changes`] is the model (every
//! key the vocabulary does not know is kept on its owner's `unknown` and
//! reported, never dropped), [`Plan`] is the operations in the order the
//! phases run with the memory tokens as placeholders, and [`load`] reads an
//! installer's folder. Findings are [`Issue`]s under codes the `rakata`
//! façade registers; nothing here builds one from rendered text.
//!
//! The executor, which applies a plan to a game and reports what it finds in
//! the tree afterwards, is the next piece and is not here.

#![forbid(unsafe_code)]

pub mod changes;
pub mod ini;
pub mod issue;
pub mod load;
pub mod namespaces;
pub mod plan;
pub mod value;

pub use changes::Changes;
pub use ini::Ini;
pub use issue::{Issue, codes};
pub use load::{LoadError, Loaded, Source, load};
pub use namespaces::{Namespace, Namespaces};
pub use plan::{Operation, Phase, Plan, Step};
pub use value::{Token, Value};
