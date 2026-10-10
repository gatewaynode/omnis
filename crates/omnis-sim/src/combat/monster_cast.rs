//! Monsters that cast (M7c, the test pack's Bob the Rat King). Until monster runbooks, each
//! individual's turn is a dice roll among what it can do now: its weapon, and each spell its
//! points pay for. Spells use the members' effects with the stat block's numbers (spell attack,
//! save DC, caster level for a cantrip's dice), aimed at members: an attack spell at anyone, a
//! missile at anyone, a save spell at the front row (anyone once it has fallen). Each cast
//! raises `EnemyCasts`. Shield is the stack's reaction by a built-in rule, as opportunity attacks
//! are: when a member's hit would miss at the higher armor class, or at a Magic Missile, while
//! the stack has its reaction and the individual the points. A shield lasts until its stack's
//! next turn and stops Magic Missile (SRD).

use super::Roller;
use super::reaction;
use super::resolve::{harm_member, monster, monster_attacks_member, reached};
use super::state::CombatState;
use crate::encounter::Stack;
use crate::event::{ActorRef, CheckKind, Event};
use crate::names::id_of;
use crate::party;
use alloc::vec::Vec;
use omnis_core::SpellId;
use omnis_data::{Attack, Data, Monster, MonsterCasting, Reach, SpellEffect};
use omnis_rules::{
    AttackRoll, Defenses, EffectKind, RollMode, RuleError, adjusted, cantrip_dice_at,
    choose_target, damage_roll, member_defenses, rejudge, save, saved_damage, spell_cost,
};

/// What one individual may do on its turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    Weapon,
    Cast(SpellId),
}

/// One individual's turn: the dice pick among its weapon and the spells it can pay for.
#[allow(clippy::too_many_arguments)]
pub(crate) fn act(
    world: &mut crate::world::World,
    data: &Data,
    state: &mut CombatState,
    stack: u8,
    index: usize,
    attack: Option<&Attack>,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let s = &state.encounter.stacks[usize::from(stack)];
    let m = monster(data, s)?;
    let caster = ActorRef::Monster {
        stack,
        index: u8::try_from(index).unwrap_or(u8::MAX),
    };
    let mut options: Vec<Choice> = attack.map(|_| Choice::Weapon).into_iter().collect();
    if let Some(casting) = &m.casting {
        let left = points_left(casting, s, index);
        for id in &casting.spells {
            let Some((spell, cost)) = turn_spell(data, id, roller)? else {
                continue;
            };
            if cost <= left {
                options.push(Choice::Cast(spell));
            }
        }
    }
    let Some(pick) = choose_target(options.len(), &mut roller.rng) else {
        events.push(Event::Waited { actor: caster });
        return Ok(());
    };
    match (options[pick], attack) {
        (Choice::Weapon, Some(attack)) => {
            super::resolve::swing(world, data, state, stack, index, attack, roller, events)
        }
        (Choice::Cast(spell), _) => cast(world, data, state, (stack, index), spell, roller, events),
        (Choice::Weapon, None) => Ok(()),
    }
}

/// A spell a monster may cast on its turn (not its reaction) and what it costs.
fn turn_spell(data: &Data, id: &str, roller: &Roller) -> Result<Option<(SpellId, u32)>, RuleError> {
    let Some(spell) = data.registry.spells.get(id) else {
        return Ok(None);
    };
    let Some(def) = data.spells.get(&spell) else {
        return Ok(None);
    };
    if matches!(def.effect, Some(SpellEffect::Reaction { .. }) | None) {
        return Ok(None);
    }
    // A copy of the stream: looking at a cost must not move the dice.
    let mut rng = roller.rng;
    Ok(Some((spell, spell_cost(def, data, &mut rng)?)))
}

/// Each individual's points left, in the stack's order; empty for a monster that does not cast.
pub(crate) fn points_of(data: &Data, stack: &Stack) -> Vec<u8> {
    let Some(casting) = data
        .monsters
        .get(&stack.monster)
        .and_then(|m| m.casting.as_ref())
    else {
        return Vec::new();
    };
    (0..stack.hp.len())
        .map(|i| u8::try_from(points_left(casting, stack, i)).unwrap_or(u8::MAX))
        .collect()
}

/// Points an individual has left.
fn points_left(casting: &MonsterCasting, stack: &Stack, index: usize) -> u32 {
    let spent = stack.spent.get(index).copied().unwrap_or(0);
    u32::from(casting.points.saturating_sub(spent))
}

/// Spend `cost` of an individual's points.
fn pay(stack: &mut Stack, index: usize, cost: u32) {
    if stack.spent.len() < stack.hp.len() {
        stack.spent.resize(stack.hp.len(), 0);
    }
    if let Some(spent) = stack.spent.get_mut(index) {
        *spent = spent.saturating_add(u8::try_from(cost).unwrap_or(u8::MAX));
    }
}

