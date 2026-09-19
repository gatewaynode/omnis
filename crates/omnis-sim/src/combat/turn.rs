//! The turn loop: initiative, one member action, monster turns until the next member, the
//! end of a round, and the end of the fight.

use super::state::{CombatState, Initiative, can_fight};
use super::{Plan, Roller};
use super::{cast, resolve};
use crate::apply::{advance, retreat};
use crate::encounter::{EncounterState, clear_once};
use crate::event::{ActorRef, CheckKind, CombatOutcome, Event, Surprise};
use crate::world::{Mode, World};
use alloc::vec::Vec;
use core::cmp::Reverse;
use omnis_core::{CharacterId, RollTrace};
use omnis_data::{Ability, Data, Disposition};
use omnis_rules::{RollMode, RuleError, check, initiative, modifier, modifier_of};

/// Minutes a round costs when the rules do not say.
const DEFAULT_ROUND_MINUTES: u32 = 1;

pub(crate) fn start(
    world: &mut World,
    data: &Data,
    encounter: EncounterState,
    surprised: Surprise,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    events.push(Event::CombatStarted { surprised });
    let (order, rolls) = roll_initiative(world, data, &encounter, roller)?;
    events.push(Event::Initiative {
        order: order.iter().map(|i| (i.actor, i.total)).collect(),
        rolls,
    });
    let mut state = CombatState {
        encounter,
        order,
        current: 0,
        round: 1,
        surprised,
        dodging: Vec::new(),
        gold: 0,
    };
    events.push(Event::RoundStarted { round: 1 });
    if !run_until_member(world, data, &mut state, roller, events)? {
        world.mode = Mode::Combat(state);
    }
    Ok(())
}

/// Members in marching order, then stacks; sorted by total, then Dexterity, then the party
/// first, then the order rolled (a stable sort keeps it).
fn roll_initiative(
    world: &World,
    data: &Data,
    encounter: &EncounterState,
    roller: &mut Roller,
) -> Result<(Vec<Initiative>, Vec<RollTrace>), RuleError> {
    let mut order = Vec::new();
    let mut rolls = Vec::new();
    for member in world.party.members.iter().filter(|m| can_fight(m, data)) {
        let dex = modifier(member.scores[Ability::Dexterity.index()]);
        let (total, trace) = initiative(data, dex, &mut roller.rng, &roller.stream)?;
        order.push(Initiative {
            actor: ActorRef::Member(member.id),
            total,
            dex,
        });
        rolls.push(trace);
    }
    for (i, stack) in encounter.stacks.iter().enumerate() {
        let monster = resolve::monster(data, stack)?;
        let dex = modifier_of(monster, Ability::Dexterity);
        let (total, trace) = initiative(data, dex, &mut roller.rng, &roller.stream)?;
        order.push(Initiative {
            actor: ActorRef::Stack(u8::try_from(i).unwrap_or(u8::MAX)),
            total,
            dex,
        });
        rolls.push(trace);
    }
    let is_stack = |i: &Initiative| matches!(i.actor, ActorRef::Stack(_));
    order.sort_by(|a, b| {
        b.total
            .cmp(&a.total)
            .then(b.dex.cmp(&a.dex))
            .then(is_stack(a).cmp(&is_stack(b)))
    });
    Ok((order, rolls))
}

/// One member's validated action, then the loop. The state leaves the world while it is
/// worked on and returns unless the fight ended.
pub(crate) fn act(
    world: &mut World,
    data: &Data,
    actor: CharacterId,
    plan: Plan,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let Mode::Combat(mut state) = core::mem::replace(&mut world.mode, Mode::Explore) else {
        return Ok(());
    };
    match act_inner(world, data, &mut state, actor, plan, roller, events) {
        Ok(true) => Ok(()),
        Ok(false) => {
            world.mode = Mode::Combat(state);
            Ok(())
        }
        Err(e) => {
            world.mode = Mode::Combat(state);
            Err(e)
        }
    }
}

