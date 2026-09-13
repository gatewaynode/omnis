//! What one attack does: the roll, the damage, a monster's death and its gold, a member's
//! fall to zero, death saves, instant death, and the burial at the end.

use super::Roller;
use super::state::{CombatState, is_dead};
use crate::encounter::Stack;
use crate::party;
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::{Attack, Data, Monster};
use omnis_rules::{
    Character, DeathSaveResult, DeathSaves, RollMode, RuleError, Weapon, armor_class, attack_bonus,
    attack_roll, choose_target, condition_id, damage_roll, death_save, flags, member_defenses,
    monster_defenses, pick_attack, wound_at_zero,
};

use crate::event::{ActorRef, Event};

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
    let roll = attack_roll(
        data,
        bonus,
        proficiency,
        ac,
        mode,
        &mut roller.rng,
        &roller.stream,
    )?;
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
    let s = &mut state.encounter.stacks[usize::from(stack)];
    let Some(lead) = s.hp.first_mut() else {
        return Ok(());
    };
    *lead = lead.saturating_sub(i32::try_from(amount).unwrap_or(i32::MAX));
    if *lead > 0 {
        return Ok(());
    }
    s.hp.remove(0);
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
        target: ActorRef::Monster { stack, index: 0 },
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
    let roll = attack_roll(
        data,
        i64::from(attack.to_hit),
        0,
        ac,
        mode,
        &mut roller.rng,
        &roller.stream,
    )?;
    let crit = roll.crit || (roll.hit && condition_flags.melee_hits_crit && !attack.ranged);
    events.push(Event::AttackResolved {
        attacker,
        target,
        roll: roll.roll.clone(),
        ac,
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
        member_defenses(member, data, attack.damage_type),
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

/// Add or remove a pack condition by name, with the event, when the pack defines it.
fn set_condition(
    member: &mut Character,
    data: &Data,
    name: &str,
    applied: bool,
    events: &mut Vec<Event>,
) {
    let Some(condition) = condition_id(data, name) else {
        return;
    };
    let at = member.conditions.iter().position(|c| *c == condition);
    match (applied, at) {
        (true, None) => member.conditions.push(condition),
        (false, Some(i)) => {
            member.conditions.remove(i);
        }
        _ => return,
    }
    events.push(Event::Condition {
        target: ActorRef::Member(member.id),
        condition,
        applied,
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
                match world.party.inventory.iter_mut().find(|(i, _)| *i == item) {
                    Some((_, have)) => *have = have.saturating_add(count),
                    None => world.party.inventory.push((item, count)),
                }
            }
            fallen.push(member.id);
        } else {
            world.party.members.push(member);
        }
    }
    fallen
}
