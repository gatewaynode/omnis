//! The fallen party's notice (presentation-ARCHITECTURE.md §9): when no member can fight, the panel
//! offers a restart, so the viewer never dead-ends. Monsters met or fought have the fight
//! screen (`shell/combat.rs`). `shell/panel.rs` shows it.

use super::notice::{Choice, Notice, Order};
use omnis_sim::World;
use omnis_sim::combat::state::can_fight;
use omnis_sim::omnis_data::Data;

/// Whether the party can no longer fight: every member is down, dead or incapacitated.
#[must_use]
pub fn fallen(world: &World, data: &Data) -> bool {
    !world.party.members.iter().any(|m| can_fight(m, data))
}

/// The fallen party's notice, or `None` while a member can still fight.
#[must_use]
pub fn notice(world: &World, data: &Data) -> Option<Notice> {
    fallen(world, data).then(|| Notice {
        title: "The party has fallen".to_owned(),
        lines: Vec::new(),
        choices: vec![Choice {
            label: "Start again".to_owned(),
            order: Order::Restart,
            blocked: None,
        }],
    })
}
