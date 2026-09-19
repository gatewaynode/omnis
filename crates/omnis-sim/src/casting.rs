//! Casting outside a fight: healing, buffs, light, and mage hand, on the `cast` stream, for
//! `cast_minutes` of the party's clock.

use crate::apply::advance;
use crate::combat::cast::{self, CastPlan, Target};
use crate::combat::{Roller, state};
use crate::command::Rejection;
use crate::event::Event;
use crate::party;
use crate::utility::open_door_ahead;
use crate::world::World;
use alloc::vec::Vec;
use omnis_data::{Data, SpellEffect, Utility};
use omnis_rules::heal_roll;

/// Minutes a cast costs when the rules do not say.
const DEFAULT_CAST_MINUTES: u32 = 1;

/// Apply an explore-mode cast: validation on a copy of the `cast` stream, then payment,
/// the effect, and the minutes.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    caster: u8,
    spell: u8,
    target: Target,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let own = usize::from(caster);
    let member = world
        .party
        .members
        .get(own)
        .ok_or(Rejection::NoSuchMember { index: caster })?;
    if state::is_dead(member, data) {
        return Err(Rejection::MemberDead { index: caster });
    }
    if member.is_down() {
        return Err(Rejection::MemberDown { index: caster });
    }
    let mut roller = Roller::take_stream(world, "cast");
    let (id, def, cost) = cast::check(world, data, own, spell, false, &mut roller.rng)?;
    let effect = def.effect.clone().ok_or(Rejection::NotCastable { spell })?;
    if let SpellEffect::Heal { .. } | SpellEffect::Buff { .. } = effect {
        let Target::Member(slot) = target else {
            return Err(Rejection::WrongTarget);
        };
        let member = world
            .party
            .members
            .get(usize::from(slot))
            .ok_or(Rejection::NoSuchMember { index: slot })?;
        if state::is_dead(member, data) {
            return Err(Rejection::MemberDead { index: slot });
        }
    }
    let plan = CastPlan {
        own,
        spell: id,
        cost,
        target,
    };
    cast::pay(world, data, &plan, events).map_err(Rejection::Rule)?;
    match (effect, target) {
        (SpellEffect::Heal { dice, add_mod }, Target::Member(slot)) => {
            let caster = &world.party.members[own];
            let heal = heal_roll(caster, data, dice, add_mod, &mut roller.rng, &roller.stream)
                .map_err(Rejection::Rule)?;
            party::heal(
                world,
                data,
                usize::from(slot),
                heal.rolls,
                heal.amount,
                events,
            );
        }
        (SpellEffect::Buff { .. }, Target::Member(slot)) => {
            cast::cast_buff(world, data, &plan, usize::from(slot), events);
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
    advance(world, minutes, events);
    Ok(())
}
