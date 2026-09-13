//! Canvas geometry shared by the renderer, the draw planner, and the UI. Internal resolution is
//! provisional (PRD §14): 320×180 with a 240×135 viewport in the top-left, a right column for
//! the minimap, the location lines, and the movement pad, and a bottom band for the message
//! line, the party, and the help line. Text is laid out on a grid of 6×8 cells (`font`).

/// Internal canvas width in pixels.
pub const CANVAS_WIDTH: u32 = 320;
/// Internal canvas height in pixels.
pub const CANVAS_HEIGHT: u32 = 180;
/// The viewport's top-left corner on the canvas.
pub const VIEWPORT_ORIGIN: (i32, i32) = (0, 0);
/// The viewport's size on the canvas; the tileset's `viewport` must match.
pub const VIEWPORT_SIZE: (u32, u32) = (240, 135);
/// The panel colour around the viewport.
pub const PANEL_COLOR: (u8, u8, u8) = (24, 24, 34);
/// The sidebar minimap rectangle on the canvas: x, y, width, height.
pub const SIDEBAR_MAP: (i32, i32, u32, u32) = (248, 8, 64, 64);
/// Pixels per tile in the sidebar minimap.
pub const SIDEBAR_MAP_SCALE: i32 = 2;
/// Pixels per tile in the large automap overlay.
pub const OVERLAY_MAP_SCALE: i32 = 4;

/// A text cell: glyphs are 5×7 in a 6×8 cell.
pub const CELL: (i32, i32) = (6, 8);
/// The canvas as text cells.
pub const COLUMNS: i32 = 53;
/// The canvas as text rows.
pub const ROWS: i32 = 22;

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

