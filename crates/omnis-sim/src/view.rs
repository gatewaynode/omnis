//! The encounter or fight as a client sees it (`combat.get`).

use crate::combat::{CombatState, monster_front_stacks, weapon_for};
use crate::encounter::{EncounterSource, Stack};
use crate::event::{ActorRef, Surprise};
use crate::party;
use crate::world::{Mode, ModeKind, World};
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, Position};
use omnis_data::{Data, Disposition};
use serde::{Deserialize, Serialize};

/// One stack as a client sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackView {
    /// Its index in the encounter, the number `attack` takes.
    pub index: u8,
    /// Monster id.
    pub monster: String,
    /// Text key of the monster's name.
    pub name: String,
    /// How many there were.
    pub initial: u8,
    /// Hit points of the living, in order.
    pub hp: Vec<i32>,
    /// Armor class.
    pub ac: u8,
    /// Whether it stands in front.
    pub front: bool,
    /// Whether anyone in it still stands.
    pub alive: bool,
    /// Whether the member whose turn it is can reach it.
    pub reachable: bool,
}

/// The encounter or fight in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatView {
    /// `Encounter` before the choice, `Combat` during the fight.
    pub phase: ModeKind,
    /// Where the monsters came from.
    pub source: EncounterSource,
    /// How they feel about the party.
    pub disposition: Disposition,
    /// The stacks.
    pub stacks: Vec<StackView>,
    /// Where Run and Flee put the party.
    pub retreat: Position,
    /// The round, zero before the fight.
    pub round: u32,
    /// Whose turn it is.
    pub current: Option<ActorRef>,
    /// The initiative order with totals.
    pub order: Vec<(ActorRef, i64)>,
    /// Who lost their first round.
    pub surprised: Surprise,
    /// Members dodging this round.
    pub dodging: Vec<CharacterId>,
    /// Gold looted so far.
    pub gold: u32,
    /// Members in the front row.
    pub front_row: usize,
    /// Stacks in front.
    pub monster_front_stacks: usize,
}

/// The view, or `None` while exploring.
#[must_use]
pub fn combat_view(world: &World, data: &Data) -> Option<CombatView> {
    let (encounter, fight) = match &world.mode {
        Mode::Explore => return None,
        Mode::Encounter(e) => (e, None),
        Mode::Combat(c) => (&c.encounter, Some(c)),
    };
    let front_count = monster_front_stacks(data);
    let acting = fight.and_then(|c| match c.current_actor() {
        Some(ActorRef::Member(id)) => world.party.members.iter().position(|m| m.id == id),
        _ => None,
    });
    let mut in_front = 0;
    let stacks = encounter
        .stacks
        .iter()
        .enumerate()
        .map(|(i, stack)| {
            let index = u8::try_from(i).unwrap_or(u8::MAX);
            let front = stack.alive() && in_front < front_count;
            if front {
                in_front += 1;
            }
            let reachable = match (fight, acting) {
                (Some(c), Some(own)) => weapon_for(c, world, data, own, index).is_ok(),
                _ => false,
            };
            stack_view(data, stack, index, front, reachable)
        })
        .collect();
    Some(CombatView {
        phase: world.mode.kind(),
        source: encounter.source,
        disposition: encounter.disposition,
        stacks,
        retreat: encounter.retreat,
        round: fight.map_or(0, |c| c.round),
        current: fight.and_then(CombatState::current_actor),
        order: fight.map_or_else(Vec::new, |c| {
            c.order.iter().map(|i| (i.actor, i.total)).collect()
        }),
        surprised: fight.map_or(Surprise::None, |c| c.surprised),
        dodging: fight.map_or_else(Vec::new, |c| c.dodging.clone()),
        gold: fight.map_or(0, |c| c.gold),
        front_row: party::front_row(data),
        monster_front_stacks: front_count,
    })
}

fn stack_view(data: &Data, stack: &Stack, index: u8, front: bool, reachable: bool) -> StackView {
    let monster = data.monsters.get(&stack.monster);
    StackView {
        index,
        monster: data
            .registry
            .monsters
            .name(stack.monster)
            .unwrap_or("?")
            .to_owned(),
        name: monster.map_or_else(|| "?".to_owned(), |m| m.name.clone()),
        initial: stack.initial,
        hp: stack.hp.clone(),
        ac: monster.map_or(0, |m| m.ac),
        front,
        alive: stack.alive(),
        reachable,
    }
}
