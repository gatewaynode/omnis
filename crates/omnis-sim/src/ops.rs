//! The client protocol (ARCHITECTURE.md §9): one [`Op`] in, one [`Reply`] or [`OpError`] out.
//! The dev socket, the headless driver, and the MCP bridge all speak it, so the bridge is a
//! pure translator. Ops that touch the host (files, packs, the window) are declared here so
//! the wire format is one enum, but [`dispatch`] refuses them; the host handles those itself
//! and calls `dispatch` for the rest. Everything arriving is untrusted (§6.2): strings and
//! scripts are bounded before they are looked at.

use crate::LOG_CAPACITY;
use crate::apply::apply;
use crate::command::{Command, Rejection};
use crate::dev::DevCommand;
use crate::event::Event;
use crate::party::PartyCommand;
pub use crate::party_view::{ItemView, MemberView, PartyView, party_view};
use crate::query::{self, ViewportModel};
use crate::service_view::{ServiceView, service_view};
use crate::time_view::{DateView, TimeView, time_view};
use crate::view::{CombatView, combat_view};
use crate::world::{Known, ModeKind, Settings, World};
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{EraId, MapId, Position};
use omnis_data::limits::{check_asset_path, string_fits};
use omnis_data::{Data, PackFingerprint};
use omnis_rules::Draft;
use serde::{Deserialize, Serialize};

/// Most commands one `sim.script` may carry.
pub const MAX_SCRIPT: usize = 10_000;

/// What a screenshot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShotTarget {
    /// The game canvas at its internal resolution.
    #[default]
    Canvas,
    /// The canvas as the window scales it, with the window-space interface over it.
    Window,
}

/// A request. On the wire it is `{"op": "<name>", "args": {...}}`, `args` omitted for ops
/// that take none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", content = "args")]
pub enum Op {
    /// Mode, turn, clock, position, packs, fingerprint.
    #[serde(rename = "game.status")]
    GameStatus,
    /// Read one value by dotted path (`query::path`).
    #[serde(rename = "world.query")]
    WorldQuery {
        /// The path.
        path: String,
    },
    /// Apply one command.
    #[serde(rename = "sim.command")]
    SimCommand {
        /// The command.
        command: Command,
    },
    /// Apply commands in order, stopping at the first rejection.
    #[serde(rename = "sim.script")]
    SimScript {
        /// The commands.
        commands: Vec<Command>,
    },
    /// The last `count` events.
    #[serde(rename = "events.tail")]
    EventsTail {
        /// How many, at most `LOG_CAPACITY`.
        #[serde(default = "default_tail")]
        count: usize,
    },
    /// The viewport model.
    #[serde(rename = "viewport.get")]
    ViewportGet,
    /// A map as text with the party marker; the party's map when `map` is absent.
    #[serde(rename = "map.text")]
    MapText {
        /// Map id such as `test:map:dungeon`.
        #[serde(default)]
        map: Option<String>,
    },
    /// Known tiles of a map; the party's map when `map` is absent.
    #[serde(rename = "automap.get")]
    AutomapGet {
        /// Map id.
        #[serde(default)]
        map: Option<String>,
    },
    /// Host: write the save to a path.
    #[serde(rename = "save.write")]
    SaveWrite {
        /// A relative `.ron` path.
        path: String,
    },
    /// Host: replace the world with a save.
    #[serde(rename = "save.read")]
    SaveRead {
        /// A relative `.ron` path.
        path: String,
        /// Skip the pack fingerprint check.
        #[serde(default)]
        force: bool,
    },
    /// Host: reload the packs from disk, keeping the world.
    #[serde(rename = "pack.reload")]
    PackReload,
    /// Host, game only: save a PNG of the canvas, or of everything the window shows.
    #[serde(rename = "screenshot")]
    Screenshot {
        /// Where, or a default under `.omnis/`.
        #[serde(default)]
        path: Option<String>,
        /// What to capture; the canvas when absent.
        #[serde(default)]
        target: ShotTarget,
    },
    /// Host, game only: the text of every open interface panel, one line per control, label
    /// or text with its rectangle, so a panel can be read without a picture.
    #[serde(rename = "screen.text")]
    ScreenText,
    /// The party: members with their derived numbers, purse, and food.
    #[serde(rename = "party.get")]
    PartyGet,
    /// Create a character from a draft and add it to the party (`Command::Party(Create)`).
    #[serde(rename = "party.create")]
    PartyCreate {
        /// The draft.
        character: Draft,
    },
    /// The encounter or fight in progress: stacks, order, whose turn, round.
    #[serde(rename = "combat.get")]
    CombatGet,
    /// The service the party is inside: what is on offer, its price, and why not.
    #[serde(rename = "service.get")]
    ServiceGet,
    /// Every rule slot with its inputs and source, plus the values and tables.
    #[serde(rename = "rules.list")]
    RulesList,
    /// One slot's inputs and source.
    #[serde(rename = "rules.get")]
    RulesGet {
        /// Slot name such as `spell_points.pool`.
        slot: String,
    },
    /// Every holder's clock and contact, and the party's age, shared time and date (M8).
    #[serde(rename = "time.clocks")]
    TimeClocks,
    /// Dev: the party meets a region as on entering it; `sim.command` with
    /// `DevCommand::Reconcile`, so a replay holds it.
    #[serde(rename = "time.reconcile")]
    TimeReconcile {
        /// `pack:region:name`.
        region: String,
    },
    /// Host: replace one slot's formula in the loaded rules (hot swap; packs on disk are
    /// untouched).
    #[serde(rename = "rules.set")]
    RulesSet {
        /// Slot name.
        slot: String,
        /// The new expression.
        source: String,
    },
}

