//! HUD text for simulation events.

use omnis_sim::{BlockReason, Event};

/// A HUD line for the events worth showing; the per-step chatter is left out.
#[must_use]
pub fn describe(event: &Event) -> Option<String> {
    Some(match event {
        Event::Moved { to, .. } => format!("Moved to ({}, {}) facing {:?}", to.x, to.y, to.facing),
        Event::Blocked { reason } => format!(
            "Blocked: {}",
            match reason {
                BlockReason::Wall => "a wall",
                BlockReason::ClosedDoor => "a closed door",
                BlockReason::Impassable => "impassable ground",
                BlockReason::MapEdge => "the map edge",
            }
        ),
        Event::Door { open, .. } => if *open {
            "The door opens"
        } else {
            "The door closes"
        }
        .to_owned(),
        Event::EncounterStarted { .. } => "Monsters ahead!".to_owned(),
        Event::Message { key } => format!("{key:?}"),
        _ => return None,
    })
}
