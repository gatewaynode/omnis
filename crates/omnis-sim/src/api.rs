//! The engine's API (ARCHITECTURE.md §4.9; `docs/api.md` is the reference): exactly what a
//! client may rely on, in two tiers that share one version, [`PROTOCOL`].
//!
//! **Tier 1, the Rust library.** Load the packs, start a world, send commands, read views:
//!
//! ```no_run
//! use omnis_sim::api::*;
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! let data = load_packs(&[std::path::Path::new("packs/base")])?;
//! let mut world = World::new(&data, 1, Settings::default())?;
//! let events = apply(&mut world, &data, Command::Step(Direction::Forward));
//! let place = here(&world, &data);
//! # let _ = (events, place); Ok(()) }
//! ```
//!
//! A client reads state only through the views below; `World`'s fields are public for the
//! simulation's own tests and tools and may change without a version bump. Anything not
//! re-exported here is internal.
//!
//! **Tier 2, the JSON op protocol.** [`Op`] in, [`Reply`] or [`OpError`] out, through
//! [`dispatch`] for the simulation's ops and a host (the dev socket, `omnis_cli::Headless`) for
//! the six host ops and their rules ([`save_files`], [`load_save`], [`reload`],
//! [`rules_set`]).

// The world, its start, its save and its replay.
pub use crate::replay::{Replay, ReplayError};
pub use crate::world::{LoadError, ModeKind, NewGameError, SAVE_SCHEMA, SaveRule, Settings, World};
// What the party knows of a map, as `automap` returns it.
pub use crate::world::{Known, layer};
pub use omnis_data::{Data, FlagScope, LoadReport, PackFingerprint, load_packs};

// Commands in, events out.
pub use crate::apply::apply;
pub use crate::combat::{CombatCommand, FeatureChoice, Pay, Target};
pub use crate::command::{Command, Rejection};
pub use crate::dev::DevCommand;
pub use crate::encounter::{EncounterChoice, EncounterSource};
pub use crate::event::{
    ActorRef, BlockReason, CheckKind, CombatOutcome, EffectEnd, EffectTarget, Event, ItemPlace,
    LayerCheck, Loot, MessageKey, Notice, SeenTile, SensedTile, Surprise,
};
pub use crate::items::ItemCommand;
pub use crate::names::Place;
pub use crate::party::PartyCommand;
pub use crate::rest::{HitDiceSpend, RestCommand};
pub use crate::service::ServiceCommand;
pub use crate::tactics::TacticsCommand;
pub use omnis_core::{CharacterId, Direction, Facing, Level, MapId, Position, Rotation, Vertical};
pub use omnis_rules::Draft;
// A declared reaction as `PutReaction` carries it and `ReactionView` shows it: by string ids.
pub use omnis_rules::{ActionRef, Cmp, Criteria, CriteriaSet, Named, Predicate, Row, Trigger, Who};
// Script words, a typing aid: they name members by slot and resolve against the party.
pub use crate::word::{ScriptError, Word, parse_script};

// The views.
pub use crate::cast_view::{CastView, cast_view};
pub use crate::creation_view::{
    BackgroundOption, ClassOption, CreationView, RaceOption, creation_view,
};
pub use crate::ops::status;
pub use crate::party_view::{
    AnswerView, EffectView, ItemView, KnownSpellView, MemberView, PartyView, ReactionView,
    TacticsView, party_view,
};
// Objects in cells (M8.5 step 2): whether a footprint blocks a step, for a client's mirror.
pub use crate::objects::{ObjectState, blocks_step as object_blocks};
// What a client reads of objects: `objects.get`'s view, and what it draws of a whole level.
pub use crate::objects_view::{
    CheckView, ItemCount, ObjectView, ObjectsView, PileView, SeenObject, VerbView, objects_view,
    on_level as objects_on_level,
};
// Groups standing in view (M8.5 step 2c), and what a client draws of a whole level.
pub use crate::beings::{BeingView, on_level as beings_on_level};
pub use crate::journal::{Change, apply_changes, archive_len, journal_len};
pub use crate::log::{Logged, history, told};
pub use crate::query::{
    EdgeView, Here, ViewTile, ViewportModel, automap, door_open, flag_scope, flags, here, map_text,
    site_ahead, step_lands, viewport,
};
pub use crate::rest_view::{CampMember, RestView, rest_view};
pub use crate::save::{
    Archived, BaseNames, Bases, CharacterSave, Checkpoint, FOLD_INTERACTIONS, FOLD_RULE, FileOut,
    Holdings, JoinError, Manifest, Missing, NameKind, NameTable, PackFit, RegionBase, RegionSize,
    Restored, Segment, Signature, Split, SplitReport, WORLDS, WorldBase, archive_path, due, fold,
    fold_at, join, load_save, save_files, split, split_report, tidy_list, unnamed,
};
pub use crate::service_view::{OfferView, ServiceView, service_view};
pub use crate::time_view::{ContactView, DateView, HolderClock, TimeView, time_view};
pub use crate::view::{
    ChoiceKind, CombatView, FeatureView, FightItemView, FighterView, SpellView, StackView,
    combat_view,
};

// The JSON op protocol.
pub use crate::ops::{
    ClockView, KnownTile, MAX_SCRIPT, Op, OpError, PROTOCOL, Reply, RulesView, ShotTarget,
    SlotView, Status, client_path, dispatch, reload, rules_set,
};
pub use omnis_core::Edges;
