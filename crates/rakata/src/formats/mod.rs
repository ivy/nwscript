//! The formats registered with the round-trip suite, one module per
//! `rakata-*` crate, each an adapter from that crate's readers and writers
//! to [`crate::conformance::Format`].

mod archive;

pub use archive::Archive;
