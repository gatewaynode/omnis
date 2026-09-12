//! Omnis simulation: `World` + `Command` -> `Vec<Event>`. Pure Rust, no Bevy, no I/O, no wall
//! clock, no threads. Every client (renderer, editor, MCP, CLI, tests) drives this library.
//!
//! M1 scope: exploration only. One party holder with its own clock, movement with walls,
//! doors, portals, a visibility cone that fills the automap, saves as RON text, fingerprints,
//! and replays.
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

extern crate alloc;

pub mod apply;
pub mod command;
pub mod query;
pub mod replay;
pub mod visibility;
pub mod world;

// Clients depend on this crate alone (ARCHITECTURE.md §3); the types they need from the
// crates below reach them through these re-exports.
pub use omnis_core;
pub use omnis_data;

pub use apply::apply;
pub use command::{BlockReason, Command, Event, MessageKey, Rejection, SeenTile};
pub use replay::{Replay, ReplayError};
pub use world::{Automap, Known, LoadError, MapState, Mode, NewGameError, Settings, World};

use omnis_core::{HolderId, PartyId};

/// Minutes in a day of the party's calendar. The calendar shape becomes data with M8.
pub const MINUTES_PER_DAY: u32 = 1440;
/// The single party of v1.
pub const PARTY: HolderId = HolderId::Party(PartyId(0));
/// Minutes opening or closing a door costs.
pub const INTERACT_MINUTES: u32 = 1;
/// How many events `World::log` keeps.
pub const LOG_CAPACITY: usize = 4096;
