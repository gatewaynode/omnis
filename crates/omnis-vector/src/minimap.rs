//! The minimap as pixels (presentation-ARCHITECTURE.md §8): what the party knows of the current map,
//! from the automap view (`SimWorld::automap`), with the pose marked. No Bevy, so it is tested directly; the shell
//! uploads the pixels into an image.

use crate::pose::Pose;
use omnis_sim::Known;
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::MapData;
use omnis_sim::world::layer;
use std::collections::BTreeMap;

pub use crate::raster::Raster;

/// The longest side of the minimap, in pixels, before scaling to whole pixels a cell.
pub const SIDE: u32 = 192;
/// The fewest pixels a cell.
pub const MIN_SCALE: u32 = 2;

/// Unknown ground.
pub const BACKGROUND: [u8; 4] = [0, 10, 4, 190];
/// A known wall.
pub const WALL: [u8; 4] = [60, 255, 120, 255];
/// A known door.
pub const DOOR: [u8; 4] = [255, 170, 40, 255];
/// A known cliff edge (provisional until the colour roles, M8.5 step 5).
pub const CLIFF: [u8; 4] = [230, 150, 60, 255];
/// A known ramp edge's tick.
pub const RAMP: [u8; 4] = [120, 220, 255, 255];
/// The party.
pub const MARKER: [u8; 4] = [255, 255, 255, 255];

/// Pixels a cell for a map of `width × height` cells.
#[must_use]
pub fn scale_for(width: u16, height: u16) -> u32 {
    (SIDE / u32::from(width.max(height)).max(1)).max(MIN_SCALE)
}

fn shade(rgb: (u8, u8, u8), k: u16) -> [u8; 4] {
    let f = |c: u8| u8::try_from(u16::from(c) * k / 100).unwrap_or(u8::MAX);
    [f(rgb.0), f(rgb.1), f(rgb.2), 230]
}

/// A known edge: horizontal (a north side) or vertical (a west side), by the column and row of
/// the cell whose north or west side it is, so an edge shared by two cells has one key.
type EdgeKey = (bool, i64, i64);

fn key(x: u16, y: u16, facing: Facing) -> EdgeKey {
    let (x, y) = (i64::from(x), i64::from(y));
    match facing {
        Facing::North => (true, x, y),
        Facing::South => (true, x, y + 1),
        Facing::West => (false, x, y),
        Facing::East => (false, x + 1, y),
    }
}

/// The known walls and doors, each once, a door on either side winning over a wall: the map
/// may record a shared edge on one side only (as `geometry::extract` assumes).
fn edges(known: &BTreeMap<(u16, u16), Known>) -> BTreeMap<EdgeKey, bool> {
    let mut out = BTreeMap::new();
    for (&(x, y), tile) in known {
        for facing in Facing::ALL {
            let door = tile.doors.has(facing);
            if door || tile.walls.has(facing) {
                *out.entry(key(x, y, facing)).or_insert(door) |= door;
            }
        }
    }
    out
}

/// What height puts on an edge: a cliff, or a ramp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rise {
    Cliff,
    Ramp,
}

/// The cliff and ramp edges of the known tiles, each once, read from the map's cells: a cliff
/// facing either way, seen from the known side.
fn rises(map: &MapData, known: &BTreeMap<(u16, u16), Known>) -> BTreeMap<EdgeKey, Rise> {
    let mut out = BTreeMap::new();
    for &(x, y) in known.keys() {
        let Some(cell) = map.cell(x, y) else { continue };
        for facing in Facing::ALL {
            let (dx, dy) = facing.delta();
            let beyond = u16::try_from(i32::from(x) + dx)
                .ok()
                .zip(u16::try_from(i32::from(y) + dy).ok())
                .and_then(|(nx, ny)| map.cell(nx, ny));
            let back = facing.opposite();
            let rise = if cell.cliffs.has(facing) || beyond.is_some_and(|b| b.cliffs.has(back)) {
                Rise::Cliff
            } else if cell.ramps.has(facing) {
                Rise::Ramp
            } else {
                continue;
            };
            out.insert(key(x, y, facing), rise);
        }
    }
    out
}

/// The pixels of a tick across the middle of an edge, half a cell long.
fn tick_pixels(raster: &Raster, (horizontal, col, row): EdgeKey) -> Vec<(i64, i64)> {
    let s = i64::from(raster.scale);
    let (mx, my) = if horizontal {
        (col * s + s / 2, row * s)
    } else {
        (col * s, row * s + s / 2)
    };
    (-s / 4..=s / 4)
        .map(|i| {
            if horizontal {
                (mx, my + i)
            } else {
                (mx + i, my)
            }
        })
        .collect()
}

