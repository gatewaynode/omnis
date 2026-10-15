//! What an interaction does (M8.5 step 2). Requirements that need no die are tried first, and
//! any that fails refuses the command with the world untouched. Then each check is rolled in
//! order by the member best at it, on a copy of the `object` stream: the first to fail ends the
//! attempt, which spends its minutes and says the kind's failure text but uses up nothing.
//! Only a success uses up items, marks a `once`, and does the effects in order.

use super::{Refusal, edit};
use crate::INTERACT_MINUTES;
use crate::apply::advance;
use crate::checks::{self, CheckSpec};
use crate::combat::Roller;
use crate::combat::state::can_fight;
use crate::command::Rejection;
use crate::event::{ActorRef, CheckKind, Event, ItemPlace};
use crate::items::{add_to, count_of, take_from};
use crate::names::id_of;
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::ItemId;
use omnis_data::omnis_expr::Value;
use omnis_data::placement::ResolvedPlacement;
use omnis_data::{Data, Interaction, ObjectEffect, Requirement};
use omnis_rules::{Character, RollMode, RuleError};

/// Resolve interaction `index` of the placement's kind, already found to be offered.
pub(super) fn resolve(
    world: &mut World,
    data: &Data,
    placement: &ResolvedPlacement,
    index: u8,
    interaction: &Interaction,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let mut roller = Roller::take_stream(world, "object");
    for requirement in &interaction.requires {
        gate(world, data, requirement, &mut roller)?;
    }
    let mut success = true;
    for requirement in &interaction.requires {
        let Some((spec, kind, dc)) = check_of(requirement) else {
            continue;
        };
        let member = checks::best(world, data, spec).ok_or(Rejection::VerbRefused {
            reason: Refusal::NoneCanAct,
        })?;
        let roll = checks::roll(world, data, member, spec, &mut roller, events)
            .map_err(Rejection::Rule)?;
        let passed = roll.total >= dc;
        events.push(Event::Check {
            actor: ActorRef::Member(world.party.members[member].id),
            kind,
            roll: Some(roll),
            dc,
            success: passed,
        });
        if !passed {
            success = false;
            break;
        }
    }
    let object = placement.id.clone();
    events.push(Event::ObjectUsed {
        object: object.clone(),
        verb: interaction.verb,
        success,
    });
    if success {
        spend(world, data, &interaction.requires, events);
        if interaction.once {
            edit(world, world.position.map, &object, |s| {
                s.used.insert(index);
            });
        }
        for effect in &interaction.effects {
            effect_of(world, data, placement, effect, events);
        }
    } else if let Some(key) = &interaction.failure {
        events.push(Event::ObjectMessage {
            object,
            key: key.clone(),
        });
    }
    roller.store(world);
    advance(
        world,
        data,
        interaction.minutes.unwrap_or(INTERACT_MINUTES),
        events,
    );
    Ok(())
}

/// A requirement that needs no die: refuse when it does not hold.
fn gate(
    world: &World,
    data: &Data,
    requirement: &Requirement,
    roller: &mut Roller,
) -> Result<(), Rejection> {
    let refused = |reason| Err(Rejection::VerbRefused { reason });
    let able = || world.party.members.iter().filter(|m| can_fight(m, data));
    match requirement {
        Requirement::Item { id, consume } => {
            let held = data
                .registry
                .items
                .get(id)
                .is_some_and(|item| source(world, item, *consume).is_some());
            if !held {
                return refused(Refusal::Lacks { item: id.clone() });
            }
        }
        Requirement::Flag { name, min } => {
            let value = data
                .registry
                .flags
                .get(name)
                .and_then(|flag| world.flags.get(&flag).copied())
                .unwrap_or(0);
            if value < *min {
                return refused(Refusal::Flag { flag: name.clone() });
            }
        }
        Requirement::School(school) => {
            let knows = able().any(|m| {
                m.known_spells
                    .iter()
                    .any(|s| data.spells.get(s).is_some_and(|s| s.school == *school))
            });
            if !knows {
                return refused(Refusal::School { school: *school });
            }
        }
        Requirement::Rule { slot } => {
            let levels: Vec<i64> = able().map(|m| i64::from(m.level)).collect();
            let size = i64::try_from(levels.len()).unwrap_or(i64::MAX);
            let level = levels.iter().sum::<i64>() / size.max(1);
            let outcome = data
                .rules
                .eval(
                    slot,
                    &[
                        ("party_level", Value::Int(level)),
                        ("party_size", Value::Int(size)),
                    ],
                    &mut roller.rng,
                    &roller.stream,
                )
                .map_err(Rejection::Rule)?;
            match outcome.value {
                Value::Bool(true) => {}
                Value::Bool(false) => return refused(Refusal::Rule { rule: slot.clone() }),
                Value::Int(_) => {
                    return Err(Rejection::Rule(RuleError::new(
                        slot,
                        "an object's rule must answer true or false",
                    )));
                }
            }
        }
        Requirement::Skill { .. } | Requirement::Ability { .. } => {}
    }
    Ok(())
}

