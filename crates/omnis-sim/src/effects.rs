//! Active effects on members and on the party: applying, expiring on the clock, ending with
//! concentration, spending a consumed buff, and clearing next-turn effects. The vectors hold
//! only live effects: `prune` runs whenever the clock moves, so readers never need the time.

use crate::event::{EffectEnd, EffectTarget, Event};
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::{CharacterId, SpellId};
use omnis_data::BuffOn;
use omnis_rules::{ActiveEffect, Expiry, consumable_index};

/// Drop every timed effect the party clock has passed.
pub(crate) fn prune(world: &mut World, events: &mut Vec<Event>) {
    let now = world.party_clock().elapsed;
    for member in &mut world.party.members {
        let target = EffectTarget::Member(member.id);
        member.effects.retain(|e| {
            let gone = e.expired(now);
            if gone {
                events.push(Event::EffectEnded {
                    target,
                    spell: e.source,
                    why: EffectEnd::Expired,
                });
            }
            !gone
        });
    }
    world.party.effects.retain(|e| {
        let gone = e.expired(now);
        if gone {
            events.push(Event::EffectEnded {
                target: EffectTarget::Party,
                spell: e.source,
                why: EffectEnd::Expired,
            });
        }
        !gone
    });
}

/// Put an effect on a member.
pub(crate) fn apply_to_member(
    world: &mut World,
    index: usize,
    effect: ActiveEffect,
    events: &mut Vec<Event>,
) {
    let member = &mut world.party.members[index];
    events.push(Event::EffectApplied {
        target: EffectTarget::Member(member.id),
        spell: effect.source,
        caster: effect.caster,
    });
    member.effects.push(effect);
}

/// Put an effect on the party.
pub(crate) fn apply_to_party(world: &mut World, effect: ActiveEffect, events: &mut Vec<Event>) {
    events.push(Event::EffectApplied {
        target: EffectTarget::Party,
        spell: effect.source,
        caster: effect.caster,
    });
    world.party.effects.push(effect);
}

/// The spell a caster is concentrating on, if any.
#[must_use]
pub fn concentrating(world: &World, caster: CharacterId) -> Option<SpellId> {
    world
        .party
        .members
        .iter()
        .flat_map(|m| m.effects.iter())
        .chain(world.party.effects.iter())
        .find(|e| e.concentration && e.caster == caster)
        .map(|e| e.source)
}

/// End every effect the caster is concentrating on, with the events.
pub(crate) fn end_concentration(
    world: &mut World,
    caster: CharacterId,
    events: &mut Vec<Event>,
) -> Option<SpellId> {
    let spell = concentrating(world, caster)?;
    let ended = |e: &ActiveEffect| e.concentration && e.caster == caster;
    for member in &mut world.party.members {
        let target = EffectTarget::Member(member.id);
        member.effects.retain(|e| {
            if ended(e) {
                events.push(Event::EffectEnded {
                    target,
                    spell: e.source,
                    why: EffectEnd::Concentration,
                });
            }
            !ended(e)
        });
    }
    world.party.effects.retain(|e| {
        if ended(e) {
            events.push(Event::EffectEnded {
                target: EffectTarget::Party,
                spell: e.source,
                why: EffectEnd::Concentration,
            });
        }
        !ended(e)
    });
    events.push(Event::Concentration {
        caster,
        spell,
        ended: true,
    });
    Some(spell)
}

/// Spend the first consumed buff on this kind of roll, when one is in force.
pub(crate) fn consume(world: &mut World, index: usize, on: BuffOn, events: &mut Vec<Event>) {
    let member = &mut world.party.members[index];
    if let Some(at) = consumable_index(&member.effects, on) {
        let effect = member.effects.remove(at);
        events.push(Event::EffectEnded {
            target: EffectTarget::Member(member.id),
            spell: effect.source,
            why: EffectEnd::Consumed,
        });
    }
}

/// Clear a member's next-turn effects (their turn began), or everyone's when the fight ends.
pub(crate) fn clear_next_turn(
    world: &mut World,
    bearer: Option<CharacterId>,
    why: EffectEnd,
    events: &mut Vec<Event>,
) {
    for member in &mut world.party.members {
        if bearer.is_some_and(|id| id != member.id) {
            continue;
        }
        let target = EffectTarget::Member(member.id);
        member.effects.retain(|e| {
            let gone = e.until == Expiry::NextTurn;
            if gone {
                events.push(Event::EffectEnded {
                    target,
                    spell: e.source,
                    why,
                });
            }
            !gone
        });
    }
}
