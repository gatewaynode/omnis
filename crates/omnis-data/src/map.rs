//! `data/maps/*.ron`: one map is a rectangular grid of tiles with walls and doors on edges.
//!
//! The grid is written as text so people and the editor produce the same thing. A map of
//! `width × height` tiles has a `layout` of `2·height + 1` rows, each `2·width + 1`
//! characters. Odd rows and columns are tiles; even ones are edges and corners:
//!
//! ```text
//! +-+-+-+      row 0: corners and north edges of tile row 0
//! |.|. .|      row 1: west edge, tile, edge, tile, edge, tile, east edge
//! + +=+-+      row 2: south edges of tile row 0 (also north edges of tile row 1)
//! |. .|.|
//! +-+-+-+
//! ```
//!
//! Tile characters are the glyphs of the map's `terrains`. Horizontal edges are `-` (wall),
//! `=` (door), or space (open). Vertical edges are `|` (wall), `:` (door), or space (open).
//! Corners are `+` and are ignored. Because an edge character sits between two tiles, both
//! tiles agree on it by construction; the outer boundary is always treated as a wall.
//! An inner edge may also be `/`, a ramp or stair: open, and walkable across any height
//! difference up to [`MAX_RAMP`].
//!
//! Height is per tile, in eighths of a cell (a wall is 8 tall), written in `heights` as
//! `height` rows of `width` whitespace-separated integers; no `heights` means every tile is at
//! 0. A step is refused as a cliff when the two tiles differ by more than the entered
//! terrain's `climb`, up or down, unless the edge is a ramp. Cliffs are worked out at load, once,
//! so the simulation and the client read the same answer.
//!
//! A map may have levels above and below its own, each a whole grid with a base height, joined
//! by stairs, ladders and holes (M8.5 step 1b): see [`crate::levels`]. A tile written `_` has
//! no ground on its level.

use crate::encounter::{self, FixedEncounter, RandomEncounters};
use crate::error::DataError;
use crate::levels::{LevelDef, NO_GROUND};
use crate::limits::{MAX_COLLECTION, MAX_MAP_SIDE, MAX_VISIBILITY_DEPTH, string_fits};
use crate::lore::{self, CellLore};
use crate::placement::{self, GroundDef, Placement};
use crate::rest_event::RestEventDef;
use crate::service::{self, Site};
use crate::stairs::StairDef;
use omnis_core::{Edges, Facing, Level};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[cfg(doc)]
use crate::limits::MAX_RAMP;

/// What kind of place a map is; rules and rendering differ, the format does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MapKind {
    /// Open country under the sky.
    Outdoor,
    /// A settlement.
    Town,
    /// An interior with a ceiling.
    Dungeon,
    /// Anything scripted.
    Special,
}

/// How a terrain joins its level to the next (M8.5 step 1b). Each lands on the same column and
/// row of the next level that way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Join {
    /// A stair up: stepping onto it climbs to the level above.
    Up,
    /// A stair down: stepping onto it goes down to the level below.
    Down,
    /// A ladder or a shaft: climbed up or down, one level at a time, by a command.
    Ladder,
    /// A hole: stepping onto it drops the party to the level below. One way.
    Hole,
}

/// A terrain type the map's tiles can be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Terrain {
    /// The character that stands for this terrain in `layout`.
    pub glyph: char,
    /// A name for tools and text keys.
    pub name: String,
    /// Tileset surface for the ground.
    pub floor: String,
    /// Tileset surface for the ceiling, or none for sky.
    #[serde(default)]
    pub ceiling: Option<String>,
    /// Tileset `Block` surface drawn over the tile, for solid terrain such as pillars.
    #[serde(default)]
    pub block: Option<String>,
    /// Whether the party may enter the tile.
    pub passable: bool,
    /// Whether the tile blocks line of sight (rock, dense trees). Water does not.
    #[serde(default)]
    pub opaque: bool,
    /// Colour for the automap and the horizon band, as RGB.
    #[serde(default = "grey")]
    pub color: (u8, u8, u8),
    /// How far the party sees while standing on this terrain.
    pub visibility_depth: u8,
    /// Minutes one step onto this terrain costs the party's clock.
    pub step_minutes: u32,
    /// The largest height difference, up or down, a step onto this terrain may cross, in
    /// eighths of a cell. More is a cliff.
    #[serde(default = "quarter_wall")]
    pub climb: u16,
    /// How a tile of this terrain joins its level to the next, if it does.
    #[serde(default)]
    pub join: Option<Join>,
}

