//! Reactions the simulation casts for a member who opted in: the shield spell when a hit
//! would land that its armor bonus turns into a miss (an SRD deviation named in the docs: the
//! SRD casts it on any hit).

use super::Roller;
use crate::effects;
use crate::event::Event;
use crate::world::World;
use alloc::vec::Vec;
use omnis_data::{Data, SpellEffect};
use omnis_rules::{
    ActiveEffect, AttackRoll, EffectKind, Expiry, RuleError, armor_class, rejudge, spell_cost,
};

/// Cast shield for the member if it turns this hit into a miss and no shield is up already
/// (one lasts until their next turn); the roll is rejudged in place.
pub(crate) fn try_shield(
    world: &mut World,
    data: &Data,
    index: usize,
    roll: &mut AttackRoll,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<bool, RuleError> {
    if !roll.hit || roll.crit {
        return Ok(false);
    }
    let member = &world.party.members[index];
    if member.is_down() {
        return Ok(false);
    }
    if member
        .effects
        .iter()
        .any(|e| matches!(e.kind, EffectKind::ArmorBonus(_)))
    {
        return Ok(false);
    }
    let Some((spell, bonus)) = member
        .auto_cast
        .iter()
        .find_map(|id| match data.spells.get(id) {
            Some(s) => match s.effect {
                Some(SpellEffect::Reaction { armor_bonus }) => Some((*id, armor_bonus)),
                _ => None,
            },
            None => None,
        })
    else {
        return Ok(false);
    };
    let def = &data.spells[&spell];
    let cost = spell_cost(def, data, &mut roller.rng)?;
    if member.spell_points < cost {
        return Ok(false);
    }
    let ac = armor_class(member, data) + bonus;
    let mut probe = roll.clone();
    rejudge(data, &mut probe, ac, &mut roller.rng, &roller.stream)?;
    if probe.hit {
        return Ok(false);
    }
    let caster = member.id;
    let member = &mut world.party.members[index];
    member.spell_points -= cost;
    events.push(Event::SpellCast {
        caster,
        spell,
        points: cost,
        components_consumed: Vec::new(),
    });
    effects::apply_to_member(
        world,
        index,
        ActiveEffect {
            source: spell,
            caster,
            concentration: def.concentration,
            kind: EffectKind::ArmorBonus(bonus),
            until: Expiry::NextTurn,
        },
        events,
    );
    *roll = probe;
    Ok(true)
}
