//! Active effects: what a spell (later an item) left on a member or on the party, and when it
//! ends. Durations are absolute party-clock minutes (a fight round is one minute), or the
//! bearer's next turn. The vectors that hold effects contain only live ones: the simulation
//! prunes them whenever the clock moves, so readers never need the time.

use alloc::vec::Vec;
use omnis_core::{CharacterId, Dice, Pcg32, RollTrace, SpellId, StreamName};
use omnis_data::BuffOn;
use omnis_expr::RuleError;
use serde::{Deserialize, Serialize};

/// When an effect ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expiry {
    /// Ends when the party clock reaches this minute.
    Minute(i64),
    /// Ends when the bearer's next turn begins, or when the fight ends.
    NextTurn,
}

/// What an effect does while it lasts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectKind {
    /// A die added to the listed rolls; `consumed` removes it after the first roll it touches.
    Buff {
        /// The die.
        bonus: Dice,
        /// Which rolls.
        on: Vec<BuffOn>,
        /// Spent by the first roll.
        consumed: bool,
    },
    /// Added to armor class.
    ArmorBonus(i64),
    /// A light source: the party sees at least this far.
    Light {
        /// Tiles.
        depth: u8,
    },
}

/// One effect in force.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveEffect {
    /// The spell that made it.
    pub source: SpellId,
    /// Who cast it.
    pub caster: CharacterId,
    /// Ends when the caster loses concentration.
    pub concentration: bool,
    /// What it does.
    pub kind: EffectKind,
    /// When it ends.
    pub until: Expiry,
}

impl ActiveEffect {
    /// Whether a timed effect has run out at `now` minutes on the party clock.
    #[must_use]
    pub const fn expired(&self, now: i64) -> bool {
        matches!(self.until, Expiry::Minute(m) if now >= m)
    }

    /// Whether the effect adds a die to this kind of roll.
    #[must_use]
    pub fn buffs(&self, on: BuffOn) -> bool {
        matches!(&self.kind, EffectKind::Buff { on: kinds, .. } if kinds.contains(&on))
    }
}

/// The first buff die that touches this kind of roll.
#[must_use]
pub fn bonus_dice(effects: &[ActiveEffect], on: BuffOn) -> Option<Dice> {
    effects.iter().find_map(|e| match &e.kind {
        EffectKind::Buff {
            bonus, on: kinds, ..
        } if kinds.contains(&on) => Some(*bonus),
        _ => None,
    })
}

/// Roll the buff die for this kind of roll, when one is in force.
pub fn roll_bonus(
    effects: &[ActiveEffect],
    on: BuffOn,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<Option<RollTrace>, RuleError> {
    match bonus_dice(effects, on) {
        Some(dice) => dice
            .roll(rng, stream)
            .map(Some)
            .map_err(|e| RuleError::new("buff", alloc::format!("{e}"))),
        None => Ok(None),
    }
}

/// The index of the first buff on this kind of roll that is spent by use.
#[must_use]
pub fn consumable_index(effects: &[ActiveEffect], on: BuffOn) -> Option<usize> {
    effects.iter().position(|e| {
        matches!(&e.kind, EffectKind::Buff { on: kinds, consumed: true, .. } if kinds.contains(&on))
    })
}

/// Every armor class bonus in force, summed.
#[must_use]
pub fn armor_bonus(effects: &[ActiveEffect]) -> i64 {
    effects
        .iter()
        .map(|e| match e.kind {
            EffectKind::ArmorBonus(n) => n,
            _ => 0,
        })
        .sum()
}

/// The farthest any light in force reaches.
#[must_use]
pub fn light_depth(effects: &[ActiveEffect]) -> Option<u8> {
    effects
        .iter()
        .filter_map(|e| match e.kind {
            EffectKind::Light { depth } => Some(depth),
            _ => None,
        })
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn effect(kind: EffectKind, until: Expiry) -> ActiveEffect {
        ActiveEffect {
            source: SpellId(0),
            caster: CharacterId(0),
            concentration: false,
            kind,
            until,
        }
    }

    #[test]
    fn buffs_are_found_by_roll_kind_and_bonuses_sum() {
        let bless = effect(
            EffectKind::Buff {
                bonus: Dice::new(1, 4),
                on: vec![BuffOn::AttackRolls, BuffOn::SavingThrows],
                consumed: false,
            },
            Expiry::Minute(10),
        );
        let guidance = effect(
            EffectKind::Buff {
                bonus: Dice::new(1, 6),
                on: vec![BuffOn::AbilityChecks],
                consumed: true,
            },
            Expiry::Minute(10),
        );
        let shield = effect(EffectKind::ArmorBonus(5), Expiry::NextTurn);
        let all = [bless, guidance, shield.clone(), shield];
        assert_eq!(bonus_dice(&all, BuffOn::AttackRolls), Some(Dice::new(1, 4)));
        assert_eq!(
            bonus_dice(&all, BuffOn::AbilityChecks),
            Some(Dice::new(1, 6))
        );
        assert_eq!(consumable_index(&all, BuffOn::AbilityChecks), Some(1));
        assert_eq!(consumable_index(&all, BuffOn::AttackRolls), None);
        assert_eq!(armor_bonus(&all), 10);
        assert_eq!(light_depth(&all), None);
        assert!(all[0].buffs(BuffOn::SavingThrows) && !all[0].buffs(BuffOn::AbilityChecks));
        let mut rng = Pcg32::for_stream(1, &StreamName::new("t"));
        let trace = roll_bonus(&all, BuffOn::AttackRolls, &mut rng, &StreamName::new("t"))
            .unwrap()
            .unwrap();
        assert!((1..=4).contains(&trace.total));
        assert!(
            roll_bonus(&[], BuffOn::AttackRolls, &mut rng, &StreamName::new("t"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn timed_effects_expire_and_turn_bound_ones_do_not_by_time() {
        let timed = effect(EffectKind::Light { depth: 8 }, Expiry::Minute(60));
        assert!(!timed.expired(59) && timed.expired(60) && timed.expired(61));
        let turn = effect(EffectKind::ArmorBonus(5), Expiry::NextTurn);
        assert!(!turn.expired(i64::MAX));
        assert_eq!(light_depth(&[timed]), Some(8));
    }
}
