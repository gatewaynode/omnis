//! Monsters before the party: where they came from, the stacks they stand in, how they feel,
//! and where the party retreats to. The trigger and the pre-combat choice arrive with M4's
//! encounter step.

use alloc::vec::Vec;
use omnis_core::{MonsterId, Position};
use omnis_data::Disposition;
use serde::{Deserialize, Serialize};

/// Where an encounter came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EncounterSource {
    /// A placement on the map, by its index in the map file.
    Fixed(u16),
    /// The map's random table.
    Random,
}

/// A stack of one monster type. Individuals are their hit points; a dead one is removed, so
/// the length is the living count. An emptied stack keeps its slot so indices stay stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stack {
    /// The monster type.
    pub monster: MonsterId,
    /// How many there were.
    pub initial: u8,
    /// Hit points of each living individual, in order.
    pub hp: Vec<i32>,
}

impl Stack {
    /// Whether anyone in it still stands.
    #[must_use]
    pub fn alive(&self) -> bool {
        !self.hp.is_empty()
    }

    /// The living count.
    #[must_use]
    pub fn count(&self) -> u8 {
        u8::try_from(self.hp.len()).unwrap_or(u8::MAX)
    }
}

/// An encounter in progress: from the trigger through the fight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterState {
    /// Where it came from.
    pub source: EncounterSource,
    /// The stacks, in order; the first ones stand in front.
    pub stacks: Vec<Stack>,
    /// How the monsters feel about the party.
    pub disposition: Disposition,
    /// Where Run and Flee put the party: the tile it came from, facing away.
    pub retreat: Position,
}
