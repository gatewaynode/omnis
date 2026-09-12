//! The forward cone the party perceives (PRD §7.2, D16). Visibility depth comes from the tile
//! the party stands on; occlusion follows walls, closed doors, and opaque terrain.
//!
//! A tile at `(depth, offset)` is visible when a straight line from the party's tile reaches
//! it: the line is walked one orthogonal tile step at a time (a supercover line, so every
//! consecutive pair of tiles shares an edge), each shared edge must be open, and every tile
//! before the target must be see-through. Opaque tiles are seen but not seen through. All
//! arithmetic is integer, so the cone is identical on every platform.

use crate::command::SeenTile;
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

/// Whether `(x1, y1)` can be seen from `(x0, y0)`.
fn line_of_sight(
    map: &MapData,
    state: Option<&MapState>,
    (x0, y0): (u16, u16),
    (x1, y1): (u16, u16),
) -> bool {
    let opaque = |x: u16, y: u16| map.cell(x, y).is_some_and(|c| map.terrain(c).opaque);
    let (dx, dy) = (i32::from(x1) - i32::from(x0), i32::from(y1) - i32::from(y0));
    let (nx, ny) = (dx.abs(), dy.abs());
    let step_x = if dx > 0 { Facing::East } else { Facing::West };
    let step_y = if dy > 0 { Facing::South } else { Facing::North };
    let (mut x, mut y) = (x0, y0);
    let (mut ix, mut iy) = (0, 0);
    while ix < nx || iy < ny {
        // Which tile boundary does the line from centre to centre cross next? Compare the
        // parameters t at which x = ix + 1/2 and y = iy + 1/2, cross-multiplied.
        let toward_x = (2 * ix + 1) * ny;
        let toward_y = (2 * iy + 1) * nx;
        let facing = if iy >= ny || (ix < nx && toward_x <= toward_y) {
            ix += 1;
            step_x
        } else {
            iy += 1;
            step_y
        };
        if (x, y) != (x0, y0) && opaque(x, y) {
            return false;
        }
        if !edge_open(map, state, x, y, facing) {
            return false;
        }
        let (fx, fy) = facing.delta();
        x = u16::try_from(i32::from(x) + fx).unwrap_or(u16::MAX);
        y = u16::try_from(i32::from(y) + fy).unwrap_or(u16::MAX);
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
        let half = i8::try_from(depth).unwrap_or(i8::MAX);
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
