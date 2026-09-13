//! The state of a fight between two player commands, and what can be read off it.

use crate::encounter::EncounterState;
use crate::event::{ActorRef, CombatOutcome, Surprise};
use crate::party::Party;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::Data;
use omnis_rules::{Character, condition_id, flags};
use serde::{Deserialize, Serialize};

/// Stacks that stand in front when the rules do not say: two.
pub const DEFAULT_FRONT_STACKS: usize = 2;

/// How many living stacks stand in front (rules value `monster_front_stacks`).
#[must_use]
pub fn monster_front_stacks(data: &Data) -> usize {
    data.rules
        .value("monster_front_stacks")
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(DEFAULT_FRONT_STACKS)
}

/// One entry of the initiative order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Initiative {
    /// Who.
    pub actor: ActorRef,
    /// The initiative total.
    pub total: i64,
    /// The Dexterity modifier behind it, the first tie-breaker.
    pub dex: i64,
}

/// A fight in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatState {
    /// The encounter being fought.
    pub encounter: EncounterState,
    /// The initiative order, fixed at the start; dead entries are skipped, never removed.
    pub order: Vec<Initiative>,
    /// Index into `order` of the actor whose turn it is.
    pub current: u8,
    /// The round, from one.
    pub round: u32,
    /// Who lost their first round.
    pub surprised: Surprise,
    /// Members dodging until the round ends, sorted.
    pub dodging: Vec<CharacterId>,
    /// Gold looted so far, paid out on victory.
    pub gold: u32,
}

impl CombatState {
    /// The stacks in front: the first living ones, as many as the rules allow. A stack that
    /// empties drops out and the next living one moves up.
    #[must_use]
    pub fn front_stacks(&self, data: &Data) -> Vec<u8> {
        self.encounter
            .stacks
            .iter()
            .enumerate()
            .filter(|(_, s)| s.alive())
            .map(|(i, _)| u8::try_from(i).unwrap_or(u8::MAX))
            .take(monster_front_stacks(data))
            .collect()
    }

    /// Whether a stack stands in front.
    #[must_use]
    pub fn is_front(&self, data: &Data, stack: u8) -> bool {
        self.front_stacks(data).contains(&stack)
    }

    /// The actor whose turn it is.
    #[must_use]
    pub fn current_actor(&self) -> Option<ActorRef> {
        self.order.get(usize::from(self.current)).map(|e| e.actor)
    }

    /// Whether the fight is over: no stack stands, or no member can fight.
    #[must_use]
    pub fn outcome(&self, party: &Party, data: &Data) -> Option<CombatOutcome> {
        if !self.encounter.stacks.iter().any(|s| s.alive()) {
            return Some(CombatOutcome::Victory);
        }
        if !party.members.iter().any(|m| can_fight(m, data)) {
            return Some(CombatOutcome::Defeat);
        }
        None
    }
}

/// Whether a member is dead: carries the pack's `dead` condition.
#[must_use]
pub fn is_dead(member: &Character, data: &Data) -> bool {
    condition_id(data, "dead").is_some_and(|dead| member.conditions.contains(&dead))
}

/// Whether a member can still take part: up, alive, and not incapacitated.
#[must_use]
pub fn can_fight(member: &Character, data: &Data) -> bool {
    !member.is_down() && !is_dead(member, data) && !flags(&member.conditions, data).incapacitated
}
