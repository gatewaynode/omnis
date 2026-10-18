//! What the party knows of a map's hidden things (M8.5 7d, owner 2026-10-17): the objects it has
//! found and the lore it has been told once. The party's, not the map's: walking past a hidden
//! lever and spotting it changes what the party knows, never the world, so a step journals
//! nothing (PRD D28).

use alloc::collections::BTreeSet;
use alloc::string::String;
use omnis_core::MapId;
use serde::{Deserialize, Serialize};

/// The party's knowledge of hidden things, by map and id; every set left out is empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Knowledge {
    /// Hidden placements the party has found, by map and placement id.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub noticed: BTreeSet<(MapId, String)>,
    /// Placements whose `once` lore the party has been told.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub told: BTreeSet<(MapId, String)>,
    /// Cells' `once` lore the party has been told, by map and lore id.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub lore: BTreeSet<(MapId, String)>,
}

impl Knowledge {
    /// Whether the party has found the placement `id` on `map`.
    #[must_use]
    pub fn has_noticed(&self, map: MapId, id: &str) -> bool {
        self.noticed.contains(&(map, id.into()))
    }

    /// The party finds the placement `id` on `map`.
    pub fn notice(&mut self, map: MapId, id: &str) {
        self.noticed.insert((map, id.into()));
    }

    /// Whether the party has been told the placement's `once` lore.
    #[must_use]
    pub fn was_told(&self, map: MapId, id: &str) -> bool {
        self.told.contains(&(map, id.into()))
    }

    /// Whether the party has been told the cell lore `id` on `map`.
    #[must_use]
    pub fn knows_lore(&self, map: MapId, id: &str) -> bool {
        self.lore.contains(&(map, id.into()))
    }

    /// Whether the party knows nothing hidden.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.noticed.is_empty() && self.told.is_empty() && self.lore.is_empty()
    }
}
