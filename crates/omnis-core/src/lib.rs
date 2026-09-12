//! Omnis core: typed IDs, integer and fixed-point math, dice, deterministic RNG, and time
//! primitives shared by every simulation crate.
//!
//! Simulation crate rules (CLAUDE.md, ARCHITECTURE.md §11): `no_std`, integers only, ordered
//! collections only, no Bevy, no wall clock, no threads, no I/O.
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

extern crate alloc;

pub mod error;
pub mod fixed;
pub mod geom;
pub mod id;
pub mod rng;
pub mod time;

pub use alloc::collections::{BTreeMap, BTreeSet};
pub use error::Error;
pub use fixed::Fixed;
pub use geom::{Direction, Edges, Facing, Position, Rotation};
pub use id::{
    ActorId, CharacterId, EraId, FlagId, HolderId, ItemId, MapId, MonsterId, PartyId, ProjectId,
    QuestId, RegionId, SpellId, StreamName, TextKey, TilesetId,
};
pub use rng::{Dice, DieRoll, Pcg32, RollTrace, fnv1a64, splitmix64};
pub use time::{Clock, Contact};
