//! `data/items`: weapons, armor, gear, and spell components.

use crate::error::DataError;
use crate::terms::{ArmorKind, DamageType, WeaponKind};
use omnis_core::Dice;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
}

impl Item {
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
    }
}
