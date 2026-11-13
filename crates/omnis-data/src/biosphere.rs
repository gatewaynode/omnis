//! `data/biospheres`: a palette for the world's lines (presentation-PRD X18, X23; M8.5 step 5,
//! owner 2026-11-13). Colour means a role, the same in every place; a biosphere says which
//! colour each role has there, so a cold cavern and a burning desert may share no hue. A region
//! names its biosphere and a map may name its own ([`crate::Data::biosphere_of`]).
//!
//! Presentation data: no rule reads it. The engine checks it and hands it to the client.

use crate::error::DataError;
use crate::limits::string_fits;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// What a colour means, wherever it is seen (presentation-PRD 5.2; the set answered by the
/// owner 2026-11-13). New roles are added at the end: a role's place is its number in a palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ColourRole {
    /// What stands and bounds: walls, solid terrain, cliffs.
    Structure,
    /// What is walked on or under: the floor and ceiling grids, ramps.
    Ground,
    /// A way through: doors, portals, stairs, ladders.
    Passage,
    /// What can be used or taken: objects, piles, activities.
    Thing,
    /// The party's own.
    Party,
    /// What may fight the party.
    Hostile,
    /// Magic.
    Magic,
    /// What harms: holes, traps, hazards.
    Danger,
    /// The interface.
    Interface,
}

impl ColourRole {
    /// Every role, in the order of their numbers.
    pub const ALL: [ColourRole; 9] = [
        ColourRole::Structure,
        ColourRole::Ground,
        ColourRole::Passage,
        ColourRole::Thing,
        ColourRole::Party,
        ColourRole::Hostile,
        ColourRole::Magic,
        ColourRole::Danger,
        ColourRole::Interface,
    ];

    /// The role's number: its place in a palette.
    #[must_use]
    pub fn index(self) -> usize {
        self as usize
    }
}

/// The glow that does not bloom, per mille.
pub const PLAIN_GLOW: u16 = 1000;

/// The most a role may glow, per mille.
pub const MAX_GLOW: u16 = 4000;

/// A role's colour in one biosphere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ink {
    /// The colour, as sRGB.
    pub rgb: (u8, u8, u8),
    /// How bright it is drawn, per mille: 1000 is the colour as written, more blooms.
    #[serde(default = "plain_glow")]
    pub glow: u16,
}

fn plain_glow() -> u16 {
    PLAIN_GLOW
}

/// One biosphere, one file under `data/biospheres`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiosphereDef {
    /// Schema version.
    pub schema: u32,
    /// `pack:biosphere:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// The colour of each role here. A role left out takes the client's own default, so a role
    /// added later never breaks an older pack.
    #[serde(default)]
    pub palette: BTreeMap<ColourRole, Ink>,
}

impl BiosphereDef {
    /// Self-contained checks; the name's text key is checked at resolution.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if !string_fits(&self.name) || self.name.is_empty() {
            errors.push(DataError::new(file, "name is empty or over the size limit"));
        }
        for (role, ink) in &self.palette {
            if ink.glow == 0 || ink.glow > MAX_GLOW {
                errors.push(DataError::new(
                    file,
                    format!(
                        "{role:?} glows {} per mille; a glow is 1..={MAX_GLOW}",
                        ink.glow
                    ),
                ));
            }
        }
    }
}
