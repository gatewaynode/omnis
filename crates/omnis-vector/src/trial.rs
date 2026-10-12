//! Commands tried before they are offered: every choice a menu shows is run on a copy of the
//! world first, so a refused one is shown dim with the simulation's reason and never sent.
//! Bevy-free.

use crate::labels;
use crate::sim::SimWorld;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, PartyView};

/// Why the simulation would refuse `command` now, in the few words of a dim control
/// (`labels::reason`).
#[must_use]
pub fn why(world: &SimWorld, data: &Data, party: &PartyView, command: &Command) -> Option<String> {
    world
        .trial(data, command)
        .err()
        .map(|r| labels::reason(party, &r))
}

/// Whether the simulation would accept `command` now.
#[must_use]
pub fn accepted(world: &SimWorld, data: &Data, command: &Command) -> bool {
    world.trial(data, command).is_ok()
}
