//! Canvas geometry shared by the renderer, the draw planner, and the UI. The internal
//! resolution is 1280×720 for the 4K target (PRD D20): a canvas with the viewport in its top-left, a right
//! column for the minimap, the location lines, and the movement pad, and a bottom band for the
//! message line, the party, and the help line. Text is laid out on a grid of 6×8 cells
//! (`font`). Every region is an expression of the inputs at the top of the file.

/// Internal canvas width in pixels.
pub const CANVAS_WIDTH: u32 = 1280;
/// Internal canvas height in pixels.
pub const CANVAS_HEIGHT: u32 = 720;
/// The viewport's top-left corner on the canvas.
pub const VIEWPORT_ORIGIN: (i32, i32) = (0, 0);
/// The viewport's size on the canvas; every tileset's `viewport` must match.
pub const VIEWPORT_SIZE: (u16, u16) = (960, 540);
/// The panel colour around the viewport.
pub const PANEL_COLOR: (u8, u8, u8) = (24, 24, 34);
/// A text cell: glyphs are 5×7 in a 6×8 cell.
pub const CELL: (i32, i32) = (6, 8);
/// Pixels per tile in the sidebar minimap.
pub const SIDEBAR_MAP_SCALE: i32 = 8;
/// Tiles across the sidebar minimap.
pub const SIDEBAR_MAP_TILES: i32 = 32;
/// Pixels per tile in the large automap overlay.
pub const OVERLAY_MAP_SCALE: i32 = 16;
/// Both maps' inset from the edges of their regions.
pub const MAP_INSET: i32 = CELL.1;
/// The gap between the minimap and the first location line.
pub const HUD_GAP: i32 = 2;
/// A pad button's size.
pub const PAD_BUTTON: (u32, u32) = (96, 40);
/// The gap between pad buttons, across and down.
pub const PAD_GAP: (i32, i32) = (8, 8);
/// The pad's inset from the right column's right and bottom edges.
pub const PAD_INSET: (i32, i32) = (8, 8);
/// The band's text inset from the canvas edge.
pub const BAND_X: i32 = 4;
/// The band's padding above the message line.
pub const BAND_PAD: i32 = 2;

/// The canvas as text cells.
pub const COLUMNS: i32 = CANVAS_WIDTH as i32 / CELL.0;
/// The canvas as text rows.
pub const ROWS: i32 = CANVAS_HEIGHT as i32 / CELL.1;

/// A rectangle in canvas pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

impl Rect {
    /// From its edges and size.
    #[must_use]
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Rect {
        Rect { x, y, w, h }
    }

    /// One past the right edge.
    #[must_use]
    pub const fn right(self) -> i32 {
        self.x + self.w as i32
    }

    /// One past the bottom edge.
    #[must_use]
    pub const fn bottom(self) -> i32 {
        self.y + self.h as i32
    }

