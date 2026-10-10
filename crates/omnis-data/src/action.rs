//! What a fight action costs from a combatant's turn budget (PRD D21, ARCHITECTURE.md §4.7),
//! and the class features that act in a fight, with their uses per rest. A spell states its
//! cost and D24's fields in its own file (`spell.rs`); a class feature carries its effect, cost
//! and uses beside its level and name (`character.rs`).

use crate::error::DataError;
use omnis_core::Dice;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The part of a turn's budget an action spends.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Cost {
    /// The turn's action.
    #[default]
    Action,
    /// The turn's bonus action.
    BonusAction,
    /// A reaction, spent outside the combatant's own turn when a declared trigger fires.
    Reaction,
    /// Nothing: Action Surge takes no action of its own (SRD).
    Free,
}

/// Which rest gives a feature's uses back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Recharge {
    /// A short or a long rest.
    ShortRest,
    /// A long rest only.
    LongRest,
}

/// How often a feature may be used before a rest gives its uses back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Uses {
    /// Uses between rests.
    pub count: u8,
    /// The rest that restores them.
    pub per: Recharge,
}

/// What a class feature does in a fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureEffect {
    /// The member heals themself: the dice, plus the class level when `per_level` (Second Wind).
    Heal {
        /// Dice rolled.
        dice: Dice,
        /// Add the member's level.
        per_level: bool,
    },
    /// One more action this turn (Action Surge).
    ExtraAction,
    /// An exchange that provokes no opportunity attack, or Hide (Cunning Action: Dash and
    /// Disengage fold into the exchange, PRD §8.3's rows standing in for movement).
    Cunning,
}

impl FeatureEffect {
    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, cost: Cost, errors: &mut Vec<DataError>) {
        if let Self::Heal { dice, .. } = self
            && (dice.count == 0 || dice.sides < 2)
        {
            errors.push(DataError::new(
                file,
                "feature heal dice must roll something",
            ));
        }
        if cost == Cost::Reaction {
            errors.push(DataError::new(
                file,
                "no class feature reacts yet: a feature costs an action, a bonus action or nothing",
            ));
        }
    }
}
