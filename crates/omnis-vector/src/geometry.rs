//! Map data to vector line segments (presentation-ARCHITECTURE.md §7.1), standing on the
//! [`Ground`] (§8.4).
//!
//! Every wall and door edge is emitted once, keyed by its canonical line: a horizontal edge at
//! row line `z` from `x` to `x + 1`, or a vertical edge at column line `x` from `z` to `z + 1`.
//! On a flat map every line is where it was before height was drawn (M8.5 step 4).

use crate::geom::wall_height;
use crate::ground::{Ground, NE, NW, Patch, SE, SW, joined};
use crate::sim::SimWorld;
use omnis_sim::omnis_core::{Facing, Level, MapId};
use omnis_sim::omnis_data::{Join, MapData};
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
    /// A cliff's rock face: hatching between the two rims.
    Cliff,
    /// A ramp's treads, lying on the ground across its edge.
    Ramp,
    /// A stair to the level above or below: treads climbing across its cell (M8.5 step 1b).
    Stair,
    /// A ladder or shaft: two rails and rungs up to the level above.
    Ladder,
    /// A hole: the cell's opening, hatched inward.
    Hole,
}

impl SegKind {
    /// Whether a line of this kind is drawn with the floor grid `grid` shown or hidden: the floor
    /// and ceiling grids are the setting's; walls, doors, terrain, cliffs, ramps and joins always
    /// show.
    #[must_use]
    pub const fn drawn(self, grid: bool) -> bool {
        grid || !matches!(self, SegKind::Floor | SegKind::Ceiling)
    }
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
pub(crate) type EdgeKey = (bool, u16, u16);

/// What stands on an edge: its kind, whether a door is open, the terrain colour, the height.
pub(crate) type EdgeLook = (SegKind, bool, (u8, u8, u8), f32);

/// Quarter cells: where a cliff is hatched along its edge.
const HATCH: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// How far a ramp's treads lie from its edge, into each cell.
const TREADS: [f32; 4] = [-0.375, -0.125, 0.125, 0.375];

/// The segments for a level of `map` on its `ground`, with doors drawn open or closed as
/// `world` has them. Tiles with no ground on the level draw nothing.
#[must_use]
pub fn extract(
    map: &MapData,
    map_id: MapId,
    level: Level,
    world: &SimWorld,
    ground: &Ground,
) -> Vec<Segment> {
    let mut out = Vec::new();
    for y in 0..map.def.height {
        for x in 0..map.def.width {
            let Some(cell) = map.cell(level, x, y) else {
                continue;
            };
            let terrain = map.terrain(cell);
            let h = wall_height(terrain);
            let patch = ground.patch(x, y).unwrap_or_default();
            cells(
                &mut out,
                (x, y),
                patch,
                terrain.color,
                (
                    terrain.passable,
                    terrain.block.is_some(),
                    terrain.ceiling.is_some(),
                ),
                h,
            );
            rims(&mut out, ground, (x, y), patch, terrain.color);
            if let Some(join) = terrain.join {
                let beyond = |up: bool| {
                    map.beyond(level, up)
                        .and_then(|l| map.cell(l, x, y))
                        .map(|c| cells_high(c.height))
                };
                joins(&mut out, (x, y), patch, join, beyond, terrain.color);
            }
        }
    }
    for (&(horizontal, col, row), &(kind, open, rgb, h)) in &edges(map, map_id, level, world) {
        let (x0, z0) = (f32::from(col), f32::from(row));
        let (x1, z1) = if horizontal {
            (x0 + 1.0, z0)
        } else {
            (x0, z0 + 1.0)
        };
        let (feet, top) = span(ground, (horizontal, col, row), h);
        edge(&mut out, [x0, z0], [x1, z1], kind, open, rgb, feet, top);
    }
    for (edge, split, rgb) in splits(map, level) {
        match split {
            Split::Cliff => cliff(&mut out, ground, edge, rgb),
            Split::Ramp => treads(&mut out, ground, edge, rgb),
        }
    }
    out
}

/// Every wall and door edge of a level, once: a door on either side wins over a wall on the
/// other, and an open door is as `world` has it.
pub(crate) fn edges(
    map: &MapData,
    map_id: MapId,
    level: Level,
    world: &SimWorld,
) -> BTreeMap<EdgeKey, EdgeLook> {
    let mut edges: BTreeMap<EdgeKey, EdgeLook> = BTreeMap::new();
    for y in 0..map.def.height {
        for x in 0..map.def.width {
            let Some(cell) = map.cell(level, x, y) else {
                continue;
            };
            let terrain = map.terrain(cell);
            let h = wall_height(terrain);
            for facing in Facing::ALL {
                let kind = if cell.doors.has(facing) {
                    SegKind::Door
                } else if cell.walls.has(facing) {
                    SegKind::Wall
                } else {
                    continue;
                };
                let open = world.door_open(map_id, level, x, y, facing);
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
    edges
}

/// A height in eighths of a cell, in cells.
fn cells_high(eighths: i16) -> f32 {
    f32::from(eighths) / 8.0
}

/// How far in from the cell's sides a stair's treads and a hole's opening sit.
pub(crate) const JOIN_INSET: f32 = 0.15;

/// A stair's foot and head between its `floor` and the floor it leads `to`: the flight always
/// climbs north to south, so a stair up and its stair down above draw the same flight.
fn flight(floor: f32, to: f32) -> [f32; 2] {
    [floor.min(to), floor.max(to)]
}

/// A join's marks on cell `(x, y)`, from its floor `p` toward the floor of the next level up
/// or down (`beyond(up)`), a wall's height when that level has none there:
/// - a stair: four treads across the cell, climbing north to south from the lower floor to the
///   higher, on two stringers;
/// - a ladder: two ladders crossed in the cell's middle, rails and a rung every quarter cell;
/// - a hole: the opening, a square in from the sides, with its corners hatched to the cell's.
fn joins(
    out: &mut Vec<Segment>,
    (x, y): (u16, u16),
    p: Patch,
    join: Join,
    beyond: impl Fn(bool) -> Option<f32>,
    rgb: (u8, u8, u8),
) {
    let (x0, z0) = (f32::from(x), f32::from(y));
    let (x1, z1) = (x0 + 1.0, z0 + 1.0);
    let floor = p.centre;
    match join {
        Join::Up | Join::Down => {
            let up = join == Join::Up;
            let to = beyond(up).unwrap_or(if up { floor + 1.0 } else { floor - 1.0 });
            let [lo, hi] = flight(floor, to);
            let (l, r) = (x0 + JOIN_INSET, x1 - JOIN_INSET);
            for k in 1u8..=4 {
                let t = f32::from(k) / 4.0;
                let (h, z) = (lo + (hi - lo) * t, z0 + t - 0.125);
                line(out, [l, h, z], [r, h, z], SegKind::Stair, rgb);
            }
            for side in [l, r] {
                line(out, [side, lo, z0], [side, hi, z1], SegKind::Stair, rgb);
            }
        }
        Join::Ladder => {
            // Two ladders crossed, so one reads face-on from any side.
            let top = beyond(true).unwrap_or(floor + 1.0).max(floor + 0.25);
            let (cx, cz) = (x0 + 0.5, z0 + 0.5);
            for ((ax, az), (bx, bz)) in [
                ((cx - 0.15, cz), (cx + 0.15, cz)),
                ((cx, cz - 0.15), (cx, cz + 0.15)),
            ] {
                line(out, [ax, floor, az], [ax, top, az], SegKind::Ladder, rgb);
                line(out, [bx, floor, bz], [bx, top, bz], SegKind::Ladder, rgb);
                let mut h = floor + 0.25;
                while h < top - 1e-3 {
                    line(out, [ax, h, az], [bx, h, bz], SegKind::Ladder, rgb);
                    h += 0.25;
                }
            }
        }
        Join::Hole => {
            let (lo, hi) = (
                [x0 + JOIN_INSET, z0 + JOIN_INSET],
                [x1 - JOIN_INSET, z1 - JOIN_INSET],
            );
            let inner = [
                [lo[0], lo[1]],
                [hi[0], lo[1]],
                [hi[0], hi[1]],
                [lo[0], hi[1]],
            ];
            let outer = [[x0, z0], [x1, z0], [x1, z1], [x0, z1]];
            for i in 0..4 {
                let (a, b) = (inner[i], inner[(i + 1) % 4]);
                line(
                    out,
                    [a[0], floor, a[1]],
                    [b[0], floor, b[1]],
                    SegKind::Hole,
                    rgb,
                );
                let o = outer[i];
                line(
                    out,
                    [o[0], floor, o[1]],
                    [a[0], floor, a[1]],
                    SegKind::Hole,
                    rgb,
                );
            }
        }
    }
}

pub(crate) fn key(x: u16, y: u16, facing: Facing) -> EdgeKey {
    match facing {
        Facing::North => (true, x, y),
        Facing::South => (true, x, y + 1),
        Facing::West => (false, x, y),
        Facing::East => (false, x + 1, y),
    }
}

/// The surfaces on either side of an edge: north then south, or west then east, with the side of
/// each that lies on the edge.
pub(crate) fn sides(ground: &Ground, (horizontal, col, row): EdgeKey) -> [Option<[f32; 2]>; 2] {
    let (before, after, toward) = if horizontal {
        (
            row.checked_sub(1).and_then(|r| ground.patch(col, r)),
            ground.patch(col, row),
            Facing::South,
        )
    } else {
        (
            col.checked_sub(1).and_then(|c| ground.patch(c, row)),
            ground.patch(col, row),
            Facing::East,
        )
    };
    [
        before.map(|p| p.side(toward)),
        after.map(|p| p.side(toward.opposite())),
    ]
}

/// A wall's feet at each end (the lower ground there) and its top (a wall above the higher
/// cell, as the engine's sight has it).
pub(crate) fn span(ground: &Ground, edge: EdgeKey, h: f32) -> ([f32; 2], f32) {
    let (horizontal, col, row) = edge;
    let centres = if horizontal {
        [
            row.checked_sub(1).and_then(|r| ground.patch(col, r)),
            ground.patch(col, row),
        ]
    } else {
        [
            col.checked_sub(1).and_then(|c| ground.patch(c, row)),
            ground.patch(col, row),
        ]
    }
    .map(|p| p.map(|p| p.centre));
    let top = centres.into_iter().flatten().fold(f32::MIN, f32::max);
    let mut feet = [f32::MAX; 2];
    for side in sides(ground, edge).into_iter().flatten() {
        feet = [feet[0].min(side[0]), feet[1].min(side[1])];
    }
    if top == f32::MIN {
        return ([0.0; 2], h);
    }
    (feet, top + h)
}

fn line(out: &mut Vec<Segment>, a: [f32; 3], b: [f32; 3], kind: SegKind, rgb: (u8, u8, u8)) {
    out.push(Segment { a, b, kind, rgb });
}

/// The per-cell lines: the floor grid's north and west sides (the far sides come from the
/// neighbours, or from the walls that close the map), a block's box, a barrier's cross, and the
/// ceiling grid. `(passable, block, ceiling)` are the terrain's.
fn cells(
    out: &mut Vec<Segment>,
    (x, y): (u16, u16),
    p: Patch,
    rgb: (u8, u8, u8),
    (passable, block, ceiling): (bool, bool, bool),
    h: f32,
) {
    let (x0, z0) = (f32::from(x), f32::from(y));
    let (x1, z1) = (x0 + 1.0, z0 + 1.0);
    let c = p.corners;
    line(out, [x0, c[NW], z0], [x1, c[NE], z0], SegKind::Floor, rgb);
    line(out, [x0, c[NW], z0], [x0, c[SW], z1], SegKind::Floor, rgb);
    let roof = p.centre + h;
    if ceiling && !block {
        line(out, [x0, roof, z0], [x1, roof, z0], SegKind::Ceiling, rgb);
        line(out, [x0, roof, z0], [x0, roof, z1], SegKind::Ceiling, rgb);
    }
    if block {
        let i = 0.1;
        boxed(out, [x0 + i, z0 + i], [x1 - i, z1 - i], p.centre, roof, rgb);
    } else if !passable {
        line(out, [x0, c[NW], z0], [x1, c[SE], z1], SegKind::Barrier, rgb);
        line(out, [x1, c[NE], z0], [x0, c[SW], z1], SegKind::Barrier, rgb);
    }
}

/// The south and east sides of a cell whose neighbour there meets it at other heights: the
/// neighbour draws its own north or west side, so both rims of a split show.
fn rims(out: &mut Vec<Segment>, ground: &Ground, (x, y): (u16, u16), p: Patch, rgb: (u8, u8, u8)) {
    let (x0, z0) = (f32::from(x), f32::from(y));
    let (x1, z1) = (x0 + 1.0, z0 + 1.0);
    let c = p.corners;
    if ground
        .patch(x, y + 1)
        .is_some_and(|n| n.side(Facing::North) != p.side(Facing::South))
    {
        line(out, [x0, c[SW], z1], [x1, c[SE], z1], SegKind::Floor, rgb);
    }
    if ground
        .patch(x + 1, y)
        .is_some_and(|n| n.side(Facing::West) != p.side(Facing::East))
    {
        line(out, [x1, c[NE], z0], [x1, c[SE], z1], SegKind::Floor, rgb);
    }
}

/// What lies on an open edge between two heights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Split {
    /// A rock face between the two rims.
    Cliff,
    /// A ramp, walked across.
    Ramp,
}

/// The cliffs and ramps of a level, once per edge (each cell's east and south sides), with the
/// colour of the higher cell's terrain.
pub(crate) fn splits(map: &MapData, level: Level) -> Vec<(EdgeKey, Split, (u8, u8, u8))> {
    let mut out = Vec::new();
    for y in 0..map.def.height {
        for x in 0..map.def.width {
            let Some(cell) = map.cell(level, x, y) else {
                continue;
            };
            for facing in [Facing::East, Facing::South] {
                let (nx, ny) = if facing == Facing::East {
                    (x + 1, y)
                } else {
                    (x, y + 1)
                };
                let Some(next) = map.cell(level, nx, ny) else {
                    continue;
                };
                let back = facing.opposite();
                let closed = cell.walls.has(facing)
                    || next.walls.has(back)
                    || cell.doors.has(facing)
                    || next.doors.has(back);
                let edge = key(x, y, facing);
                let rgb = if next.height > cell.height {
                    map.terrain(next).color
                } else {
                    map.terrain(cell).color
                };
                if !closed && !joined(map, level, x, y, facing) {
                    out.push((edge, Split::Cliff, rgb));
                } else if cell.ramps.has(facing) || next.ramps.has(back) {
                    out.push((edge, Split::Ramp, rgb));
                }
            }
        }
    }
    out
}

/// The ends of a canonical edge on the ground plane.
pub(crate) fn ends((horizontal, col, row): EdgeKey) -> ([f32; 2], [f32; 2]) {
    let (x0, z0) = (f32::from(col), f32::from(row));
    let far = if horizontal {
        [x0 + 1.0, z0]
    } else {
        [x0, z0 + 1.0]
    };
    ([x0, z0], far)
}

/// A rock face: verticals every quarter cell between the two rims.
fn cliff(out: &mut Vec<Segment>, ground: &Ground, edge: EdgeKey, rgb: (u8, u8, u8)) {
    let [Some(near), Some(far)] = sides(ground, edge) else {
        return;
    };
    let (a, b) = ends(edge);
    let lerp = |s: [f32; 2], t: f32| s[0] + (s[1] - s[0]) * t;
    for t in HATCH {
        let (lo, hi) = (lerp(near, t), lerp(far, t));
        if (hi - lo).abs() < 1e-4 {
            continue;
        }
        let (px, pz) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
        line(out, [px, lo, pz], [px, hi, pz], SegKind::Cliff, rgb);
    }
}

/// A ramp's treads: lines along the edge, a quarter cell apart, lying on the ground.
fn treads(out: &mut Vec<Segment>, ground: &Ground, edge: EdgeKey, rgb: (u8, u8, u8)) {
    let (a, b) = ends(edge);
    let normal = if edge.0 { [0.0, 1.0] } else { [1.0, 0.0] };
    for d in TREADS {
        let at = |t: f32| {
            let x = a[0] + (b[0] - a[0]) * t + normal[0] * d;
            let z = a[1] + (b[1] - a[1]) * t + normal[1] * d;
            [x, ground.height(x, z), z]
        };
        for t in [0.0, 0.25, 0.5, 0.75] {
            line(out, at(t), at(t + 0.25), SegKind::Ramp, rgb);
        }
    }
}

/// The twelve edges of a box over the ground rectangle `lo..hi`, from `base` to `top`.
fn boxed(
    out: &mut Vec<Segment>,
    lo: [f32; 2],
    hi: [f32; 2],
    base: f32,
    top: f32,
    rgb: (u8, u8, u8),
) {
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
            [p[0], base, p[1]],
            [q[0], base, q[1]],
            SegKind::Block,
            rgb,
        );
        line(
            out,
            [p[0], top, p[1]],
            [q[0], top, q[1]],
            SegKind::Block,
            rgb,
        );
        line(
            out,
            [p[0], base, p[1]],
            [p[0], top, p[1]],
            SegKind::Block,
            rgb,
        );
    }
}

/// Where a door's opening lies along its edge.
pub(crate) const DOOR: [f32; 2] = [0.2, 0.8];

/// The height of a door's head: 85% of the way from its higher foot to the wall's top.
pub(crate) fn door_head(feet: [f32; 2], top: f32) -> f32 {
    let high = feet[0].max(feet[1]);
    high + (top - high) * 0.85
}

/// A wall's outline from its `feet` at each end up to `top`, or a door: the lintel over a frame,
/// crossed while closed.
#[allow(clippy::too_many_arguments)]
fn edge(
    out: &mut Vec<Segment>,
    a: [f32; 2],
    b: [f32; 2],
    kind: SegKind,
    open: bool,
    rgb: (u8, u8, u8),
    feet: [f32; 2],
    top: f32,
) {
    let at = |t: f32, y: f32| [a[0] + (b[0] - a[0]) * t, y, a[1] + (b[1] - a[1]) * t];
    let foot = |t: f32| feet[0] + (feet[1] - feet[0]) * t;
    match kind {
        SegKind::Door => {
            let ([l, r], head) = (DOOR, door_head(feet, top));
            line(out, at(0.0, top), at(1.0, top), SegKind::Wall, rgb);
            line(out, at(0.0, feet[0]), at(0.0, top), SegKind::Wall, rgb);
            line(out, at(1.0, feet[1]), at(1.0, top), SegKind::Wall, rgb);
            line(out, at(l, foot(l)), at(l, head), kind, rgb);
            line(out, at(r, foot(r)), at(r, head), kind, rgb);
            line(out, at(l, head), at(r, head), kind, rgb);
            if !open {
                line(out, at(l, foot(l)), at(r, head), kind, rgb);
                line(out, at(r, foot(r)), at(l, head), kind, rgb);
            }
        }
        _ => {
            line(out, at(0.0, feet[0]), at(1.0, feet[1]), kind, rgb);
            line(out, at(0.0, top), at(1.0, top), kind, rgb);
            line(out, at(0.0, feet[0]), at(0.0, top), kind, rgb);
            line(out, at(1.0, feet[1]), at(1.0, top), kind, rgb);
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
            [0.0; 2],
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
            [0.0; 2],
            1.0,
        );
        assert_eq!(closed.len(), open.len() + 2);
    }

    #[test]
    fn a_box_has_twelve_edges() {
        let mut out = Vec::new();
        boxed(&mut out, [0.0, 0.0], [1.0, 1.0], 0.0, 1.0, (0, 0, 0));
        assert_eq!(out.len(), 12);
    }
}