    /// Whether the point is inside.
    #[must_use]
    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Whether the two share a pixel.
    #[must_use]
    pub const fn overlaps(self, other: Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// Whether `inner` lies entirely inside.
    #[must_use]
    pub const fn encloses(self, inner: Rect) -> bool {
        inner.x >= self.x
            && inner.y >= self.y
            && inner.right() <= self.right()
            && inner.bottom() <= self.bottom()
    }

    /// As the `(x, y, width, height)` tuple the older helpers take.
    #[must_use]
    pub const fn tuple(self) -> (i32, i32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }
}

/// The canvas.
pub const CANVAS: Rect = Rect::new(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
/// The viewport.
pub const VIEWPORT: Rect = Rect::new(
    VIEWPORT_ORIGIN.0,
    VIEWPORT_ORIGIN.1,
    VIEWPORT_SIZE.0 as u32,
    VIEWPORT_SIZE.1 as u32,
);
/// The menu screens fill the viewport; their text uses its columns and rows.
pub const MENU_COLUMNS: i32 = VIEWPORT.w as i32 / CELL.0;
/// Rows of text a menu screen may use.
pub const MENU_ROWS: i32 = VIEWPORT.h as i32 / CELL.1;
/// The large automap overlay is clipped to this, inside the viewport.
pub const OVERLAY_MAP_CLIP: Rect = Rect::new(
    VIEWPORT.x + MAP_INSET,
    VIEWPORT.y + MAP_INSET,
    VIEWPORT.w - 2 * MAP_INSET as u32,
    VIEWPORT.h - 2 * MAP_INSET as u32,
);
/// The right column: minimap, location lines, movement pad.
pub const RIGHT_COLUMN: Rect = Rect::new(
    VIEWPORT.right(),
    VIEWPORT.y,
    CANVAS_WIDTH - VIEWPORT.right() as u32,
    VIEWPORT.h,
);
/// The sidebar minimap's side in pixels.
const SIDEBAR_MAP_SIDE: u32 = (SIDEBAR_MAP_TILES * SIDEBAR_MAP_SCALE) as u32;
/// The sidebar minimap rectangle on the canvas: x, y, width, height; centred in the column.
pub const SIDEBAR_MAP: (i32, i32, u32, u32) = (
    RIGHT_COLUMN.x + (RIGHT_COLUMN.w - SIDEBAR_MAP_SIDE) as i32 / 2,
    RIGHT_COLUMN.y + MAP_INSET,
    SIDEBAR_MAP_SIDE,
    SIDEBAR_MAP_SIDE,
);
/// Text cells across the right column.
pub const HUD_COLUMNS: usize = ((RIGHT_COLUMN.w as i32 - 2) / CELL.0) as usize;
/// Top-left of the three location lines: map name, position, clock.
pub const HUD_LINES: [(i32, i32); 3] = [hud_line(0), hud_line(1), hud_line(2)];
/// The location line `i`: under the minimap, one pixel in from the column's edge.
const fn hud_line(i: i32) -> (i32, i32) {
    (
        cell(MENU_COLUMNS, 0).0,
        SIDEBAR_MAP.1 + SIDEBAR_MAP.3 as i32 + HUD_GAP + i * CELL.1,
    )
}
/// The pad's size: three buttons across, three down, the gaps between.
const PAD_SIZE: (u32, u32) = (
    3 * PAD_BUTTON.0 + 2 * PAD_GAP.0 as u32,
    3 * PAD_BUTTON.1 + 2 * PAD_GAP.1 as u32,
);
/// The movement pad, against the column's right and bottom edges less its inset.
pub const PAD: Rect = Rect::new(
    RIGHT_COLUMN.right() - PAD_INSET.0 - PAD_SIZE.0 as i32,
    RIGHT_COLUMN.bottom() - PAD_INSET.1 - PAD_SIZE.1 as i32,
    PAD_SIZE.0,
    PAD_SIZE.1,
);
/// The pad button at a column and row of the pad's grid.
pub const fn pad_button(column: i32, row: i32) -> Rect {
    Rect::new(
        PAD.x + column * (PAD_BUTTON.0 as i32 + PAD_GAP.0),
        PAD.y + row * (PAD_BUTTON.1 as i32 + PAD_GAP.1),
        PAD_BUTTON.0,
        PAD_BUTTON.1,
    )
}
/// The pad's buttons in reading order: turn left, forward, turn right, sidestep left, back,
/// sidestep right, use (the full width of the bottom row).
pub const PAD_BUTTONS: [Rect; 7] = [
    pad_button(0, 0),
    pad_button(1, 0),
    pad_button(2, 0),
    pad_button(0, 1),
    pad_button(1, 1),
    pad_button(2, 1),
    Rect::new(PAD.x, pad_button(0, 2).y, PAD.w, PAD_BUTTON.1),
];
/// The bottom band: message line, party rows, help line.
pub const BAND: Rect = Rect::new(
    0,
    VIEWPORT.bottom(),
    CANVAS_WIDTH,
    CANVAS_HEIGHT - VIEWPORT.bottom() as u32,
);
/// Top-left of the message line.
pub const BAND_MESSAGE: (i32, i32) = (BAND_X, BAND.y + BAND_PAD);
/// Text cells across the message and help lines.
pub const BAND_COLUMNS: usize = ((CANVAS_WIDTH as i32 - 2 * BAND_X) / CELL.0) as usize;
/// The top of party row `i`: a pixel under the message line, then one row each.
const fn band_row(i: i32) -> i32 {
    BAND_MESSAGE.1 + CELL.1 + 1 + i * CELL.1
}
/// Top of each of the three party rows.
pub const BAND_ROWS: [i32; 3] = [band_row(0), band_row(1), band_row(2)];
/// Left edge of the front-row column.
pub const BAND_FRONT_X: i32 = BAND_X;
/// Left edge of the back-row column: the second half of the canvas.
pub const BAND_BACK_X: i32 = BAND_X + CANVAS_WIDTH as i32 / 2;
/// Text cells per party row.
pub const BAND_ROW_COLUMNS: usize = ((CANVAS_WIDTH as i32 / 2 - BAND_X) / CELL.0) as usize;
/// Top-left of the help line: a pixel under the last party row.
pub const BAND_HELP: (i32, i32) = (BAND_X, band_row(2) + CELL.1 + 1);

/// The canvas pixel of a text cell in the menu area: one pixel in from the left edge so the
/// first glyph has a margin, and the column past the viewport lands one pixel clear of it.
#[must_use]
pub const fn cell(column: i32, row: i32) -> (i32, i32) {
    (1 + column * CELL.0, row * CELL.1)
}

/// The canvas y of a text row.
#[must_use]
pub const fn row_y(row: i32) -> i32 {
    row * CELL.1
}

/// The viewport-wide strip from row `from` up to row `to` (exclusive).
#[must_use]
pub const fn rows(from: i32, to: i32) -> Rect {
    Rect::new(
        VIEWPORT.x,
        VIEWPORT.y + row_y(from),
        VIEWPORT.w,
        ((to - from) * CELL.1) as u32,
    )
}

/// The integer scale the canvas is shown at in a window of this logical size: the largest
/// whole multiple that fits both ways, at least 1.
#[must_use]
pub fn window_scale(window_width: f32, window_height: f32) -> f32 {
    let h = window_width / CANVAS_WIDTH as f32;
    let v = window_height / CANVAS_HEIGHT as f32;
    h.min(v).floor().max(1.0)
}

/// The window pixel offset of the canvas's top-left corner: the letterbox that centres it.
fn letterbox(window_width: f32, window_height: f32) -> (f32, f32, f32) {
    let s = window_scale(window_width, window_height);
    (
        (window_width - CANVAS_WIDTH as f32 * s) / 2.0,
        (window_height - CANVAS_HEIGHT as f32 * s) / 2.0,
        s,
    )
}

/// A canvas rectangle as window pixels `(left, top, width, height)`: scaled by the integer
/// scale and offset by the letterbox that centres the canvas.
#[must_use]
pub fn canvas_rect_to_window(
    rect: (i32, i32, u32, u32),
    window_width: f32,
    window_height: f32,
) -> (f32, f32, f32, f32) {
    let (offset_x, offset_y, s) = letterbox(window_width, window_height);
    (
        offset_x + rect.0 as f32 * s,
        offset_y + rect.1 as f32 * s,
        rect.2 as f32 * s,
        rect.3 as f32 * s,
    )
}

/// The canvas pixel under a window position (logical pixels, origin top-left), or `None` in the
/// letterbox or outside the window.
#[must_use]
pub fn window_to_canvas(
    cursor: (f32, f32),
    window_width: f32,
    window_height: f32,
) -> Option<(i32, i32)> {
    let (offset_x, offset_y, s) = letterbox(window_width, window_height);
    let x = ((cursor.0 - offset_x) / s).floor();
    let y = ((cursor.1 - offset_y) / s).floor();
    if x < 0.0 || y < 0.0 || x >= CANVAS_WIDTH as f32 || y >= CANVAS_HEIGHT as f32 {
        return None;
    }
    Some((x as i32, y as i32))
}

/// The same camera the bake tool uses (`omnis-cli`, `bake.rs`): eye at the near edge of the
/// party's tile, half a tile up; focal length `0.9 × viewport height`. The renderer needs it
/// only for the procedural horizon band beyond detail depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Viewport width in pixels.
    pub width: f32,
    /// Viewport height in pixels.
    pub height: f32,
    /// Focal length in pixels.
    pub focal: f32,
}

impl Camera {
    /// For a viewport of the given size.
    #[must_use]
    pub const fn new(viewport: (u16, u16)) -> Camera {
        let height = viewport.1 as f32;
        Camera {
            width: viewport.0 as f32,
            height,
            focal: 0.9 * height,
        }
    }

