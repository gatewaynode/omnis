//! The ground the view stands on (presentation-ARCHITECTURE.md §8.4, M8.5 step 4).
//!
//! Each cell's centre is at its authored height (`Cell.height` in eighths, so a wall's 1.0 is 8).
//! Its four corners are shared with the cells it is joined to: across an edge that is neither a
//! wall nor a cliff either way, so the ground runs smooth where the party can walk and splits
//! where it cannot. A cell's surface is four triangles from its centre to each side's corners.
//! Relief noise is not here yet (step 5).

use crate::geom::EYE_HEIGHT;
use crate::pose::Pose;
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::MapData;

/// The corners of a cell, in the order of [`Patch::corners`].
pub const NW: usize = 0;
/// North-east.
pub const NE: usize = 1;
/// South-east.
pub const SE: usize = 2;
/// South-west.
pub const SW: usize = 3;

/// One cell's surface.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Patch {
    /// The height at the centre: the cell's own.
    pub centre: f32,
    /// The heights at the corners: north-west, north-east, south-east, south-west.
    pub corners: [f32; 4],
}

impl Patch {
    /// The two corners of the side toward `facing`, west to east or north to south.
    #[must_use]
    pub const fn side(&self, facing: Facing) -> [f32; 2] {
        let c = self.corners;
        match facing {
            Facing::North => [c[NW], c[NE]],
            Facing::South => [c[SW], c[SE]],
            Facing::West => [c[NW], c[SW]],
            Facing::East => [c[NE], c[SE]],
        }
    }
}

/// The ground of one map.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ground {
    width: u16,
    height: u16,
    patches: Vec<Patch>,
    /// Whether each tile has ground on the level; one without has no patch.
    present: Vec<bool>,
}

/// A height in eighths of a cell, in cells.
fn cells(eighths: i16) -> f32 {
    f32::from(eighths) / 8.0
}

/// Whether the ground of `level` runs on from `(x, y)` across its side toward `facing`: there
/// is a cell beyond, and the edge is no wall and no cliff from either side. Doors and ramps
/// join.
#[must_use]
pub fn joined(map: &MapData, level: Level, x: u16, y: u16, facing: Facing) -> bool {
    let Some(here) = map.cell(level, x, y) else {
        return false;
    };
    let (dx, dy) = facing.delta();
    let (Ok(nx), Ok(ny)) = (
        u16::try_from(i32::from(x) + dx),
        u16::try_from(i32::from(y) + dy),
    ) else {
        return false;
    };
    let Some(there) = map.cell(level, nx, ny) else {
        return false;
    };
    let back = facing.opposite();
    !(here.walls.has(facing)
        || there.walls.has(back)
        || here.cliffs.has(facing)
        || there.cliffs.has(back))
}

