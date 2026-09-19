//! What one attack does: the roll, the damage, a monster's death and its gold, a member's
//! fall to zero, death saves, instant death, and the burial at the end.

use super::Roller;
use super::reaction::try_shield;
use super::state::{CombatState, is_dead};
use crate::effects;
use crate::encounter::Stack;
use crate::event::{ActorRef, CheckKind, Event};
use crate::items::add_to;
use crate::party::{self, set_condition};
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::{Ability, Attack, BuffOn, Data, Monster};
use omnis_rules::{
    AttackBonus, Character, DeathSaveResult, DeathSaves, RollMode, RuleError, Weapon, armor_class,
    attack_bonus, attack_roll, attack_roll_with, choose_target, concentration_dc, damage_roll,
    death_save, flags, member_defenses, monster_defenses, pick_attack, roll_bonus, save,
    wound_at_zero,
};

/// The stat block of a stack, or the rule error a hand-edited save would cause.
pub(crate) fn monster<'a>(data: &'a Data, stack: &Stack) -> Result<&'a Monster, RuleError> {
    data.monsters
        .get(&stack.monster)
        .ok_or_else(|| RuleError::new("combat", "a stack names an unknown monster"))
}

/// A member attacks the lead individual of a stack.
#[allow(clippy::too_many_arguments)]
pub(crate) fn member_attacks(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    actor: CharacterId,
    stack: u8,
    weapon: &Weapon,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let Some(member) = world.party.members.iter().find(|m| m.id == actor) else {
        return Ok(());
    };
    let target = ActorRef::Monster { stack, index: 0 };
    let monster = monster(data, &state.encounter.stacks[usize::from(stack)])?;
    let (bonus, proficiency) = attack_bonus(member, data, weapon)?;
    let mode = RollMode::combine(
        false,
        flags(&member.conditions, data).own_attacks_disadvantage,
    );
    let ac = i64::from(monster.ac);
    let extra = roll_bonus(
        &member.effects,
        BuffOn::AttackRolls,
        &mut roller.rng,
        &roller.stream,
    )?;
    let with = AttackBonus {
        modifier: bonus,
        proficiency,
        extra,
    };
    let roll = attack_roll_with(data, with, ac, mode, &mut roller.rng, &roller.stream)?;
    events.push(Event::AttackResolved {
        attacker: ActorRef::Member(actor),
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
        weapon.damage,
        bonus,
        roll.crit,
        monster_defenses(monster, weapon.damage_type),
        &mut roller.rng,
        &roller.stream,
    )?;
    events.push(Event::Damage {
        target,
        kind: weapon.damage_type,
        rolls: damage.rolls.clone(),
        raw: damage.raw,
        amount: damage.amount,
        adjust: damage.adjust,
    });
    hurt_stack(state, data, stack, damage.amount, roller, events)
}

/// Damage to a stack's lead individual; at zero it dies and drops its gold.
fn hurt_stack(
    state: &mut CombatState,
    data: &Data,
    stack: u8,
    amount: i64,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    hurt_individual(state, data, stack, 0, amount, roller, events)
}

/// Damage to one individual of a stack; at zero it dies, leaves the stack, and drops its gold.
pub(crate) fn hurt_individual(
    state: &mut CombatState,
    data: &Data,
    stack: u8,
    index: usize,
    amount: i64,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let s = &mut state.encounter.stacks[usize::from(stack)];
    let Some(hp) = s.hp.get_mut(index) else {
        return Ok(());
    };
    *hp = hp.saturating_sub(i32::try_from(amount).unwrap_or(i32::MAX));
    if *hp > 0 {
        return Ok(());
    }
    s.hp.remove(index);
    let gold = match monster(data, s)?.gold {
        Some(dice) => {
            let trace = dice
                .roll(&mut roller.rng, &roller.stream)
                .map_err(|e| RuleError::new("gold", alloc::format!("{e}")))?;
            state.gold = state
                .gold
                .saturating_add(u32::try_from(trace.total.max(0)).unwrap_or(u32::MAX));
            Some(trace)
        }
        None => None,
    };
    events.push(Event::Death {
        target: ActorRef::Monster {
            stack,
            index: u8::try_from(index).unwrap_or(u8::MAX),
        },
        gold,
    });
    Ok(())
}

/// Every living individual of a stack attacks once: a front stack in melee against the front
/// row (the back row once the front has fallen), a back stack with a ranged attack against
/// anyone, or the stack waits.
pub(crate) fn monster_turn(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    stack: u8,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let s = &state.encounter.stacks[usize::from(stack)];
    let front = state.is_front(data, stack);
    let Some(attack) = pick_attack(monster(data, s)?, !front).cloned() else {
        events.push(Event::Waited {
            actor: ActorRef::Stack(stack),
        });
        return Ok(());
    };
    let front_row = party::front_row(data);
    for index in 0..s.hp.len() {
        if state.outcome(&world.party, data).is_some() {
            break;
        }
        let living: Vec<usize> = world
            .party
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.is_down())
            .map(|(i, _)| i)
            .collect();
        let candidates: Vec<usize> = if attack.ranged {
            living
        } else {
            let in_front: Vec<usize> = living.iter().copied().filter(|i| *i < front_row).collect();
            if in_front.is_empty() {
                living
            } else {
                in_front
            }
        };
        let Some(pick) = choose_target(candidates.len(), &mut roller.rng) else {
            break;
        };
        let attacker = ActorRef::Monster {
            stack,
            index: u8::try_from(index).unwrap_or(u8::MAX),
        };
        monster_attacks_member(
            world,
            data,
            state,
            attacker,
            candidates[pick],
            &attack,
            roller,
            events,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn monster_attacks_member(
    world: &mut World,
    data: &Data,
    state: &CombatState,
    attacker: ActorRef,
    member_index: usize,
    attack: &Attack,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let member = &world.party.members[member_index];
    let target = ActorRef::Member(member.id);
    let condition_flags = flags(&member.conditions, data);
    let dodging = state.dodging.binary_search(&member.id).is_ok();
    let mode = RollMode::combine(condition_flags.attacks_against_advantage, dodging);
    let ac = armor_class(member, data);
    let defenses = member_defenses(member, data, attack.damage_type);
    let mut roll = attack_roll(
        data,
        i64::from(attack.to_hit),
        0,
        ac,
        mode,
        &mut roller.rng,
        &roller.stream,
    )?;
    try_shield(world, data, member_index, &mut roll, roller, events)?;
    let crit = roll.crit || (roll.hit && condition_flags.melee_hits_crit && !attack.ranged);
    events.push(Event::AttackResolved {
        attacker,
        target,
        roll: roll.roll.clone(),
        ac: roll.ac,
        hit: roll.hit,
        crit,
    });
    if !roll.hit {
        return Ok(());
    }
    let damage = damage_roll(
        data,
        Some(attack.damage),
        0,
        crit,
        defenses,
        &mut roller.rng,
        &roller.stream,
    )?;
    events.push(Event::Damage {
        target,
        kind: attack.damage_type,
        rolls: damage.rolls.clone(),
        raw: damage.raw,
        amount: damage.amount,
        adjust: damage.adjust,
    });
    hurt_member(world, data, member_index, damage.amount, crit, events);
    keep_concentration(world, data, member_index, damage.amount, roller, events)
}

/// A concentrating member who took damage saves on Constitution against `concentration.dc`
/// or loses the spell.
fn keep_concentration(
    world: &mut World,
    data: &Data,
    index: usize,
    damage: i64,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let member = &world.party.members[index];
    if damage <= 0 || member.is_down() || effects::concentrating(world, member.id).is_none() {
        return Ok(());
    }
    let dc = concentration_dc(data, damage, &mut roller.rng, &roller.stream)?;
    let roll = save(
        member,
        data,
        Ability::Constitution,
        RollMode::Normal,
        &mut roller.rng,
        &roller.stream,
    )?;
    let success = roll.total >= dc;
    let id = member.id;
    events.push(Event::Check {
        actor: ActorRef::Member(id),
        kind: CheckKind::Save(Ability::Constitution),
        roll: Some(roll),
        dc,
        success,
    });
    if !success {
        effects::end_concentration(world, id, events);
    }
    Ok(())
}

/// Damage to a member: down at zero, dead when the rest reaches the maximum, a failed death
/// save when already down.
fn hurt_member(
    world: &mut World,
    data: &Data,
    index: usize,
    amount: i64,
    crit: bool,
    events: &mut Vec<Event>,
) {
    let amount = i32::try_from(amount).unwrap_or(i32::MAX);
    let member = &mut world.party.members[index];
    if is_dead(member, data) || amount <= 0 {
        return;
    }
    if member.hp > 0 {
        let remaining = amount.saturating_sub(member.hp);
        member.hp = (member.hp - amount).max(0);
        if member.hp > 0 {
            return;
        }
        if remaining >= member.hp_max {
            die(member, data, events);
        } else {
            member.death_saves = DeathSaves::default();
            events.push(Event::Down { target: member.id });
            set_condition(member, data, "unconscious", true, events);
        }
        return;
    }
    if amount >= member.hp_max {
        die(member, data, events);
        return;
    }
    let result = wound_at_zero(&mut member.death_saves, crit);
    events.push(Event::Wounded {
        member: member.id,
        failures: member.death_saves.failures,
    });
    if result == DeathSaveResult::Died {
        die(member, data, events);
    }
}

/// A death saving throw at the end of a round for a member lying at zero.
pub(crate) fn death_save_member(
    world: &mut World,
    data: &Data,
    index: usize,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let member = &mut world.party.members[index];
    let (roll, result) = death_save(
        data,
        &mut member.death_saves,
        &mut roller.rng,
        &roller.stream,
    )?;
    events.push(Event::DeathSave {
        member: member.id,
        roll,
        result,
        successes: member.death_saves.successes,
        failures: member.death_saves.failures,
    });
    match result {
        DeathSaveResult::Revived => {
            member.hp = 1;
            set_condition(member, data, "unconscious", false, events);
        }
        DeathSaveResult::Died => die(member, data, events),
        _ => {}
    }
    Ok(())
}

fn die(member: &mut Character, data: &Data, events: &mut Vec<Event>) {
    member.hp = 0;
    member.death_saves.failures = 3;
    set_condition(member, data, "unconscious", false, events);
    set_condition(member, data, "dead", true, events);
    events.push(Event::Death {
        target: ActorRef::Member(member.id),
        gold: None,
    });
}

/// Under permadeath the dead leave the party and their kit goes to the shared inventory;
/// otherwise they keep their slot for the temple. Returns who left.
pub(crate) fn bury(world: &mut World, data: &Data) -> Vec<CharacterId> {
    if !world.settings.permadeath {
        return Vec::new();
    }
    let members = core::mem::take(&mut world.party.members);
    let mut fallen = Vec::new();
    for member in members {
        if is_dead(&member, data) {
            for (item, count) in member.equipment {
                add_to(&mut world.party.inventory, item, count);
            }
            fallen.push(member.id);
        } else {
            world.party.members.push(member);
        }
    }
    fallen
}
