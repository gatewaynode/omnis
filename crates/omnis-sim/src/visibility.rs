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

use crate::event::SeenTile;
use crate::world::{MapState, World};
use alloc::vec::Vec;
use omnis_core::{Facing, Position};
use omnis_data::{Data, MapData};

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

/// The visibility depth from the party's tile.
#[must_use]
pub fn depth_from(map: &MapData, pos: Position) -> u8 {
    map.cell(pos.x, pos.y)
        .map_or(0, |c| map.terrain(c).visibility_depth)
}

/// Whether `to` can be seen from `from`: a ray from the centre of `from` reaches the centre
/// of `to` or any of its four corners. Each ray is tried both ways round every exact corner
/// crossing.
fn line_of_sight(
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

/// One supercover walk from the point `start` (a tile centre) to the point `end`, in half-tile
/// coordinates where tile `(x, y)` spans `[2x, 2x + 2]`. Every boundary crossing checks the
/// edge it passes; a tile other than the origin blocks further travel when it is opaque.
/// `x_first` decides which way round an exact corner crossing goes. Succeeds on entering
/// `target`, or on arriving at `end` (a point on the target's boundary) unblocked.
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
    while ix < crossings_x || iy < crossings_y {
        let toward_x = (2 * ix + 1) * ny;
        let toward_y = (2 * iy + 1) * nx;
        let step_x_now = iy >= crossings_y
            || (ix < crossings_x && (toward_x < toward_y || (toward_x == toward_y && x_first)));
        let facing = if step_x_now {
            ix += 1;
            step_x
        } else {
            iy += 1;
            step_y
        };
        if (x, y) != from && map.cell(x, y).is_some_and(|c| map.terrain(c).opaque) {
            return false;
        }
        if !edge_open(map, state, x, y, facing) {
            return false;
        }
        let (fx, fy) = facing.delta();
        x = u16::try_from(i32::from(x) + fx).unwrap_or(u16::MAX);
        y = u16::try_from(i32::from(y) + fy).unwrap_or(u16::MAX);
        if map.cell(x, y).is_none() {
            return false;
        }
        if (x, y) == target {
            return true;
        }
    }
    true
}

/// Every visible tile, nearest row first, centre outward within a row.
#[must_use]
pub fn cone(world: &World, data: &Data) -> Vec<SeenTile> {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return Vec::new();
    };
    let state = world.maps.get(&pos.map);
    let max_depth = depth_from(map, pos);
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