/// The pixels of an edge, `inset` pixels in from each end. An edge on the map's far side is
/// drawn on the last pixel row or column.
fn edge_pixels(raster: &Raster, (horizontal, col, row): EdgeKey, inset: i64) -> Vec<(i64, i64)> {
    let s = i64::from(raster.scale);
    let (x0, y0) = (
        (col * s).min(i64::from(raster.width) - 1),
        (row * s).min(i64::from(raster.height) - 1),
    );
    (inset..s - inset)
        .map(|i| {
            if horizontal {
                (col * s + i, y0)
            } else {
                (x0, row * s + i)
            }
        })
        .collect()
}

/// Paint the known tiles of `map` (visited ones brighter than ones only seen), their cliffs and
/// ramps (M8.5 step 4), their walls and doors over those, then the party as a triangle pointing along the yaw.
#[must_use]
pub fn paint(map: &MapData, known: Option<&BTreeMap<(u16, u16), Known>>, pose: Pose) -> Raster {
    let scale = scale_for(map.def.width, map.def.height);
    let mut out = Raster::filled(
        u32::from(map.def.width) * scale,
        u32::from(map.def.height) * scale,
        scale,
        BACKGROUND,
    );
    let s = i64::from(scale);
    let empty = BTreeMap::new();
    let known = known.unwrap_or(&empty);
    for (&(x, y), tile) in known {
        let Some(terrain) = map.def.terrains.get(usize::from(tile.terrain)) else {
            continue;
        };
        let k = if tile.layers & layer::VISITED != 0 {
            60
        } else {
            35
        };
        let fill = shade(terrain.color, k);
        for dy in 0..s {
            for dx in 0..s {
                out.put(i64::from(x) * s + dx, i64::from(y) * s + dy, fill);
            }
        }
    }
    for (edge, rise) in rises(map, known) {
        let (pixels, colour) = match rise {
            Rise::Cliff => (edge_pixels(&out, edge, 0), CLIFF),
            Rise::Ramp => (tick_pixels(&out, edge), RAMP),
        };
        for (px, py) in pixels {
            out.put(px, py, colour);
        }
    }
    for (edge, door) in edges(known) {
        let (inset, colour) = if door { (s / 4, DOOR) } else { (0, WALL) };
        for (px, py) in edge_pixels(&out, edge, inset) {
            out.put(px, py, colour);
        }
    }
    marker(&mut out, pose);
    out
}

/// The party: a filled triangle a cell and a half long (never under nine pixels), its tip
/// toward the yaw and its base close behind the pose, so the heading reads at a glance.
fn marker(out: &mut Raster, pose: Pose) {
    #[allow(clippy::cast_precision_loss)]
    let size = (out.scale as f32 * 1.5).max(9.0);
    let s = out.scale as f32;
    let c = (pose.x * s, pose.z * s);
    let f = crate::geom::forward(pose.yaw);
    let r = crate::geom::right(pose.yaw);
    let at = |along: f32, across: f32| {
        (
            c.0 + f.0 * along * size + r.0 * across * size,
            c.1 + f.1 * along * size + r.1 * across * size,
        )
    };
    let tri = [at(0.7, 0.0), at(-0.3, 0.35), at(-0.3, -0.35)];
    let side = |a: (f32, f32), b: (f32, f32), p: (f32, f32)| {
        (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
    };
    #[allow(clippy::cast_possible_truncation)]
    let (lo_x, hi_x, lo_y, hi_y) = (
        (c.0 - size).floor() as i64,
        (c.0 + size).ceil() as i64,
        (c.1 - size).floor() as i64,
        (c.1 + size).ceil() as i64,
    );
    for py in lo_y..=hi_y {
        for px in lo_x..=hi_x {
            #[allow(clippy::cast_precision_loss)]
            let p = (px as f32 + 0.5, py as f32 + 0.5);
            let d = [
                side(tri[0], tri[1], p),
                side(tri[1], tri[2], p),
                side(tri[2], tri[0], p),
            ];
            if d.iter().all(|v| *v >= 0.0) || d.iter().all(|v| *v <= 0.0) {
                out.put(px, py, MARKER);
            }
        }
    }
}
