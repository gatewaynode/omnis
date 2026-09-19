//! The canvas at a width. The layout constants (`layout.rs`) describe the *core*: the narrow
//! 1280-wide canvas with the viewport in its top-left and the right column beside it. A wider
//! window keeps the core and adds room around it: the viewport is centred, a *wing* to its left
//! holds the roster, and the band under the viewport spans the wing and the core. A narrower
//! window gets the core centred with the band under it, as on the narrow canvas. Every
//! painter takes the `Layout`; the core's painters paint through its origin (`Frame::within`).

use crate::layout::{
    BAND, CANVAS_HEIGHT, CANVAS_WIDTH, CELL, OVERLAY_MAP_CLIP, RIGHT_COLUMN, Rect, SIDEBAR_MAP,
    VIEWPORT, cell,
};

/// The band's first text row on the canvas grid: its top is not on the grid.
pub const BAND_ROW0: i32 = (BAND.y + CELL.1 - 1) / CELL.1;
/// The message line's row, from the band's first row.
pub const MESSAGE_ROW: i32 = 0;
/// The captions over the roster and the log.
pub const CAPTION_ROW: i32 = 2;
/// The first member's row; one row per slot follows.
pub const FIRST_MEMBER_ROW: i32 = 3;
/// Rows the roster has: the party's six slots.
pub const MEMBER_ROWS: usize = 6;
/// The column the event log starts at on the narrow canvas, beside the roster.
pub const LOG_COLUMN: i32 = 63;
/// Rows of the event log, from the first member's row down.
pub const LOG_ROWS: usize = 18;
/// The help line's row.
pub const HELP_ROW: i32 = 21;
/// Rows the band uses.
pub const ROWS_USED: i32 = 22;
/// Text cells across the narrow band from its first column.
pub const BAND_COLUMNS: usize = ((CANVAS_WIDTH as i32 - band_cell(0, 0).0) / CELL.0) as usize;
/// Cells of a roster row.
pub const ROSTER_CELLS: usize = 61;
/// Cells of a log line on the narrow canvas: the least a log line gets.
pub const LOG_CELLS: usize = BAND_COLUMNS - LOG_COLUMN as usize;
/// Text cells across the wing: a roster row, the marker's cell before it, one clear after.
pub const WING_COLUMNS: i32 = ROSTER_CELLS as i32 + 2;
/// The wing's width in pixels.
pub const WING: u32 = (WING_COLUMNS * CELL.0) as u32;
// The rows fit the band, the roster leaves room for the rule before the log, the log's rows
// end above the help line, and the members follow their caption.
const _: () = assert!((BAND_ROW0 + ROWS_USED) * CELL.1 <= BAND.y + BAND.h as i32);
const _: () = assert!(ROSTER_CELLS as i32 + 2 <= LOG_COLUMN);
const _: () = assert!(FIRST_MEMBER_ROW + LOG_ROWS as i32 <= HELP_ROW);
const _: () = assert!(FIRST_MEMBER_ROW + MEMBER_ROWS as i32 <= HELP_ROW);
const _: () = assert!(FIRST_MEMBER_ROW == CAPTION_ROW + 1);

/// The canvas pixel of a band cell on the narrow canvas: one column in, so the marker fits
/// before the roster.
#[must_use]
pub const fn band_cell(column: i32, row: i32) -> (i32, i32) {
    cell(1 + column, BAND_ROW0 + row)
}

/// Where the regions sit on a canvas of some width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy::prelude::Resource)]
pub struct Layout {
    /// The canvas width in pixels, at least `CANVAS_WIDTH`.
    pub width: u32,
    /// The core's top-left corner: the viewport's corner, the right column beside it.
    pub core: (i32, i32),
    /// The roster's column left of the viewport, on a canvas wide enough for it.
    pub wing: Option<Rect>,
    /// The band under the viewport: the wing and the core wide, or the core alone.
    pub band: Rect,
}

/// The narrow canvas: the layout the constants describe.
pub const NARROW: Layout = Layout::for_width(CANVAS_WIDTH);

impl Default for Layout {
    fn default() -> Self {
        NARROW
    }
}

