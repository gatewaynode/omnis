//! The state of a fight between two player commands, and what can be read off it.

use crate::encounter::{ENGAGED_FEET, EncounterState};
use crate::event::{ActorRef, CombatOutcome, Surprise};
use crate::party::Party;
use alloc::vec::Vec;
use omnis_core::{CharacterId, ItemId};
use omnis_data::Data;
use omnis_rules::{Character, condition_id, flags};
use serde::{Deserialize, Serialize};

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
    /// Items the dead dropped so far, laid on the fight's cell on victory (M8.5 step 2);
    /// unwritten when none, so a save of a fight without loot reads as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loot: Vec<(ItemId, u16)>,
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
    /// Monster individuals under Shield until their stack's next turn, `(stack, individual)`;
    /// sorted.
    #[serde(default)]
    pub monster_shields: Vec<(u8, u8)>,
    /// The member whose spell opened the fight from the road (the opening strike): the strike
    /// was their action for the first round, taken off when that turn begins. Unwritten when
    /// none, so a save of a fight that began any other way reads as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub struck: Option<CharacterId>,
    /// Feet the party's line has advanced this round, dashes included. Unwritten when none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub advanced: u16,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero(n: &u16) -> bool {
    *n == 0
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

    /// The engaged stacks: the living ones five feet from the party's front line. What "in
    /// front" means since the fight is counted in feet (owner, 2026-11-14).
    #[must_use]
    pub fn engaged_stacks(&self) -> Vec<u8> {
        self.encounter
            .stacks
            .iter()
            .enumerate()
            .filter(|(_, s)| s.alive() && s.distance <= ENGAGED_FEET)
            .map(|(i, _)| u8::try_from(i).unwrap_or(u8::MAX))
            .collect()
    }

    /// Whether a stack is engaged.
    #[must_use]
    pub fn is_engaged(&self, stack: u8) -> bool {
        self.encounter
            .stacks
            .get(usize::from(stack))
            .is_some_and(|s| s.alive() && s.distance <= ENGAGED_FEET)
    }

    /// How many of `stack` may attack in melee this round: the front row presents one
    /// five-foot square a member, the stacks within their melee reach fill those squares
    /// nearest first (the earlier stack of two as near), and a longer reach lets further ranks
    /// strike past the first.
    #[must_use]
    pub fn melee_slots(&self, party: &Party, data: &Data, stack: u8) -> u16 {
        let mut left = frontage(party, data);
        // A split leaves the stacks out of the order they stand in.
        let mut nearest: Vec<_> = self.encounter.stacks.iter().enumerate().collect();
        nearest.sort_by_key(|(index, s)| (s.distance, *index));
        for (index, s) in nearest {
            let Some(monster) = data.monsters.get(&s.monster) else {
                continue;
            };
            let Some(melee) = monster.attacks.iter().find(|a| !a.ranged) else {
                continue;
            };
            if !s.alive() || s.distance > u16::from(melee.reach) {
                continue;
            }
            let taken = monster.line(u16::from(s.count())).min(left);
            if index == usize::from(stack) {
                let ranks = (u16::from(melee.reach) / ENGAGED_FEET).max(1);
                return monster.abreast(taken).saturating_mul(ranks);
            }
            left -= taken;
        }
        0
    }

    /// Whether a foe is upon the member in slot `own`: they stand in the front row and a stack
    /// is engaged. Their ranged attacks have disadvantage (the SRD's rule for a hostile within
    /// five feet).
    #[must_use]
    pub fn presses(&self, own: usize, data: &Data) -> bool {
        own < crate::party::front_row(data) && !self.engaged_stacks().is_empty()
    }

    /// Feet to the nearest stack that stands.
    #[must_use]
    pub fn nearest(&self) -> Option<u16> {
        let standing = self.encounter.stacks.iter().filter(|s| s.alive());
        standing.map(|s| s.distance).min()
    }

    /// Feet the party's line may still advance without a dash: what is left of its pace this
    /// round, and never past the nearest stack, which stops it once engaged.
    #[must_use]
    pub fn room(&self, party: &Party, data: &Data) -> u16 {
        let allowed = pace(party, data);
        let gap = self.nearest().map_or(0, |d| d.saturating_sub(ENGAGED_FEET));
        allowed.saturating_sub(self.advanced).min(gap)
    }

    /// The line advances: every stack is that much nearer, none nearer than engaged.
    pub(crate) fn advance(&mut self, feet: u16, events: &mut Vec<crate::event::Event>) {
        if feet == 0 {
            return;
        }
        for stack in &mut self.encounter.stacks {
            stack.distance = stack.distance.saturating_sub(feet).max(ENGAGED_FEET);
        }
        self.advanced = self.advanced.saturating_add(feet);
        events.push(crate::event::Event::PartyAdvanced { feet });
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

/// The party's pace in feet a round: its line moves as one, at the speed of the slowest member
/// standing in the front row (of the rest once the front row has fallen).
#[must_use]
pub fn pace(party: &Party, data: &Data) -> u16 {
    let front_row = crate::party::front_row(data);
    let split = front_row.min(party.members.len());
    let (front, back) = party.members.split_at(split);
    let slowest = |row: &[Character]| {
        let standing = row.iter().filter(|m| !m.is_down());
        standing
            .filter_map(|m| data.races.get(&m.race))
            .map(|r| u16::from(r.speed))
            .min()
    };
    slowest(front).or_else(|| slowest(back)).unwrap_or(30)
}

/// The five-foot squares the party's front presents: one for each member of the front row still
/// standing, or for each of the rest once the front row has fallen; one at the least.
#[must_use]
pub fn frontage(party: &Party, data: &Data) -> u16 {
    let front_row = crate::party::front_row(data);
    let standing = |range: &[Character]| range.iter().filter(|m| !m.is_down()).count();
    let split = front_row.min(party.members.len());
    let (front, back) = party.members.split_at(split);
    let squares = match standing(front) {
        0 => standing(back),
        some => some,
    };
    u16::try_from(squares).unwrap_or(u16::MAX).max(1)
}
