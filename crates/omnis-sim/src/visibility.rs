//! The forward cone the party perceives (PRD §7.2, D16). Visibility depth comes from the tile
//! the party stands on; occlusion follows walls, closed doors, and opaque terrain. Row `d` spans
//! offsets `-(d + 1)..=(d + 1)`: the viewport shows the tile one beyond the diagonal at the far
//! end of every row, and the tiles beside the party at row 0.
//!
//! A tile at `(depth, offset)` is visible when a straight line from the party's tile reaches
//! it: the line is walked one orthogonal tile step at a time (a supercover line, so every
//! consecutive pair of tiles shares an edge), each shared edge must be open, and every tile
//! before the target must be see-through. Opaque tiles are seen but not seen through. A tile
//! is seen if the ray to its centre or to any of its corners gets there, and where a ray
//! passes exactly through a corner either way round it may be taken, so a pillar hides only
//! what lies squarely behind it. All arithmetic is integer, so the cone is identical on every
//! platform.
//!
//! Height (in eighths of a cell; a wall is [`WALL`] tall) is a second test along the same walk.
//! The sight line runs from the eye, [`EYE`] above the party's floor, down or up to the
//! target's floor. It is blocked where something rises above it: the ground of a tile at its
//! entry or exit, a wall or a closed door ([`WALL`] above the higher floor beside it), or an
//! opaque tile ([`WALL`] above its floor). On a flat map every wall and opaque tile rises
//! above every sight line and no ground does, so the cone is the flat one. Standing above the
//! ground around extends how far the party sees (rule value `sight_rise_per_tile`).

use crate::event::SeenTile;
use crate::world::{MapState, World};
use alloc::vec::Vec;
use omnis_core::{Facing, Position};
use omnis_data::limits::MAX_VISIBILITY_DEPTH;
use omnis_data::{Data, MapData};
use omnis_rules::light_depth;

/// A wall's height, and an opaque tile's, in eighths of a cell.
pub const WALL: i64 = 8;
/// The eye's height above the party's floor: half a wall.
pub const EYE: i64 = 4;

/// Whether the edge on `facing` of `(x, y)` lets sight and movement through.
#[must_use]
pub fn edge_open(map: &MapData, state: Option<&MapState>, x: u16, y: u16, facing: Facing) -> bool {
    match map.cell(x, y) {
        None => false,
        Some(cell) if cell.walls.has(facing) => false,
        Some(cell) if cell.doors.has(facing) => state.is_some_and(|s| s.door_open(x, y, facing)),
        Some(_) => true,
    }
}

/// The map coordinates `depth` tiles ahead and `offset` tiles to the right of `pos`.
#[must_use]
pub fn project(pos: Position, depth: u8, offset: i8) -> Option<(u16, u16)> {
    let (fx, fy) = pos.facing.delta();
    let (rx, ry) = pos.facing.right().delta();
    let x = i32::from(pos.x) + fx * i32::from(depth) + rx * i32::from(offset);
    let y = i32::from(pos.y) + fy * i32::from(depth) + ry * i32::from(offset);
    Some((u16::try_from(x).ok()?, u16::try_from(y).ok()?))
}

/// The visibility depth from the party's tile: the terrain's.
#[must_use]
pub fn depth_from(map: &MapData, pos: Position) -> u8 {
    map.cell(pos.x, pos.y)
        .map_or(0, |c| map.terrain(c).visibility_depth)
}

/// How far the party sees: the terrain's depth, or a light in effect when it reaches farther
/// (daylight is not extended by a torch), plus a tile for every `sight_rise_per_tile` the
/// party stands above the lowest tile around it, capped at the map limit.
#[must_use]
pub fn depth(world: &World, data: &Data, map: &MapData) -> u8 {
    let terrain = depth_from(map, world.position);
    let lit = light_depth(&world.party.effects).map_or(terrain, |light| terrain.max(light));
    let rise = data.rules.value("sight_rise_per_tile").unwrap_or(0);
    let extra = if rise > 0 {
        above(map, world.position) / rise
    } else {
        0
    };
    let extra = u8::try_from(extra).unwrap_or(u8::MAX);
    lit.saturating_add(extra).min(MAX_VISIBILITY_DEPTH)
}