    /// Screen x of lateral world offset `x` at distance `z`.
    #[must_use]
    pub const fn sx(&self, x: f32, z: f32) -> f32 {
        self.width / 2.0 + x * self.focal / z
    }

    /// Screen y of world height `h` (0 floor, 1 ceiling) at distance `z`.
    #[must_use]
    pub const fn sy(&self, h: f32, z: f32) -> f32 {
        self.height / 2.0 - (h - 0.5) * self.focal / z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canvas as floats, for windows of a whole multiple of it.
    const W: f32 = CANVAS_WIDTH as f32;
    const H: f32 = CANVAS_HEIGHT as f32;

    #[test]
    fn canvas_rectangles_land_on_the_letterboxed_canvas() {
        assert_eq!(window_scale(4.0 * W, 4.0 * H), 4.0);
        assert_eq!(window_scale(12.0 * W, 12.0 * H), 12.0);
        assert_eq!(window_scale(3.0 * W + 40.0, 3.0 * H + 60.0), 3.0);
        assert_eq!(window_scale(W / 3.0, H / 3.0), 1.0, "never below one");
        let (x, y, w, h) = PAD.tuple();
        assert_eq!(
            canvas_rect_to_window(PAD.tuple(), 4.0 * W, 4.0 * H),
            (
                4.0 * x as f32,
                4.0 * y as f32,
                4.0 * w as f32,
                4.0 * h as f32
            )
        );
        // A window 120 px taller than 6x shows the canvas at 6x with 60 px above and below.
        assert_eq!(
            canvas_rect_to_window(CANVAS.tuple(), 6.0 * W, 6.0 * H + 120.0),
            (0.0, 60.0, 6.0 * W, 6.0 * H)
        );
    }

    #[test]
    fn the_scale_never_overflows_the_window() {
        // Just under 4x: rounding gave 4 and a canvas larger than the window.
        assert_eq!(window_scale(4.0 * W - 130.0, 4.0 * H - 70.0), 3.0);
        assert_eq!(window_scale(4.0 * W - 1.0, 4.0 * H), 3.0);
        assert_eq!(window_scale(7.0 * W + 222.0, 7.0 * H + 24.0), 7.0);
    }

    #[test]
    fn window_positions_map_back_to_canvas_pixels() {
        let (px, py) = (PAD.x as f32, PAD.y as f32);
        let four = (4.0 * W, 4.0 * H);
        assert_eq!(
            window_to_canvas((4.0 * px, 4.0 * py), four.0, four.1),
            Some((PAD.x, PAD.y))
        );
        assert_eq!(
            window_to_canvas((4.0 * px + 3.9, 4.0 * py + 3.9), four.0, four.1),
            Some((PAD.x, PAD.y))
        );
        assert_eq!(window_to_canvas((0.0, 0.0), four.0, four.1), Some((0, 0)));
        let last = (CANVAS_WIDTH as i32 - 1, CANVAS_HEIGHT as i32 - 1);
        assert_eq!(
            window_to_canvas((four.0 - 1.0, four.1 - 1.0), four.0, four.1),
            Some(last)
        );
        assert_eq!(
            window_to_canvas((four.0, four.1 - 1.0), four.0, four.1),
            None
        );
        // 120 px taller than 6x: the top 60 px are letterbox.
        let six = (6.0 * W, 6.0 * H + 120.0);
        assert_eq!(window_to_canvas((0.0, 30.0), six.0, six.1), None);
        assert_eq!(window_to_canvas((0.0, 60.0), six.0, six.1), Some((0, 0)));
        assert_eq!(
            window_to_canvas((six.0 - 1.0, 59.0 + 6.0 * H), six.0, six.1),
            Some(last)
        );
        assert_eq!(
            window_to_canvas((six.0 - 1.0, 60.0 + 6.0 * H), six.0, six.1),
            None
        );
        assert_eq!(window_to_canvas((-1.0, 100.0), four.0, four.1), None);
    }

    #[test]
    fn rectangles_test_points_and_overlaps() {
        let r = Rect::new(10, 20, 5, 4);
        assert!(r.contains(10, 20) && r.contains(14, 23));
        assert!(!r.contains(15, 23) && !r.contains(14, 24) && !r.contains(9, 20));
        assert!(r.overlaps(Rect::new(14, 23, 3, 3)));
        assert!(!r.overlaps(Rect::new(15, 20, 3, 3)));
        assert!(!r.overlaps(Rect::new(10, 24, 3, 3)));
        assert!(r.encloses(Rect::new(11, 21, 4, 3)));
        assert!(!r.encloses(Rect::new(11, 21, 5, 3)));
        assert_eq!(r.tuple(), (10, 20, 5, 4));
    }

    #[test]
    fn regions_are_inside_the_canvas_and_disjoint() {
        let regions = [VIEWPORT, RIGHT_COLUMN, BAND];
        for (i, a) in regions.iter().enumerate() {
            assert!(CANVAS.encloses(*a), "{a:?}");
            for b in &regions[i + 1..] {
                assert!(!a.overlaps(*b), "{a:?} overlaps {b:?}");
            }
        }
        assert!(VIEWPORT.encloses(OVERLAY_MAP_CLIP));
        let map = Rect::new(SIDEBAR_MAP.0, SIDEBAR_MAP.1, SIDEBAR_MAP.2, SIDEBAR_MAP.3);
        assert!(RIGHT_COLUMN.encloses(map) && RIGHT_COLUMN.encloses(PAD));
        let hud_width = HUD_COLUMNS as u32 * CELL.0 as u32;
        for (x, y) in HUD_LINES {
            let line = Rect::new(x, y, hud_width, CELL.1 as u32);
            assert!(RIGHT_COLUMN.encloses(line), "{line:?}");
            assert!(!line.overlaps(map) && !line.overlaps(PAD), "{line:?}");
        }
        for (i, a) in PAD_BUTTONS.iter().enumerate() {
            assert!(PAD.encloses(*a), "{a:?}");
            for b in &PAD_BUTTONS[i + 1..] {
                assert!(!a.overlaps(*b), "{a:?} overlaps {b:?}");
            }
        }
        let row_width = BAND_ROW_COLUMNS as u32 * CELL.0 as u32;
        for y in BAND_ROWS {
            let front = Rect::new(BAND_FRONT_X, y, row_width, CELL.1 as u32);
            let back = Rect::new(BAND_BACK_X, y, row_width, CELL.1 as u32);
            assert!(BAND.encloses(front) && BAND.encloses(back) && !front.overlaps(back));
        }
        let line_width = BAND_COLUMNS as u32 * CELL.0 as u32;
        let message = Rect::new(BAND_MESSAGE.0, BAND_MESSAGE.1, line_width, CELL.1 as u32);
        let help = Rect::new(BAND_HELP.0, BAND_HELP.1, line_width, CELL.1 as u32);
        assert!(BAND.encloses(message) && BAND.encloses(help));
        assert!(message.bottom() <= BAND_ROWS[0] && BAND_ROWS[2] + CELL.1 <= help.y);
        assert_eq!(cell(0, 0), (1, 0));
        assert_eq!(
            cell(MENU_COLUMNS, 0).0,
            VIEWPORT.right() + 1,
            "the column past the menu grid clears the viewport by a pixel"
        );
        assert!(cell(MENU_COLUMNS - 1, MENU_ROWS - 1).1 + CELL.1 <= VIEWPORT.bottom());
        let (w, h) = (CANVAS_WIDTH as i32, CANVAS_HEIGHT as i32);
        assert!(COLUMNS * CELL.0 <= w && (COLUMNS + 1) * CELL.0 > w);
        assert!(ROWS * CELL.1 <= h && (ROWS + 1) * CELL.1 > h);
        assert!(
            HUD_LINES[2].1 + CELL.1 <= PAD.y,
            "the location lines end above the pad"
        );
    }
}
