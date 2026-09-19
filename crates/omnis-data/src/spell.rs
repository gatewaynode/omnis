//! `data/spells`: spells with a point cost, a component list, and a typed effect (PRD §8.2,
//! D11, D12). The effect is the shape of what a spell does; the arithmetic lives in rule slots.

use crate::error::DataError;
use crate::limits::MAX_VISIBILITY_DEPTH;
use crate::terms::{Ability, DamageType, School};
use omnis_core::Dice;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// How far a spell reaches in the two-row, stacked fight (PRD §8.3, positioning).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Reach {
    /// One individual (the lead of a stack) or one member.
    #[default]
    One,
    /// Every individual of one stack.
    Stack,
    /// Every living stack (no base spell uses it yet).
    AllStacks,
}

/// Which rolls a buff adds its die to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BuffOn {
    /// Attack rolls.
    AttackRolls,
    /// Saving throws (not death saves).
    SavingThrows,
    /// Ability checks (skills included).
    AbilityChecks,
}

/// A utility effect on the world rather than a creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Utility {
    /// Toggle the first door within `range_tiles` straight ahead (mage hand).
    OpenDoor {
        /// Tiles ahead the hand reaches.
        range_tiles: u8,
    },
}

/// What a spell does when it lands. The shape is data; the numbers are rule slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellEffect {
    /// A spell attack roll against armor class; damage on a hit.
    Attack {
        /// Damage dice.
        dice: Dice,
        /// Damage type.
        damage_type: DamageType,
    },
    /// Damage with no roll to hit.
    AutoHit {
        /// Damage dice.
        dice: Dice,
        /// Damage type.
        damage_type: DamageType,
    },
    /// Each target saves; full damage on a failure, half or none on a success.
    Save {
        /// The saving throw.
        ability: Ability,
        /// Damage dice.
        dice: Dice,
        /// Damage type.
        damage_type: DamageType,
        /// Half damage on a success instead of none.
        half_on_save: bool,
    },
    /// Hit points regained, plus the casting modifier when `add_mod`. Members only.
    Heal {
        /// Healing dice.
        dice: Dice,
        /// Add the caster's casting ability modifier.
        add_mod: bool,
    },
    /// A die added to the listed rolls of up to `targets` members for `minutes`.
    Buff {
        /// The bonus die.
        bonus: Dice,
        /// Which rolls it touches.
        on: Vec<BuffOn>,
        /// How many members, fanning out from the target in marching order.
        targets: u8,
        /// Spent by the first roll it touches.
        consumed: bool,
        /// Party-clock minutes it lasts (a fight round is one minute).
        minutes: u32,
    },
    /// Cast by the sim as a reaction when a hit would land: armor class bonus until the
    /// caster's next turn.
    Reaction {
        /// Added to armor class.
        armor_bonus: i64,
    },
    /// A light source: the party's visibility depth is at least `depth` for `minutes`.
    Light {
        /// Tiles seen at least.
        depth: u8,
        /// Party-clock minutes it lasts.
        minutes: u32,
    },
    /// An effect on the world.
    Utility(Utility),
}

impl SpellEffect {
    /// Whether the effect is aimed at party members rather than monsters.
    #[must_use]
    pub const fn targets_members(&self) -> bool {
        matches!(self, SpellEffect::Heal { .. } | SpellEffect::Buff { .. })
    }

    /// Whether the effect can be cast outside a fight.
    #[must_use]
    pub const fn explore_castable(&self) -> bool {
        matches!(
            self,
            SpellEffect::Heal { .. }
                | SpellEffect::Buff { .. }
                | SpellEffect::Light { .. }
                | SpellEffect::Utility(_)
        )
    }

    /// The damage or healing dice, when the effect rolls any.
    #[must_use]
    pub const fn dice(&self) -> Option<Dice> {
        match self {
            SpellEffect::Attack { dice, .. }
            | SpellEffect::AutoHit { dice, .. }
            | SpellEffect::Save { dice, .. }
            | SpellEffect::Heal { dice, .. } => Some(*dice),
            SpellEffect::Buff { bonus, .. } => Some(*bonus),
            SpellEffect::Reaction { .. } | SpellEffect::Light { .. } | SpellEffect::Utility(_) => {
                None
            }
        }
    }

    /// Whether the effect only makes sense against one target.
    const fn needs_one_target(&self) -> bool {
        !matches!(self, SpellEffect::AutoHit { .. } | SpellEffect::Save { .. })
    }

    fn validate(&self, file: &Path, reach: Reach, errors: &mut Vec<DataError>) {
        if let Some(dice) = self.dice()
            && (dice.count == 0 || dice.sides == 0)
        {
            errors.push(DataError::new(file, "spell effect dice need dice"));
        }
        if self.needs_one_target() && reach != Reach::One {
            errors.push(DataError::new(
                file,
                "attack, heal, buff, reaction, light and utility spells reach one target",
            ));
        }
        match self {
            SpellEffect::Buff {
                on,
                targets,
                minutes,
                ..
            } => {
                if on.is_empty() {
                    errors.push(DataError::new(
                        file,
                        "a buff must name the rolls it touches",
                    ));
                }
                if *targets == 0 || *minutes == 0 {
                    errors.push(DataError::new(
                        file,
                        "a buff needs at least one target and one minute",
                    ));
                }
            }
            SpellEffect::Light { depth, minutes } => {
                if *depth == 0 || *depth > MAX_VISIBILITY_DEPTH || *minutes == 0 {
                    errors.push(DataError::new(
                        file,
                        format!(
                            "light depth must be 1..={MAX_VISIBILITY_DEPTH} for at least one minute"
                        ),
                    ));
                }
            }
            SpellEffect::Utility(Utility::OpenDoor { range_tiles }) if *range_tiles == 0 => {
                errors.push(DataError::new(file, "open door range must be at least 1"));
            }
            _ => {}
        }
    }
}

