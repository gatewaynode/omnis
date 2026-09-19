//! `data/items`: weapons, armor, gear, and spell components. `kind` says what an item is
//! mechanically; `use_effect` says what Use does; `consumable` says what happens to the count.

use crate::error::DataError;
use crate::sense::SenseSource;
use crate::terms::{ArmorKind, DamageType, WeaponKind};
use omnis_core::Dice;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Where a worn or wielded item goes. Head, ring, and amulet arrive with magic items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    /// A melee weapon.
    MainHand,
    /// A shield.
    OffHand,
    /// A ranged weapon, slung and drawn to shoot.
    Ranged,
    /// Armor.
    Body,
}

impl EquipSlot {
    /// Every slot in display order.
    pub const ALL: [EquipSlot; 4] = [
        EquipSlot::MainHand,
        EquipSlot::OffHand,
        EquipSlot::Ranged,
        EquipSlot::Body,
    ];
}

/// What using an item does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseEffect {
    /// Restore hit points to one member.
    Heal {
        /// Healing dice.
        dice: Dice,
    },
    /// Reveal tiles into the automap.
    Sense(SenseSource),
}

/// What an item is mechanically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    /// A weapon.
    Weapon {
        /// Proficiency category.
        kind: WeaponKind,
        /// Damage on a hit.
        damage: Dice,
        /// Damage type.
        damage_type: DamageType,
        /// Reaches the back row and is used from it.
        #[serde(default)]
        ranged: bool,
        /// Needs both hands, so no shield.
        #[serde(default)]
        two_handed: bool,
    },
    /// Armor or a shield.
    Armor {
        /// Proficiency category.
        kind: ArmorKind,
        /// Base armor class, or the bonus for a shield.
        base_ac: u8,
        /// Cap on the Dexterity modifier added; `None` is uncapped.
        #[serde(default)]
        dex_cap: Option<u8>,
        /// Strength needed to wear it without penalty.
        #[serde(default)]
        strength: u8,
        /// Stealth checks have disadvantage.
        #[serde(default)]
        stealth_disadvantage: bool,
    },
    /// Adventuring gear with no combat role.
    Gear,
    /// A consumable spell component (gems and reagents).
    Component,
    /// A spellcasting focus.
    Focus,
}

/// One item type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Schema version.
    pub schema: u32,
    /// `pack:item:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// What it is.
    pub kind: ItemKind,
    /// Price in copper pieces.
    #[serde(default)]
    pub cost_cp: u32,
    /// Weight in tenths of a pound.
    #[serde(default)]
    pub weight_tenths: u16,
    /// What Use does; `None` for items with no use.
    #[serde(default)]
    pub use_effect: Option<UseEffect>,
    /// One count is spent per use.
    #[serde(default)]
    pub consumable: bool,
    /// Text key of a description, when the pack writes one.
    #[serde(default)]
    pub description: Option<String>,
}

impl Item {
    /// The slot the item equips into, from its kind; `None` for the rest.
    #[must_use]
    pub const fn slot(&self) -> Option<EquipSlot> {
        match &self.kind {
            ItemKind::Weapon { ranged: true, .. } => Some(EquipSlot::Ranged),
            ItemKind::Weapon { .. } => Some(EquipSlot::MainHand),
            ItemKind::Armor {
                kind: ArmorKind::Shield,
                ..
            } => Some(EquipSlot::OffHand),
            ItemKind::Armor { .. } => Some(EquipSlot::Body),
            ItemKind::Gear | ItemKind::Component | ItemKind::Focus => None,
        }
    }

    /// Whether wielding it leaves no hand for a shield. A ranged weapon is slung when the
    /// shield is up, so it is exempt.
    #[must_use]
    pub const fn two_handed(&self) -> bool {
        matches!(
            &self.kind,
            ItemKind::Weapon {
                two_handed: true,
                ranged: false,
                ..
            }
        )
    }