impl Layout {
    /// The layout for a canvas this wide: the viewport centred with the wing beside it when
    /// the wing fits left of a centred viewport, else the core centred.
    #[must_use]
    pub const fn for_width(width: u32) -> Layout {
        let width = if width < CANVAS_WIDTH {
            CANVAS_WIDTH
        } else {
            width
        };
        let centred = (width - VIEWPORT.w) as i32 / 2;
        if centred >= WING as i32 {
            let wing = Rect::new(centred - WING as i32, 0, WING, VIEWPORT.h);
            Layout {
                width,
                core: (centred, 0),
                wing: Some(wing),
                band: Rect::new(wing.x, BAND.y, WING + CANVAS_WIDTH, BAND.h),
            }
        } else {
            let core = (width - CANVAS_WIDTH) as i32 / 2;
            Layout {
                width,
                core: (core, 0),
                wing: None,
                band: Rect::new(core, BAND.y, BAND.w, BAND.h),
            }
        }
    }

    /// Whether the roster sits in the wing rather than the band.
    #[must_use]
    pub const fn is_wide(&self) -> bool {
        self.wing.is_some()
    }

    /// The whole canvas.
    #[must_use]
    pub const fn canvas(&self) -> Rect {
        Rect::new(0, 0, self.width, CANVAS_HEIGHT)
    }

    /// A core rectangle on this canvas.
    #[must_use]
    pub const fn shift(&self, rect: Rect) -> Rect {
        rect.shifted(self.core.0, self.core.1)
    }

    /// The viewport.
    #[must_use]
    pub const fn viewport(&self) -> Rect {
        self.shift(VIEWPORT)
    }

    /// The right column.
    #[must_use]
    pub const fn right_column(&self) -> Rect {
        self.shift(RIGHT_COLUMN)
    }

    /// The sidebar minimap as `(x, y, width, height)`.
    #[must_use]
    pub const fn minimap(&self) -> (i32, i32, u32, u32) {
        (
            SIDEBAR_MAP.0 + self.core.0,
            SIDEBAR_MAP.1 + self.core.1,
            SIDEBAR_MAP.2,
            SIDEBAR_MAP.3,
        )
    }

    /// The large automap's clip inside the viewport.
    #[must_use]
    pub const fn overlay_clip(&self) -> Rect {
        self.shift(OVERLAY_MAP_CLIP)
    }

    /// The canvas pixel of a band cell: one column in from the band's edge.
    #[must_use]
    pub const fn band_cell(&self, column: i32, row: i32) -> (i32, i32) {
        (
            self.band.x + 1 + CELL.0 * (1 + column),
            CELL.1 * (BAND_ROW0 + row),
        )
    }

    /// Text cells across the band from its first column.
    #[must_use]
    pub const fn band_columns(&self) -> usize {
        ((self.band.w as i32 - (1 + CELL.0)) / CELL.0) as usize
    }

    /// The column the event log starts at: the band's first when the roster is in the wing.
    #[must_use]
    pub const fn log_column(&self) -> i32 {
        if self.is_wide() { 0 } else { LOG_COLUMN }
    }

    /// Cells of a log line.
    #[must_use]
    pub const fn log_cells(&self) -> usize {
        self.band_columns() - self.log_column() as usize
    }

    /// The canvas pixel of the roster's caption row; the members follow a row each.
    #[must_use]
    pub const fn roster_origin(&self) -> (i32, i32) {
        match self.wing {
            Some(wing) => (wing.x + 1 + CELL.0, wing.y + CELL.1),
            None => self.band_cell(0, CAPTION_ROW),
        }
    }

