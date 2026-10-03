//! Opportunity attacks (PRD §8.3, owner 2026-10-03): until monsters have runbooks of their own,
//! a built-in rule. A front-row member who leaves the engagement (exchanging to the back row,
//! or the party running) draws one melee attack from each front stack with a reaction left,
//! made by its lead individual, and the stack's reaction is spent. Cunning Action's exchange
//! provokes none.

use super::state::CombatState;
use super::{Roller, resolve};
use crate::event::{ActorRef, Event};
use crate::party;
use crate::world::World;
use alloc::vec::Vec;
use omnis_data::Data;
use omnis_rules::{RuleError, choose_target, pick_attack};

/// The member in slot `member` leaves the front row: each front stack with a reaction and a
/// melee attack swings once, while the member stands.
pub(crate) fn provoke(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    member: usize,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    for stack in state.front_stacks(data) {
        if world.party.members[member].is_down() {
            break;
        }
        swing(world, data, state, stack, member, roller, events)?;
    }
    Ok(())
}

/// The party gets away: each front stack with a reaction swings at a random front-row member
/// still standing.
pub(crate) fn provoke_run(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let front_row = party::front_row(data);
    for stack in state.front_stacks(data) {
        let standing: Vec<usize> = (0..world.party.members.len().min(front_row))
            .filter(|i| !world.party.members[*i].is_down())
            .collect();
        if state.reactions_left(ActorRef::Stack(stack)) == 0 {
            continue;
        }
        let Some(pick) = choose_target(standing.len(), &mut roller.rng) else {
            break;
        };
        swing(world, data, state, stack, standing[pick], roller, events)?;
    }
    Ok(())
}

/// One stack's opportunity attack at `member`, when it has a reaction and a melee attack.
fn swing(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    stack: u8,
    member: usize,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let s = &state.encounter.stacks[usize::from(stack)];
    let Some(attack) = pick_attack(resolve::monster(data, s)?, false)
        .filter(|a| !a.ranged)
        .cloned()
    else {
        return Ok(());
    };
    if !state.spend_reaction(ActorRef::Stack(stack)) {
        return Ok(());
    }
    events.push(Event::OpportunityAttack {
        stack,
        member: world.party.members[member].id,
    });
    let attacker = ActorRef::Monster { stack, index: 0 };
    resolve::monster_attacks_member(
        world, data, state, attacker, member, &attack, roller, events,
    )
}