/// The whole canvas.
pub const CANVAS: Rect = Rect::new(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
/// The viewport.
pub const VIEWPORT: Rect = Rect::new(0, 0, VIEWPORT_SIZE.0, VIEWPORT_SIZE.1);
/// The menu screens fill the viewport; their text uses 40 columns × 16 rows of it.
pub const MENU_COLUMNS: i32 = 40;
/// Rows of text a menu screen may use.
pub const MENU_ROWS: i32 = 16;
/// The large automap overlay is clipped to this, inside the viewport.
pub const OVERLAY_MAP_CLIP: Rect = Rect::new(8, 8, 224, 119);
/// The right column: minimap, location lines, movement pad.
pub const RIGHT_COLUMN: Rect = Rect::new(240, 0, 80, 135);
/// Text cells across the right column.
pub const HUD_COLUMNS: usize = 13;
/// Top-left of the three location lines: map name, position, clock.
pub const HUD_LINES: [(i32, i32); 3] = [(241, 74), (241, 82), (241, 90)];
/// The movement pad.
pub const PAD: Rect = Rect::new(244, 100, 76, 35);
/// The pad's buttons in reading order: turn left, forward, turn right, sidestep left, back,
/// sidestep right, use.
pub const PAD_BUTTONS: [Rect; 7] = [
    Rect::new(244, 100, 24, 11),
    Rect::new(270, 100, 24, 11),
    Rect::new(296, 100, 24, 11),
    Rect::new(244, 112, 24, 11),
    Rect::new(270, 112, 24, 11),
    Rect::new(296, 112, 24, 11),
    Rect::new(244, 124, 76, 11),
];
/// The bottom band: message line, party rows, help line.
pub const BAND: Rect = Rect::new(0, 135, 320, 45);
/// Top-left of the message line.
pub const BAND_MESSAGE: (i32, i32) = (4, 137);
/// Text cells across the message and help lines.
pub const BAND_COLUMNS: usize = 52;
/// Top of each of the three party rows.
pub const BAND_ROWS: [i32; 3] = [146, 154, 162];
/// Left edge of the front-row column and the back-row column.
pub const BAND_FRONT_X: i32 = 4;
/// Left edge of the back-row column.
pub const BAND_BACK_X: i32 = 164;
/// Text cells per party row.
pub const BAND_ROW_COLUMNS: usize = 26;
/// Top-left of the help line.
pub const BAND_HELP: (i32, i32) = (4, 171);

/// The canvas pixel of a text cell in the menu area: one pixel in from the left edge so the
/// first glyph has a margin, and column 40 lands one pixel clear of the viewport.
#[must_use]
pub const fn cell(column: i32, row: i32) -> (i32, i32) {
    (1 + column * CELL.0, row * CELL.1)
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
    pub fn new(viewport: (u16, u16)) -> Camera {
        let height = f32::from(viewport.1);
        Camera {
            width: f32::from(viewport.0),
            height,
            focal: 0.9 * height,
        }
    }

    /// Screen x of lateral world offset `x` at distance `z`.
    #[must_use]
    pub fn sx(&self, x: f32, z: f32) -> f32 {
        self.width / 2.0 + x * self.focal / z
    }

    /// Screen y of world height `h` (0 floor, 1 ceiling) at distance `z`.
    #[must_use]
    pub fn sy(&self, h: f32, z: f32) -> f32 {
        self.height / 2.0 - (h - 0.5) * self.focal / z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_rectangles_land_on_the_letterboxed_canvas() {
        assert_eq!(window_scale(1280.0, 720.0), 4.0);
        assert_eq!(window_scale(3840.0, 2160.0), 12.0);
        assert_eq!(window_scale(1000.0, 600.0), 3.0);
        assert_eq!(window_scale(100.0, 50.0), 1.0);
        assert_eq!(
            canvas_rect_to_window(PAD.tuple(), 1280.0, 720.0),
            (976.0, 400.0, 304.0, 140.0)
        );
        // 1920x1200 shows the canvas at 6x with 60 px of letterbox above and below.
        assert_eq!(
            canvas_rect_to_window((0, 0, 320, 180), 1920.0, 1200.0),
            (0.0, 60.0, 1920.0, 1080.0)
        );
    }

    #[test]
    fn the_scale_never_overflows_the_window() {
        // 1150x650 is just under 4x: rounding gave 4 and a 1280x720 canvas in a smaller window.
        assert_eq!(window_scale(1150.0, 650.0), 3.0);
        assert_eq!(window_scale(1279.0, 720.0), 3.0);
        assert_eq!(window_scale(2462.0, 1284.0), 7.0);
    }

    #[test]
    fn window_positions_map_back_to_canvas_pixels() {
        assert_eq!(
            window_to_canvas((976.0, 304.0), 1280.0, 720.0),
            Some((244, 76))
        );
        assert_eq!(
            window_to_canvas((979.9, 307.9), 1280.0, 720.0),
            Some((244, 76))
        );
        assert_eq!(window_to_canvas((0.0, 0.0), 1280.0, 720.0), Some((0, 0)));
        assert_eq!(
            window_to_canvas((1279.0, 719.0), 1280.0, 720.0),
            Some((319, 179))
        );
        assert_eq!(window_to_canvas((1280.0, 719.0), 1280.0, 720.0), None);
        // 1920x1200: the top 60 px are letterbox.
        assert_eq!(window_to_canvas((0.0, 30.0), 1920.0, 1200.0), None);
        assert_eq!(window_to_canvas((0.0, 60.0), 1920.0, 1200.0), Some((0, 0)));
        assert_eq!(
            window_to_canvas((1919.0, 1139.0), 1920.0, 1200.0),
            Some((319, 179))
        );
        assert_eq!(window_to_canvas((1919.0, 1140.0), 1920.0, 1200.0), None);
        assert_eq!(window_to_canvas((-1.0, 100.0), 1280.0, 720.0), None);
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
        assert_eq!(cell(MENU_COLUMNS, 0).0, 241);
        assert!(cell(MENU_COLUMNS - 1, MENU_ROWS - 1).1 + CELL.1 <= VIEWPORT.bottom());
        assert_eq!((COLUMNS * CELL.0, ROWS * CELL.1), (318, 176));
    }
}
