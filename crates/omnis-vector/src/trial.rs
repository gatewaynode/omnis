//! Commands tried before they are offered: every choice a menu shows is run on a copy of the
//! world first, so a refused one is shown dim with the simulation's reason and never sent.
//! Bevy-free.

use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, World, apply};

/// Why the simulation would refuse `command` now, found on a copy of the world.
#[must_use]
pub fn refusal(world: &World, data: &Data, command: &Command) -> Option<String> {
    apply(&mut world.clone(), data, command.clone())
        .err()
        .map(|r| r.to_string())
}

/// Whether the simulation would accept `command` now.
#[must_use]
pub fn accepted(world: &World, data: &Data, command: &Command) -> bool {
    apply(&mut world.clone(), data, command.clone()).is_ok()
}
