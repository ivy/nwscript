//! KotOR's conversations: the DLG model over `rakata-gff`, stable node
//! keys, and the transcript.
//!
//! A [`Dialog`] keeps the GFF it was read from and reads through it:
//! entries and replies as [`Node`]s (speaker, listener, text, voice, sound,
//! script, delay, animations with their participants, camera angle and id,
//! quest, comment), the starting [`Link`]s into the entries and each node's
//! links onward with their condition scripts and `IsChild`. Every field the
//! model does not name stays in the tree, so a dialogue written back is the
//! bytes it was read from.
//!
//! A [`NodeKey`] addresses a node by its voice resref, its strref or its
//! text, and fails with a diagnostic naming the key and the dialogue when
//! no node matches or several do; an index key exists for what content
//! cannot name and is marked fragile wherever it is shown.
//!
//! Edits keep every index: [`Dialog::push_entry`], [`Dialog::push_reply`],
//! [`Dialog::push_link`] and [`Dialog::push_start`] add at the ends of the
//! lists (or inside a node's own links), and [`Dialog::reorder`], which
//! moves indices, is named apart. [`check()`] lists the references the
//! install cannot satisfy (a script or condition, a voice-over, a strref, a
//! global a condition reads), asked through [`References`]; [`lint()`] adds
//! the rules that read the whole dialogue: dangling links, nodes no start
//! reaches, voiced nodes without lip sync, globals read by a computed name,
//! and `SoundExists` disagreeing with the audio the node names.
//!
//! A [`Transcript`] renders the conversation for reading, as text or as
//! structured nodes, with what lies outside the dialogue (the talk table,
//! the voice files, the decompiled scripts) supplied by a [`Lookup`]. The
//! crate does no I/O; `rakata::dialog` supplies the install.
//!
//! The field names and types are those of BioWare's Conversation document
//! as xoreos publishes it, checked against the vanilla files; this crate
//! was written without sight of any other implementation.

#![forbid(unsafe_code)]

pub mod audio;
mod check;
mod edit;
mod error;
pub mod facts;
mod key;
mod lint;
mod model;
mod transcript;

pub use check::{Finding, GlobalReads, Place, References, check};
pub use edit::{CAMERA_STATIC, NO_DELAY, NewNode, Text};
pub use error::{Error, Result, codes};
pub use key::NodeKey;
pub use lint::lint;
pub use model::{Animation, Dialog, Info, Kind, Link, Node, NodeRef, localized};
pub use transcript::{
    AnimationView, Embedded, LinkView, Lookup, NodeView, Options, Row, Script, ScriptStatus,
    Transcript, Voice, VoiceView, Warning,
};