fn grey() -> (u8, u8, u8) {
    (128, 128, 128)
}

fn quarter_wall() -> u16 {
    2
}

/// A tile trigger that moves the party to another map or tile when stepped on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portal {
    /// The trigger tile's level.
    #[serde(default)]
    pub level: Level,
    /// Trigger tile column.
    pub x: u16,
    /// Trigger tile row.
    pub y: u16,
    /// Destination map id.
    pub to_map: String,
    /// Destination level.
    #[serde(default)]
    pub to_level: Level,
    /// Destination column.
    pub to_x: u16,
    /// Destination row.
    pub to_y: u16,
    /// Facing on arrival.
    pub to_facing: Facing,
    /// Tileset `Object` surface standing on the trigger tile, so the way on can be seen.
    /// Required; `None` is only so a file without one gets a named error.
    #[serde(default)]
    pub marker: Option<String>,
}

/// The three surfaces a wall needs, one per viewing angle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WallSurfaces {
    /// Facing the party.
    pub front: String,
    /// On the left, at an angle.
    pub left: String,
    /// On the right, at an angle.
    pub right: String,
}

/// The map file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapDef {
    /// Schema version of this file.
    pub schema: u32,
    /// `pack:map:name`.
    pub id: String,
    /// Text key for the display name.
    pub name: String,
    /// The kind of place.
    pub kind: MapKind,
    /// Tileset id.
    pub tileset: String,
    /// Tiles across.
    pub width: u16,
    /// Tiles down.
    pub height: u16,
    /// Where a new party appears: column, row, facing.
    pub start: (u16, u16, Facing),
    /// Tileset surfaces for walls on this map.
    pub wall: WallSurfaces,
    /// Tileset surface for closed doors on this map.
    pub door: String,
    /// Tileset surface for open doors, or none to draw an open doorway as empty.
    #[serde(default)]
    pub door_open: Option<String>,
    /// The terrains, each with a distinct glyph.
    pub terrains: Vec<Terrain>,
    /// The grid as text; see the module documentation.
    pub layout: Vec<String>,
    /// Tile heights in eighths of a cell, one row of numbers per tile row; empty is all 0.
    #[serde(default)]
    pub heights: Vec<String>,
    /// Levels above and below this one (M8.5 step 1b); none is a map of one level.
    #[serde(default)]
    pub levels: Vec<LevelDef>,
    /// Which way each stair climbs (M8.5 step 1c); every stair up or down is listed.
    #[serde(default)]
    pub stairs: Vec<StairDef>,
    /// Tile triggers.
    #[serde(default)]
    pub portals: Vec<Portal>,
    /// Groups placed on tiles.
    #[serde(default)]
    pub encounters: Vec<FixedEncounter>,
    /// The random encounter table, if the map has one.
    #[serde(default)]
    pub random: Option<RandomEncounters>,
    /// Services placed on tiles.
    #[serde(default)]
    pub sites: Vec<Site>,
    /// What may happen while the party rests here, by terrain.
    #[serde(default)]
    pub rest_events: Vec<RestEventDef>,
    /// Objects standing in cells (M8.5 step 2).
    #[serde(default)]
    pub objects: Vec<Placement>,
    /// Items lying on the ground, one pile a cell.
    #[serde(default)]
    pub ground: Vec<GroundDef>,
    /// Text a search of a cell turns up (owner, 2026-10-16).
    #[serde(default)]
    pub lore: Vec<CellLore>,
}

/// One tile after the layout is parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// Index into `MapDef::terrains`.
    pub terrain: u8,
    /// Edges with a wall (doors excluded).
    pub walls: Edges,
    /// Edges with a door.
    pub doors: Edges,
    /// Floor height in eighths of a cell.
    pub height: i16,
    /// Edges with a ramp or stair.
    pub ramps: Edges,
    /// Edges a step may not cross because the height difference is more than the terrain
    /// beyond allows (derived; set whatever else is on the edge).
    pub cliffs: Edges,
}

