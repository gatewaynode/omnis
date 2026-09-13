//! `data/conditions`: the SRD conditions by id, with localized names and descriptions and the
//! mechanical flags combat reads. Every flag defaults to off, so a file that names and
//! describes a condition is complete on its own.

use serde::{Deserialize, Serialize};

/// One condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    /// Schema version.
    pub schema: u32,
    /// `pack:condition:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// Text key of the description.
    pub description: String,
    /// The bearer cannot act; skipped in initiative.
    #[serde(default)]
    pub incapacitated: bool,
    /// Attack rolls against the bearer have advantage.
    #[serde(default)]
    pub attacks_against_advantage: bool,
    /// The bearer's own attack rolls have disadvantage.
    #[serde(default)]
    pub own_attacks_disadvantage: bool,
    /// Strength and Dexterity saving throws fail without a roll.
    #[serde(default)]
    pub auto_fail_str_dex_saves: bool,
    /// A melee hit on the bearer is a critical hit.
    #[serde(default)]
    pub melee_hits_crit: bool,
    /// Every damage type is resisted.
    #[serde(default)]
    pub resist_all: bool,
}
