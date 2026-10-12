//! The dark solid surfaces under the lines (presentation-ARCHITECTURE.md §8.2, M8.5 step 3):
//! triangles in the same world space as [`crate::geometry`]'s segments, so a wall, a hill or a
//! floor hides what lies behind it. Fills carry no colour; the renderer draws them all the
//! clear colour. Bevy-free, so it is tested directly.
//!
//! Where a level joins the next, the fills open: a stair or ladder up has no ceiling, a stair
//! or ladder down no floor, and a hole's middle is open, so the way through can be seen.
//! Stairs and ladders stay lines: a stair's flight has no direction in the map yet, so a solid
//! slab would face half its walkers with its underside.

use crate::geom::wall_height;
use crate::geometry::{
    DOOR, EdgeKey, JOIN_INSET, SegKind, Split, door_head, edges, ends, sides, span, splits,
};
use crate::ground::{Ground, NE, NW, Patch, SE, SW};
use crate::sim::SimWorld;
use omnis_sim::omnis_core::{Level, MapId};
use omnis_sim::omnis_data::{Join, MapData};

/// One triangle in world space.
pub type Tri = [[f32; 3]; 3];

/// The fills of a level of `map` on its `ground`, with doors open or closed as `world` has
/// them. Tiles with no ground on the level fill nothing.
#[must_use]
pub fn fills(
    map: &MapData,
    map_id: MapId,
    level: Level,
    world: &SimWorld,
    ground: &Ground,
) -> Vec<Tri> {
    let mut out = Vec::new();
    let below = map.beyond(level, false);
    let above = map.beyond(level, true);
    for y in 0..map.def.height {
        for x in 0..map.def.width {
            let Some(cell) = map.cell(level, x, y) else {
                continue;
            };
            let terrain = map.terrain(cell);
            let p = ground.patch(x, y).unwrap_or_default();
            let through = |near: Option<Level>| near.and_then(|l| map.cell(l, x, y)).is_some();
            let (down, up) = match terrain.join {
                Some(Join::Down) => (true, false),
                Some(Join::Up) => (false, true),
                Some(Join::Ladder) => (through(below), through(above)),
                Some(Join::Hole) | None => (false, false),
            };
            match terrain.join {
                Some(Join::Hole) => ring(&mut out, (x, y), p.centre),
                _ if !down => floor(&mut out, (x, y), p),
                _ => {}
            }
            let roof = p.centre + wall_height(terrain);
            let (x0, z0) = (f32::from(x), f32::from(y));
            if terrain.block.is_some() {
                let i = 0.1;
                block(
                    &mut out,
                    [x0 + i, z0 + i],
                    [x0 + 1.0 - i, z0 + 1.0 - i],
                    p.centre,
                    roof,
                );
            } else if terrain.ceiling.is_some() && !up {
                let c = |dx: f32, dz: f32| [x0 + dx, roof, z0 + dz];
                quad(
                    &mut out,
                    [c(0.0, 0.0), c(1.0, 0.0), c(1.0, 1.0), c(0.0, 1.0)],
                );
            }
        }
    }
    for (&edge, &(kind, open, _, h)) in &edges(map, map_id, level, world) {
        let (feet, top) = span(ground, edge, h);
        wall(&mut out, edge, kind == SegKind::Door && open, feet, top);
    }
    for (edge, split, _) in splits(map, level) {
        if split == Split::Cliff {
            cliff(&mut out, ground, edge);
        }
    }
    out
}

/// Two triangles over the quad `a b c d`, in order round it.
fn quad(out: &mut Vec<Tri>, [a, b, c, d]: [[f32; 3]; 4]) {
    out.push([a, b, c]);
    out.push([a, c, d]);
}

/// A cell's ground: four triangles from its centre to each side's corners, as
/// [`Ground::height`] samples it.
fn floor(out: &mut Vec<Tri>, (x, y): (u16, u16), p: Patch) {
    let (x0, z0) = (f32::from(x), f32::from(y));
    let (x1, z1) = (x0 + 1.0, z0 + 1.0);
    let c = p.corners;
    let centre = [x0 + 0.5, p.centre, z0 + 0.5];
    let (nw, ne, se, sw) = (
        [x0, c[NW], z0],
        [x1, c[NE], z0],
        [x1, c[SE], z1],
        [x0, c[SW], z1],
    );
    out.extend([
        [centre, nw, ne],
        [centre, ne, se],
        [centre, se, sw],
        [centre, sw, nw],
    ]);
}

/// A hole's rim: the ground between the cell's sides and the opening, at the hole's floor.
fn ring(out: &mut Vec<Tri>, (x, y): (u16, u16), floor: f32) {
    let (x0, z0) = (f32::from(x), f32::from(y));
    let at = |dx: f32, dz: f32| [x0 + dx, floor, z0 + dz];
    let (i, o) = (JOIN_INSET, 1.0 - JOIN_INSET);
    let outer = [at(0.0, 0.0), at(1.0, 0.0), at(1.0, 1.0), at(0.0, 1.0)];
    let inner = [at(i, i), at(o, i), at(o, o), at(i, o)];
    for k in 0..4 {
        let n = (k + 1) % 4;
        quad(out, [outer[k], outer[n], inner[n], inner[k]]);
    }
}

/// A block's four sides and its top over the ground rectangle `lo..hi`, from `base` to `top`.
fn block(out: &mut Vec<Tri>, lo: [f32; 2], hi: [f32; 2], base: f32, top: f32) {
    let corners = [
        [lo[0], lo[1]],
        [hi[0], lo[1]],
        [hi[0], hi[1]],
        [lo[0], hi[1]],
    ];
    for k in 0..4 {
        let (p, q) = (corners[k], corners[(k + 1) % 4]);
        quad(
            out,
            [
                [p[0], base, p[1]],
                [q[0], base, q[1]],
                [q[0], top, q[1]],
                [p[0], top, p[1]],
            ],
        );
    }
    let t = corners.map(|c| [c[0], top, c[1]]);
    quad(out, t);
}

/// A wall's face from its `feet` at each end up to `top`; an open door leaves its opening, the
/// frame up to the head.
fn wall(out: &mut Vec<Tri>, edge: EdgeKey, open: bool, feet: [f32; 2], top: f32) {
    let (a, b) = ends(edge);
    let at = |t: f32, y: f32| [a[0] + (b[0] - a[0]) * t, y, a[1] + (b[1] - a[1]) * t];
    let foot = |t: f32| feet[0] + (feet[1] - feet[0]) * t;
    let strip = |out: &mut Vec<Tri>, t0: f32, t1: f32| {
        quad(
            out,
            [at(t0, foot(t0)), at(t1, foot(t1)), at(t1, top), at(t0, top)],
        );
    };
    if open {
        let [l, r] = DOOR;
        let head = door_head(feet, top);
        strip(out, 0.0, l);
        strip(out, r, 1.0);
        quad(out, [at(l, head), at(r, head), at(r, top), at(l, top)]);
    } else {
        strip(out, 0.0, 1.0);
    }
}

/// A cliff's rock face between its two rims.
fn cliff(out: &mut Vec<Tri>, ground: &Ground, edge: EdgeKey) {
    let [Some(near), Some(far)] = sides(ground, edge) else {
        return;
    };
    if (near[0] - far[0]).abs() < 1e-4 && (near[1] - far[1]).abs() < 1e-4 {
        return;
    }
    let (a, b) = ends(edge);
    quad(
        out,
        [
            [a[0], near[0], a[1]],
            [b[0], near[1], b[1]],
            [b[0], far[1], b[1]],
            [a[0], far[0], a[1]],
        ],
    );
}
