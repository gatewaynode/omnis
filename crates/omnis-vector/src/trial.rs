//! Commands tried before they are offered: every choice a menu shows is run on a copy of the
//! world first, so a refused one is shown dim with the simulation's reason and never sent.
//! Bevy-free.

use crate::sim::SimWorld;
use omnis_sim::Command;
use omnis_sim::omnis_data::Data;

/// Why the simulation would refuse `command` now, found on a copy of the world.
#[must_use]
pub fn refusal(world: &SimWorld, data: &Data, command: &Command) -> Option<String> {
    world.trial(data, command).err().map(|r| r.to_string())
}

/// Whether the simulation would accept `command` now.
#[must_use]
pub fn accepted(world: &SimWorld, data: &Data, command: &Command) -> bool {
    world.trial(data, command).is_ok()
}
