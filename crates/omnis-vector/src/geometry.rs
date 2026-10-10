//! Map data to vector line segments (alt-ARCHITECTURE.md §7.1).
//!
//! Every wall and door edge is emitted once, keyed by its canonical line: a horizontal edge at
//! row line `z` from `x` to `x + 1`, or a vertical edge at column line `x` from `z` to `z + 1`.

use crate::geom::wall_height;
use omnis_sim::World;
use omnis_sim::omnis_core::{Facing, MapId};
use omnis_sim::omnis_data::MapData;
use std::collections::BTreeMap;

/// What a segment belongs to; the renderer picks the colour from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SegKind {
    /// A wall's outline.
    Wall,
    /// A door's frame, with a cross while it is closed.
    Door,
    /// Solid terrain (a pillar, trees).
    Block,
    /// The floor grid.
    Floor,
    /// Impassable open terrain (water), crossed on the floor.
    Barrier,
    /// The ceiling grid.
    Ceiling,
}

/// One line in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// One end.
    pub a: [f32; 3],
    /// The other end.
    pub b: [f32; 3],
    /// What it belongs to.
    pub kind: SegKind,
    /// The terrain colour of the cell it came from.
    pub rgb: (u8, u8, u8),
}

/// A canonical edge: `(horizontal, column, row)` of its lower-left corner.
type EdgeKey = (bool, u16, u16);

/// What stands on an edge: its kind, whether a door is open, the terrain colour, the height.
type EdgeLook = (SegKind, bool, (u8, u8, u8), f32);

/// The segments for `map`, with doors drawn open or closed as `world` has them.
#[must_use]
pub fn extract(map: &MapData, map_id: MapId, world: &World) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut edges: BTreeMap<EdgeKey, EdgeLook> = BTreeMap::new();
    let state = world.maps.get(&map_id);
    for y in 0..map.def.height {
        for x in 0..map.def.width {
            let Some(cell) = map.cell(x, y) else { continue };
            let terrain = map.terrain(cell);
            let h = wall_height(terrain);
            cells(
                &mut out,
                x,
                y,
                terrain.color,
                terrain.passable,
                terrain.block.is_some(),
                terrain.ceiling.is_some(),
                h,
            );
            for facing in Facing::ALL {
                let kind = if cell.doors.has(facing) {
                    SegKind::Door
                } else if cell.walls.has(facing) {
                    SegKind::Wall
                } else {
                    continue;
                };
                let open = state.is_some_and(|s| s.door_open(x, y, facing));
                let entry =
                    edges
                        .entry(key(x, y, facing))
                        .or_insert((kind, open, terrain.color, h));
                // A door on either side wins over a wall on the other.
                if kind == SegKind::Door {
                    *entry = (kind, open, terrain.color, h);
                }
            }
        }
    }
    for (&(horizontal, col, row), &(kind, open, rgb, h)) in &edges {
        let (x0, z0) = (f32::from(col), f32::from(row));
        let (x1, z1) = if horizontal {
            (x0 + 1.0, z0)
        } else {
            (x0, z0 + 1.0)
        };
        edge(&mut out, [x0, z0], [x1, z1], kind, open, rgb, h);
    }
    out
}

fn key(x: u16, y: u16, facing: Facing) -> EdgeKey {
    match facing {
        Facing::North => (true, x, y),
        Facing::South => (true, x, y + 1),
        Facing::West => (false, x, y),
        Facing::East => (false, x + 1, y),
    }
}

fn line(out: &mut Vec<Segment>, a: [f32; 3], b: [f32; 3], kind: SegKind, rgb: (u8, u8, u8)) {
    out.push(Segment { a, b, kind, rgb });
}

