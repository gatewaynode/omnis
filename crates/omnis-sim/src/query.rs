//! Read-only views for clients (ARCHITECTURE.md §4.3). Queries never mutate and never roll.

use crate::visibility;
use crate::world::{Known, World};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Facing, MapId, TilesetId};
use omnis_data::Data;
use serde::{Deserialize, Serialize};

mod path;

/// How an edge of a viewed tile looks from the party's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeView {
    /// Nothing there.
    Open,
    /// A wall.
    Wall,
    /// A door.
    Door {
        /// Whether it stands open.
        open: bool,
    },
}

/// One tile in the viewport, oriented to the party's facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewTile {
    /// Tiles ahead.
    pub depth: u8,
    /// Tiles to the right; negative is left.
    pub offset: i8,
    /// Map column.
    pub x: u16,
    /// Map row.
    pub y: u16,
    /// Terrain index into the map's terrains.
    pub terrain: u8,
    /// The edge away from the party.
    pub front: EdgeView,
    /// The edge on the party's left.
    pub left: EdgeView,
    /// The edge on the party's right.
    pub right: EdgeView,
}

/// What the renderer draws (ARCHITECTURE.md §8.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewportModel {
    /// The map.
    pub map: MapId,
    /// Its tileset.
    pub tileset: TilesetId,
    /// Which way the party looks.
    pub facing: Facing,
    /// Rows drawn with sprites.
    pub detail_depth: u8,
    /// Rows visible at all.
    pub visibility_depth: u8,
    /// Visible tiles, nearest first.
    pub tiles: Vec<ViewTile>,
}

/// The viewport model, or `None` when the party's map is not loaded.
#[must_use]
pub fn viewport(world: &World, data: &Data) -> Option<ViewportModel> {
    let pos = world.position;
    let map = data.maps.get(&pos.map)?;
    let state = world.maps.get(&pos.map);
    let edge = |x: u16, y: u16, facing: Facing| {
        let Some(cell) = map.cell(x, y) else {
            return EdgeView::Open;
        };
        if cell.walls.has(facing) {
            EdgeView::Wall
        } else if cell.doors.has(facing) {
            EdgeView::Door {
                open: state.is_some_and(|s| s.door_open(x, y, facing)),
            }
        } else {
            EdgeView::Open
        }
    };
    let tiles = visibility::cone(world, data)
        .into_iter()
        .filter_map(|t| {
            let cell = map.cell(t.x, t.y)?;
            Some(ViewTile {
                depth: t.depth,
                offset: t.offset,
                x: t.x,
                y: t.y,
                terrain: cell.terrain,
                front: edge(t.x, t.y, pos.facing),
                left: edge(t.x, t.y, pos.facing.left()),
                right: edge(t.x, t.y, pos.facing.right()),
            })
        })
        .collect();
    Some(ViewportModel {
        map: pos.map,
        tileset: map.tileset,
        facing: pos.facing,
        detail_depth: data
            .tilesets
            .get(&map.tileset)
            .map_or(0, |t| t.detail_depth),
        visibility_depth: visibility::depth(world, map),
        tiles,
    })
}

/// The party's knowledge of a map.
#[must_use]
pub fn automap(world: &World, map: MapId) -> Option<&BTreeMap<(u16, u16), Known>> {
    world.automap.map(map)
}

/// A map as text in the layout format of `omnis_data::map` (`2h+1` rows of `2w+1`
/// characters), with the world's door state and the party: walls `-` and `|`, closed doors
/// `=` and `:`, open doors `_` and `'`, and the party as `^`, `>`, `v`, or `<` on its tile.
/// `None` when the map is not loaded.
#[must_use]
pub fn map_text(world: &World, data: &Data, map_id: MapId) -> Option<String> {
    let map = data.maps.get(&map_id)?;
    let state = world.maps.get(&map_id);
    let (w, h) = (map.def.width, map.def.height);
    let edge = |x: u16, y: u16, facing: Facing, glyphs: [char; 3]| -> char {
        let Some(cell) = map.cell(x, y) else {
            return ' ';
        };
        if cell.walls.has(facing) {
            glyphs[0]
        } else if cell.doors.has(facing) {
            if state.is_some_and(|s| s.door_open(x, y, facing)) {
                glyphs[2]
            } else {
                glyphs[1]
            }
        } else {
            ' '
        }
    };
    let row_edge = ['-', '=', '_'];
    let column_edge = ['|', ':', '\''];
    let party = world.position;
    let mut out = String::with_capacity((usize::from(w) * 2 + 2) * (usize::from(h) * 2 + 1));
    for y in 0..h {
        for x in 0..w {
            out.push('+');
            out.push(edge(x, y, Facing::North, row_edge));
        }
        out.push_str("+\n");
        for x in 0..w {
            out.push(edge(x, y, Facing::West, column_edge));
            let glyph = if party.map == map_id && (party.x, party.y) == (x, y) {
                match party.facing {
                    Facing::North => '^',
                    Facing::East => '>',
                    Facing::South => 'v',
                    Facing::West => '<',
                }
            } else {
                map.cell(x, y).map_or('?', |c| map.terrain(c).glyph)
            };
            out.push(glyph);
        }
        out.push(edge(w.saturating_sub(1), y, Facing::East, column_edge));
        out.push('\n');
    }
    for x in 0..w {
        out.push('+');
        out.push(edge(x, h.saturating_sub(1), Facing::South, row_edge));
    }
    out.push_str("+\n");
    Some(out)
}

/// A value inside the world by dotted path, rendered as text: `position.x`,
/// `clocks.Party(0).elapsed`, `maps.3.open_doors.0`, `automap.maps.0.4,7.layers`. Map keys
/// are written the way `key_string` renders them; sequence elements by index. A compound
/// value at the end of the path renders as a summary such as `<struct Position>`.
#[must_use]
pub fn path(world: &World, path: &str) -> Option<String> {
    path::find(world, path)
}