fn default_tail() -> usize {
    32
}

impl Op {
    /// Whether the host must handle this op; `dispatch` refuses it.
    #[must_use]
    pub const fn is_host(&self) -> bool {
        matches!(
            self,
            Op::SaveWrite { .. }
                | Op::SaveRead { .. }
                | Op::PackReload
                | Op::Screenshot { .. }
                | Op::ScreenText
                | Op::RulesSet { .. }
        )
    }
}

/// The party's age, broken down; the date it believes is `Status::date`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockView {
    /// Minutes since the party's origin.
    pub elapsed: i64,
    /// Whole days.
    pub day: i64,
    /// Minute of the day.
    pub minute: i64,
    /// The era.
    pub era: EraId,
}

/// `game.status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// What the party is doing.
    pub mode: ModeKind,
    /// Commands applied.
    pub turn: u64,
    /// Where the party is.
    pub position: Position,
    /// The id of the party's map.
    pub map: String,
    /// The party's age.
    pub clock: ClockView,
    /// The date the party believes, on the pack's calendar (M8).
    pub date: DateView,
    /// The packs the world runs on.
    pub packs: Vec<PackFingerprint>,
    /// The world fingerprint as sixteen hex digits.
    pub fingerprint: String,
    /// The id of the service the party is inside, if it is inside one.
    #[serde(default)]
    pub service: Option<String>,
    /// The groups placed `once` on the party's map that are cleared, and how many there are.
    #[serde(default)]
    pub groups_cleared: (u16, u16),
    /// Whether the save rule allows a save here (M8 step 8).
    #[serde(default)]
    pub may_save: bool,
    /// The world seed (M8 step 8).
    #[serde(default)]
    pub seed: u64,
    /// The difficulty options the game started with (M8 step 8).
    #[serde(default)]
    pub settings: Settings,
}

/// One known tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownTile {
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// What is known.
    pub known: Known,
}

/// One rule slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotView {
    /// Slot name.
    pub name: String,
    /// Declared inputs.
    pub inputs: Vec<String>,
    /// The expression.
    pub source: String,
}

/// The loaded rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesView {
    /// Every slot.
    pub slots: Vec<SlotView>,
    /// Plain values.
    pub values: BTreeMap<String, i64>,
    /// Tables.
    pub tables: BTreeMap<String, Vec<i64>>,
}

/// A successful result. Serialized untagged, so the wire carries the plain object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Reply {
    /// `game.status`, and `save.read` once the world is replaced.
    Status(Status),
    /// `viewport.get`.
    Viewport(ViewportModel),
    /// `sim.script`.
    Script {
        /// How many commands were applied.
        applied: usize,
        /// Their events, in order.
        events: Vec<Event>,
        /// Why the script stopped early, if it did.
        rejected: Option<Rejection>,
    },
    /// `sim.command` and `events.tail`.
    Events {
        /// The events.
        events: Vec<Event>,
    },
    /// `automap.get`.
    Automap {
        /// The map id.
        map: String,
        /// Known tiles in row-major order.
        tiles: Vec<KnownTile>,
    },
    /// `map.text`.
    Text {
        /// The text.
        text: String,
    },
    /// `save.write` and `screenshot`.
    Written {
        /// The file written.
        path: String,
    },
    /// `party.get`.
    Party {
        /// The party.
        party: PartyView,
    },
    /// `rules.list`.
    Rules {
        /// The rules.
        rules: RulesView,
    },
    /// `rules.get` and `rules.set`.
    Rule {
        /// The slot.
        rule: SlotView,
    },
    /// `combat.get`.
    Combat {
        /// The encounter or fight.
        combat: CombatView,
    },
    /// `service.get`.
    Service {
        /// The service.
        service: ServiceView,
    },
    /// `time.clocks`.
    Time {
        /// Clocks, contacts and the party's time.
        time: TimeView,
    },
    /// `world.query`: `None` when the path does not exist. Untagged deserialization tries
    /// variants in order and an absent `Option` field reads as `None`, so this variant and
    /// `Done` stay last: they would swallow any object.
    Value {
        /// The value as text.
        value: Option<String>,
    },
    /// `pack.reload`.
    Done {},
}

