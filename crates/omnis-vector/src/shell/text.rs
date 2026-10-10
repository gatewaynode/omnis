//! HUD text for simulation events.

use crate::labels;
use omnis_sim::omnis_data::Data;
use omnis_sim::{BlockReason, CombatOutcome, Event};

/// A HUD line for the events worth showing; the per-step chatter is left out.
#[must_use]
pub fn describe(event: &Event, data: &Data) -> Option<String> {
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
        Event::Bribed { cost } => format!("Paid {}; the monsters leave", labels::gold(*cost)),
        Event::CombatStarted { .. } => "The fight begins".to_owned(),
        Event::CombatEnded {
            outcome, xp, gold, ..
        } => match outcome {
            CombatOutcome::Victory => format!("Victory: {xp} xp each, {}", labels::gold(*gold)),
            CombatOutcome::Fled => "The party got away".to_owned(),
            CombatOutcome::Defeat => "The party has fallen".to_owned(),
        },
        Event::Message { key } => labels::message(data, *key).to_owned(),
        _ => return None,
    })
}