/// One spell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spell {
    /// Schema version.
    pub schema: u32,
    /// `pack:spell:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// Spell level; 0 is a cantrip.
    pub level: u8,
    /// School of magic.
    pub school: School,
    /// Class ids whose lists carry it.
    pub classes: Vec<String>,
    /// Requires concentration.
    #[serde(default)]
    pub concentration: bool,
    /// Can be cast as a ritual.
    #[serde(default)]
    pub ritual: bool,
    /// Point cost; the spell level when absent, and cantrips are free.
    #[serde(default)]
    pub points: Option<u32>,
    /// Consumed components as `(item id, count)`, all required.
    #[serde(default)]
    pub components: Vec<(String, u16)>,
    /// Text key of the description.
    pub description: String,
    /// What the spell does; `None` is loadable but not castable yet.
    #[serde(default)]
    pub effect: Option<SpellEffect>,
    /// How far it reaches in a fight.
    #[serde(default)]
    pub reach: Reach,
}

impl Spell {
    /// Points to cast: the declared cost, else the level.
    #[must_use]
    pub fn point_cost(&self) -> u32 {
        self.points.unwrap_or(u32::from(self.level))
    }

    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.level > 9 {
            errors.push(DataError::new(file, "level must be 0..=9"));
        }
        if self.classes.is_empty() {
            errors.push(DataError::new(
                file,
                "a spell must be on at least one class list",
            ));
        }
        if self.components.iter().any(|(_, n)| *n == 0) {
            errors.push(DataError::new(file, "component counts must be at least 1"));
        }
        if let Some(effect) = &self.effect {
            effect.validate(file, self.reach, errors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ron_io::{from_str, to_string};

    fn spell(effect: Option<SpellEffect>, reach: Reach) -> Spell {
        Spell {
            schema: 1,
            id: "t:spell:x".into(),
            name: "t:text:spell.x.name".into(),
            level: 1,
            school: School::Evocation,
            classes: vec!["t:class:c".into()],
            concentration: false,
            ritual: false,
            points: None,
            components: Vec::new(),
            description: "t:text:spell.x.description".into(),
            effect,
            reach,
        }
    }

    fn problems(spell: &Spell) -> Vec<String> {
        let mut errors = Vec::new();
        spell.validate(Path::new("x.ron"), &mut errors);
        errors.iter().map(|e| e.message.clone()).collect()
    }

    #[test]
    fn every_effect_survives_text_and_back() {
        let d = Dice::new(1, 8);
        let effects = [
            SpellEffect::Attack {
                dice: d,
                damage_type: DamageType::Fire,
            },
            SpellEffect::AutoHit {
                dice: d,
                damage_type: DamageType::Force,
            },
            SpellEffect::Save {
                ability: Ability::Dexterity,
                dice: d,
                damage_type: DamageType::Radiant,
                half_on_save: true,
            },
            SpellEffect::Heal {
                dice: d,
                add_mod: true,
            },
            SpellEffect::Buff {
                bonus: Dice::new(1, 4),
                on: vec![BuffOn::AttackRolls, BuffOn::SavingThrows],
                targets: 3,
                consumed: false,
                minutes: 10,
            },
            SpellEffect::Reaction { armor_bonus: 5 },
            SpellEffect::Light {
                depth: 8,
                minutes: 60,
            },
            SpellEffect::Utility(Utility::OpenDoor { range_tiles: 6 }),
        ];
        for effect in effects {
            let reach = if effect.needs_one_target() {
                Reach::One
            } else {
                Reach::Stack
            };
            let s = spell(Some(effect), reach);
            let back: Spell = from_str(&to_string(&s).unwrap(), Path::new("memory")).unwrap();
            assert_eq!(back, s);
            assert!(problems(&s).is_empty(), "{:?}", problems(&s));
        }
        let plain = spell(None, Reach::One);
        let back: Spell = from_str(&to_string(&plain).unwrap(), Path::new("memory")).unwrap();
        assert_eq!(back.effect, None);
    }

    #[test]
    fn effects_are_checked() {
        let zero = SpellEffect::Attack {
            dice: Dice::new(0, 8),
            damage_type: DamageType::Fire,
        };
        assert_eq!(
            problems(&spell(Some(zero), Reach::Stack)),
            [
                "spell effect dice need dice",
                "attack, heal, buff, reaction, light and utility spells reach one target",
            ]
        );
        let buff = SpellEffect::Buff {
            bonus: Dice::new(1, 4),
            on: Vec::new(),
            targets: 0,
            consumed: false,
            minutes: 10,
        };
        assert_eq!(
            problems(&spell(Some(buff), Reach::One)),
            [
                "a buff must name the rolls it touches",
                "a buff needs at least one target and one minute",
            ]
        );
        let light = SpellEffect::Light {
            depth: 40,
            minutes: 60,
        };
        assert_eq!(problems(&spell(Some(light), Reach::One)).len(), 1);
        let hand = SpellEffect::Utility(Utility::OpenDoor { range_tiles: 0 });
        assert_eq!(
            problems(&spell(Some(hand), Reach::One)),
            ["open door range must be at least 1"]
        );
        let area = SpellEffect::Save {
            ability: Ability::Dexterity,
            dice: Dice::new(3, 6),
            damage_type: DamageType::Fire,
            half_on_save: true,
        };
        assert!(problems(&spell(Some(area), Reach::Stack)).is_empty());
    }
}
