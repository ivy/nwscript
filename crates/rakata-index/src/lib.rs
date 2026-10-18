//! The index of a game tree: facts, references and full text over every
//! resource the tree holds, keyed by content hash, in SQLite.
//!
//! A tree is a game install, or a checkpoint of a modded build, under a
//! label. [`build`] reads every file its walk names, lists every member of
//! every archive, and records where each content lies (`locations`), what
//! each content is (`resources`, `contents`), and what each says: a
//! dialogue's nodes and links, a GFF's fields, a table's cells, the talk
//! table's strings, a script's engine calls with their literal arguments
//! (from the bytecode, through `nwscript-ncs`), every reference each makes
//! to a resref, strref, tag or global (`refs`), and full text over node
//! text, comments, strings, call arguments and script source (`search`).
//! A content is parsed once however many trees, paths and archives hold
//! it, so a tree built from its parent and the paths that changed since
//! reads only those paths and parses only the hashes it has not met.
//!
//! The extractors are the format crates' (`rakata_dialog::facts`,
//! `rakata_gff::facts`, `rakata_2da::facts`, `rakata_tlk::facts`,
//! `nwscript_ncs::facts`) and are pure; this crate is the store, and the
//! one crate of the workspace that owns a file. The database is built in
//! memory and written once. [`digest`] proves two databases hold the same
//! facts about a tree, [`coverage`] says what each file contributed, and
//! [`query`] is the query surface: SQL over the tables `docs/index.md`
//! describes, with [`refs`] reading the references in either direction.

#![forbid(unsafe_code)]

mod build;
mod digest;
mod error;
mod extract;
mod query;
mod refs;
mod schema;
mod store;

pub use build::{BuildOptions, BuildReport, Timings, build, parse_changed};
pub use digest::digest;
pub use error::{Error, Result, Warning, codes};
pub use extract::{Param, Routine, Status, is_archive, is_gff, ref_kind};
pub use nwscript_ncs::facts::ArgType;
pub use query::{CoverageRow, Rows, TreeCoverage, coverage, query, query_script};
pub use rakata_res::RefKind;
pub use refs::{Answer, Direction, Hit, Location, RefsQuery, refs};
pub use rusqlite::Connection;
pub use schema::VERSION as SCHEMA_VERSION;
pub use store::{COUNTED, open};
