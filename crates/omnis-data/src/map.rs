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

use crate::error::DataError;
use crate::limits::{MAX_COLLECTION, MAX_MAP_SIDE, MAX_VISIBILITY_DEPTH, string_fits};
use omnis_core::{Edges, Facing};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

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
    /// Whether the party may enter the tile.
    pub passable: bool,
    /// Whether the tile blocks line of sight (rock, dense trees). Water does not.
    #[serde(default)]
    pub opaque: bool,
    /// How far the party sees while standing on this terrain.
    pub visibility_depth: u8,
    /// Minutes one step onto this terrain costs the party's clock.
    pub step_minutes: u32,
}

/// A tile trigger that moves the party to another map or tile when stepped on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portal {
    /// Trigger tile column.
    pub x: u16,
    /// Trigger tile row.
    pub y: u16,
    /// Destination map id.
    pub to_map: String,
    /// Destination column.
    pub to_x: u16,
    /// Destination row.
    pub to_y: u16,
    /// Facing on arrival.
    pub to_facing: Facing,
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
    /// Tileset surface for doors on this map.
    pub door: String,
    /// The terrains, each with a distinct glyph.
    pub terrains: Vec<Terrain>,
    /// The grid as text; see the module documentation.
    pub layout: Vec<String>,
    /// Tile triggers.
    #[serde(default)]
    pub portals: Vec<Portal>,
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
}

impl MapDef {
    /// Parse `layout` into cells, row-major. Every problem is reported; cells are returned
    /// only when there were none.
    pub fn cells(&self, file: &Path, errors: &mut Vec<DataError>) -> Option<Vec<Cell>> {
        let before = errors.len();
        let width = usize::from(self.width);
        let height = usize::from(self.height);
        let glyphs: BTreeMap<char, u8> = self
            .terrains
            .iter()
            .enumerate()
            .filter_map(|(i, t)| u8::try_from(i).ok().map(|i| (t.glyph, i)))
            .collect();
        if self.layout.len() != 2 * height + 1 {
            errors.push(DataError::new(
                file,
                format!(
                    "layout has {} rows; a {}-tile-high map needs {}",
                    self.layout.len(),
                    height,
                    2 * height + 1
                ),
            ));
            return None;
        }
        let mut rows: Vec<Vec<char>> = Vec::with_capacity(self.layout.len());
        for (r, line) in self.layout.iter().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            if chars.len() != 2 * width + 1 {
                errors.push(DataError::new(
                    file,
                    format!(
                        "layout row {r} has {} characters; a {}-tile-wide map needs {}",
                        chars.len(),
                        width,
                        2 * width + 1
                    ),
                ));
            }
            rows.push(chars);
        }
        if errors.len() > before {
            return None;
        }
        let mut cells = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let glyph = rows[2 * y + 1][2 * x + 1];
                let Some(&terrain) = glyphs.get(&glyph) else {
                    errors.push(DataError::new(
                        file,
                        format!("tile ({x}, {y}) has unknown glyph '{glyph}'"),
                    ));
                    continue;
                };
                let mut walls = Edges::NONE;
                let mut doors = Edges::NONE;
                let edges = [
                    (Facing::North, rows[2 * y][2 * x + 1], true),
                    (Facing::South, rows[2 * y + 2][2 * x + 1], true),
                    (Facing::West, rows[2 * y + 1][2 * x], false),
                    (Facing::East, rows[2 * y + 1][2 * x + 2], false),
                ];
                for (facing, c, horizontal) in edges {
                    let boundary = match facing {
                        Facing::North => y == 0,
                        Facing::South => y + 1 == height,
                        Facing::West => x == 0,
                        Facing::East => x + 1 == width,
                    };
                    match (c, horizontal) {
                        ('-', true) | ('|', false) => walls = walls.with(facing),
                        ('=', true) | (':', false) => doors = doors.with(facing),
                        (' ', _) => {}
                        _ => errors.push(DataError::new(
                            file,
                            format!("tile ({x}, {y}) {facing} edge has unknown character '{c}'"),
                        )),
                    }
                    if boundary && !walls.has(facing) {
                        errors.push(DataError::new(
                            file,
                            format!("tile ({x}, {y}) {facing} edge is on the map boundary and must be a wall"),
                        ));
                    }
                }
                cells.push(Cell {
                    terrain,
                    walls,
                    doors,
                });
            }
        }
        (errors.len() == before).then_some(cells)
    }

    /// Checks that need no other file: sizes, glyphs, start tile, portal coordinates.
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
            if matches!(terrain.glyph, '-' | '=' | '|' | ':' | '+' | ' ') {
                errors.push(DataError::new(
                    file,
                    format!("terrain glyph '{}' is reserved for edges", terrain.glyph),
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
        }
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
            terrains: vec![Terrain {
                glyph: '.',
                name: "floor".into(),
                floor: "floor".into(),
                ceiling: None,
                passable: true,
                opaque: false,
                visibility_depth: 4,
                step_minutes: 1,
            }],
            layout: layout.iter().map(|s| (*s).to_owned()).collect(),
            portals: vec![],
        }
    }

    #[test]
    fn layout_edges_are_shared_between_neighbours() {
        let map = tiny(&["+-+-+", "|. .|", "+=+ +", "|.|.|", "+-+-+"]);
        let mut errors = vec![];
        let cells = map.cells(Path::new("m"), &mut errors).expect("valid");
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
        assert_eq!(map.cells(Path::new("m"), &mut errors), None);
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
        assert_eq!(open.cells(Path::new("m"), &mut errors), None);
        assert_eq!(
            errors[0].message,
            "tile (0, 1) west edge is on the map boundary and must be a wall"
        );
        let short = tiny(&["+-+-+", "|. .|", "+-+-+"]);
        let mut errors = vec![];
        assert_eq!(short.cells(Path::new("m"), &mut errors), None);
        assert_eq!(
            errors[0].message,
            "layout has 3 rows; a 2-tile-high map needs 5"
        );
    }
}
