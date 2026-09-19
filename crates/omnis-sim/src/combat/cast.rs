//! Casting in a fight: the checks a spell passes before any die (known, castable, affordable,
//! components in the stores, a target the effect reaches), the payment, and the effects that
//! land on monsters or members. Rust rolls; the `casting` slots add and compare.

use super::Roller;
use super::resolve::{hurt_individual, monster};
use super::state::CombatState;
use crate::command::Rejection;
use crate::event::{ActorRef, CheckKind, Event};
use crate::items::{consume, has_all};
use crate::party;
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::{CharacterId, ItemId, Pcg32, SpellId};
use omnis_data::{Data, Reach, Spell, SpellEffect};
use omnis_rules::{
    RollMode, RuleError, cantrip_dice, damage_roll, flags, heal_roll, monster_defenses,
    monster_save, needs_components, save_dc, saved_damage, spell_attack, spell_cost,
};
use serde::{Deserialize, Serialize};

/// Whom a spell goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Target {
    /// A stack, by its index in the encounter; the whole stack under `Reach::Stack`.
    Stack(u8),
    /// A member, by marching-order slot.
    Member(u8),
}

/// A cast that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CastPlan {
    /// The caster's slot.
    pub own: usize,
    /// The spell.
    pub spell: SpellId,
    /// Points it costs.
    pub cost: u32,
    /// Whom it goes to.
    pub target: Target,
}

/// The target-free half of validation: the spell a member knows at `index`, castable and
/// affordable, its components in the stores. What a picker shows as the reason a row is grey.
pub fn check<'a>(
    world: &World,
    data: &'a Data,
    own: usize,
    index: u8,
    rng: &mut Pcg32,
) -> Result<(SpellId, &'a Spell, u32), Rejection> {
    let member = world.party.members.get(own).ok_or(Rejection::NotYourTurn)?;
    let id = *member
        .known_spells
        .get(usize::from(index))
        .ok_or(Rejection::UnknownSpell { spell: index })?;
    let spell = data
        .spells
        .get(&id)
        .ok_or(Rejection::UnknownSpell { spell: index })?;
    match &spell.effect {
        Some(SpellEffect::Attack { .. } | SpellEffect::AutoHit { .. })
        | Some(SpellEffect::Save { .. } | SpellEffect::Heal { .. }) => {}
        _ => return Err(Rejection::NotCastable { spell: index }),
    }
    let cost = spell_cost(spell, data, rng).map_err(Rejection::Rule)?;
    if member.spell_points < cost {
        return Err(Rejection::NotEnoughPoints {
            need: cost,
            have: member.spell_points,
        });
    }
    if (needs_components(spell, data) && spell.components.is_empty())
        || !has_all(&world.party, &component_ids(data, spell))
    {
        return Err(Rejection::MissingComponents { spell: index });
    }
    Ok((id, spell, cost))
}

/// The spell's component list as interned ids; an id no pack defines never matches the stores.
fn component_ids(data: &Data, spell: &Spell) -> Vec<(ItemId, u16)> {
    spell
        .components
        .iter()
        .map(|(name, count)| {
            (
                data.registry.items.get(name).unwrap_or(ItemId(u32::MAX)),
                *count,
            )
        })
        .collect()
}

/// The whole validation: the checks above, then a target the effect reaches.
pub(crate) fn validate(
    state: &CombatState,
    world: &World,
    data: &Data,
    own: usize,
    index: u8,
    target: Target,
    rng: &mut Pcg32,
) -> Result<CastPlan, Rejection> {
    let (spell, def, cost) = check(world, data, own, index, rng)?;
    let effect = def
        .effect
        .as_ref()
        .ok_or(Rejection::NotCastable { spell: index })?;
    match (effect.targets_members(), target) {
        (true, Target::Member(slot)) => {
            let member = world
                .party
                .members
                .get(usize::from(slot))
                .ok_or(Rejection::NoSuchMember { index: slot })?;
            if super::state::is_dead(member, data) {
                return Err(Rejection::MemberDead { index: slot });
            }
        }
        (false, Target::Stack(stack)) => {
            let s = state
                .encounter
                .stacks
                .get(usize::from(stack))
                .ok_or(Rejection::NoSuchStack { stack })?;
            if !s.alive() {
                return Err(Rejection::StackDead { stack });
            }
        }
        _ => return Err(Rejection::WrongTarget),
    }
    Ok(CastPlan {
        own,
        spell,
        cost,
        target,
    })
}

