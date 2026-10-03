//! Declared reactions (PRD D22, §7.9; ARCHITECTURE.md §4.7): at a trigger the simulation walks
//! the members in marching order and, for each with reactions on and a reaction left, the
//! default runbook's sets for that trigger in order; the first whose criteria hold and whose
//! cost can be paid fires, spends the reaction and emits `Reaction` ahead of its own events. One
//! trigger fires at most one reaction a member. Proximity follows PRD §8.3: an attack on a
//! member is answered by that member (`Attacked`) and the members of their row
//! (`MemberAttacked`), a wound or a fall by the row, a cast by anyone. The M6 shield rule is a
//! criteria set: shield, `Attacked`, `WouldChangeOutcome`.

use super::Roller;
use super::cast::{self, CastPlan, Target};
use super::state::CombatState;
use crate::effects;
use crate::event::{ActorRef, Event};
use crate::party;
use crate::tactics::answers;
use crate::world::World;
use alloc::vec::Vec;
use omnis_data::{Data, SpellEffect};
use omnis_rules::{
    ActionRef, ActiveEffect, AttackRoll, CriteriaSet, EffectKind, Expiry, Facts, MemberFacts, Row,
    RuleError, Trigger, armor_class, heal_roll, rejudge,
};

/// A monster's attack on the member in slot `subject` is about to be judged.
pub(crate) fn on_attack(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    subject: usize,
    roll: &mut AttackRoll,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let front = party::front_row(data);
    let trigger = |reactor: usize| {
        if reactor == subject {
            Some(Trigger::Attacked)
        } else {
            same_row(reactor, subject, front).then_some(Trigger::MemberAttacked)
        }
    };
    fire(
        world,
        data,
        state,
        &trigger,
        subject,
        Some(roll),
        roller,
        events,
    )
}

/// The member in slot `subject` took damage, and fell when `dying`.
pub(crate) fn on_wound(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    subject: usize,
    dying: bool,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let front = party::front_row(data);
    let row = |reactor: usize| reactor != subject && same_row(reactor, subject, front);
    let wounded = |reactor: usize| row(reactor).then_some(Trigger::MemberWounded);
    fire(world, data, state, &wounded, subject, None, roller, events)?;
    if dying {
        let falling = |reactor: usize| row(reactor).then_some(Trigger::MemberDying);
        fire(world, data, state, &falling, subject, None, roller, events)?;
    }
    Ok(())
}

/// The member in slot `caster` cast a spell: anyone else in the fight may answer.
pub(crate) fn on_cast(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    caster: usize,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let anyone = |reactor: usize| (reactor != caster).then_some(Trigger::SpellCast);
    fire(world, data, state, &anyone, caster, None, roller, events)
}

const fn same_row(a: usize, b: usize, front: usize) -> bool {
    (a < front) == (b < front)
}

#[allow(clippy::too_many_arguments)]
fn fire(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    trigger_for: &dyn Fn(usize) -> Option<Trigger>,
    subject: usize,
    mut roll: Option<&mut AttackRoll>,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    for reactor in 0..world.party.members.len() {
        let Some(trigger) = trigger_for(reactor) else {
            continue;
        };
        let member = &world.party.members[reactor];
        if member.is_down()
            || !member.tactics.reactions_on
            || state.reactions_left(ActorRef::Member(member.id)) == 0
        {
            continue;
        }
        let sets: Vec<CriteriaSet> = member
            .tactics
            .reactions()
            .into_iter()
            .filter(|s| s.trigger == trigger)
            .cloned()
            .collect();
        for set in sets {
            if react(
                world,
                data,
                state,
                (reactor, subject),
                &set,
                roll.as_deref_mut(),
                roller,
                events,
            )? {
                break;
            }
        }
    }
    Ok(())
}

