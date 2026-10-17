//! The outcome notice (owner, 2026-10-16; presentation-ARCHITECTURE §6.6): what a command the
//! player started said, kept with the place it was done, so it fades as the party walks away and
//! is dropped once out of range: it fades from the first movement and is gone by the cell's edge
//! (owner, 2026-10-17). Distance alone fades it: a turn on the spot keeps it whole. Bevy-free.

use crate::pose::Pose;
use crate::style::Role;
use omnis_sim::omnis_core::{Level, MapId, Position};

/// At this many cells from where it was done, half a cell, the cell's edge, a notice is gone.
pub const GONE: f32 = 0.5;

/// A command's words and where it was done.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    /// Its lines, oldest first.
    pub lines: Vec<String>,
    /// The role they are drawn in: the interface, or the alert for a refusal.
    pub role: Role,
    /// The map it was done on.
    pub map: MapId,
    /// The level.
    pub level: Level,
    /// The pose's ground point, `(X, Z)`.
    pub at: (f32, f32),
}

impl Notice {
    /// `lines` in `role`, done at `position` with the camera at `pose`.
    #[must_use]
    pub fn new(lines: Vec<String>, role: Role, position: Position, pose: Pose) -> Notice {
        Notice {
            lines,
            role,
            map: position.map,
            level: position.level,
            at: (pose.x, pose.z),
        }
    }
}

/// How opaque `notice` is with the party at `position` and the camera at `pose`: 1 where it was
/// done, falling evenly to 0 at [`GONE`]; 0 on another map or level.
#[must_use]
pub fn alpha(notice: &Notice, position: &Position, pose: &Pose) -> f32 {
    if position.map != notice.map || position.level != notice.level {
        return 0.0;
    }
    let distance = (pose.x - notice.at.0).hypot(pose.z - notice.at.1);
    (1.0 - distance / GONE).clamp(0.0, 1.0)
}
