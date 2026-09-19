//! The one way simulation code rolls an ability check: `stats::check` with the member's
//! effects, then the guidance die it used is spent. Hide, Run, Flee, and Sense come here, never
//! to `stats::check` directly, so a consumed buff is consumed everywhere.

use crate::combat::Roller;
use crate::effects;
use crate::event::Event;
use crate::world::World;
use alloc::vec::Vec;
use omnis_data::{Ability, BuffOn, Data, Skill};
use omnis_rules::{Roll, RollMode, RuleError, check};

/// What a check is for.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckSpec {
    /// The skill, when one applies.
    pub skill: Option<Skill>,
    /// The ability.
    pub ability: Ability,
    /// Advantage or disadvantage.
    pub mode: RollMode,
}

/// Roll a check for the member in `index`, spending a consumed buff it used.
pub(crate) fn roll(
    world: &mut World,
    data: &Data,
    index: usize,
    spec: CheckSpec,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<Roll, RuleError> {
    let roll = check(
        &world.party.members[index],
        data,
        spec.skill,
        spec.ability,
        spec.mode,
        &mut roller.rng,
        &roller.stream,
    )?;
    if roll.bonus.is_some() {
        effects::consume(world, index, BuffOn::AbilityChecks, events);
    }
    Ok(roll)
}