/// How far `pos`'s floor stands above the lowest of the eight tiles around it on the map, or
/// 0 when none is lower.
#[must_use]
pub fn above(map: &MapData, pos: Position) -> i64 {
    let Some(here) = map.cell(pos.x, pos.y) else {
        return 0;
    };
    let mut lowest = i64::from(here.height);
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            let x = u16::try_from(i32::from(pos.x) + dx).ok();
            let y = u16::try_from(i32::from(pos.y) + dy).ok();
            if let Some(cell) = x.zip(y).and_then(|(x, y)| map.cell(x, y)) {
                lowest = lowest.min(i64::from(cell.height));
            }
        }
    }
    i64::from(here.height) - lowest
}

/// Whether `to` can be seen from `from`: a ray from the centre of `from` reaches the centre
/// of `to` or any of its four corners. Each ray is tried both ways round every exact corner
/// crossing.
#[must_use]
pub fn line_of_sight(
    map: &MapData,
    state: Option<&MapState>,
    from: (u16, u16),
    to: (u16, u16),
) -> bool {
    let start = (2 * i32::from(from.0) + 1, 2 * i32::from(from.1) + 1);
    let (tx, ty) = (2 * i32::from(to.0), 2 * i32::from(to.1));
    let ends = [
        (tx + 1, ty + 1),
        (tx, ty),
        (tx + 2, ty),
        (tx, ty + 2),
        (tx + 2, ty + 2),
    ];
    ends.iter().any(|&end| {
        walk(map, state, from, to, start, end, true)
            || walk(map, state, from, to, start, end, false)
    })
}

/// The sight line's height at a fraction `p / q` of the way along: from `eye` to `floor`.
#[derive(Clone, Copy)]
struct Sight {
    eye: i64,
    floor: i64,
}

impl Sight {
    /// Whether `top` rises above the sight line at `p / q` of the way along (`q > 0`).
    fn below(self, top: i64, (p, q): (i64, i64)) -> bool {
        top * q > self.eye * q + (self.floor - self.eye) * p
    }
}

fn floor_of(map: &MapData, x: u16, y: u16) -> i64 {
    map.cell(x, y).map_or(0, |c| i64::from(c.height))
}