/// Cast `spell` at members: pay, announce, raise `EnemyCasts`, resolve.
fn cast(
    world: &mut crate::world::World,
    data: &Data,
    state: &mut CombatState,
    (stack, index): (u8, usize),
    spell: SpellId,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let caster = ActorRef::Monster {
        stack,
        index: u8::try_from(index).unwrap_or(u8::MAX),
    };
    let m = monster(data, &state.encounter.stacks[usize::from(stack)])?;
    let casting = m
        .casting
        .clone()
        .ok_or_else(|| RuleError::new("cast", "a monster without casting cast"))?;
    let def = data
        .spells
        .get(&spell)
        .ok_or_else(|| RuleError::new("cast", "a monster's spell vanished from the data"))?;
    let mut rng = roller.rng;
    let cost = spell_cost(def, data, &mut rng)?;
    pay(&mut state.encounter.stacks[usize::from(stack)], index, cost);
    events.push(Event::MonsterCast {
        caster,
        spell: id_of(&data.registry.spells, spell),
    });
    reaction::on_enemy_cast(world, data, state, roller, events)?;
    let scale = |dice, roller: &mut Roller| {
        if def.level == 0 {
            cantrip_dice_at(
                casting.caster_level,
                data,
                dice,
                &mut roller.rng,
                &roller.stream,
            )
        } else {
            Ok(dice)
        }
    };
    match def.effect.clone() {
        Some(SpellEffect::Attack { dice, damage_type }) => {
            let attack = Attack {
                name: def.name.clone(),
                to_hit: casting.spell_attack,
                damage: scale(dice, roller)?,
                damage_type,
                ranged: true,
            };
            match reached(world, data, true, &mut roller.rng) {
                Some(target) => monster_attacks_member(
                    world, data, state, caster, target, &attack, roller, events,
                ),
                None => Ok(()),
            }
        }
        Some(SpellEffect::AutoHit { dice, damage_type }) => {
            let Some(target) = reached(world, data, true, &mut roller.rng) else {
                return Ok(());
            };
            reaction::on_missile(world, data, state, target, roller, events)?;
            let member = &world.party.members[target];
            if member
                .effects
                .iter()
                .any(|e| matches!(e.kind, EffectKind::ArmorBonus(_)))
            {
                events.push(Event::ShieldStops {
                    target: ActorRef::Member(member.id),
                });
                return Ok(());
            }
            let dice = scale(dice, roller)?;
            let damage = damage_roll(
                data,
                Some(dice),
                0,
                false,
                member_defenses(member, data, damage_type),
                &mut roller.rng,
                &roller.stream,
            )?;
            events.push(Event::Damage {
                target: ActorRef::Member(member.id),
                kind: damage_type,
                rolls: damage.rolls.clone(),
                raw: damage.raw,
                amount: damage.amount,
                adjust: damage.adjust,
            });
            harm_member(
                world,
                data,
                state,
                target,
                (damage.amount, false),
                roller,
                events,
            )
        }
        Some(SpellEffect::Save {
            ability,
            dice,
            damage_type,
            half_on_save,
        }) => {
            let targets = match def.reach {
                Reach::One => reached(world, data, false, &mut roller.rng)
                    .into_iter()
                    .collect(),
                Reach::Stack | Reach::AllStacks => front_or_all(world, data),
            };
            let dice = scale(dice, roller)?;
            let damage = damage_roll(
                data,
                Some(dice),
                0,
                false,
                Defenses::default(),
                &mut roller.rng,
                &roller.stream,
            )?;
            for target in targets {
                let member = &world.party.members[target];
                let id = member.id;
                let roll = save(
                    member,
                    data,
                    ability,
                    RollMode::Normal,
                    &mut roller.rng,
                    &roller.stream,
                )?;
                let dc = i64::from(casting.save_dc);
                let success = roll.total >= dc;
                events.push(Event::Check {
                    actor: ActorRef::Member(id),
                    kind: CheckKind::Save(ability),
                    roll: Some(roll),
                    dc,
                    success,
                });
                let defenses = member_defenses(member, data, damage_type);
                let (amount, adjust) =
                    adjusted(data, damage.raw, defenses, &mut roller.rng, &roller.stream)?;
                let amount = saved_damage(
                    data,
                    amount,
                    success,
                    half_on_save,
                    &mut roller.rng,
                    &roller.stream,
                )?;
                events.push(Event::Damage {
                    target: ActorRef::Member(id),
                    kind: damage_type,
                    rolls: damage.rolls.clone(),
                    raw: damage.raw,
                    amount,
                    adjust,
                });
                harm_member(world, data, state, target, (amount, false), roller, events)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The living front-row members, or every living member once the front has fallen.
fn front_or_all(world: &crate::world::World, data: &Data) -> Vec<usize> {
    let front = party::front_row(data);
    let living: Vec<usize> = world
        .party
        .members
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.is_down())
        .map(|(i, _)| i)
        .collect();
    let in_front: Vec<usize> = living.iter().copied().filter(|i| *i < front).collect();
    if in_front.is_empty() {
        living
    } else {
        in_front
    }
}

/// The armor-bonus reaction a monster casts, and its bonus.
fn shield_spell(data: &Data, m: &Monster) -> Option<(SpellId, i64)> {
    m.casting.as_ref()?.spells.iter().find_map(|id| {
        let spell = data.registry.spells.get(id)?;
        match data.spells.get(&spell)?.effect {
            Some(SpellEffect::Reaction { armor_bonus }) => Some((spell, armor_bonus)),
            _ => None,
        }
    })
}

/// The armor class an individual's shield adds while it is up.
pub(crate) fn shield_bonus(state: &CombatState, data: &Data, stack: u8, index: u8) -> i64 {
    if state
        .monster_shields
        .binary_search(&(stack, index))
        .is_err()
    {
        return 0;
    }
    state
        .encounter
        .stacks
        .get(usize::from(stack))
        .and_then(|s| data.monsters.get(&s.monster))
        .and_then(|m| shield_spell(data, m))
        .map_or(0, |(_, bonus)| bonus)
}

/// Raise an individual's shield when its stack has the reaction and it has the points: spend
/// both, announce the cast, and keep it up until the stack's next turn. Returns the bonus.
fn raise_shield(
    data: &Data,
    state: &mut CombatState,
    (stack, index): (u8, u8),
    roller: &Roller,
    events: &mut Vec<Event>,
) -> Result<Option<i64>, RuleError> {
    if state.reactions_left(ActorRef::Stack(stack)) == 0
        || state.monster_shields.binary_search(&(stack, index)).is_ok()
    {
        return Ok(None);
    }
    let s = &state.encounter.stacks[usize::from(stack)];
    let m = monster(data, s)?;
    let (Some(casting), Some((spell, bonus))) = (&m.casting, shield_spell(data, m)) else {
        return Ok(None);
    };
    let Some(def) = data.spells.get(&spell) else {
        return Ok(None);
    };
    let mut rng = roller.rng;
    let cost = spell_cost(def, data, &mut rng)?;
    if cost > points_left(casting, s, usize::from(index)) {
        return Ok(None);
    }
    state.spend_reaction(ActorRef::Stack(stack));
    pay(
        &mut state.encounter.stacks[usize::from(stack)],
        usize::from(index),
        cost,
    );
    if let Err(at) = state.monster_shields.binary_search(&(stack, index)) {
        state.monster_shields.insert(at, (stack, index));
    }
    events.push(Event::MonsterCast {
        caster: ActorRef::Monster { stack, index },
        spell: id_of(&data.registry.spells, spell),
    });
    Ok(Some(bonus))
}

/// A member's attack roll at an individual is about to be judged: the individual raises its
/// shield when that turns the hit into a miss. Returns the armor class the roll is judged at.
pub(crate) fn shield_on_hit(
    data: &Data,
    state: &mut CombatState,
    (stack, index): (u8, u8),
    roll: &mut AttackRoll,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<i64, RuleError> {
    let base = i64::from(monster(data, &state.encounter.stacks[usize::from(stack)])?.ac);
    let up = shield_bonus(state, data, stack, index);
    if up > 0 || !roll.hit || roll.crit {
        return Ok(base + up);
    }
    let Some((_, bonus)) = data
        .monsters
        .get(&state.encounter.stacks[usize::from(stack)].monster)
        .and_then(|m| shield_spell(data, m))
    else {
        return Ok(base);
    };
    let mut rejudged = roll.clone();
    rejudge(
        data,
        &mut rejudged,
        base + bonus,
        &mut roller.rng,
        &roller.stream,
    )?;
    if rejudged.hit {
        return Ok(base);
    }
    if raise_shield(data, state, (stack, index), roller, events)?.is_none() {
        return Ok(base);
    }
    *roll = rejudged;
    Ok(base + bonus)
}

/// A member's Magic Missile at an individual: true when a shield, up or raised now, stops it.
pub(crate) fn shield_on_missile(
    data: &Data,
    state: &mut CombatState,
    (stack, index): (u8, u8),
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<bool, RuleError> {
    if shield_bonus(state, data, stack, index) > 0 {
        return Ok(true);
    }
    Ok(raise_shield(data, state, (stack, index), roller, events)?.is_some())
}

/// A stack's shields drop at the start of its turn.
pub(crate) fn drop_shields(state: &mut CombatState, stack: u8) {
    state.monster_shields.retain(|(s, _)| *s != stack);
}

/// An individual died: its shield goes, and those behind it in the stack move up one.
pub(crate) fn forget(shields: &mut Vec<(u8, u8)>, stack: u8, index: usize) {
    let index = u8::try_from(index).unwrap_or(u8::MAX);
    shields.retain(|e| *e != (stack, index));
    for (s, i) in shields.iter_mut() {
        if *s == stack && *i > index {
            *i -= 1;
        }
    }
}
