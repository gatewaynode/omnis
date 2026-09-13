//! `data/spells`: spells with a point cost and a component list (PRD §8.2, D11, D12).

use crate::error::DataError;
use crate::terms::School;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    }
}