/// A failed op. The world is unchanged unless the message says otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OpError {
    /// The rules refused the command.
    Rejected {
        /// Why.
        rejection: Rejection,
    },
    /// No such map is loaded.
    UnknownMap {
        /// The id given.
        map: String,
    },
    /// No rule slot has that name.
    UnknownSlot {
        /// The name given.
        slot: String,
    },
    /// A string argument is longer than `limits::MAX_STRING_BYTES`.
    TooLong {
        /// The limit in bytes.
        limit: usize,
    },
    /// A script has more than `MAX_SCRIPT` commands.
    TooMany {
        /// The limit.
        limit: usize,
    },
    /// The op needs the host; the simulation alone cannot do it.
    HostOnly,
    /// No encounter or fight is in progress.
    NoEncounter,
    /// The party is not inside a service.
    NoService,
    /// The request itself was malformed: not JSON, not an op, or too long.
    BadRequest {
        /// What was wrong.
        message: String,
    },
    /// The host or the data layer failed; the message says how.
    Failed {
        /// What went wrong.
        message: String,
    },
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpError::Rejected { rejection } => write!(f, "{rejection}"),
            OpError::UnknownMap { map } => write!(f, "no map '{map}' is loaded"),
            OpError::UnknownSlot { slot } => write!(f, "no rule slot '{slot}'"),
            OpError::TooLong { limit } => write!(f, "string longer than {limit} bytes"),
            OpError::TooMany { limit } => write!(f, "more than {limit} commands"),
            OpError::HostOnly => f.write_str("this op needs the host, not the simulation"),
            OpError::NoEncounter => f.write_str("no encounter or fight is in progress"),
            OpError::NoService => f.write_str("the party is not inside a service"),
            OpError::BadRequest { message } => write!(f, "bad request: {message}"),
            OpError::Failed { message } => f.write_str(message),
        }
    }
}

impl OpError {
    /// A host failure from anything that displays.
    pub fn failed<E: fmt::Display>(error: E) -> OpError {
        OpError::Failed {
            message: error.to_string(),
        }
    }

    /// A malformed request from anything that displays.
    pub fn bad_request<E: fmt::Display>(error: E) -> OpError {
        OpError::BadRequest {
            message: error.to_string(),
        }
    }
}

/// A file path from a client, allowed only when relative, free of `.` and `..` components,
/// and ending in one of `extensions` (§6.2). Returns the path unchanged.
pub fn client_path<'a>(path: &'a str, extensions: &[&str]) -> Result<&'a str, OpError> {
    bounded(path)?;
    check_asset_path(path, extensions)
        .map(|()| path)
        .map_err(|fault| OpError::failed(format!("path '{path}': {}", fault.reason())))
}

/// Answer a simulation op. Host ops return `OpError::HostOnly`.
pub fn dispatch(world: &mut World, data: &Data, op: &Op) -> Result<Reply, OpError> {
    match op {
        Op::GameStatus => status(world, data).map(Reply::Status),
        Op::WorldQuery { path } => {
            bounded(path)?;
            Ok(Reply::Value {
                value: query::path(world, path),
            })
        }
        Op::SimCommand { command } => apply(world, data, command.clone())
            .map(|events| Reply::Events { events })
            .map_err(|rejection| OpError::Rejected { rejection }),
        Op::SimScript { commands } => script(world, data, commands),
        Op::EventsTail { count } => {
            let keep = (*count).min(LOG_CAPACITY).min(world.log.len());
            Ok(Reply::Events {
                events: world.log[world.log.len() - keep..].to_vec(),
            })
        }
        Op::ViewportGet => query::viewport(world, data)
            .map(Reply::Viewport)
            .ok_or_else(|| unknown(world.position.map, data)),
        Op::MapText { map } => {
            let id = resolve_map(world, data, map.as_deref())?;
            query::map_text(world, data, id)
                .map(|text| Reply::Text { text })
                .ok_or_else(|| unknown(id, data))
        }
        Op::AutomapGet { map } => {
            let id = resolve_map(world, data, map.as_deref())?;
            let tiles = world
                .automap
                .map(id)
                .map(|known| {
                    known
                        .iter()
                        .map(|(&(x, y), &known)| KnownTile { x, y, known })
                        .collect()
                })
                .unwrap_or_default();
            Ok(Reply::Automap {
                map: map_name(id, data),
                tiles,
            })
        }
        Op::PartyGet => Ok(Reply::Party {
            party: party_view(world, data),
        }),
        Op::PartyCreate { character } => apply(
            world,
            data,
            Command::Party(PartyCommand::Create(character.clone())),
        )
        .map(|events| Reply::Events { events })
        .map_err(|rejection| OpError::Rejected { rejection }),
        Op::CombatGet => combat_view(world, data)
            .map(|combat| Reply::Combat { combat })
            .ok_or(OpError::NoEncounter),
        Op::ServiceGet => service_view(world, data)
            .map(|service| Reply::Service { service })
            .ok_or(OpError::NoService),
        Op::TimeClocks => Ok(Reply::Time {
            time: time_view(world, data),
        }),
        Op::TimeReconcile { region } => apply(
            world,
            data,
            Command::Dev(DevCommand::Reconcile {
                region: region.clone(),
            }),
        )
        .map(|events| Reply::Events { events })
        .map_err(|rejection| OpError::Rejected { rejection }),
        Op::RulesList => Ok(Reply::Rules {
            rules: rules_view(data),
        }),
        Op::RulesGet { slot } => slot_view(data, slot).map(|rule| Reply::Rule { rule }),
        Op::SaveWrite { .. }
        | Op::SaveRead { .. }
        | Op::PackReload
        | Op::Screenshot { .. }
        | Op::ScreenText
        | Op::RulesSet { .. } => Err(OpError::HostOnly),
    }
}