/// The per-cell lines: the floor grid's north and west sides (the far sides come from the
/// neighbours, or from the walls that close the map), a block's box, a barrier's cross, and the
/// ceiling grid.
#[allow(clippy::too_many_arguments)]
fn cells(
    out: &mut Vec<Segment>,
    x: u16,
    y: u16,
    rgb: (u8, u8, u8),
    passable: bool,
    block: bool,
    ceiling: bool,
    h: f32,
) {
    let (x0, z0) = (f32::from(x), f32::from(y));
    let (x1, z1) = (x0 + 1.0, z0 + 1.0);
    line(out, [x0, 0.0, z0], [x1, 0.0, z0], SegKind::Floor, rgb);
    line(out, [x0, 0.0, z0], [x0, 0.0, z1], SegKind::Floor, rgb);
    if ceiling && !block {
        line(out, [x0, h, z0], [x1, h, z0], SegKind::Ceiling, rgb);
        line(out, [x0, h, z0], [x0, h, z1], SegKind::Ceiling, rgb);
    }
    if block {
        let i = 0.1;
        boxed(out, [x0 + i, z0 + i], [x1 - i, z1 - i], h, rgb);
    } else if !passable {
        line(out, [x0, 0.0, z0], [x1, 0.0, z1], SegKind::Barrier, rgb);
        line(out, [x1, 0.0, z0], [x0, 0.0, z1], SegKind::Barrier, rgb);
    }
}

/// The twelve edges of a box over the ground rectangle `lo..hi`.
fn boxed(out: &mut Vec<Segment>, lo: [f32; 2], hi: [f32; 2], h: f32, rgb: (u8, u8, u8)) {
    let corners = [
        [lo[0], lo[1]],
        [hi[0], lo[1]],
        [hi[0], hi[1]],
        [lo[0], hi[1]],
    ];
    for i in 0..4 {
        let (p, q) = (corners[i], corners[(i + 1) % 4]);
        line(
            out,
            [p[0], 0.0, p[1]],
            [q[0], 0.0, q[1]],
            SegKind::Block,
            rgb,
        );
        line(out, [p[0], h, p[1]], [q[0], h, q[1]], SegKind::Block, rgb);
        line(out, [p[0], 0.0, p[1]], [p[0], h, p[1]], SegKind::Block, rgb);
    }
}

/// A wall's rectangle, or a door: the lintel over a frame, crossed while closed.
fn edge(
    out: &mut Vec<Segment>,
    a: [f32; 2],
    b: [f32; 2],
    kind: SegKind,
    open: bool,
    rgb: (u8, u8, u8),
    h: f32,
) {
    let at = |t: f32, y: f32| [a[0] + (b[0] - a[0]) * t, y, a[1] + (b[1] - a[1]) * t];
    match kind {
        SegKind::Door => {
            let (l, r, top) = (0.2, 0.8, h * 0.85);
            line(out, at(0.0, h), at(1.0, h), SegKind::Wall, rgb);
            line(out, at(0.0, 0.0), at(0.0, h), SegKind::Wall, rgb);
            line(out, at(1.0, 0.0), at(1.0, h), SegKind::Wall, rgb);
            line(out, at(l, 0.0), at(l, top), kind, rgb);
            line(out, at(r, 0.0), at(r, top), kind, rgb);
            line(out, at(l, top), at(r, top), kind, rgb);
            if !open {
                line(out, at(l, 0.0), at(r, top), kind, rgb);
                line(out, at(r, 0.0), at(l, top), kind, rgb);
            }
        }
        _ => {
            line(out, at(0.0, 0.0), at(1.0, 0.0), kind, rgb);
            line(out, at(0.0, h), at(1.0, h), kind, rgb);
            line(out, at(0.0, 0.0), at(0.0, h), kind, rgb);
            line(out, at(1.0, 0.0), at(1.0, h), kind, rgb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_edges_have_one_key() {
        assert_eq!(key(3, 4, Facing::South), key(3, 5, Facing::North));
        assert_eq!(key(3, 4, Facing::East), key(4, 4, Facing::West));
        assert_ne!(key(3, 4, Facing::North), key(3, 4, Facing::West));
    }

    #[test]
    fn a_closed_door_is_crossed_and_an_open_one_is_not() {
        let mut closed = Vec::new();
        edge(
            &mut closed,
            [0.0, 0.0],
            [1.0, 0.0],
            SegKind::Door,
            false,
            (0, 0, 0),
            1.0,
        );
        let mut open = Vec::new();
        edge(
            &mut open,
            [0.0, 0.0],
            [1.0, 0.0],
            SegKind::Door,
            true,
            (0, 0, 0),
            1.0,
        );
        assert_eq!(closed.len(), open.len() + 2);
    }

    #[test]
    fn a_box_has_twelve_edges() {
        let mut out = Vec::new();
        boxed(&mut out, [0.0, 0.0], [1.0, 1.0], 1.0, (0, 0, 0));
        assert_eq!(out.len(), 12);
    }
}