impl MapDef {
    /// Checks that need no other file: sizes, glyphs, start tile, portal, encounter, and site
    /// shapes.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.width == 0
            || self.height == 0
            || self.width > MAX_MAP_SIDE
            || self.height > MAX_MAP_SIDE
        {
            errors.push(DataError::new(
                file,
                format!(
                    "size {}x{} must be within 1x1 and {MAX_MAP_SIDE}x{MAX_MAP_SIDE}",
                    self.width, self.height
                ),
            ));
        }
        if self.terrains.is_empty() || self.terrains.len() > 256 {
            errors.push(DataError::new(
                file,
                "a map needs between 1 and 256 terrains",
            ));
        }
        let mut glyphs = BTreeMap::new();
        for terrain in &self.terrains {
            if glyphs.insert(terrain.glyph, ()).is_some() {
                errors.push(DataError::new(
                    file,
                    format!("terrain glyph '{}' is used twice", terrain.glyph),
                ));
            }
            if matches!(terrain.glyph, '-' | '=' | '|' | ':' | '/' | '+' | ' ' | '*') {
                errors.push(DataError::new(
                    file,
                    format!(
                        "terrain glyph '{}' is reserved for edges and portals",
                        terrain.glyph
                    ),
                ));
            }
            if terrain.glyph == NO_GROUND {
                errors.push(DataError::new(
                    file,
                    format!("terrain glyph '{NO_GROUND}' is reserved for no ground"),
                ));
            }
            if terrain.visibility_depth == 0 || terrain.visibility_depth > MAX_VISIBILITY_DEPTH {
                errors.push(DataError::new(
                    file,
                    format!(
                        "terrain '{}' visibility_depth must be 1..={MAX_VISIBILITY_DEPTH}",
                        terrain.name
                    ),
                ));
            }
            if !string_fits(&terrain.name) || !string_fits(&terrain.floor) {
                errors.push(DataError::new(
                    file,
                    format!(
                        "terrain '{}' has a string over the size limit",
                        terrain.name
                    ),
                ));
            }
        }
        let (sx, sy, _) = self.start;
        if sx >= self.width || sy >= self.height {
            errors.push(DataError::new(
                file,
                format!("start ({sx}, {sy}) is outside the map"),
            ));
        }
        if self.portals.len() > MAX_COLLECTION {
            errors.push(DataError::new(file, "too many portals"));
        }
        for portal in &self.portals {
            if portal.x >= self.width || portal.y >= self.height {
                errors.push(DataError::new(
                    file,
                    format!("portal at ({}, {}) is outside the map", portal.x, portal.y),
                ));
            }
            if portal.marker.is_none() {
                errors.push(DataError::new(
                    file,
                    format!(
                        "portal at ({}, {}) has no marker; every portal must be visible",
                        portal.x, portal.y
                    ),
                ));
            }
        }
        let has =
            |level: Level| level.is_ground() || self.levels.iter().any(|l| Level(l.level) == level);
        let placed = self
            .portals
            .iter()
            .map(|p| ("portal", p.level, p.x, p.y))
            .chain(
                self.encounters
                    .iter()
                    .map(|e| ("encounter", e.level, e.x, e.y)),
            )
            .chain(self.sites.iter().map(|s| ("site", s.level, s.x, s.y)));
        for (what, level, x, y) in placed {
            if !has(level) {
                errors.push(DataError::new(
                    file,
                    format!(
                        "{what} at ({x}, {y}) is on level {level}, which the map does not have"
                    ),
                ));
            }
        }
        encounter::validate(
            (self.width, self.height),
            &self.encounters,
            self.random.as_ref(),
            file,
            errors,
        );
        service::validate_sites(self, file, errors);
        placement::validate(self, file, errors);
        lore::validate(self, file, errors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny(layout: &[&str]) -> MapDef {
        MapDef {
            schema: 1,
            id: "t:map:tiny".into(),
            name: "t:text:tiny".into(),
            kind: MapKind::Dungeon,
            tileset: "t:tileset:d".into(),
            width: 2,
            height: 2,
            start: (0, 0, Facing::East),
            wall: WallSurfaces {
                front: "wall".into(),
                left: "wall.left".into(),
                right: "wall.right".into(),
            },
            door: "door".into(),
            door_open: None,
            terrains: vec![Terrain {
                glyph: '.',
                name: "floor".into(),
                floor: "floor".into(),
                ceiling: None,
                block: None,
                passable: true,
                opaque: false,
                color: (128, 128, 128),
                visibility_depth: 4,
                step_minutes: 1,
                climb: 2,
                join: None,
            }],
            layout: layout.iter().map(|s| (*s).to_owned()).collect(),
            heights: vec![],
            levels: vec![],
            stairs: vec![],
            portals: vec![],
            encounters: vec![],
            random: None,
            sites: vec![],
            rest_events: vec![],
            objects: vec![],
            ground: vec![],
            lore: vec![],
        }
    }

    /// The map's own level's cells, every tile with ground.
    fn ground(map: &MapDef, errors: &mut Vec<DataError>) -> Option<Vec<Cell>> {
        let levels = crate::levels::parse(map, Path::new("m"), errors)?;
        let ground = levels.into_iter().find(|l| l.level == Level::GROUND)?;
        Some(
            ground
                .cells
                .into_iter()
                .map(|c| c.expect("ground"))
                .collect(),
        )
    }

    #[test]
    fn layout_edges_are_shared_between_neighbours() {
        let map = tiny(&["+-+-+", "|. .|", "+=+ +", "|.|.|", "+-+-+"]);
        let mut errors = vec![];
        let cells = ground(&map, &mut errors).expect("valid");
        assert!(errors.is_empty(), "{errors:?}");
        let n = Facing::North;
        let (e, s, w) = (Facing::East, Facing::South, Facing::West);
        assert_eq!(cells[0].walls, Edges::NONE.with(n).with(w));
        assert_eq!(cells[0].doors, Edges::NONE.with(s));
        assert_eq!(cells[1].walls, Edges::NONE.with(n).with(e));
        assert_eq!(cells[1].doors, Edges::NONE);
        assert_eq!(
            cells[2].doors,
            Edges::NONE.with(n),
            "door seen from the south tile too"
        );
        assert_eq!(cells[2].walls, Edges::NONE.with(w).with(s).with(e));
        assert_eq!(cells[3].walls, Edges::NONE.with(w).with(s).with(e));
    }

    #[test]
    fn layout_problems_are_all_reported() {
        let map = tiny(&["+-+-+", "|.#.|", "+ + +", "|. .|", "+-+-+"]);
        let mut errors = vec![];
        assert_eq!(ground(&map, &mut errors), None);
        let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        assert_eq!(
            messages,
            [
                "tile (0, 0) east edge has unknown character '#'",
                "tile (1, 0) west edge has unknown character '#'",
            ]
        );
        let open = tiny(&["+-+-+", "|. .|", "+ + +", " . .|", "+-+-+"]);
        let mut errors = vec![];
        assert_eq!(ground(&open, &mut errors), None);
        assert_eq!(
            errors[0].message,
            "tile (0, 1) west edge is on the map boundary and must be a wall"
        );
        let short = tiny(&["+-+-+", "|. .|", "+-+-+"]);
        let mut errors = vec![];
        assert_eq!(ground(&short, &mut errors), None);
        assert_eq!(
            errors[0].message,
            "layout has 3 rows; a 2-tile-high map needs 5"
        );
    }

    /// A 3x1 strip with the given terrains' climbs, layout middle row and heights.
    fn strip(climbs: [u16; 3], tiles: &str, heights: &str) -> MapDef {
        let mut map = tiny(&["+-+-+-+", tiles, "+-+-+-+"]);
        map.width = 3;
        map.height = 1;
        let floor = map.terrains[0].clone();
        map.terrains = ['a', 'b', 'c']
            .into_iter()
            .zip(climbs)
            .map(|(glyph, climb)| Terrain {
                glyph,
                climb,
                ..floor.clone()
            })
            .collect();
        map.heights = vec![heights.to_owned()];
        map
    }

    fn cells_of(map: &MapDef) -> Vec<Cell> {
        let mut errors = vec![];
        let cells = ground(map, &mut errors);
        assert!(errors.is_empty(), "{errors:?}");
        cells.expect("valid")
    }

    #[test]
    fn no_heights_is_flat_and_without_cliffs() {
        let map = tiny(&["+-+-+", "|. .|", "+ + +", "|. .|", "+-+-+"]);
        for cell in cells_of(&map) {
            assert_eq!(
                (cell.height, cell.cliffs, cell.ramps),
                (0, Edges::NONE, Edges::NONE)
            );
        }
    }

    #[test]
    fn the_terrain_entered_sets_the_climb_up_and_down() {
        let (e, w) = (Facing::East, Facing::West);
        // a (climb 1) at 0, b (climb 4) at 3, c (climb 2) at 5.
        let cells = cells_of(&strip([1, 4, 2], "|a b c|", "0 3 5"));
        assert_eq!(
            cells.iter().map(|c| c.height).collect::<Vec<_>>(),
            [0, 3, 5]
        );
        assert!(
            !cells[0].cliffs.has(e),
            "into b, which allows 4: 3 is a climb"
        );
        assert!(
            cells[1].cliffs.has(w),
            "back into a, which allows 1: 3 is a cliff"
        );
        assert!(
            !cells[1].cliffs.has(e),
            "into c, which allows 2: 2 is a climb"
        );
        assert!(
            !cells[2].cliffs.has(w),
            "into b, which allows 4: 2 is a drop"
        );
        let steep = cells_of(&strip([1, 4, 2], "|a b c|", "0 3 6"));
        assert!(
            steep[1].cliffs.has(e),
            "into c, which allows 2: 3 is a cliff"
        );
        assert!(
            !steep[2].cliffs.has(w),
            "and the drop of 3 into b is within its 4"
        );
    }

    #[test]
    fn a_ramp_crosses_up_to_two_walls_and_no_more() {
        let (e, w) = (Facing::East, Facing::West);
        let cells = cells_of(&strip([0, 0, 0], "|a/b c|", "0 16 16"));
        assert_eq!(cells[0].ramps, Edges::NONE.with(e));
        assert_eq!(cells[1].ramps, Edges::NONE.with(w));
        assert!(
            !cells[0].cliffs.has(e) && !cells[1].cliffs.has(w),
            "16 on a ramp walks"
        );
        let over = strip([0, 0, 0], "|a/b c|", "0 17 17");
        let mut errors = vec![];
        assert_eq!(ground(&over, &mut errors), None);
        let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        assert_eq!(
            messages,
            ["tile (1, 0) west ramp joins heights 17 and 0, more than 16 apart"],
            "reported once"
        );
    }

    #[test]
    fn heights_problems_are_all_reported() {
        let mut map = strip([2, 2, 2], "|a b c|", "0 1");
        let mut errors = vec![];
        assert_eq!(ground(&map, &mut errors), None);
        assert_eq!(
            errors[0].message,
            "heights row 0 has 2 numbers; a 3-tile-wide map needs 3"
        );
        map.heights = vec!["0 x 4097".into()];
        let mut errors = vec![];
        assert_eq!(ground(&map, &mut errors), None);
        let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        assert_eq!(
            messages,
            [
                "tile (1, 0) height 'x' must be a whole number within -4096..=4096",
                "tile (2, 0) height '4097' must be a whole number within -4096..=4096",
            ]
        );
        map.heights = vec!["0 0 0".into(), "0 0 0".into()];
        let mut errors = vec![];
        assert_eq!(ground(&map, &mut errors), None);
        assert_eq!(
            errors[0].message,
            "heights has 2 rows; a 1-tile-high map needs 1"
        );
        let ramp_out = strip([2, 2, 2], "/a b c|", "0 0 0");
        let mut errors = vec![];
        assert_eq!(ground(&ramp_out, &mut errors), None);
        assert_eq!(
            errors[0].message,
            "tile (0, 0) west edge is on the map boundary and must be a wall"
        );
    }
}
