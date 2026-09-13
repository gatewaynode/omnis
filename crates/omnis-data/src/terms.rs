//! Closed vocabularies the SRD content shares: abilities, skills, sizes, alignments, damage
//! types, armor and weapon categories, spell schools. Enums, so a typo in a pack is a parse
//! error with a line number rather than an unknown string at runtime.

use serde::{Deserialize, Serialize};

/// The six ability scores, in SRD order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Ability {
    /// Physical power.
    Strength,
    /// Agility and reflexes.
    Dexterity,
    /// Endurance.
    Constitution,
    /// Reasoning and memory.
    Intelligence,
    /// Perception and insight.
    Wisdom,
    /// Force of personality.
    Charisma,
}

impl Ability {
    /// Every ability in score order.
    pub const ALL: [Ability; 6] = [
        Ability::Strength,
        Ability::Dexterity,
        Ability::Constitution,
        Ability::Intelligence,
        Ability::Wisdom,
        Ability::Charisma,
    ];

    /// Position in a six-score array.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The SRD three-letter abbreviation.
    #[must_use]
    pub const fn short(self) -> &'static str {
        match self {
            Ability::Strength => "STR",
            Ability::Dexterity => "DEX",
            Ability::Constitution => "CON",
            Ability::Intelligence => "INT",
            Ability::Wisdom => "WIS",
            Ability::Charisma => "CHA",
        }
    }
}

/// The eighteen SRD skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum Skill {
    Acrobatics,
    AnimalHandling,
    Arcana,
    Athletics,
    Deception,
    History,
    Insight,
    Intimidation,
    Investigation,
    Medicine,
    Nature,
    Perception,
    Performance,
    Persuasion,
    Religion,
    SleightOfHand,
    Stealth,
    Survival,
}

impl Skill {
    /// Every skill in SRD order.
    pub const ALL: [Skill; 18] = [
        Skill::Acrobatics,
        Skill::AnimalHandling,
        Skill::Arcana,
        Skill::Athletics,
        Skill::Deception,
        Skill::History,
        Skill::Insight,
        Skill::Intimidation,
        Skill::Investigation,
        Skill::Medicine,
        Skill::Nature,
        Skill::Perception,
        Skill::Performance,
        Skill::Persuasion,
        Skill::Religion,
        Skill::SleightOfHand,
        Skill::Stealth,
        Skill::Survival,
    ];

    /// The ability a check with this skill uses.
    #[must_use]
    pub const fn ability(self) -> Ability {
        match self {
            Skill::Athletics => Ability::Strength,
            Skill::Acrobatics | Skill::SleightOfHand | Skill::Stealth => Ability::Dexterity,
            Skill::Arcana
            | Skill::History
            | Skill::Investigation
            | Skill::Nature
            | Skill::Religion => Ability::Intelligence,
            Skill::AnimalHandling
            | Skill::Insight
            | Skill::Medicine
            | Skill::Perception
            | Skill::Survival => Ability::Wisdom,
            Skill::Deception | Skill::Intimidation | Skill::Performance | Skill::Persuasion => {
                Ability::Charisma
            }
        }
    }
}

/// Creature size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
    Gargantuan,
}

/// The nine-way SRD alignment grid. Gating by alignment is per-pack content (PRD §8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum Alignment {
    LawfulGood,
    NeutralGood,
    ChaoticGood,
    LawfulNeutral,
    TrueNeutral,
    ChaoticNeutral,
    LawfulEvil,
    NeutralEvil,
    ChaoticEvil,
}

impl Alignment {
    /// Every alignment, lawful to chaotic within good to evil.
    pub const ALL: [Alignment; 9] = [
        Alignment::LawfulGood,
        Alignment::NeutralGood,
        Alignment::ChaoticGood,
        Alignment::LawfulNeutral,
        Alignment::TrueNeutral,
        Alignment::ChaoticNeutral,
        Alignment::LawfulEvil,
        Alignment::NeutralEvil,
        Alignment::ChaoticEvil,
    ];
}

/// The thirteen SRD damage types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum DamageType {
    Acid,
    Bludgeoning,
    Cold,
    Fire,
    Force,
    Lightning,
    Necrotic,
    Piercing,
    Poison,
    Psychic,
    Radiant,
    Slashing,
    Thunder,
}

/// Armor proficiency categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ArmorKind {
    Light,
    Medium,
    Heavy,
    Shield,
}

/// Weapon proficiency categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum WeaponKind {
    Simple,
    Martial,
}

/// The eight schools of magic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum School {
    Abjuration,
    Conjuration,
    Divination,
    Enchantment,
    Evocation,
    Illusion,
    Necromancy,
    Transmutation,
}

/// What a saving-throw advantage from a racial trait applies against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum SaveAgainst {
    Poison,
    Charm,
    Fear,
    Sleep,
    Magic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abilities_index_their_array_and_skills_map_to_one_ability() {
        for (i, ability) in Ability::ALL.iter().enumerate() {
            assert_eq!(ability.index(), i);
            assert_eq!(ability.short().len(), 3);
        }
        assert_eq!(Skill::ALL.len(), 18);
        assert_eq!(Skill::Stealth.ability(), Ability::Dexterity);
        assert_eq!(Skill::Religion.ability(), Ability::Intelligence);
        assert_eq!(Skill::Persuasion.ability(), Ability::Charisma);
        let wisdom = Skill::ALL
            .iter()
            .filter(|s| s.ability() == Ability::Wisdom)
            .count();
        assert_eq!(wisdom, 5);
        assert_eq!(Alignment::ALL.len(), 9);
    }
}
