//! Casting outside a fight: healing, buffs, light, and mage hand, on the `cast` stream, for
//! `cast_minutes` of the party's clock.

use crate::apply::advance;
use crate::combat::cast::{self, Aim, CastPlan, Target};
use crate::combat::{Roller, state};
use crate::command::Rejection;
use crate::event::Event;
use crate::party;
use crate::utility::open_door_ahead;
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::{Data, SpellEffect, Utility};
use omnis_rules::heal_roll;

/// Minutes a cast costs when the rules do not say.
const DEFAULT_CAST_MINUTES: u32 = 1;

/// Apply an explore-mode cast: validation on a copy of the `cast` stream, then payment,
/// the effect, and the minutes.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    caster: CharacterId,
    spell: u8,
    target: Target,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let own = usize::from(world.party.slot_of(caster)?);
    let member = &world.party.members[own];
    if state::is_dead(member, data) {
        return Err(Rejection::MemberDead { member: caster });
    }
    if member.is_down() {
        return Err(Rejection::MemberDown { member: caster });
    }
    let mut roller = Roller::take_stream(world, "cast");
    let (id, def, cost) = cast::check(world, data, own, spell, false, &mut roller.rng)?;
    let effect = def.effect.clone().ok_or(Rejection::NotCastable { spell })?;
    if let SpellEffect::Heal { .. } | SpellEffect::Buff { .. } = effect {
        let Target::Member(id) = target else {
            return Err(Rejection::WrongTarget);
        };
        let member = &world.party.members[usize::from(world.party.slot_of(id)?)];
        if state::is_dead(member, data) {
            return Err(Rejection::MemberDead { member: id });
        }
    }
    let aim = match target {
        Target::Member(id) => Aim::Member(usize::from(world.party.slot_of(id)?)),
        Target::Stack(stack) => Aim::Stack(stack),
    };
    let plan = CastPlan {
        own,
        spell: id,
        cost,
        target: aim,
    };
    cast::pay(world, data, &plan, events).map_err(Rejection::Rule)?;
    match (effect, plan.target) {
        (SpellEffect::Heal { dice, add_mod }, Aim::Member(slot)) => {
            let caster = &world.party.members[own];
            let heal = heal_roll(caster, data, dice, add_mod, &mut roller.rng, &roller.stream)
                .map_err(Rejection::Rule)?;
            party::heal(world, data, slot, heal.rolls, heal.amount, events);
        }
        (SpellEffect::Buff { .. }, Aim::Member(slot)) => {
            cast::cast_buff(world, data, &plan, slot, events);
        }
        (SpellEffect::Light { depth, minutes }, _) => {
            cast::cast_light(world, &plan, def, depth, minutes, events);
        }
        (SpellEffect::Utility(Utility::OpenDoor { range_tiles }), _) => {
            open_door_ahead(world, data, range_tiles, events);
        }
        _ => {}
    }
    roller.store(world);
    let minutes = data
        .rules
        .value("cast_minutes")
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(DEFAULT_CAST_MINUTES);
    advance(world, data, minutes, events);
    Ok(())
}
