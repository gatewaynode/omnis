//! `data/monsters`: SRD stat blocks reduced to what the crawler uses (M4 consumes them).

use crate::error::DataError;
use crate::object::{Look, Shape, check_look};
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

/// A monster's spellcasting (homebrew until monster runbooks: a dice roll picks among what it
/// can cast; ARCHITECTURE.md §4.7). Spells use the members' effects with the stat block's own
/// numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterCasting {
    /// Spell attack bonus.
    pub spell_attack: i8,
    /// Spell save DC.
    pub save_dc: u8,
    /// Caster level, for a cantrip's dice.
    pub caster_level: u8,
    /// Spell points each individual has (a spell costs its level; cantrips are free).
    pub points: u8,
    /// `pack:spell:name` ids: attack, auto-hit and save spells, and an armor-bonus reaction.
    pub spells: Vec<String>,
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
    /// Spells it casts, if any.
    #[serde(default)]
    pub casting: Option<MonsterCasting>,
    /// `pack:loot:name`: the table its items are drawn from when it dies (M8.5 step 2).
    #[serde(default)]
    pub loot: Option<String>,
    /// How it is drawn when its group stands in view (M8.5 step 2c), a `Shape::Figure`; left
    /// out, its size gives one (`figure_for`).
    #[serde(default)]
    pub look: Option<Look>,
}

/// The figure a monster of a size is drawn as, in eighths of a cell (a wall is 8 high): a cell's
/// width at most, so a Large one fills its cell and the bigger ones only stand taller.
#[must_use]
pub const fn figure_for(size: Size) -> Look {
    let (width, height) = match size {
        Size::Tiny => (2, 2),
        Size::Small => (3, 3),
        Size::Medium => (4, 5),
        Size::Large => (8, 8),
        Size::Huge => (8, 12),
        Size::Gargantuan => (8, 16),
    };
    Look {
        shape: Shape::Figure,
        width,
        depth: width,
        height,
    }
}

impl Monster {
    /// How it is drawn: its own look, or its size's figure.
    #[must_use]
    pub fn figure(&self) -> Look {
        self.look.unwrap_or(figure_for(self.size))
    }

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
        if let Some(casting) = &self.casting {
            if casting.save_dc == 0 || casting.save_dc > 30 {
                errors.push(DataError::new(file, "casting save_dc must be 1..=30"));
            }
            if casting.caster_level == 0 || casting.caster_level > 20 {
                errors.push(DataError::new(file, "casting caster_level must be 1..=20"));
            }
            if casting.spells.is_empty() {
                errors.push(DataError::new(file, "casting needs at least one spell"));
            }
        }
        if let Some(look) = &self.look {
            if look.shape != Shape::Figure {
                errors.push(DataError::new(file, "look must be a Figure"));
            }
            check_look(look, "look", &mut |m| errors.push(DataError::new(file, m)));
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