    /// The sense source, when using the item looks at the world.
    #[must_use]
    pub const fn sense(&self) -> Option<&SenseSource> {
        match &self.use_effect {
            Some(UseEffect::Sense(source)) => Some(source),
            _ => None,
        }
    }

    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        match &self.kind {
            ItemKind::Weapon { damage, .. } if damage.count == 0 || damage.sides == 0 => {
                errors.push(DataError::new(file, "weapon damage needs dice"));
            }
            ItemKind::Armor { base_ac, .. } if *base_ac == 0 || *base_ac > 30 => {
                errors.push(DataError::new(file, "armor base_ac must be 1..=30"));
            }
            _ => {}
        }
        match &self.use_effect {
            Some(UseEffect::Heal { dice }) if dice.count == 0 || dice.sides == 0 => {
                errors.push(DataError::new(file, "heal dice need dice"));
            }
            Some(UseEffect::Sense(source)) => source.validate(file, errors),
            None if self.consumable => {
                errors.push(DataError::new(file, "a consumable item needs a use effect"));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ron_io::{from_str, to_string};

    fn item(kind: ItemKind) -> Item {
        Item {
            schema: 1,
            id: "t:item:x".into(),
            name: "t:text:item.x.name".into(),
            kind,
            cost_cp: 0,
            weight_tenths: 0,
            use_effect: None,
            consumable: false,
            description: None,
        }
    }

    fn weapon(ranged: bool, two_handed: bool) -> ItemKind {
        ItemKind::Weapon {
            kind: WeaponKind::Simple,
            damage: Dice::new(1, 6),
            damage_type: DamageType::Piercing,
            ranged,
            two_handed,
        }
    }

    fn armor(kind: ArmorKind) -> ItemKind {
        ItemKind::Armor {
            kind,
            base_ac: 2,
            dex_cap: None,
            strength: 0,
            stealth_disadvantage: false,
        }
    }

    #[test]
    fn slots_follow_the_kind_and_only_melee_is_two_handed() {
        assert_eq!(item(weapon(false, false)).slot(), Some(EquipSlot::MainHand));
        assert_eq!(item(weapon(true, true)).slot(), Some(EquipSlot::Ranged));
        assert_eq!(
            item(armor(ArmorKind::Shield)).slot(),
            Some(EquipSlot::OffHand)
        );
        assert_eq!(item(armor(ArmorKind::Heavy)).slot(), Some(EquipSlot::Body));
        assert_eq!(item(ItemKind::Gear).slot(), None);
        assert_eq!(item(ItemKind::Focus).slot(), None);
        assert!(item(weapon(false, true)).two_handed());
        assert!(
            !item(weapon(true, true)).two_handed(),
            "a slung bow leaves a hand free"
        );
        assert!(!item(weapon(false, false)).two_handed());
    }

    #[test]
    fn a_potion_and_a_spyglass_read_back_and_are_checked() {
        let potion: Item = from_str(
            r#"(schema: 1, id: "t:item:potion", name: "t:text:item.potion.name", kind: Gear,
                cost_cp: 5000, weight_tenths: 5,
                use_effect: Some(Heal(dice: (count: 2, sides: 4, modifier: 2))), consumable: true)"#,
            Path::new("memory"),
        )
        .unwrap();
        assert!(potion.consumable && potion.sense().is_none());
        let glass: Item = from_str(
            r#"(schema: 1, id: "t:item:glass", name: "t:text:item.glass.name", kind: Gear,
                use_effect: Some(Sense((geometry: Ray(range: 16), fidelity: Structure, check: Some(Perception)))))"#,
            Path::new("memory"),
        )
        .unwrap();
        assert_eq!(glass.sense().map(|s| s.minutes), Some(1));
        for it in [&potion, &glass] {
            let back: Item = from_str(&to_string(it).unwrap(), Path::new("memory")).unwrap();
            assert_eq!(&back, it);
            let mut errors = Vec::new();
            it.validate(Path::new("x.ron"), &mut errors);
            assert!(errors.is_empty());
        }
        let mut flask = item(ItemKind::Gear);
        flask.consumable = true;
        let mut errors = Vec::new();
        flask.validate(Path::new("x.ron"), &mut errors);
        assert_eq!(errors[0].message, "a consumable item needs a use effect");
        flask.use_effect = Some(UseEffect::Heal {
            dice: Dice::new(0, 4),
        });
        errors.clear();
        flask.validate(Path::new("x.ron"), &mut errors);
        assert_eq!(errors[0].message, "heal dice need dice");
    }
}
