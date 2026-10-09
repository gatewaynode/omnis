//! Read-only views for clients (ARCHITECTURE.md §4.3). Queries never mutate and never roll.

use crate::names::{Place, id_of};
use crate::time_view::{DateView, party_date};
use crate::visibility;
use crate::world::{Known, Mode, ModeKind, Settings, World};
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Direction, Facing, MapId, Position, ServiceId};
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
    /// The map's string id.
    pub map: String,
    /// Its tileset's string id.
    pub tileset: String,
    /// Which way the party looks.
    pub facing: Facing,
    /// Rows drawn with sprites.
    pub detail_depth: u8,
    /// Rows visible at all.
    pub visibility_depth: u8,
    /// Visible tiles, nearest first.
    pub tiles: Vec<ViewTile>,
}

/// Where the party is and what it is doing: the cheap read a client makes every frame (M8 step 8).
/// `game.status` adds what costs more to work out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Here {
    /// What the party is doing.
    pub mode: ModeKind,
    /// Commands applied.
    pub turn: u64,
    /// Where the party is.
    pub position: Place,
    /// The id of the service the party is inside, if it is inside one.
    pub service: Option<String>,
    /// The party's age in minutes; it never reverses.
    pub age: i64,
    /// The date the party believes, on the pack's calendar.
    pub date: DateView,
    /// Whether the save rule allows a save here.
    pub may_save: bool,
    /// The world seed, shown so a world can be shared.
    pub seed: u64,
    /// The difficulty options the game started with.
    pub settings: Settings,
}

/// Where the party is and what it is doing.
#[must_use]
pub fn here(world: &World, data: &Data) -> Here {
    Here {
        mode: world.mode.kind(),
        turn: world.turn,
        position: Place::of(world.position, data),
        service: match world.mode {
            Mode::Town(state) => data.services.get(&state.service).map(|d| d.id.clone()),
            _ => None,
        },
        age: world.party_clock().elapsed,
        date: party_date(world, data),
        may_save: world.may_save(),
        seed: world.seed,
        settings: world.settings,
    }
}

/// A map's id, or `#n` for one no pack names.
#[must_use]
pub fn map_name(id: MapId, data: &Data) -> String {
    id_of(&data.registry.maps, id)
}

/// Every flag the packs declare with its value (0 when never set), in the packs' order.
#[must_use]
pub fn flags(world: &World, data: &Data) -> Vec<(String, i64)> {
    data.registry
        .flags
        .names()
        .map(|name| {
            let value = data
                .registry
                .flags
                .get(name)
                .and_then(|id| world.flags.get(&id).copied())
                .unwrap_or(0);
            (name.to_owned(), value)
        })
        .collect()
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
        map: id_of(&data.registry.maps, pos.map),
        tileset: id_of(&data.registry.tilesets, map.tileset),
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
/// `=` and `:`, open doors `_` and `'`, a portal as `*` on its tile (a glyph no terrain may
/// use), and the party as `^`, `>`, `v`, or `<` on its tile, over a portal.
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
            } else if map.portal_at(x, y).is_some() {
                '*'
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

/// Where a step in `direction` would leave the party (through a portal, at its far end), or
/// `None` when a wall, a closed door, the terrain or the map's edge stops it. Clients ask
/// before a step that needs a confirmation.
#[must_use]
pub fn step_lands(world: &World, data: &Data, direction: Direction) -> Option<Position> {
    crate::apply::landing(world, data, direction)
        .ok()
        .map(crate::apply::Landing::end)
}

/// The service a step in `direction` would go into, if it lands on a site.
#[must_use]
pub fn site_ahead(world: &World, data: &Data, direction: Direction) -> Option<ServiceId> {
    let at = step_lands(world, data, direction)?;
    data.maps.get(&at.map)?.site_at(at.x, at.y)
}
