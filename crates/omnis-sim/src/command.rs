//! What a client may ask the simulation to do, and what it says happened (ARCHITECTURE.md §4.2).
//! Commands carry no client state so a command stream is a replay and, later, a network
//! protocol. Events carry keys, never text.

use alloc::vec::Vec;
use omnis_core::{Direction, Facing, HolderId, MapId, Position, Rotation};
use serde::{Deserialize, Serialize};

/// One player action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Command {
    /// Move one tile relative to the facing, without turning.
    Step(Direction),
    /// Turn in place.
    Turn(Rotation),
    /// Use whatever is on the facing edge or tile: a door in M1.
    Interact,
}

/// Why a step did not happen. Not an error and not a rejection: the turn was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BlockReason {
    /// A wall on the edge.
    Wall,
    /// A closed door on the edge.
    ClosedDoor,
    /// The target terrain cannot be entered.
    Impassable,
    /// The target is off the map or the map is unknown.
    MapEdge,
}

/// A message for the player, named so packs can localize it under `sim:message:<name>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MessageKey {
    /// Interact found nothing.
    NothingHere,
}

impl MessageKey {
    /// The text key clients look up.
    #[must_use]
    pub const fn text_key(self) -> &'static str {
        match self {
            MessageKey::NothingHere => "sim:message:nothing_here",
        }
    }
}

/// A tile the party perceived this turn, in viewport coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeenTile {
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Tiles ahead of the party.
    pub depth: u8,
    /// Tiles to the right of the facing line; negative is left.
    pub offset: i8,
}

/// What happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// The party moved, possibly to another map through a portal.
    Moved {
        /// Where it was.
        from: Position,
        /// Where it is.
        to: Position,
    },
    /// A step did not happen.
    Blocked {
        /// Why.
        reason: BlockReason,
    },
    /// A holder's clock advanced.
    TimeAdvanced {
        /// Whose clock.
        holder: HolderId,
        /// By how much.
        minutes: u32,
        /// Whether a day boundary was crossed.
        day_rolled: bool,
    },
    /// What the party perceives after the command.
    Visible {
        /// Every visible tile, nearest first.
        tiles: Vec<SeenTile>,
    },
    /// A door changed state.
    Door {
        /// The map.
        map: MapId,
        /// The tile the party stands on.
        x: u16,
        /// The tile the party stands on.
        y: u16,
        /// The edge the door is on.
        facing: Facing,
        /// Its new state.
        open: bool,
    },
    /// A message for the player.
    Message {
        /// Which message.
        key: MessageKey,
    },
}

/// A command the rules refuse. Not an error: the world is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rejection {
    /// The command does not apply in the current mode.
    WrongMode,
}

impl core::fmt::Display for Rejection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Rejection::WrongMode => f.write_str("command does not apply in the current mode"),
        }
    }
}
