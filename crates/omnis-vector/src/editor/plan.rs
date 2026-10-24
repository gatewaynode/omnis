//! The plan view (M5.4, `tasks/plans/m5-editor-v1.md` §Client side): one level of the open
//! map from above, as pixels, at a whole number of pixels a cell. Only the cells in the pane
//! are painted, so a map at the largest size never makes a picture larger than the pane. A
//! click's pixel maps back to its cell (`hit`). Bevy-free.

use super::brush::Target;
use crate::minimap::{BACKGROUND, CLIFF, DOOR, HOLE, JOIN, RAMP, WALL};
use crate::raster::{Raster, edge_key, edge_pixels, join_pixels, line_pixels, tick_pixels};
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::{EdgeKind, Grid};
use omnis_sim::omnis_data::{Join, MapData, Terrain};

/// The zooms, in pixels a cell.
pub const ZOOMS: [u32; 6] = [4, 8, 12, 16, 24, 32];

/// Where the pane looks: the cell at its top left (which may lie before the map, so a map
/// smaller than the pane can be centred) and the zoom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    /// The map cell at the pane's top-left pixel; negative is before the map.
    pub origin: (i64, i64),
    /// An index into [`ZOOMS`].
    pub zoom: usize,
}

impl Default for View {
    fn default() -> View {
        View {
            origin: (0, 0),
            zoom: 3,
        }
    }
}

impl View {
    /// Pixels a cell.
    #[must_use]
    pub fn scale(self) -> u32 {
        ZOOMS[self.zoom.min(ZOOMS.len() - 1)]
    }

    /// One zoom in, keeping the cell at the pane's centre there.
    #[must_use]
    pub fn zoomed(self, by: i8, pane: (u32, u32)) -> View {
        let next = usize::try_from(i64::try_from(self.zoom).unwrap_or(0) + i64::from(by))
            .unwrap_or(0)
            .min(ZOOMS.len() - 1);
        if next == self.zoom {
            return self;
        }
        let (cx, cy) = self.centre_cell(pane);
        let mut out = View { zoom: next, ..self };
        out.origin = (
            cx - i64::from(pane.0 / out.scale() / 2),
            cy - i64::from(pane.1 / out.scale() / 2),
        );
        out
    }

    /// The cell at the pane's middle.
    #[must_use]
    pub fn centre_cell(self, pane: (u32, u32)) -> (i64, i64) {
        let s = self.scale();
        (
            self.origin.0 + i64::from(pane.0 / s / 2),
            self.origin.1 + i64::from(pane.1 / s / 2),
        )
    }

    /// The view with `cell` at the pane's middle.
    #[must_use]
    pub fn centred_on(self, cell: (u16, u16), pane: (u32, u32)) -> View {
        let s = self.scale();
        View {
            origin: (
                i64::from(cell.0) - i64::from(pane.0 / s / 2),
                i64::from(cell.1) - i64::from(pane.1 / s / 2),
            ),
            ..self
        }
    }

    /// The view that shows the whole map, `width × height` cells, as large as the pane allows,
    /// centred.
    #[must_use]
    pub fn fitted(size: (u16, u16), pane: (u32, u32)) -> View {
        let fits = |s: u32| u32::from(size.0) * s <= pane.0 && u32::from(size.1) * s <= pane.1;
        let zoom = ZOOMS.iter().rposition(|&s| fits(s)).unwrap_or(0);
        let s = ZOOMS[zoom];
        let spare = |px: u32, cells: u16| (i64::from(px / s) - i64::from(cells)) / 2;
        View {
            origin: (-spare(pane.0, size.0), -spare(pane.1, size.1)),
            zoom,
        }
    }

    /// The map cell under pixel `(px, py)` of the pane, when it lies on the map.
    #[must_use]
    pub fn hit(self, size: (u16, u16), px: f32, py: f32) -> Option<(u16, u16)> {
        if px < 0.0 || py < 0.0 {
            return None;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (col, row) = (
            self.origin.0 + (px / self.scale() as f32).floor() as i64,
            self.origin.1 + (py / self.scale() as f32).floor() as i64,
        );
        let (x, y) = (u16::try_from(col).ok()?, u16::try_from(row).ok()?);
        (x < size.0 && y < size.1).then_some((x, y))
    }

    /// What pixel `(px, py)` of the pane points at: the cell under it and, when the pixel
    /// lies within a quarter of the cell's side of one of its borders, that edge (the nearer
    /// at a corner). None off the map.
    #[must_use]
    pub fn target(self, size: (u16, u16), px: f32, py: f32) -> Option<Target> {
        self.aimed(size, px, py, 0.25)
    }

    /// What an edge brush at pixel `(px, py)` aims at: the cell under it and its nearest side,
    /// wherever in the cell the pixel lies (the first of North, East, South, West at a tie).
    /// None off the map.
    #[must_use]
    pub fn edge_target(self, size: (u16, u16), px: f32, py: f32) -> Option<Target> {
        self.aimed(size, px, py, 0.5)
    }

    /// The cell under the pixel and its nearest side when that side is within `band` of the
    /// cell's side away.
    fn aimed(self, size: (u16, u16), px: f32, py: f32, band: f32) -> Option<Target> {
        let cell = self.hit(size, px, py)?;
        let (left, top, side) = self.cell_rect(cell);
        #[allow(clippy::cast_precision_loss)]
        let (fx, fy, s) = (px - left as f32, py - top as f32, side as f32);
        let band = s * band;
        let edge = [
            (Facing::North, fy),
            (Facing::East, s - fx),
            (Facing::South, s - fy),
            (Facing::West, fx),
        ]
        .into_iter()
        .filter(|(_, d)| *d <= band)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(facing, _)| facing);
        Some(Target { cell, edge })
    }