/// One supercover walk from the point `start` (a tile centre) to the point `end`, in half-tile
/// coordinates where tile `(x, y)` spans `[2x, 2x + 2]`. Every boundary crossing checks the
/// edge it passes; a tile other than the origin blocks further travel when it is opaque.
/// `x_first` decides which way round an exact corner crossing goes. Succeeds on entering
/// `target`, or on arriving at `end` (a point on the target's boundary) unblocked. Height is
/// checked at every crossing, as the module documentation says.
fn walk(
    map: &MapData,
    state: Option<&MapState>,
    from: (u16, u16),
    target: (u16, u16),
    start: (i32, i32),
    end: (i32, i32),
    x_first: bool,
) -> bool {
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    let (nx, ny) = (dx.abs(), dy.abs());
    // Boundaries lie at even coordinates; from an odd start they sit at distances 1, 3, 5, ...
    let (crossings_x, crossings_y) = ((nx + 1) / 2, (ny + 1) / 2);
    let step_x = if dx > 0 { Facing::East } else { Facing::West };
    let step_y = if dy > 0 { Facing::South } else { Facing::North };
    let (mut x, mut y) = from;
    let (mut ix, mut iy) = (0, 0);
    if (x, y) == target {
        return true;
    }
    let sight = Sight {
        eye: floor_of(map, from.0, from.1) + EYE,
        floor: floor_of(map, target.0, target.1),
    };
    // Where the walk entered the current tile, as a fraction of the way along.
    let mut entered = (0_i64, 1_i64);
    while ix < crossings_x || iy < crossings_y {
        let toward_x = (2 * ix + 1) * ny;
        let toward_y = (2 * iy + 1) * nx;
        let step_x_now = iy >= crossings_y
            || (ix < crossings_x && (toward_x < toward_y || (toward_x == toward_y && x_first)));
        let (facing, at) = if step_x_now {
            ix += 1;
            (step_x, (i64::from(2 * ix - 1), i64::from(nx)))
        } else {
            iy += 1;
            (step_y, (i64::from(2 * iy - 1), i64::from(ny)))
        };
        let floor = floor_of(map, x, y);
        if (x, y) != from
            && map.cell(x, y).is_some_and(|c| map.terrain(c).opaque)
            && (sight.below(floor + WALL, entered) || sight.below(floor + WALL, at))
        {
            return false;
        }
        if sight.below(floor, at) {
            return false;
        }
        let (fx, fy) = facing.delta();
        let (bx, by) = (
            u16::try_from(i32::from(x) + fx).unwrap_or(u16::MAX),
            u16::try_from(i32::from(y) + fy).unwrap_or(u16::MAX),
        );
        if !edge_open(map, state, x, y, facing) {
            let top = floor.max(floor_of(map, bx, by)) + WALL;
            if map.cell(bx, by).is_none() || sight.below(top, at) {
                return false;
            }
        }
        (x, y) = (bx, by);
        if map.cell(x, y).is_none() {
            return false;
        }
        if (x, y) == target {
            return true;
        }
        if sight.below(floor_of(map, x, y), at) {
            return false;
        }
        entered = at;
    }
    true
}

/// The tiles straight ahead of `pos`, nearest first, up to `range` of them: the walk stops
/// before a wall or a closed door, after an opaque tile (seen, not seen through), at the map's
/// edge, and at the first tile the ground or a height hides. What a spyglass looks along.
#[must_use]
pub fn ray(map: &MapData, state: Option<&MapState>, pos: Position, range: u8) -> Vec<(u16, u16)> {
    let mut tiles = Vec::new();
    let (mut x, mut y) = (pos.x, pos.y);
    for _ in 0..range {
        if !edge_open(map, state, x, y, pos.facing) {
            break;
        }
        let Some((nx, ny)) = (Position { x, y, ..pos }).neighbour(pos.facing) else {
            break;
        };
        let Some(cell) = map.cell(nx, ny) else {
            break;
        };
        if !line_of_sight(map, state, (pos.x, pos.y), (nx, ny)) {
            break;
        }
        tiles.push((nx, ny));
        if map.terrain(cell).opaque {
            break;
        }
        (x, y) = (nx, ny);
    }
    tiles
}

/// Every visible tile, nearest row first, centre outward within a row.
#[must_use]
pub fn cone(world: &World, data: &Data) -> Vec<SeenTile> {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return Vec::new();
    };
    let state = world.maps.get(&pos.map);
    let max_depth = depth(world, data, map);
    let mut seen = Vec::new();
    for depth in 0..=max_depth {
        // One tile wider than the diagonal: the far end of each row shows the tile beyond
        // it (a face at distance `z` is `0.9 * height / z` tall, so the canvas edge lies at
        // offset `0.99 * z`). Line of sight through the shared edge handles a wall beside
        // the party.
        let half = i8::try_from(depth.saturating_add(1)).unwrap_or(i8::MAX);
        let mut offsets: Vec<i8> = (0..=half).collect();
        offsets.extend((1..=half).map(|o| -o));
        for offset in offsets {
            let Some((x, y)) = project(pos, depth, offset) else {
                continue;
            };
            if map.cell(x, y).is_none() {
                continue;
            }
            if line_of_sight(map, state, (pos.x, pos.y), (x, y)) {
                seen.push(SeenTile {
                    x,
                    y,
                    depth,
                    offset,
                });
            }
        }
    }
    seen
}
