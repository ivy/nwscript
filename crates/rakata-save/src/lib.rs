//! KotOR 1 saved games by name. A save is a directory,
//! `Saves/NNNNNN - GameN/`, holding `SAVEGAME.sav` (an ERF of the party
//! members' `AVAILNPC<n>.utc`, the inventory, the factions, and one nested
//! ERF per visited module with that module's GIT, ARE and IFO) and, beside
//! it, `GLOBALVARS.res`, `PARTYTABLE.res`, `savenfo.res` and a screenshot.
//!
//! [`Save::open`] reads it; [`Save::globals`] gives the globals by name and
//! type, [`Save::party`] the party table, [`Save::creatures`] each
//! creature's tag as the world has it, `Commandable`, faction, animation,
//! effects, local booleans and numbers, and action queue, and
//! [`Save::events`] a module's pending events. [`LocalNames`] names the
//! local variables from the game's own include scripts, which this crate
//! does not carry; numbers whose names live only in the engine (effect
//! types, animations, most action ids) stay numbers.
//!
//! Every GFF inside is read through `rakata-gff`, so it writes back as the
//! bytes it came from; [`Save::gff_members`] lists them all. The layout
//! follows the measured facts in `docs/formats/save.md`; this crate was
//! written without sight of any other implementation.

#![forbid(unsafe_code)]

mod creature;
mod error;
mod fields;
mod globals;
mod names;
mod save;

pub use creature::{Action, Creature, Effect, OBJECT_INVALID, ObjectRef, Param, Place};
pub use error::{Error, Result, codes};
pub use globals::{Global, GlobalValue, Globals, Location};
pub use names::{LocalNames, action_name};
pub use save::{Available, Event, GffMember, Info, Member, Module, Party, Save};