fn act_inner(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    actor: CharacterId,
    plan: Plan,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<bool, RuleError> {
    match plan {
        Plan::Attack { stack, weapon } => {
            resolve::member_attacks(world, data, state, actor, stack, &weapon, roller, events)?;
        }
        Plan::Cast(plan) => {
            cast::pay(world, data, &plan, events)?;
            cast::resolve(world, data, state, &plan, roller, events)?;
        }
        Plan::Dodge => {
            if let Err(at) = state.dodging.binary_search(&actor) {
                state.dodging.insert(at, actor);
            }
            events.push(Event::Dodging {
                actor: ActorRef::Member(actor),
            });
        }
        Plan::Exchange { own, with } => {
            world.party.members.swap(own, with);
            events.push(Event::Exchanged {
                a: u8::try_from(own).unwrap_or(u8::MAX),
                b: u8::try_from(with).unwrap_or(u8::MAX),
            });
            events.push(Event::PartyChanged);
        }
        Plan::Run => {
            if flee(world, data, state, roller, events)? {
                finish(world, data, state, CombatOutcome::Fled, events);
                return Ok(true);
            }
        }
    }
    step_current(world, data, state, roller, events)?;
    run_until_member(world, data, state, roller, events)
}

/// The party's best Dexterity against the run difficulty; a friendly group lets them go.
fn flee(
    world: &World,
    data: &Data,
    state: &CombatState,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<bool, RuleError> {
    let dc = run_dc(data, state.encounter.disposition);
    let Some(runner) = world
        .party
        .members
        .iter()
        .filter(|m| can_fight(m, data))
        .min_by_key(|m| Reverse(modifier(m.scores[Ability::Dexterity.index()])))
    else {
        return Ok(false);
    };
    let (roll, success) = if state.encounter.disposition == Disposition::Friendly {
        (None, true)
    } else {
        let roll = check(
            runner,
            data,
            None,
            Ability::Dexterity,
            RollMode::Normal,
            &mut roller.rng,
            &roller.stream,
        )?;
        let success = roll.total >= dc;
        (Some(roll), success)
    };
    events.push(Event::Check {
        actor: ActorRef::Member(runner.id),
        kind: CheckKind::Flee,
        roll,
        dc,
        success,
    });
    Ok(success)
}

/// The run difficulty for a disposition (rules table `run_dc`).
#[must_use]
pub fn run_dc(data: &Data, disposition: Disposition) -> i64 {
    data.rules
        .table("run_dc")
        .and_then(|t| t.get(disposition.index()).copied())
        .unwrap_or(10)
}

/// Advance to the next entry, ending the round at the end of the order.
fn step_current(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    state.current = state.current.saturating_add(1);
    if usize::from(state.current) >= state.order.len() {
        end_of_round(world, data, state, roller, events)?;
        state.current = 0;
    }
    Ok(())
}

/// Monster turns until a member can act (`false`) or the fight ends (`true`). Every pass
/// advances the order, a round with no member action ends with round processing, and a
/// party that cannot fight at all is a defeat, so the loop terminates.
pub(crate) fn run_until_member(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<bool, RuleError> {
    loop {
        if let Some(outcome) = state.outcome(&world.party, data) {
            finish(world, data, state, outcome, events);
            return Ok(true);
        }
        let Some(actor) = state.current_actor() else {
            finish(world, data, state, CombatOutcome::Defeat, events);
            return Ok(true);
        };
        match actor {
            ActorRef::Member(id) => {
                if member_acts(state, world, data, id) {
                    events.push(Event::Turn { actor });
                    return Ok(false);
                }
            }
            ActorRef::Stack(i) => {
                if stack_acts(state, i) {
                    events.push(Event::Turn { actor });
                    resolve::monster_turn(world, data, state, i, roller, events)?;
                }
            }
            ActorRef::Monster { .. } => {}
        }
        step_current(world, data, state, roller, events)?;
    }
}

fn member_acts(state: &CombatState, world: &World, data: &Data, id: CharacterId) -> bool {
    let surprised = state.round == 1 && state.surprised == Surprise::Party;
    !surprised
        && world
            .party
            .members
            .iter()
            .any(|m| m.id == id && can_fight(m, data))
}

fn stack_acts(state: &CombatState, stack: u8) -> bool {
    let surprised = state.round == 1 && state.surprised == Surprise::Monsters;
    !surprised
        && state
            .encounter
            .stacks
            .get(usize::from(stack))
            .is_some_and(|s| s.alive())
}

/// Death saves for members lying at zero, dodges end, the clock moves, the next round.
fn end_of_round(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    for index in 0..world.party.members.len() {
        let member = &world.party.members[index];
        if member.is_down() && !super::state::is_dead(member, data) && !member.death_saves.stable {
            resolve::death_save_member(world, data, index, roller, events)?;
        }
    }
    state.dodging.clear();
    let minutes = data
        .rules
        .value("combat_round_minutes")
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(DEFAULT_ROUND_MINUTES);
    advance(world, minutes, events);
    state.round = state.round.saturating_add(1);
    events.push(Event::RoundStarted { round: state.round });
    Ok(())
}

/// Experience and gold on victory, the retreat on flight, permadeath, and back to exploring.
fn finish(
    world: &mut World,
    data: &Data,
    state: &CombatState,
    outcome: CombatOutcome,
    events: &mut Vec<Event>,
) {
    let (mut xp, mut gold) = (0, 0);
    match outcome {
        CombatOutcome::Victory => {
            xp = award_xp(world, data, state);
            gold = state.gold;
            world.party.gold = world.party.gold.saturating_add(gold);
            clear_once(world, data, state.encounter.source);
        }
        CombatOutcome::Fled => retreat(world, data, state.encounter.retreat, events),
        CombatOutcome::Defeat => {}
    }
    let fallen = resolve::bury(world, data);
    events.push(Event::CombatEnded {
        outcome,
        xp,
        gold,
        fallen,
    });
    world.mode = Mode::Explore;
}

/// The monsters' experience for every individual killed, split equally among the members
/// who are not dead, rounded down; each gets the share.
fn award_xp(world: &mut World, data: &Data, state: &CombatState) -> u32 {
    let total: u32 = state
        .encounter
        .stacks
        .iter()
        .map(|s| {
            let each = data.monsters.get(&s.monster).map_or(0, |m| m.xp);
            each.saturating_mul(u32::from(s.initial.saturating_sub(s.count())))
        })
        .fold(0u32, u32::saturating_add);
    let survivors: Vec<usize> = world
        .party
        .members
        .iter()
        .enumerate()
        .filter(|(_, m)| !super::state::is_dead(m, data))
        .map(|(i, _)| i)
        .collect();
    let Ok(count) = u32::try_from(survivors.len()) else {
        return 0;
    };
    if count == 0 {
        return 0;
    }
    let share = total / count;
    for i in survivors {
        let member = &mut world.party.members[i];
        member.xp = member.xp.saturating_add(share);
    }
    share
}