impl Ground {
    /// The ground of a level of `map`; a tile with no ground there has no patch, so nothing
    /// stands on it and no wall beside it reaches down to it.
    #[must_use]
    pub fn from_map(map: &MapData, level: Level) -> Ground {
        let (width, height) = (map.def.width, map.def.height);
        let mut patches: Vec<Patch> = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| {
                let h = map.cell(level, x, y).map_or(0.0, |c| cells(c.height));
                Patch {
                    centre: h,
                    corners: [h; 4],
                }
            })
            .collect();
        for cy in 0..=height {
            for cx in 0..=width {
                Self::corner(map, level, &mut patches, cx, cy);
            }
        }
        let present = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| map.cell(level, x, y).is_some())
            .collect();
        Ground {
            width,
            height,
            patches,
            present,
        }
    }

    /// Settle the grid corner `(cx, cy)`: the cells around it, grouped by the edges that join
    /// them, each take their group's mean height there.
    fn corner(map: &MapData, level: Level, patches: &mut [Patch], cx: u16, cy: u16) {
        let width = usize::from(map.def.width);
        // The four cells around the corner and which of their corners it is: north-west,
        // north-east, south-east, south-west of the point.
        let around: [(Option<(u16, u16)>, usize); 4] = [
            (cx.checked_sub(1).zip(cy.checked_sub(1)), SE),
            (Some(cx).zip(cy.checked_sub(1)), SW),
            (Some((cx, cy)), NW),
            (cx.checked_sub(1).zip(Some(cy)), NE),
        ];
        let at = |i: usize| {
            around[i]
                .0
                .filter(|&(x, y)| x < map.def.width && y < map.def.height)
        };
        // The edges between neighbours around the point, as pairs of `around` indices.
        let links = [
            (0, 1, Facing::East),
            (3, 2, Facing::East),
            (0, 3, Facing::South),
            (1, 2, Facing::South),
        ];
        let mut group = [0, 1, 2, 3];
        let root = |group: &[usize; 4], mut i: usize| {
            while group[i] != i {
                i = group[i];
            }
            i
        };
        for (a, b, facing) in links {
            if let (Some((x, y)), Some(_)) = (at(a), at(b))
                && joined(map, level, x, y, facing)
            {
                let (ra, rb) = (root(&group, a), root(&group, b));
                group[ra.max(rb)] = ra.min(rb);
            }
        }
        let index = |(x, y): (u16, u16)| usize::from(y) * width + usize::from(x);
        for (i, &(_, which)) in around.iter().enumerate() {
            let Some(cell) = at(i) else { continue };
            let r = root(&group, i);
            let (mut sum, mut n) = (0.0, 0.0);
            for j in 0..4 {
                if let Some(other) = at(j)
                    && root(&group, j) == r
                {
                    sum += patches[index(other)].centre;
                    n += 1.0;
                }
            }
            patches[index(cell)].corners[which] = sum / n;
        }
    }

    /// The surface of cell `(x, y)`, if it is on the map and has ground on the level.
    #[must_use]
    pub fn patch(&self, x: u16, y: u16) -> Option<Patch> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = usize::from(y) * usize::from(self.width) + usize::from(x);
        self.present
            .get(i)
            .copied()
            .unwrap_or(false)
            .then(|| self.patches.get(i).copied())
            .flatten()
    }

    /// The ground's crests seen from `eye`: each edge of its triangles where the surface turns
    /// from facing the eye to facing away, within `reach` of it across the ground.
    #[must_use]
    pub fn crests(&self, eye: [f32; 3], reach: f32) -> Vec<([f32; 3], [f32; 3])> {
        let mut out = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let Some(p) = self.patch(x, y) else { continue };
                let (fx, fz) = (f32::from(x), f32::from(y));
                if (fx + 0.5 - eye[0]).hypot(fz + 0.5 - eye[2]) > reach {
                    continue;
                }
                let at = |(u, v): (f32, f32), h: f32| [fx + u, h, fz + v];
                let c = at((0.5, 0.5), p.centre);
                let k = CORNERS.map(|(uv, i)| at(uv, p.corners[i]));
                // The triangles toward north, east, south and west, each from the centre.
                let faces = |i: usize| facing(c, k[(i + 3) % 4], k[i], eye);
                let sides = [0, 1, 2, 3].map(|i| faces((i + 1) % 4));
                // A diagonal from the centre to a corner lies between the triangles either side.
                for i in 0..4 {
                    if sides[(i + 3) % 4] != sides[i] {
                        out.push((c, k[i]));
                    }
                }
                // A side shared with the next cell east or south, where the ground runs on.
                for (dir, (a, b), there) in [
                    (Facing::East, (NE, SE), Facing::West),
                    (Facing::South, (SE, SW), Facing::North),
                ] {
                    let (nx, ny) = if dir == Facing::East {
                        (x + 1, y)
                    } else {
                        (x, y + 1)
                    };
                    let Some(q) = self.patch(nx, ny) else {
                        continue;
                    };
                    if p.side(dir) != q.side(there) {
                        continue;
                    }
                    let mine = sides[dir as usize];
                    let (gx, gz) = (f32::from(nx), f32::from(ny));
                    let theirs = facing([gx + 0.5, q.centre, gz + 0.5], k[a], k[b], eye);
                    if mine != theirs {
                        out.push((k[a], k[b]));
                    }
                }
            }
        }
        out
    }

    /// The ground's height at the point `(x, z)`; off the map, the nearest cell's surface.
    #[must_use]
    pub fn height(&self, x: f32, z: f32) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 0.0;
        }
        let clamp = |v: f32, n: u16| v.clamp(0.0, f32::from(n) - 1e-4);
        let (x, z) = (clamp(x, self.width), clamp(z, self.height));
        // In range after the clamp.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (cx, cy) = (x.floor() as u16, z.floor() as u16);
        let Some(p) = self.patch(cx, cy) else {
            return 0.0;
        };
        let (u, v) = (x - f32::from(cx), z - f32::from(cy));
        let c = p.corners;
        // The triangle from the centre to the side the point is nearest.
        let (a, b, ha, hb) = if v <= u && v <= 1.0 - u {
            ((0.0, 0.0), (1.0, 0.0), c[NW], c[NE])
        } else if v >= u && v >= 1.0 - u {
            ((0.0, 1.0), (1.0, 1.0), c[SW], c[SE])
        } else if u < v {
            ((0.0, 0.0), (0.0, 1.0), c[NW], c[SW])
        } else {
            ((1.0, 0.0), (1.0, 1.0), c[NE], c[SE])
        };
        barycentric((u, v), (0.5, 0.5), a, b, (p.centre, ha, hb))
    }
}

