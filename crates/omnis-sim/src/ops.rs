//! The client protocol (ARCHITECTURE.md §9): one [`Op`] in, one [`Reply`] or [`OpError`] out.
//! The dev socket, the headless driver, and the MCP bridge all speak it, so the bridge is a
//! pure translator. Ops that touch the host (files, packs, the window) are declared here so
//! the wire format is one enum, but [`dispatch`] refuses them; the host handles those itself
//! and calls `dispatch` for the rest. Everything arriving is untrusted (§6.2): strings and
//! scripts are bounded before they are looked at.

use crate::apply::apply;
use crate::command::{Command, Event, Rejection};
use crate::query::{self, ViewportModel};
use crate::world::{Known, Mode, World};
use crate::{LOG_CAPACITY, MINUTES_PER_DAY};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{EraId, MapId, Position};
use omnis_data::limits::{check_asset_path, string_fits};
use omnis_data::{Data, PackFingerprint};
use serde::{Deserialize, Serialize};

/// Most commands one `sim.script` may carry.
pub const MAX_SCRIPT: usize = 10_000;

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
    /// Host, game only: save a PNG of the canvas.
    #[serde(rename = "screenshot")]
    Screenshot {
        /// Where, or a default under `.omnis/`.
        #[serde(default)]
        path: Option<String>,
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
            Op::SaveWrite { .. } | Op::SaveRead { .. } | Op::PackReload | Op::Screenshot { .. }
        )
    }
}

/// The party's clock, broken down.
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
    pub mode: Mode,
    /// Commands applied.
    pub turn: u64,
    /// Where the party is.
    pub position: Position,
    /// The id of the party's map.
    pub map: String,
    /// The party's clock.
    pub clock: ClockView,
    /// The packs the world runs on.
    pub packs: Vec<PackFingerprint>,
    /// The world fingerprint as sixteen hex digits.
    pub fingerprint: String,
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
    /// `world.query`: `None` when the path does not exist.
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
            OpError::TooLong { limit } => write!(f, "string longer than {limit} bytes"),
            OpError::TooMany { limit } => write!(f, "more than {limit} commands"),
            OpError::HostOnly => f.write_str("this op needs the host, not the simulation"),
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
        Op::SimCommand { command } => apply(world, data, *command)
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
        Op::SaveWrite { .. } | Op::SaveRead { .. } | Op::PackReload | Op::Screenshot { .. } => {
            Err(OpError::HostOnly)
        }
    }
}

/// `game.status`.
pub fn status(world: &World, data: &Data) -> Result<Status, OpError> {
    let clock = world.party_clock();
    let day_length = i64::from(MINUTES_PER_DAY);
    Ok(Status {
        mode: world.mode,
        turn: world.turn,
        position: world.position,
        map: map_name(world.position.map, data),
        clock: ClockView {
            elapsed: clock.elapsed,
            day: clock.elapsed.div_euclid(day_length),
            minute: clock.elapsed.rem_euclid(day_length),
            era: clock.era,
        },
        packs: world.packs.clone(),
        fingerprint: format!("{:016x}", world.fingerprint().map_err(OpError::failed)?),
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
        match apply(world, data, *command) {
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

fn bounded(s: &str) -> Result<(), OpError> {
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

fn map_name(id: MapId, data: &Data) -> String {
    data.registry
        .maps
        .name(id)
        .map_or_else(|| format!("#{}", id.0), ToString::to_string)
}

fn unknown(id: MapId, data: &Data) -> OpError {
    OpError::UnknownMap {
        map: map_name(id, data),
    }
}
