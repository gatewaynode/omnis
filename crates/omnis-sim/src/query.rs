//! Read-only views for clients (ARCHITECTURE.md §4.3). Queries never mutate and never roll.

use crate::visibility;
use crate::world::{Known, World};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Facing, MapId, TilesetId};
use omnis_data::Data;

mod path;

/// How an edge of a viewed tile looks from the party's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
        visibility_depth: visibility::depth_from(map, pos),
        tiles,
    })
}

/// The party's knowledge of a map.
#[must_use]
pub fn automap(world: &World, map: MapId) -> Option<&BTreeMap<(u16, u16), Known>> {
    world.automap.map(map)
}

/// A value inside the world by dotted path, rendered as text: `position.x`,
/// `clocks.Party(0).elapsed`, `maps.3.open_doors.0`, `automap.maps.0.4,7.layers`. Map keys
/// are written the way `key_string` renders them; sequence elements by index. A compound
/// value at the end of the path renders as a summary such as `<struct Position>`.
#[must_use]
pub fn path(world: &World, path: &str) -> Option<String> {
    path::find(world, path)
}