/// A cell's corners in order, north-west, north-east, south-east, south-west: where each lies
/// in the cell and its index in [`Patch::corners`].
const CORNERS: [((f32, f32), usize); 4] = [
    ((0.0, 0.0), NW),
    ((1.0, 0.0), NE),
    ((1.0, 1.0), SE),
    ((0.0, 1.0), SW),
];

/// Whether the triangle `o, a, b`'s upper side faces `eye`.
fn facing(o: [f32; 3], a: [f32; 3], b: [f32; 3], eye: [f32; 3]) -> bool {
    let (u, v) = (sub(a, o), sub(b, o));
    let mut n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    if n[1] < 0.0 {
        n = n.map(|c| -c);
    }
    let e = sub(eye, o);
    n[0] * e[0] + n[1] * e[1] + n[2] * e[2] > 0.0
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The height at `p` in the triangle `o, a, b` with heights `h`.
fn barycentric(
    p: (f32, f32),
    o: (f32, f32),
    a: (f32, f32),
    b: (f32, f32),
    h: (f32, f32, f32),
) -> f32 {
    let det = (a.1 - b.1) * (o.0 - b.0) + (b.0 - a.0) * (o.1 - b.1);
    let wo = ((a.1 - b.1) * (p.0 - b.0) + (b.0 - a.0) * (p.1 - b.1)) / det;
    let wa = ((b.1 - o.1) * (p.0 - b.0) + (o.0 - b.0) * (p.1 - b.1)) / det;
    let wb = 1.0 - wo - wa;
    wo * h.0 + wa * h.1 + wb * h.2
}

/// Where the camera's eye belongs: the ground under the pose plus the eye height.
#[must_use]
pub fn eye(ground: &Ground, pose: Pose) -> f32 {
    ground.height(pose.x, pose.z) + EYE_HEIGHT
}

/// How long the eye takes to close most of a change in height, in seconds.
const EYE_EASE: f32 = 0.1;

/// The camera's height moved from `now` toward `target` over `dt` seconds: eased, or at once
/// under reduced motion (`snap`) and for a jump of more than a wall (a portal, a teleport).
#[must_use]
pub fn ease_eye(now: f32, target: f32, dt: f32, snap: bool) -> f32 {
    if snap || (target - now).abs() > 1.0 {
        return target;
    }
    now + (target - now) * (1.0 - (-dt / EYE_EASE).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_triangle_gives_its_corners_and_centre() {
        let h = (2.0, 0.0, 4.0);
        let tri = |p| barycentric(p, (0.5, 0.5), (0.0, 0.0), (1.0, 0.0), h);
        assert!((tri((0.5, 0.5)) - 2.0).abs() < 1e-6);
        assert!((tri((0.0, 0.0)) - 0.0).abs() < 1e-6);
        assert!((tri((1.0, 0.0)) - 4.0).abs() < 1e-6);
        assert!((tri((0.5, 0.0)) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn the_eye_eases_and_snaps() {
        let eased = ease_eye(0.5, 1.0, 1.0 / 60.0, false);
        assert!(eased > 0.5 && eased < 1.0, "part of the way: {eased}");
        let mut y = 0.5;
        for _ in 0..120 {
            y = ease_eye(y, 1.0, 1.0 / 60.0, false);
        }
        assert!((y - 1.0).abs() < 1e-3, "settles: {y}");
        assert_eq!(ease_eye(0.5, 1.0, 1.0 / 60.0, true), 1.0);
        assert_eq!(ease_eye(0.5, 2.5, 1.0 / 60.0, false), 2.5);
    }
}
