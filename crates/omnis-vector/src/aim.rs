//! What the pointer is on in the 3D view (owner, 2026-11-12): a pile within the party's reach
//! can be clicked. No Bevy here: a point of the world is put on the screen through the eye
//! (where it is, its yaw and pitch, the vertical field of view), a pile's box becomes the
//! rectangle around its eight corners, and the pointer is on the pile when it is in that
//! rectangle, grown a little, or anywhere on the floor of the pile's cell, so a small heap is
//! not a small target. Only piles in reach are
//! asked about (the party's cell and the one it faces across an open edge), so nothing can
//! stand between and no wall is tested.

use crate::ground::Ground;
use crate::shapes::{Body, PILE_LOOK};
use omnis_sim::api::PileView;
use omnis_sim::omnis_core::Facing;

/// Pixels a pile's rectangle grows by on each side.
pub const MARGIN: f32 = 12.0;

/// Nearer than this in front of the eye, a point is not on the screen.
const NEAR: f32 = 0.05;

/// Where the view is seen from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eye {
    /// The eye, `[X, Y, Z]` in cells.
    pub at: [f32; 3],
    /// The body's yaw, radians; 0 looks north (−Z), positive turns left.
    pub yaw: f32,
    /// The head's pitch, radians, positive up.
    pub pitch: f32,
    /// The vertical field of view, radians.
    pub fov: f32,
}

/// Where a point of the world is on a screen `size` wide and high, in pixels from its top left;
/// `None` behind the eye.
#[must_use]
pub fn project(eye: &Eye, size: (f32, f32), point: [f32; 3]) -> Option<(f32, f32)> {
    let d = [
        point[0] - eye.at[0],
        point[1] - eye.at[1],
        point[2] - eye.at[2],
    ];
    let (sy, cy) = eye.yaw.sin_cos();
    let (sp, cp) = eye.pitch.sin_cos();
    // Into the eye's own axes: the yaw undone, then the pitch.
    let x = d[0] * cy - d[2] * sy;
    let z = d[0] * sy + d[2] * cy;
    let y = d[1] * cp + z * sp;
    let depth = d[1] * sp - z * cp;
    if depth < NEAR {
        return None;
    }
    let half = (eye.fov / 2.0).tan();
    let aspect = size.0 / size.1;
    Some((
        (x / (depth * half * aspect) + 1.0) / 2.0 * size.0,
        (1.0 - y / (depth * half)) / 2.0 * size.1,
    ))
}

/// The rectangle around a body on the screen, `[left, top, right, bottom]`; `None` when any
/// corner is behind the eye.
#[must_use]
pub fn rect(eye: &Eye, size: (f32, f32), body: &Body) -> Option<[f32; 4]> {
    let mut out = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for u in [-1.0, 1.0] {
        for v in [-1.0, 1.0] {
            for h in [0.0, 1.0] {
                let (x, y) = project(eye, size, body.at(u, v, h))?;
                out = [out[0].min(x), out[1].min(y), out[2].max(x), out[3].max(y)];
            }
        }
    }
    Some(out)
}

/// Where the floor of cell `(x, y)` is on the screen: its corners north-west, north-east,
/// south-east and south-west at the ground's heights; `None` when a corner is behind the eye,
/// as the party's own cell's always are, or the cell is off the map.
#[must_use]
pub fn floor(
    eye: &Eye,
    size: (f32, f32),
    ground: &Ground,
    x: u16,
    y: u16,
) -> Option<[(f32, f32); 4]> {
    let heights = ground.patch(x, y)?.corners;
    let (w, n) = (f32::from(x), f32::from(y));
    let at = [(w, n), (w + 1.0, n), (w + 1.0, n + 1.0), (w, n + 1.0)];
    let mut out = [(0.0, 0.0); 4];
    for i in 0..4 {
        out[i] = project(eye, size, [at[i].0, heights[i], at[i].1])?;
    }
    Some(out)
}

/// Whether a screen point is inside a convex quad, its corners in order either way round.
#[must_use]
pub fn inside(point: (f32, f32), quad: &[(f32, f32); 4]) -> bool {
    let side = |i: usize| {
        let (a, b) = (quad[i], quad[(i + 1) % 4]);
        (b.0 - a.0) * (point.1 - a.1) - (b.1 - a.1) * (point.0 - a.0)
    };
    let sides = [side(0), side(1), side(2), side(3)];
    sides.iter().all(|s| *s >= 0.0) || sides.iter().all(|s| *s <= 0.0)
}

/// Which of `piles` the screen point is on, as they are drawn on `ground`: the first whose
/// rectangle, grown by [`MARGIN`], holds it, or whose cell's floor does (owner, 2026-11-13: the
/// heap alone was too small a target).
#[must_use]
pub fn pile_under(
    point: (f32, f32),
    size: (f32, f32),
    eye: &Eye,
    piles: &[PileView],
    ground: &Ground,
) -> Option<usize> {
    piles.iter().position(|p| {
        let body = Body::of(p.x, p.y, p.pos, Facing::North, &PILE_LOOK, ground);
        on_heap(point, size, eye, &body)
            || floor(eye, size, ground, p.x, p.y).is_some_and(|quad| inside(point, &quad))
    })
}

/// Whether the screen point is in a body's rectangle grown by [`MARGIN`].
#[must_use]
pub fn on_heap(point: (f32, f32), size: (f32, f32), eye: &Eye, body: &Body) -> bool {
    rect(eye, size, body).is_some_and(|[l, t, r, b]| {
        point.0 >= l - MARGIN
            && point.0 <= r + MARGIN
            && point.1 >= t - MARGIN
            && point.1 <= b + MARGIN
    })
}