    /// The pixel rectangle of `cell` in the pane, `(left, top, side)`, wherever it falls (a cell
    /// outside the pane has a rectangle outside it).
    #[must_use]
    pub fn cell_rect(self, cell: (u16, u16)) -> (i64, i64, u32) {
        let s = self.scale();
        (
            (i64::from(cell.0) - self.origin.0) * i64::from(s),
            (i64::from(cell.1) - self.origin.1) * i64::from(s),
            s,
        )
    }
}

/// What the plan is painted from.
#[derive(Clone, Copy)]
pub struct Source<'a> {
    /// The level's grid as the document holds it.
    pub grid: &'a Grid,
    /// The map's terrains, by the grid's index.
    pub terrains: &'a [Terrain],
    /// The level's size.
    pub size: (u16, u16),
    /// The level painted.
    pub level: Level,
    /// The map as the loader last took it, for what is derived (cliffs, a stair's rise); none
    /// while the map has never resolved.
    pub resolved: Option<&'a MapData>,
    /// The map's start, marked when it is on this level.
    pub start: Option<(u16, u16)>,
}

/// A terrain's fill, brighter the higher the tile stands: level ground at 60%, each wall's
/// height up adds 10 points, down takes them, within 25% to 100%.
#[must_use]
pub fn fill(rgb: (u8, u8, u8), height: i16) -> [u8; 4] {
    let k = (60 + i32::from(height) * 10 / 8).clamp(25, 100);
    let f = |c: u8| u8::try_from(i32::from(c) * k / 100).unwrap_or(u8::MAX);
    [f(rgb.0), f(rgb.1), f(rgb.2), 255]
}

/// No ground: air above, rock below.
pub const NO_GROUND: [u8; 4] = [8, 8, 12, 255];
/// Outside the map.
pub const OUTSIDE: [u8; 4] = [0, 0, 0, 255];
/// The map's start.
pub const START: [u8; 4] = [255, 255, 255, 255];

