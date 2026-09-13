//! The state of a fight between two player commands.

use crate::encounter::EncounterState;
use crate::event::{ActorRef, Surprise};
use alloc::vec::Vec;
use omnis_core::CharacterId;
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
    /// Gold looted so far, paid out on victory.
    pub gold: u32,
}
