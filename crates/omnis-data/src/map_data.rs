//! A map as the simulation reads it: its parsed levels and cells, and the references the
//! loader resolved (portals, encounters, sites, rest events), with the lookups by tile.

use crate::encounter::{ResolvedEncounter, ResolvedRandom};
use crate::levels::{self, LevelData};
use crate::map::{Cell, MapDef, Terrain};
use crate::placement::{Pile, ResolvedPlacement};
use crate::rest_event::ResolvedRestEvent;
use crate::service::ResolvedSite;
use omnis_core::{Facing, Level, MapId, RegionId, ServiceId, TextKey, TilesetId};
use std::collections::BTreeMap;

/// A portal with its destination resolved to an interned map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPortal {
    /// Trigger level.
    pub level: Level,
    /// Trigger column.
    pub x: u16,
    /// Trigger row.
    pub y: u16,
    /// Destination map.
    pub to_map: MapId,
    /// Destination level.
    pub to_level: Level,
    /// Destination column.
    pub to_x: u16,
    /// Destination row.
    pub to_y: u16,
    /// Facing on arrival.
    pub to_facing: Facing,
}

/// A map ready for the simulation: parsed cells and resolved references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapData {
    /// The file as written.
    pub def: MapDef,
    /// Every level, lowest first, the map's own (level 0) among them; each row-major,
    /// `width × height`, `None` where there is no ground.
    pub levels: Vec<LevelData>,
    /// The interned tileset.
    pub tileset: TilesetId,
    /// The interned display-name key.
    pub name: TextKey,
    /// Portals with interned destinations.
    pub portals: Vec<ResolvedPortal>,
    /// Fixed encounters with interned monsters, in file order.
    pub encounters: Vec<ResolvedEncounter>,
    /// The random table with interned monsters, if the map has one.
    pub random: Option<ResolvedRandom>,
    /// Services placed on tiles, in file order.
    pub sites: Vec<ResolvedSite>,
    /// The region that owns the map.
    pub region: RegionId,
    /// What may happen while resting here, in file order.
    pub rest_events: Vec<ResolvedRestEvent>,
    /// Objects placed in the map, in file order (M8.5 step 2).
    pub objects: Vec<ResolvedPlacement>,
    /// For each cell any of an object's looks reaches into, the objects' indices.
    pub object_cells: BTreeMap<(Level, u16, u16), Vec<u16>>,
    /// Piles placed on the ground, by cell.
    pub ground: BTreeMap<(Level, u16, u16), Pile>,
}

impl MapData {
    /// The cell at a coordinate of a level, if inside the map, on a level it has, with ground.
    #[must_use]
    pub fn cell(&self, level: Level, x: u16, y: u16) -> Option<&Cell> {
        if !self.inside(x, y) {
            return None;
        }
        self.level(level)?
            .cells
            .get(usize::from(y) * usize::from(self.def.width) + usize::from(x))?
            .as_ref()
    }

    /// Whether a coordinate is inside the map.
    #[must_use]
    pub const fn inside(&self, x: u16, y: u16) -> bool {
        x < self.def.width && y < self.def.height
    }

    /// A level of the map, if it has it.
    #[must_use]
    pub fn level(&self, level: Level) -> Option<&LevelData> {
        self.levels.iter().find(|l| l.level == level)
    }

    /// The way the stair at `(x, y)` of `level` rises, if there is one (M8.5 step 1c).
    #[must_use]
    pub fn rise(&self, level: Level, x: u16, y: u16) -> Option<Facing> {
        self.level(level)?.rises.get(&(x, y)).copied()
    }

    /// The next level up (`up`) or down from `level`, if the map has one.
    #[must_use]
    pub fn beyond(&self, level: Level, up: bool) -> Option<Level> {
        levels::beyond(&self.levels, level, up).map(|l| l.level)
    }

    /// The terrain of a cell.
    #[must_use]
    pub fn terrain(&self, cell: &Cell) -> &Terrain {
        &self.def.terrains[usize::from(cell.terrain)]
    }

    /// The portal on a tile, if any.
    #[must_use]
    pub fn portal_at(&self, level: Level, x: u16, y: u16) -> Option<&ResolvedPortal> {
        self.portals
            .iter()
            .find(|p| p.level == level && p.x == x && p.y == y)
    }

    /// The fixed encounter on a tile with its index (the key a cleared encounter is remembered
    /// by), if any.
    #[must_use]
    pub fn encounter_at(&self, level: Level, x: u16, y: u16) -> Option<(u16, &ResolvedEncounter)> {
        self.encounters
            .iter()
            .enumerate()
            .find(|(_, e)| e.level == level && e.x == x && e.y == y)
            .and_then(|(i, e)| u16::try_from(i).ok().map(|i| (i, e)))
    }

    /// The `Object` surface marking the portal on a tile, if any.
    #[must_use]
    pub fn marker_at(&self, level: Level, x: u16, y: u16) -> Option<&str> {
        self.def
            .portals
            .iter()
            .find(|p| p.level == level && p.x == x && p.y == y)
            .and_then(|p| p.marker.as_deref())
    }

    /// The objects whose footprints reach into a cell, with their indices, in file order.
    pub fn objects_at(
        &self,
        level: Level,
        x: u16,
        y: u16,
    ) -> impl Iterator<Item = (u16, &ResolvedPlacement)> {
        self.object_cells
            .get(&(level, x, y))
            .into_iter()
            .flatten()
            .filter_map(|&i| self.objects.get(usize::from(i)).map(|p| (i, p)))
    }

    /// The service placed on a tile, if any.
    #[must_use]
    pub fn site_at(&self, level: Level, x: u16, y: u16) -> Option<ServiceId> {
        self.sites
            .iter()
            .find(|s| s.level == level && s.x == x && s.y == y)
            .map(|s| s.service)
    }
}
