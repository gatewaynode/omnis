//! `data/monsters`: SRD stat blocks reduced to what the crawler uses (M4 consumes them).

use crate::error::DataError;
use crate::terms::{DamageType, Size};
use omnis_core::Dice;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One attack in a stat block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attack {
    /// Text key of the attack's name.
    pub name: String,
    /// Attack bonus.
    pub to_hit: i8,
    /// Damage on a hit.
    pub damage: Dice,
    /// Damage type.
    pub damage_type: DamageType,
    /// Usable from the back and against the party's back row.
    #[serde(default)]
    pub ranged: bool,
}

/// One monster type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monster {
    /// Schema version.
    pub schema: u32,
    /// `pack:monster:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// Creature size.
    pub size: Size,
    /// Armor class.
    pub ac: u8,
    /// Hit dice.
    pub hit_points: Dice,
    /// Speed in feet.
    pub speed: u8,
    /// The six ability scores in SRD order.
    pub abilities: [u8; 6],
    /// Challenge rating as a fraction `(numerator, denominator)`.
    pub challenge: (u16, u16),
    /// Experience for defeating one.
    pub xp: u32,
    /// Attacks, in order of preference.
    #[serde(default)]
    pub attacks: Vec<Attack>,
    /// Gold carried by one individual, rolled when it dies.
    #[serde(default)]
    pub gold: Option<Dice>,
    /// Damage types dealt at half.
    #[serde(default)]
    pub resistances: Vec<DamageType>,
    /// Damage types that do nothing.
    #[serde(default)]
    pub immunities: Vec<DamageType>,
    /// Damage types dealt double.
    #[serde(default)]
    pub vulnerabilities: Vec<DamageType>,
}

impl Monster {
    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.ac == 0 || self.ac > 30 {
            errors.push(DataError::new(file, "ac must be 1..=30"));
        }
        if self.hit_points.count == 0 || self.hit_points.sides == 0 {
            errors.push(DataError::new(file, "hit_points needs dice"));
        }
        if self.abilities.iter().any(|a| *a == 0 || *a > 30) {
            errors.push(DataError::new(file, "abilities must be 1..=30"));
        }
        if self.challenge.1 == 0 {
            errors.push(DataError::new(
                file,
                "challenge denominator must be at least 1",
            ));
        }
        if self
            .attacks
            .iter()
            .any(|a| a.damage.count == 0 || a.damage.sides == 0)
        {
            errors.push(DataError::new(file, "attack damage needs dice"));
        }
        if self.gold.is_some_and(|g| g.count == 0 || g.sides == 0) {
            errors.push(DataError::new(file, "gold needs dice"));
        }
        let mut lists: BTreeMap<DamageType, usize> = BTreeMap::new();
        for list in [&self.resistances, &self.immunities, &self.vulnerabilities] {
            for kind in list.iter().collect::<BTreeSet<_>>() {
                *lists.entry(*kind).or_default() += 1;
            }
        }
        for (kind, n) in lists {
            if n > 1 {
                errors.push(DataError::new(
                    file,
                    format!(
                        "damage type {kind:?} appears in two of resistances, immunities, vulnerabilities"
                    ),
                ));
            }
        }
    }
}