    /// The canvas pixel of a member's roster row, for the party's slots.
    #[must_use]
    pub const fn slot_origin(&self, slot: usize) -> Option<(i32, i32)> {
        if slot >= MEMBER_ROWS {
            return None;
        }
        let (x, y) = self.roster_origin();
        Some((x, y + CELL.1 * (1 + slot as i32)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roster_rows(layout: &Layout) -> Vec<Rect> {
        (0..MEMBER_ROWS)
            .map(|slot| {
                let (x, y) = layout.slot_origin(slot).expect("a row per slot");
                Rect::new(x, y, ROSTER_CELLS as u32 * CELL.0 as u32, CELL.1 as u32)
            })
            .collect()
    }

    #[test]
    fn the_narrow_layout_is_the_one_the_constants_describe() {
        let narrow = Layout::for_width(CANVAS_WIDTH);
        assert_eq!(narrow, NARROW);
        assert_eq!(Layout::default(), NARROW);
        assert_eq!(
            Layout::for_width(100),
            NARROW,
            "narrower windows get the narrow canvas"
        );
        assert!(!narrow.is_wide());
        assert_eq!(narrow.core, (0, 0));
        assert_eq!(narrow.band, BAND);
        assert_eq!(
            narrow.canvas(),
            Rect::new(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT)
        );
        assert_eq!(narrow.viewport(), VIEWPORT);
        assert_eq!(narrow.right_column(), RIGHT_COLUMN);
        assert_eq!(narrow.minimap(), SIDEBAR_MAP);
        assert_eq!(narrow.overlay_clip(), OVERLAY_MAP_CLIP);
        for row in 0..ROWS_USED {
            for column in 0..=BAND_COLUMNS as i32 {
                assert_eq!(narrow.band_cell(column, row), band_cell(column, row));
            }
        }
        assert_eq!(narrow.band_columns(), BAND_COLUMNS);
        assert_eq!(narrow.log_column(), LOG_COLUMN);
        assert_eq!(narrow.log_cells(), LOG_CELLS);
        assert_eq!(narrow.roster_origin(), band_cell(0, CAPTION_ROW));
        for slot in 0..MEMBER_ROWS {
            assert_eq!(
                narrow.slot_origin(slot),
                Some(band_cell(0, FIRST_MEMBER_ROW + slot as i32))
            );
        }
        assert_eq!(narrow.slot_origin(MEMBER_ROWS), None);
        for row in roster_rows(&narrow) {
            assert!(BAND.encloses(row));
        }
    }

    #[test]
    fn wide_layouts_centre_the_viewport_and_seat_the_roster_in_the_wing() {
        for width in [1716, 1720, 1920, 2560, 5120] {
            let layout = Layout::for_width(width);
            let wing = layout.wing.unwrap_or_else(|| panic!("{width} is wide"));
            let viewport = layout.viewport();
            let centred = viewport.x * 2 + viewport.w as i32;
            assert!(
                centred == width as i32 || centred + 1 == width as i32,
                "{width}"
            );
            let canvas = layout.canvas();
            let regions = [wing, viewport, layout.right_column(), layout.band];
            for (i, a) in regions.iter().enumerate() {
                assert!(canvas.encloses(*a), "{width}: {a:?}");
                for b in &regions[i + 1..] {
                    assert!(!a.overlaps(*b), "{width}: {a:?} overlaps {b:?}");
                }
            }
            assert_eq!(wing.right(), viewport.x);
            assert_eq!(layout.band.x, wing.x);
            assert_eq!(layout.band.right(), layout.right_column().right());
            assert_eq!(layout.band.y, viewport.bottom());
            for row in roster_rows(&layout) {
                assert!(wing.encloses(row), "{width}: {row:?} leaves the wing");
                let marker = Rect::new(row.x - 5, row.y, 3, row.h);
                assert!(wing.encloses(marker), "{width}: the marker leaves the wing");
            }
            assert_eq!(layout.log_column(), 0);
            assert!(layout.log_cells() >= NARROW.log_cells());
            let last = layout.band_cell(layout.band_columns() as i32, 0).0;
            assert!(
                last <= layout.band.right(),
                "{width}: the last column overflows"
            );
            assert!(layout.minimap().0 + layout.minimap().2 as i32 <= canvas.right());
        }
    }

    #[test]
    fn widths_short_of_the_wing_centre_the_core_and_the_ultrawide_has_the_numbers() {
        for width in [1281, 1666, 1715] {
            let layout = Layout::for_width(width);
            assert!(!layout.is_wide(), "{width}");
            assert_eq!(layout.core, ((width as i32 - CANVAS_WIDTH as i32) / 2, 0));
            assert_eq!(
                layout.band,
                Rect::new(layout.core.0, BAND.y, BAND.w, BAND.h)
            );
            assert_eq!(layout.log_cells(), LOG_CELLS);
            assert!(layout.canvas().encloses(layout.right_column()));
        }
        let ultrawide = Layout::for_width(2560);
        assert_eq!(ultrawide.core, (800, 0));
        assert_eq!(ultrawide.wing, Some(Rect::new(422, 0, 378, 540)));
        assert_eq!(ultrawide.viewport(), Rect::new(800, 0, 960, 540));
        assert_eq!(ultrawide.right_column(), Rect::new(1760, 0, 320, 540));
        assert_eq!(ultrawide.band, Rect::new(422, 540, 1658, 180));
        assert_eq!(ultrawide.minimap(), (1792, 8, 256, 256));
        assert_eq!(ultrawide.band_columns(), 275);
        assert_eq!(ultrawide.log_cells(), 275);
        assert_eq!(ultrawide.roster_origin(), (429, 8));
        assert_eq!(ultrawide.slot_origin(0), Some((429, 16)));
        assert_eq!(ultrawide.slot_origin(5), Some((429, 56)));
        assert_eq!(ultrawide.band_cell(0, MESSAGE_ROW), (429, 544));
    }
}