/// One set considered for one member: fire it if it can be paid and its criteria hold.
#[allow(clippy::too_many_arguments)]
fn react(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    (reactor, subject): (usize, usize),
    set: &CriteriaSet,
    roll: Option<&mut AttackRoll>,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<bool, RuleError> {
    let member = &world.party.members[reactor];
    let ActionRef::Spell(spell) = set.action else {
        return Ok(false);
    };
    if !answers(member, data, &set.action, set.trigger) {
        return Ok(false);
    }
    let Some(cost) = cast::reaction_cost(world, data, reactor, spell, &mut roller.rng)? else {
        return Ok(false);
    };
    let Some(effect) = data.spells.get(&spell).and_then(|s| s.effect.clone()) else {
        return Ok(false);
    };
    // An armor bonus answers a hit only, and never on top of another.
    let mut probe = None;
    if let SpellEffect::Reaction { armor_bonus } = effect {
        let Some(roll) = roll.as_deref() else {
            return Ok(false);
        };
        let shielded = member
            .effects
            .iter()
            .any(|e| matches!(e.kind, EffectKind::ArmorBonus(_)));
        if !roll.hit || roll.crit || shielded {
            return Ok(false);
        }
        let mut rejudged = roll.clone();
        let ac = armor_class(member, data) + armor_bonus;
        rejudge(data, &mut rejudged, ac, &mut roller.rng, &roller.stream)?;
        probe = Some((rejudged, armor_bonus));
    }
    let would_change = probe.as_ref().is_some_and(|(r, _)| !r.hit);
    if !set
        .when
        .holds(&facts(world, data, state, reactor, subject, would_change))
    {
        return Ok(false);
    }
    let id = member.id;
    state.spend_reaction(ActorRef::Member(id));
    events.push(Event::Reaction {
        actor: id,
        trigger: set.trigger,
        action: set.action.clone(),
    });
    let target = u8::try_from(subject).unwrap_or(u8::MAX);
    let plan = CastPlan {
        own: reactor,
        spell,
        cost,
        target: Target::Member(target),
    };
    cast::pay(world, data, &plan, events)?;
    match (effect, probe) {
        (SpellEffect::Reaction { .. }, Some((rejudged, bonus))) => {
            let concentration = data.spells.get(&spell).is_some_and(|s| s.concentration);
            effects::apply_to_member(
                world,
                reactor,
                ActiveEffect {
                    source: spell,
                    caster: id,
                    concentration,
                    kind: EffectKind::ArmorBonus(bonus),
                    until: Expiry::NextTurn,
                },
                events,
            );
            if let Some(roll) = roll {
                *roll = rejudged;
            }
        }
        (SpellEffect::Heal { dice, add_mod }, _) => {
            let caster = &world.party.members[reactor];
            let heal = heal_roll(caster, data, dice, add_mod, &mut roller.rng, &roller.stream)?;
            party::heal(world, data, subject, heal.rolls, heal.amount, events);
        }
        _ => {}
    }
    Ok(true)
}

/// The integers a criteria tree reads at this moment.
fn facts(
    world: &World,
    data: &Data,
    state: &CombatState,
    me: usize,
    subject: usize,
    would_change: bool,
) -> Facts {
    let front = party::front_row(data);
    let of = |index: usize| {
        let m = &world.party.members[index];
        let percent = |now: i64, max: i64| if max <= 0 { 0 } else { now.max(0) * 100 / max };
        MemberFacts {
            hp_percent: percent(i64::from(m.hp), i64::from(m.hp_max)),
            sp_percent: percent(i64::from(m.spell_points), i64::from(m.spell_points_max)),
            conditions: m.conditions.clone(),
            row: if index < front { Row::Front } else { Row::Back },
        }
    };
    Facts {
        me: of(me),
        subject: of(subject),
        monsters: state
            .encounter
            .stacks
            .iter()
            .filter(|s| s.alive())
            .map(|s| (s.monster, u16::try_from(s.hp.len()).unwrap_or(u16::MAX)))
            .collect(),
        round: state.round,
        would_change,
    }
}
