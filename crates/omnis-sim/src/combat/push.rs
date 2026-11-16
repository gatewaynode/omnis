//! A spell that pushes (owner, 2026-11-16; the SRD's thunderwave): those who fail its save are
//! thrown back. A stack stands at one distance, so the thrown leave it as a stack of their own
//! when others of it held their ground (the owner's choice), and the party's line, which every
//! distance is measured from, is driven back whole or not at all.

use super::state::{CombatState, Initiative};
use crate::encounter::{MAX_FIGHT_STACKS, Stack};
use crate::event::ActorRef;
use crate::event::Event;
use alloc::vec::Vec;

/// The individuals of `stack` in `thrown` (indices among its living, ascending) failed a save
/// against a push of `feet`: the stack moves when that is all of it, and they leave it when it
/// is not. A full fight splits nothing: the stack moves when `half` of those caught failed.
pub(crate) fn thrown(
    state: &mut CombatState,
    stack: u8,
    thrown: &[u8],
    half: bool,
    feet: u16,
    events: &mut Vec<Event>,
) {
    let stacks = &mut state.encounter.stacks;
    let Some(s) = stacks.get_mut(usize::from(stack)) else {
        return;
    };
    if feet == 0 || thrown.is_empty() || !s.alive() {
        return;
    }
    let from = s.distance;
    let to = from.saturating_add(feet);
    let all = thrown.len() >= s.hp.len();
    let full = stacks.len() >= MAX_FIGHT_STACKS || u8::try_from(stacks.len()).is_err();
    if all || (full && half) {
        stacks[usize::from(stack)].distance = to;
        events.push(Event::StackMoved { stack, from, to });
    } else if !full {
        let into = split(state, stack, thrown, to);
        events.push(Event::StackSplit {
            stack,
            into,
            moved: thrown.to_vec(),
            from,
            to,
        });
    }
}

/// The individuals `moved` of `stack` become the fight's last stack, at `distance`: each with
/// its hit points, its spent points and its shield; counted where it now stands, so a kill is
/// counted once; with a turn straight after the stack they left and its reactions.
fn split(state: &mut CombatState, stack: u8, moved: &[u8], distance: u16) -> u8 {
    let into = u8::try_from(state.encounter.stacks.len()).unwrap_or(u8::MAX);
    let parent = &mut state.encounter.stacks[usize::from(stack)];
    let leaves = |index: usize| moved.iter().any(|m| usize::from(*m) == index);
    let (mut hp, mut spent) = (Vec::new(), Vec::new());
    let (mut kept_hp, mut kept_spent) = (Vec::new(), Vec::new());
    // Where each individual of the parent now stands: `(stack, index)`.
    let mut places = Vec::with_capacity(parent.hp.len());
    for (index, points) in parent.hp.iter().enumerate() {
        let used = parent.spent.get(index).copied().unwrap_or(0);
        let (to, hp, spent) = if leaves(index) {
            (into, &mut hp, &mut spent)
        } else {
            (stack, &mut kept_hp, &mut kept_spent)
        };
        places.push((to, u8::try_from(hp.len()).unwrap_or(u8::MAX)));
        hp.push(*points);
        spent.push(used);
    }
    let gone = u8::try_from(hp.len()).unwrap_or(u8::MAX);
    parent.hp = kept_hp;
    parent.spent = kept_spent;
    parent.initial = parent.initial.saturating_sub(gone);
    let thrown = Stack {
        monster: parent.monster,
        initial: gone,
        hp,
        spent,
        distance,
    };
    state.encounter.stacks.push(thrown);
    for shield in &mut state.monster_shields {
        if shield.0 == stack
            && let Some(place) = places.get(usize::from(shield.1))
        {
            *shield = *place;
        }
    }
    state.monster_shields.sort_unstable();
    // Its turn follows its parent's; whoever's turn it is keeps it.
    let actor = ActorRef::Stack(into);
    let own = ActorRef::Stack(stack);
    if let Some(at) = state.order.iter().position(|e| e.actor == own) {
        let entry = Initiative {
            actor,
            ..state.order[at]
        };
        state.order.insert(at + 1, entry);
        if at < usize::from(state.current) {
            state.current = state.current.saturating_add(1);
        }
    }
    let left = state.reactions.iter().find(|(a, _)| *a == own);
    if let Some(left) = left.map(|(_, left)| *left) {
        state.reactions.push((actor, left));
        state.reactions.sort_unstable();
    }
    into
}

/// The party's line is driven back: every stack that stands is `feet` further off.
pub(crate) fn driven_back(state: &mut CombatState, feet: u16, events: &mut Vec<Event>) {
    if feet == 0 {
        return;
    }
    for stack in state.encounter.stacks.iter_mut().filter(|s| s.alive()) {
        stack.distance = stack.distance.saturating_add(feet);
    }
    events.push(Event::PartyDrivenBack { feet });
}