/// Pay the points and the components, with the event. Validation guaranteed both.
pub(crate) fn pay(
    world: &mut World,
    data: &Data,
    plan: &CastPlan,
    events: &mut Vec<Event>,
) -> Result<CharacterId, RuleError> {
    let components = data
        .spells
        .get(&plan.spell)
        .map(|s| component_ids(data, s))
        .unwrap_or_default();
    consume(&mut world.party, &components)
        .map_err(|_| RuleError::new("cast", "components left the stores mid-cast"))?;
    let member = &mut world.party.members[plan.own];
    member.spell_points = member.spell_points.saturating_sub(plan.cost);
    events.push(Event::SpellCast {
        caster: member.id,
        spell: plan.spell,
        points: plan.cost,
        components_consumed: components,
    });
    Ok(member.id)
}

/// Resolve a paid cast against its target.
pub(crate) fn resolve(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    plan: &CastPlan,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let spell = data
        .spells
        .get(&plan.spell)
        .ok_or_else(|| RuleError::new("cast", "the spell vanished from the data"))?;
    let Some(effect) = &spell.effect else {
        return Ok(());
    };
    match (effect, plan.target) {
        (SpellEffect::Attack { dice, damage_type }, Target::Stack(stack)) => {
            let dice = scaled(world, data, plan, spell, *dice, roller)?;
            cast_attack(
                world,
                data,
                state,
                plan,
                stack,
                (dice, *damage_type),
                roller,
                events,
            )
        }
        (SpellEffect::AutoHit { dice, damage_type }, Target::Stack(stack)) => {
            let dice = scaled(world, data, plan, spell, *dice, roller)?;
            cast_auto(data, state, stack, (dice, *damage_type), roller, events)
        }
        (
            SpellEffect::Save {
                ability,
                dice,
                damage_type,
                half_on_save,
            },
            Target::Stack(stack),
        ) => {
            let dice = scaled(world, data, plan, spell, *dice, roller)?;
            let save = SaveSpell {
                ability: *ability,
                dice,
                damage_type: *damage_type,
                half_on_save: *half_on_save,
                reach: spell.reach,
            };
            cast_save(world, data, state, plan, stack, &save, roller, events)
        }
        (SpellEffect::Heal { dice, add_mod }, Target::Member(slot)) => {
            let caster = &world.party.members[plan.own];
            let heal = heal_roll(
                caster,
                data,
                *dice,
                *add_mod,
                &mut roller.rng,
                &roller.stream,
            )?;
            party::heal(
                world,
                data,
                usize::from(slot),
                heal.rolls,
                heal.amount,
                events,
            );
            Ok(())
        }
        _ => Ok(()),
    }
}

/// A cantrip's dice at the caster's level; a levelled spell's as written.
fn scaled(
    world: &World,
    data: &Data,
    plan: &CastPlan,
    spell: &Spell,
    dice: omnis_core::Dice,
    roller: &mut Roller,
) -> Result<omnis_core::Dice, RuleError> {
    if spell.level > 0 {
        return Ok(dice);
    }
    let caster = &world.party.members[plan.own];
    cantrip_dice(caster, data, dice, &mut roller.rng, &roller.stream)
}

fn push_damage(
    events: &mut Vec<Event>,
    target: ActorRef,
    kind: omnis_data::DamageType,
    damage: &omnis_rules::DamageRoll,
) {
    events.push(Event::Damage {
        target,
        kind,
        rolls: damage.rolls.clone(),
        raw: damage.raw,
        amount: damage.amount,
        adjust: damage.adjust,
    });
}

