//! The fallen party's notice (presentation-ARCHITECTURE.md §9): when no member can fight, the panel
//! offers a restart, so the viewer never dead-ends. Monsters met or fought have the fight
//! screen (`shell/combat.rs`). `shell/panel.rs` shows it.

use super::notice::{Choice, Notice, Order};
use crate::sim::Views;

/// Whether the party can no longer fight: every member is down, dead or incapacitated.
#[must_use]
pub fn fallen(views: &Views) -> bool {
    !views.party.members.iter().any(|m| m.can_fight)
}

/// The fallen party's notice, or `None` while a member can still fight.
#[must_use]
pub fn notice(views: &Views) -> Option<Notice> {
    fallen(views).then(|| Notice {
        title: "The party has fallen".to_owned(),
        lines: Vec::new(),
        choices: vec![Choice {
            label: "Start again".to_owned(),
            order: Order::Restart,
            blocked: None,
        }],
    })
}
