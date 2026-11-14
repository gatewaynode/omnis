//! `data/monsters`: SRD stat blocks reduced to what the crawler uses (M4 consumes them).

use crate::durability::{
    DEFAULT_RANGE, MAX_PER_SQUARE, is_melee_reach, melee_reach, span_problems,
};
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
    /// How far it reaches in melee, in feet.
    #[serde(default = "melee_reach", skip_serializing_if = "is_melee_reach")]
    pub reach: u8,
    /// Normal and long range in feet, for a ranged attack; left out it is 80 and 320.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<(u16, u16)>,
}

impl Attack {
    /// Normal and long range in feet; `None` for a melee attack.
    #[must_use]
    pub fn span(&self) -> Option<(u16, u16)> {
        self.ranged.then(|| self.range.unwrap_or(DEFAULT_RANGE))
    }
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
    /// How many of it fit a five-foot square (owner, 2026-11-14); left out, its size says
    /// ([`Monster::per_square`]). Not for a Large or bigger one, which takes several squares.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_square: Option<u8>,
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

    /// How many of it fit a five-foot square.
    #[must_use]
    pub fn per_square(&self) -> u8 {
        self.per_square.unwrap_or(self.size.per_square())
    }

    /// The squares `count` of it take: one each for a big one's whole space, shared for the
    /// rest. At least one for any.
    #[must_use]
    pub fn squares(&self, count: u16) -> u16 {
        if count == 0 {
            return 0;
        }
        let space = u16::from(self.size.space());
        if space > 1 {
            count.saturating_mul(space)
        } else {
            count.div_ceil(u16::from(self.per_square().max(1)))
        }
    }

    /// How many of it `squares` squares hold: at least one, since something always fits.
    #[must_use]
    pub fn fitting(&self, squares: u16) -> u16 {
        let space = u16::from(self.size.space());
        let held = if space > 1 {
            squares / space
        } else {
            squares.saturating_mul(u16::from(self.per_square().max(1)))
        };
        held.max(1)
    }

    /// The squares of a line `count` of it stand across, shoulder to shoulder.
    #[must_use]
    pub fn line(&self, count: u16) -> u16 {
        let front = u16::from(self.size.front());
        if front > 1 {
            count.saturating_mul(front)
        } else {
            count.div_ceil(u16::from(self.per_square().max(1)))
        }
    }

    /// How many of it stand abreast across `squares` squares of a line: none across none, and
    /// at least one across any.
    #[must_use]
    pub fn abreast(&self, squares: u16) -> u16 {
        if squares == 0 {
            return 0;
        }
        let front = u16::from(self.size.front());
        if front > 1 {
            (squares / front).max(1)
        } else {
            squares.saturating_mul(u16::from(self.per_square().max(1)))
        }
    }

    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        for attack in &self.attacks {
            for problem in span_problems(attack.reach, attack.range, attack.ranged) {
                errors.push(DataError::new(file, problem));
            }
        }
        match self.per_square {
            Some(n) if n == 0 || n > MAX_PER_SQUARE => errors.push(DataError::new(
                file,
                format!("per_square must be 1..={MAX_PER_SQUARE}"),
            )),
            Some(_) if self.size.space() > 1 => errors.push(DataError::new(
                file,
                "per_square is for a Medium or smaller monster; a bigger one takes several squares",
            )),
            _ => {}
        }
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
