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
    /// Copper looted so far, paid out on victory.
    pub gold: u32,
    /// What the actor whose turn it is has left to spend (PRD D21, ARCHITECTURE.md §4.7).
    #[serde(default)]
    pub budget: Budget,
    /// Reactions left to each combatant (a member, or a stack as one), refreshed at the start
    /// of its own turn; sorted by actor.
    #[serde(default)]
    pub reactions: Vec<(ActorRef, u8)>,
    /// Members hidden by Cunning Action: their next attack has advantage. Sorted.
    #[serde(default)]
    pub hidden: Vec<CharacterId>,
}

/// The actions and bonus actions left in the current turn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    /// Actions left.
    pub actions: u8,
    /// Bonus actions left.
    pub bonus_actions: u8,
}

impl CombatState {
    /// Reactions `actor` has left this round.
    #[must_use]
    pub fn reactions_left(&self, actor: ActorRef) -> u8 {
        self.reactions
            .binary_search_by(|(a, _)| a.cmp(&actor))
            .map_or(0, |at| self.reactions[at].1)
    }

    /// Set what `actor` has left.
    pub fn set_reactions(&mut self, actor: ActorRef, count: u8) {
        match self.reactions.binary_search_by(|(a, _)| a.cmp(&actor)) {
            Ok(at) => self.reactions[at].1 = count,
            Err(at) => self.reactions.insert(at, (actor, count)),
        }
    }

    /// Spend one of `actor`'s reactions; `false` when none was left.
    pub fn spend_reaction(&mut self, actor: ActorRef) -> bool {
        let left = self.reactions_left(actor);
        if left == 0 {
            return false;
        }
        self.set_reactions(actor, left - 1);
        true
    }

    /// Whether a member is hidden, and no longer once they attack: their attack has
    /// advantage.
    pub fn reveal(&mut self, member: CharacterId) -> bool {
        match self.hidden.binary_search(&member) {
            Ok(at) => {
                self.hidden.remove(at);
                true
            }
            Err(_) => false,
        }
    }

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
