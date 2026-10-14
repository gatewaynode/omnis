//! The room an object takes on the ground, in whole sixteenths of a cell (M8.5 step 2), shared
//! by the engine's step and sight checks and the client's collision mirror so the two cannot
//! drift. Cell `(x, y)` spans `16x..16x + 16` east and `16y..16y + 16` south; its centre is at
//! `16x + 8`. A look's eighths are two sixteenths each, so a width of `w` eighths reaches `w`
//! sixteenths either side of the object's centre.
//!
//! Boxes are open: two that only touch do not overlap, and a ray that only grazes an edge does
//! not pass through.

use omnis_core::Facing;

/// An axis-aligned box on the ground, in sixteenths of a cell, open on every side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footprint {
    /// West edge.
    pub x0: i32,
    /// North edge.
    pub y0: i32,
    /// East edge.
    pub x1: i32,
    /// South edge.
    pub y1: i32,
}

/// A fraction with a positive denominator: where along a ray something happens, 0 at its start
/// and 1 at its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Along {
    /// Numerator.
    pub num: i64,
    /// Denominator, above 0.
    pub den: i64,
}

impl Along {
    const START: Along = Along { num: 0, den: 1 };
    const END: Along = Along { num: 1, den: 1 };

    fn less(self, other: Along) -> bool {
        i128::from(self.num) * i128::from(other.den) < i128::from(other.num) * i128::from(self.den)
    }

    fn max(self, other: Along) -> Along {
        if self.less(other) { other } else { self }
    }

    fn min(self, other: Along) -> Along {
        if other.less(self) { other } else { self }
    }
}

/// Half the width of the path a step takes, in sixteenths: the path is half a cell wide.
pub const PATH_HALF_WIDTH: i32 = 4;

impl Footprint {
    /// The footprint of an object in cell `(x, y)` with its centre `pos` eighths east and south
    /// of the cell's centre. `width` runs across `facing` and `depth` along it.
    #[must_use]
    pub fn of(x: u16, y: u16, pos: (i8, i8), facing: Facing, width: u8, depth: u8) -> Self {
        let (ex, ey) = match facing {
            Facing::North | Facing::South => (i32::from(width), i32::from(depth)),
            Facing::East | Facing::West => (i32::from(depth), i32::from(width)),
        };
        let cx = 16 * i32::from(x) + 8 + 2 * i32::from(pos.0);
        let cy = 16 * i32::from(y) + 8 + 2 * i32::from(pos.1);
        Footprint {
            x0: cx - ex,
            y0: cy - ey,
            x1: cx + ex,
            y1: cy + ey,
        }
    }

    /// The path of a step out of cell `(x, y)` toward `heading`: from the cell's centre to the
    /// next cell's centre, `PATH_HALF_WIDTH` either side.
    #[must_use]
    pub fn path(x: u16, y: u16, heading: Facing) -> Self {
        let (cx, cy) = (16 * i32::from(x) + 8, 16 * i32::from(y) + 8);
        let w = PATH_HALF_WIDTH;
        let (x0, y0, x1, y1) = match heading {
            Facing::North => (cx - w, cy - 16, cx + w, cy),
            Facing::South => (cx - w, cy, cx + w, cy + 16),
            Facing::East => (cx, cy - w, cx + 16, cy + w),
            Facing::West => (cx - 16, cy - w, cx, cy + w),
        };
        Footprint { x0, y0, x1, y1 }
    }

    /// Whether two open boxes share any point.
    #[must_use]
    pub const fn overlaps(&self, other: &Footprint) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    /// Every cell the box reaches into, west to east within north to south. Cells may lie
    /// outside the map; the caller filters them.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32)> + use<> {
        let (cx0, cx1) = (self.x0.div_euclid(16), (self.x1 - 1).div_euclid(16));
        let (cy0, cy1) = (self.y0.div_euclid(16), (self.y1 - 1).div_euclid(16));
        (cy0..=cy1).flat_map(move |cy| (cx0..=cx1).map(move |cx| (cx, cy)))
    }

    /// Where the segment from `start` to `end` (sixteenths) is inside the box: the first and
    /// last points, as fractions of the way along, or `None` when it misses or only grazes.
    #[must_use]
    pub fn ray_span(&self, start: (i64, i64), end: (i64, i64)) -> Option<(Along, Along)> {
        let (mut enter, mut leave) = (Along::START, Along::END);
        let axes = [
            (
                i64::from(self.x0),
                i64::from(self.x1),
                start.0,
                end.0 - start.0,
            ),
            (
                i64::from(self.y0),
                i64::from(self.y1),
                start.1,
                end.1 - start.1,
            ),
        ];
        for (lo, hi, s, d) in axes {
            if d == 0 {
                if !(lo < s && s < hi) {
                    return None;
                }
                continue;
            }
            let (near, far) = if d > 0 {
                (
                    Along {
                        num: lo - s,
                        den: d,
                    },
                    Along {
                        num: hi - s,
                        den: d,
                    },
                )
            } else {
                (
                    Along {
                        num: s - hi,
                        den: -d,
                    },
                    Along {
                        num: s - lo,
                        den: -d,
                    },
                )
            };
            enter = enter.max(near);
            leave = leave.min(far);
        }
        enter.less(leave).then_some((enter, leave))
    }
}
