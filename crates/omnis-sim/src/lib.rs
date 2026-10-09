//! Omnis simulation: `World` + `Command` -> `Vec<Event>`. Pure Rust, no Bevy, no I/O, no wall
//! clock, no threads. Every client (renderer, editor, MCP, CLI, tests) drives this library.
//!
//! Exploration and the party: one party holder with its own clock, movement with walls,
//! doors, portals, a visibility cone that fills the automap, a party of up to six characters
//! built from pack data, difficulty settings, saves as RON text with migrations, fingerprints,
//! and replays. Combat: encounters on map tiles and the fight state (M4). Town: the services
//! placed on map tiles, entered by a step (M7).
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

extern crate alloc;

pub mod api;
pub mod apply;
pub mod bus;
pub mod cast_view;
mod casting;
mod checks;
pub mod combat;
pub mod command;
pub mod dev;
pub mod effects;
pub mod encounter;
pub mod event;
pub mod items;
mod migrate;
pub mod names;
pub mod ops;
pub mod party;
pub mod party_view;
pub mod query;
pub mod replay;
pub mod rest;
pub mod rest_view;
mod sense;
pub mod service;
mod service_level;
pub mod service_view;
pub mod tactics;
pub mod time;
pub mod time_view;
mod utility;
pub mod view;
pub mod visibility;
pub mod word;
pub mod world;

// Clients depend on this crate alone (ARCHITECTURE.md §3); the types they need from the
// crates below reach them through these re-exports.
pub use omnis_core;
pub use omnis_data;
pub use omnis_rules;

pub use apply::apply;
pub use cast_view::{CastView, cast_view};
pub use combat::{Budget, CombatCommand, CombatState, FeatureChoice, Initiative, Pay, Target};
pub use command::{Command, Rejection};
pub use dev::DevCommand;
pub use encounter::{
    EncounterChoice, EncounterSource, EncounterState, Stack, bribe_cost, groups_cleared,
};
pub use event::{
    ActorRef, BlockReason, CheckKind, CombatOutcome, EffectEnd, EffectTarget, Event, ItemPlace,
    LayerCheck, MessageKey, SeenTile, SensedTile, Surprise,
};
pub use items::ItemCommand;
pub use names::Place;
pub use ops::{Op, OpError, Reply, Status, dispatch};
pub use party::{Party, PartyCommand};
pub use party_view::{
    AnswerView, EffectView, ItemView, MemberView, PartyView, ReactionView, TacticsView, party_view,
};
pub use replay::{Replay, ReplayError};
pub use rest::RestCommand;
pub use rest_view::{CampMember, RestView, rest_view};
pub use service::{ServiceCommand, ServiceState};
pub use service_view::{OfferView, ServiceView, service_view};
pub use view::{
    ChoiceKind, CombatView, FeatureView, FighterView, SpellView, StackView, combat_view,
};
pub use word::{ScriptError, Word, parse_script};
pub use world::{
    Automap, Known, LoadError, MapState, Mode, ModeKind, NewGameError, SaveRule, Settings, World,
};

use omnis_core::{HolderId, PartyId};

/// The single party of v1.
pub const PARTY: HolderId = HolderId::Party(PartyId(0));
/// Minutes opening or closing a door costs.
pub const INTERACT_MINUTES: u32 = 1;
/// How many events `World::log` keeps.
pub const LOG_CAPACITY: usize = 4096;