/// Damage with no roll to hit, on the lead individual.
fn cast_auto(
    data: &Data,
    state: &mut CombatState,
    stack: u8,
    (dice, damage_type): (omnis_core::Dice, omnis_data::DamageType),
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let target = ActorRef::Monster { stack, index: 0 };
    let monster = monster(data, &state.encounter.stacks[usize::from(stack)])?;
    let damage = damage_roll(
        data,
        Some(dice),
        0,
        false,
        monster_defenses(monster, damage_type),
        &mut roller.rng,
        &roller.stream,
    )?;
    push_damage(events, target, damage_type, &damage);
    hurt_individual(state, data, stack, 0, damage.amount, roller, events)
}

#[allow(clippy::too_many_arguments)]
fn cast_attack(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    plan: &CastPlan,
    stack: u8,
    (dice, damage_type): (omnis_core::Dice, omnis_data::DamageType),
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let caster = &world.party.members[plan.own];
    let target = ActorRef::Monster { stack, index: 0 };
    let monster = monster(data, &state.encounter.stacks[usize::from(stack)])?;
    let mode = RollMode::combine(
        false,
        flags(&caster.conditions, data).own_attacks_disadvantage,
    );
    let ac = i64::from(monster.ac);
    let roll = spell_attack(
        caster,
        data,
        ac,
        mode,
        None,
        &mut roller.rng,
        &roller.stream,
    )?;
    events.push(Event::AttackResolved {
        attacker: ActorRef::Member(caster.id),
        target,
        roll: roll.roll.clone(),
        ac,
        hit: roll.hit,
        crit: roll.crit,
    });
    if !roll.hit {
        return Ok(());
    }
    let damage = damage_roll(
        data,
        Some(dice),
        0,
        roll.crit,
        monster_defenses(monster, damage_type),
        &mut roller.rng,
        &roller.stream,
    )?;
    push_damage(events, target, damage_type, &damage);
    hurt_individual(state, data, stack, 0, damage.amount, roller, events)
}

/// A save-or-damage spell as resolved.
struct SaveSpell {
    ability: omnis_data::Ability,
    dice: omnis_core::Dice,
    damage_type: omnis_data::DamageType,
    half_on_save: bool,
    reach: Reach,
}

/// One damage roll, then each individual saves, from the last index down so a removal never
/// shifts a pending one.
#[allow(clippy::too_many_arguments)]
fn cast_save(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    plan: &CastPlan,
    stack: u8,
    save: &SaveSpell,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let caster = &world.party.members[plan.own];
    let dc = save_dc(caster, data, &mut roller.rng, &roller.stream)?;
    let stacks: Vec<u8> = match save.reach {
        Reach::One | Reach::Stack => alloc::vec![stack],
        Reach::AllStacks => (0..state.encounter.stacks.len())
            .rev()
            .filter(|i| state.encounter.stacks[*i].alive())
            .map(|i| u8::try_from(i).unwrap_or(u8::MAX))
            .collect(),
    };
    for stack in stacks {
        let monster = monster(data, &state.encounter.stacks[usize::from(stack)])?;
        let damage = damage_roll(
            data,
            Some(save.dice),
            0,
            false,
            monster_defenses(monster, save.damage_type),
            &mut roller.rng,
            &roller.stream,
        )?;
        let count = state.encounter.stacks[usize::from(stack)].hp.len();
        let individuals: Vec<usize> = match save.reach {
            Reach::One => alloc::vec![0],
            Reach::Stack | Reach::AllStacks => (0..count).rev().collect(),
        };
        for index in individuals {
            let actor = ActorRef::Monster {
                stack,
                index: u8::try_from(index).unwrap_or(u8::MAX),
            };
            let (roll, success) =
                monster_save(monster, save.ability, dc, &mut roller.rng, &roller.stream)?;
            events.push(Event::Check {
                actor,
                kind: CheckKind::Save(save.ability),
                roll: Some(roll),
                dc,
                success,
            });
            let amount = saved_damage(
                data,
                damage.amount,
                success,
                save.half_on_save,
                &mut roller.rng,
                &roller.stream,
            )?;
            events.push(Event::Damage {
                target: actor,
                kind: save.damage_type,
                rolls: damage.rolls.clone(),
                raw: damage.raw,
                amount,
                adjust: damage.adjust,
            });
            hurt_individual(state, data, stack, index, amount, roller, events)?;
        }
    }
    Ok(())
}