/// `rules.list`.
#[must_use]
pub fn rules_view(data: &Data) -> RulesView {
    RulesView {
        slots: data
            .rules
            .slot_names()
            .filter_map(|name| slot_view(data, name).ok())
            .collect(),
        values: data.rules.values().clone(),
        tables: data.rules.tables().clone(),
    }
}

/// `rules.get`, and what `rules.set` answers with.
pub fn slot_view(data: &Data, name: &str) -> Result<SlotView, OpError> {
    bounded(name)?;
    let slot = data.rules.slot(name).ok_or_else(|| OpError::UnknownSlot {
        slot: name.to_owned(),
    })?;
    Ok(SlotView {
        name: name.to_owned(),
        inputs: slot.inputs().to_vec(),
        source: slot.source().to_owned(),
    })
}

/// `game.status`.
pub fn status(world: &World, data: &Data) -> Result<Status, OpError> {
    let clock = world.party_clock();
    let day_length = i64::from(data.calendar().minutes_per_day.max(1));
    let here = query::here(world, data);
    Ok(Status {
        mode: here.mode,
        turn: here.turn,
        position: here.position,
        map: here.map,
        clock: ClockView {
            elapsed: clock.elapsed,
            day: clock.elapsed.div_euclid(day_length),
            minute: clock.elapsed.rem_euclid(day_length),
            era: clock.era,
        },
        date: here.date,
        packs: world.packs.clone(),
        fingerprint: format!("{:016x}", world.fingerprint().map_err(OpError::failed)?),
        service: here.service,
        groups_cleared: crate::encounter::groups_cleared(world, data, world.position.map),
        may_save: here.may_save,
        seed: here.seed,
        settings: here.settings,
    })
}

fn script(world: &mut World, data: &Data, commands: &[Command]) -> Result<Reply, OpError> {
    if commands.len() > MAX_SCRIPT {
        return Err(OpError::TooMany { limit: MAX_SCRIPT });
    }
    let mut events = Vec::new();
    let mut applied = 0;
    let mut rejected = None;
    for command in commands {
        match apply(world, data, command.clone()) {
            Ok(more) => {
                events.extend(more);
                applied += 1;
            }
            Err(rejection) => {
                rejected = Some(rejection);
                break;
            }
        }
    }
    Ok(Reply::Script {
        applied,
        events,
        rejected,
    })
}

/// Refuse a string argument longer than the limit.
pub fn bounded(s: &str) -> Result<(), OpError> {
    if string_fits(s) {
        Ok(())
    } else {
        Err(OpError::TooLong {
            limit: omnis_data::limits::MAX_STRING_BYTES,
        })
    }
}

fn resolve_map(world: &World, data: &Data, name: Option<&str>) -> Result<MapId, OpError> {
    match name {
        None => Ok(world.position.map),
        Some(name) => {
            bounded(name)?;
            data.registry
                .maps
                .get(name)
                .ok_or_else(|| OpError::UnknownMap {
                    map: name.to_string(),
                })
        }
    }
}

use query::map_name;

fn unknown(id: MapId, data: &Data) -> OpError {
    OpError::UnknownMap {
        map: map_name(id, data),
    }
}