/// The check a requirement rolls, its kind in the event, and its difficulty.
fn check_of(requirement: &Requirement) -> Option<(CheckSpec, CheckKind, i64)> {
    let mode = RollMode::Normal;
    match *requirement {
        Requirement::Skill { skill, dc } => Some((
            CheckSpec {
                skill: Some(skill),
                ability: skill.ability(),
                mode,
            },
            CheckKind::Skill(skill),
            i64::from(dc),
        )),
        Requirement::Ability { ability, dc } => Some((
            CheckSpec {
                skill: None,
                ability,
                mode,
            },
            CheckKind::Ability(ability),
            i64::from(dc),
        )),
        _ => None,
    }
}

/// Where the party has an item from: the stores first, then members' kits in marching order.
/// One to use up must be a spare, not one a member wears or wields.
fn source(world: &World, item: ItemId, spare: bool) -> Option<ItemPlace> {
    if count_of(&world.party.inventory, item) > 0 {
        return Some(ItemPlace::Stores);
    }
    world
        .party
        .members
        .iter()
        .find(|m| count_of(&m.equipment, item) > if spare { worn(m, item) } else { 0 })
        .map(|m| ItemPlace::Member(m.id))
}

/// How many of an item a member wears or wields.
fn worn(member: &Character, item: ItemId) -> u16 {
    let n = member.equipped.values().filter(|i| **i == item).count();
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// Use up one of each item a successful interaction consumes.
fn spend(world: &mut World, data: &Data, requires: &[Requirement], events: &mut Vec<Event>) {
    for requirement in requires {
        let Requirement::Item { id, consume: true } = requirement else {
            continue;
        };
        let Some(item) = data.registry.items.get(id) else {
            continue;
        };
        let Some(from) = source(world, item, true) else {
            continue;
        };
        let party = &mut world.party;
        let list = match from {
            ItemPlace::Stores => &mut party.inventory,
            ItemPlace::Member(id) => match party.members.iter_mut().find(|m| m.id == id) {
                Some(m) => &mut m.equipment,
                None => continue,
            },
            // `source` names only what the party carries.
            ItemPlace::Ground | ItemPlace::Object(_) => continue,
        };
        if take_from(list, item, 1).is_ok() {
            events.push(Event::ItemSpent {
                item: id.clone(),
                count: 1,
                from,
            });
        }
    }
}

/// Do one effect of a successful interaction.
fn effect_of(
    world: &mut World,
    data: &Data,
    placement: &ResolvedPlacement,
    effect: &ObjectEffect,
    events: &mut Vec<Event>,
) {
    let map = world.position.map;
    let object = || placement.id.clone();
    match effect {
        ObjectEffect::SetState(state) => {
            let start = placement.state == *state;
            edit(world, map, &placement.id, |s| {
                s.state = (!start).then(|| state.clone());
            });
            events.push(Event::ObjectState {
                object: object(),
                state: state.clone(),
            });
        }
        ObjectEffect::SetFlag { name, value } => {
            if let Some(flag) = data.registry.flags.get(name) {
                world.flags.insert(flag, *value);
            }
        }
        ObjectEffect::Message(key) => events.push(Event::ObjectMessage {
            object: object(),
            key: key.clone(),
        }),
        ObjectEffect::GiveGold(amount) => {
            world.party.gold = world.party.gold.saturating_add(*amount);
            events.push(Event::GoldFound { amount: *amount });
        }
        ObjectEffect::GiveItem { id, count } => {
            if let Some(item) = data.registry.items.get(id) {
                add_to(&mut world.party.inventory, item, *count);
                events.push(Event::ItemFound {
                    item: id_of(&data.registry.items, item),
                    count: *count,
                });
            }
        }
        ObjectEffect::Remove => {
            edit(world, map, &placement.id, |s| s.removed = true);
            events.push(Event::ObjectRemoved { object: object() });
        }
    }
}