/// Paint the cells of `source` the pane shows under `view`: terrain fills shaded by height, no
/// ground, the map's edge, then ramps and cliffs, joins, walls and doors over them, and the
/// start's mark.
#[must_use]
pub fn paint(source: Source, view: View, pane: (u32, u32)) -> Raster {
    let s = view.scale();
    let mut out = Raster::filled(pane.0.max(1), pane.1.max(1), s, OUTSIDE);
    let cols = i64::from(pane.0 / s) + 1;
    let rows = i64::from(pane.1 / s) + 1;
    let (w, h) = (i64::from(source.size.0), i64::from(source.size.1));
    let cells = || {
        (0..rows).flat_map(move |r| {
            (0..cols).filter_map(move |c| {
                let (x, y) = (view.origin.0 + c, view.origin.1 + r);
                if x < 0 || y < 0 || x >= w || y >= h {
                    return None;
                }
                Some((c, r, u16::try_from(x).ok()?, u16::try_from(y).ok()?))
            })
        })
    };
    let si = i64::from(s);
    for (c, r, x, y) in cells() {
        let colour = match source.grid.tile(x, y) {
            Some(Some(t)) => source
                .terrains
                .get(usize::from(t))
                .map_or(BACKGROUND, |terrain| {
                    fill(terrain.color, source.grid.height_at(x, y).unwrap_or(0))
                }),
            Some(None) => NO_GROUND,
            None => OUTSIDE,
        };
        for dy in 0..si {
            for dx in 0..si {
                out.put(c * si + dx, r * si + dy, colour);
            }
        }
    }
    let size = (out.width, out.height);
    // Ramps and cliffs first, so walls and doors stand over them.
    for (c, r, x, y) in cells() {
        let cell = source.resolved.and_then(|m| m.cell(source.level, x, y));
        for facing in Facing::ALL {
            let key = edge_key(c, r, facing);
            if source.grid.edge(x, y, facing) == Some(EdgeKind::Ramp) {
                for (px, py) in tick_pixels(s, key) {
                    out.put(px, py, RAMP);
                }
            }
            if cell.is_some_and(|cell| cell.cliffs.has(facing)) {
                for (px, py) in edge_pixels(s, size, key, 0) {
                    out.put(px, py, CLIFF);
                }
            }
        }
    }
    for (c, r, x, y) in cells() {
        let Some(join) = source
            .grid
            .tile(x, y)
            .flatten()
            .and_then(|t| source.terrains.get(usize::from(t)))
            .and_then(|t| t.join)
        else {
            continue;
        };
        let colour = if join == Join::Hole { HOLE } else { JOIN };
        let rise = source
            .resolved
            .and_then(|m| m.rise(source.level, x, y))
            .unwrap_or(Facing::South);
        for (px, py) in join_pixels(s, (c, r), join, rise) {
            out.put(px, py, colour);
        }
    }
    for (c, r, x, y) in cells() {
        for facing in Facing::ALL {
            let (inset, colour) = match source.grid.edge(x, y, facing) {
                Some(EdgeKind::Wall) => (0, WALL),
                Some(EdgeKind::Door) => (si / 4, DOOR),
                _ => continue,
            };
            for (px, py) in edge_pixels(s, size, edge_key(c, r, facing), inset) {
                out.put(px, py, colour);
            }
        }
    }
    if let Some(start) = source.start {
        let (left, top, side) = view.cell_rect(start);
        let side = i64::from(side);
        // A diamond in the cell.
        let m = (left + side / 2, top + side / 2);
        let q = side / 4;
        for (a, b) in [
            ((m.0, m.1 - q), (m.0 + q, m.1)),
            ((m.0 + q, m.1), (m.0, m.1 + q)),
            ((m.0, m.1 + q), (m.0 - q, m.1)),
            ((m.0 - q, m.1), (m.0, m.1 - q)),
        ] {
            for (px, py) in line_pixels(a, b) {
                out.put(px, py, START);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fitted_view_takes_the_largest_zoom_that_shows_the_whole_map_centred() {
        let view = View::fitted((12, 5), (800, 600));
        assert_eq!(view.scale(), 32, "12 × 32 = 384 fits 800");
        // 800 / 32 = 25 cells across, 12 shown: 6 spare each side.
        assert_eq!(view.origin, (-6, -6));
        let tall = View::fitted((40, 40), (300, 300));
        assert_eq!(tall.scale(), 4, "40 × 8 = 320 does not fit 300");
        let huge = View::fitted((256, 256), (100, 100));
        assert_eq!(huge.scale(), 4, "the smallest zoom when nothing fits");
    }

    #[test]
    fn a_pixel_hits_its_cell_and_nothing_off_the_map() {
        let view = View {
            origin: (-1, 2),
            zoom: 1,
        };
        assert_eq!(view.hit((10, 10), 0.0, 0.0), None, "before the map");
        assert_eq!(view.hit((10, 10), 8.0, 0.0), Some((0, 2)));
        assert_eq!(view.hit((10, 10), 15.9, 7.9), Some((0, 2)));
        assert_eq!(view.hit((10, 10), 16.0, 8.0), Some((1, 3)));
        assert_eq!(view.hit((10, 10), 8.0, 64.0), None, "past the last row");
        assert_eq!(view.hit((10, 10), -1.0, 5.0), None);
        assert_eq!(view.cell_rect((0, 2)), (8, 0, 8));
    }

    #[test]
    fn zooming_keeps_the_middle_cell_in_the_middle() {
        let pane = (160, 80);
        let view = View::default().centred_on((20, 20), pane);
        assert_eq!(view.centre_cell(pane), (20, 20));
        let closer = view.zoomed(1, pane);
        assert_eq!(closer.scale(), 24);
        assert_eq!(closer.centre_cell(pane), (20, 20));
        let farthest = view.zoomed(-9, pane);
        assert_eq!(farthest.scale(), 4, "stops at the smallest");
        assert_eq!(view.zoomed(9, pane).scale(), 32, "and the largest");
    }

    #[test]
    fn fills_brighten_with_height_within_bounds() {
        assert_eq!(fill((100, 100, 100), 0), [60, 60, 60, 255]);
        assert!(fill((100, 100, 100), 8)[0] > 60, "a wall up is brighter");
        assert!(fill((100, 100, 100), -8)[0] < 60, "down is darker");
        assert_eq!(fill((100, 100, 100), 4000)[0], 100, "never past the colour");
        assert_eq!(fill((100, 100, 100), -4000)[0], 25, "never black");
    }
}
