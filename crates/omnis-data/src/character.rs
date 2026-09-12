//! `data/races`, `data/classes`, `data/backgrounds`: what a character is made of. Names and
//! feature names are text keys; the loader checks that every key exists in some language and
//! that every item and spell a class names is defined.

use crate::error::DataError;
use crate::terms::{Ability, ArmorKind, DamageType, SaveAgainst, Size, Skill, WeaponKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// A named trait with an optional mechanical effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feature {
    /// Text key of the trait's name.
    pub name: String,
    /// What the rules do with it; `Text` is flavour only.
    #[serde(default)]
    pub effect: Effect,
}

/// The mechanical effects a feature can carry in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Effect {
    /// Flavour text only.
    #[default]
    Text,
    /// Sight in darkness out to a distance in feet.
    Darkvision {
        /// Range in feet.
        feet: u8,
    },
    /// Hit point maximum per level (hill dwarf toughness).
    HitPointsPerLevel(i8),
    /// Resistance to one damage type.
    Resistance(DamageType),
    /// Proficiency in one skill.
    SkillProficiency(Skill),
    /// Advantage on saving throws against something.
    SaveAdvantage(SaveAgainst),
}

/// A playable race (subraces folded in; SRD 5.1 offers one each).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Race {
    /// Schema version.
    pub schema: u32,
    /// `pack:race:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// Score increases applied at creation.
    #[serde(default)]
    pub ability_bonuses: BTreeMap<Ability, i8>,
    /// Creature size.
    pub size: Size,
    /// Walking speed in feet.
    pub speed: u8,
    /// Age at which a created character starts.
    pub starting_age: u16,
    /// Racial traits.
    #[serde(default)]
    pub features: Vec<Feature>,
}

impl Race {
    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.speed == 0 || self.speed > 120 {
            errors.push(DataError::new(file, "speed must be 1..=120"));
        }
        if self.starting_age == 0 {
            errors.push(DataError::new(file, "starting_age must be at least 1"));
        }
        for (ability, bonus) in &self.ability_bonuses {
            if !(-5..=5).contains(bonus) {
                errors.push(DataError::new(
                    file,
                    format!("{ability:?} bonus {bonus} must be -5..=5"),
                ));
            }
        }
    }
}

/// How many skills a class picks and from which list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillChoice {
    /// How many.
    pub choose: u8,
    /// The candidates.
    pub from: Vec<Skill>,
}

/// A class's spellcasting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Casting {
    /// The casting ability (PRD D12: it scales the point pool).
    pub ability: Ability,
    /// Half casters use half their level in the pool formula.
    #[serde(default)]
    pub half: bool,
    /// Cantrips known at level 1.
    pub cantrips_at_1: u8,
    /// Levelled spells known at level 1.
    pub spells_at_1: u8,
    /// Spell ids on the class list.
    #[serde(default)]
    pub list: Vec<String>,
}

/// A class feature gained at a level (display only in v1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassFeature {
    /// Level gained.
    pub level: u8,
    /// Text key of the feature's name.
    pub name: String,
}

/// A character class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Class {
    /// Schema version.
    pub schema: u32,
    /// `pack:class:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// Faces of the hit die.
    pub hit_die: u8,
    /// The two proficient saving throws.
    pub saving_throws: [Ability; 2],
    /// Armor proficiencies.
    #[serde(default)]
    pub armor: Vec<ArmorKind>,
    /// Weapon category proficiencies.
    #[serde(default)]
    pub weapons: Vec<WeaponKind>,
    /// Individual weapon proficiencies by item id.
    #[serde(default)]
    pub weapon_ids: Vec<String>,
    /// Skill picks at creation.
    pub skills: SkillChoice,
    /// Starting equipment as `(item id, count)`; the first option of each SRD line.
    #[serde(default)]
    pub starting_equipment: Vec<(String, u16)>,
    /// Spellcasting, for casters.
    #[serde(default)]
    pub casting: Option<Casting>,
    /// Features by level.
    #[serde(default)]
    pub features: Vec<ClassFeature>,
}

impl Class {
    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if ![6, 8, 10, 12].contains(&self.hit_die) {
            errors.push(DataError::new(file, "hit_die must be 6, 8, 10, or 12"));
        }
        if self.saving_throws[0] == self.saving_throws[1] {
            errors.push(DataError::new(
                file,
                "saving_throws must name two abilities",
            ));
        }
        if usize::from(self.skills.choose) > self.skills.from.len() {
            errors.push(DataError::new(
                file,
                format!(
                    "skills: cannot choose {} from {}",
                    self.skills.choose,
                    self.skills.from.len()
                ),
            ));
        }
        if self.starting_equipment.iter().any(|(_, n)| *n == 0) {
            errors.push(DataError::new(
                file,
                "starting_equipment counts must be at least 1",
            ));
        }
        if let Some(casting) = &self.casting
            && casting.spells_at_1 > 0
            && casting.list.is_empty()
        {
            errors.push(DataError::new(
                file,
                "casting: spells_at_1 > 0 needs a spell list",
            ));
        }
    }
}

/// A character background.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Background {
    /// Schema version.
    pub schema: u32,
    /// `pack:background:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// Skill proficiencies granted.
    #[serde(default)]
    pub skills: Vec<Skill>,
    /// Starting equipment as `(item id, count)`.
    #[serde(default)]
    pub equipment: Vec<(String, u16)>,
    /// Starting gold pieces.
    #[serde(default)]
    pub gold: u16,
    /// The background feature.
    pub feature: Feature,
}

impl Background {
    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        let mut seen = self.skills.clone();
        seen.sort();
        seen.dedup();
        if seen.len() != self.skills.len() {
            errors.push(DataError::new(file, "skills are listed twice"));
        }
        if self.equipment.iter().any(|(_, n)| *n == 0) {
            errors.push(DataError::new(file, "equipment counts must be at least 1"));
        }
    }
}
