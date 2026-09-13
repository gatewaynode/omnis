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
/// The menu grid's top-left cell on the canvas grid, and its size in cells: an 80×16 area
/// centred in the viewport; the row indices the menu models use are rows of this grid.
pub const MENU_ORIGIN: (i32, i32) = (40, 25);
/// Text cells across the menu grid.
pub const MENU_COLUMNS: i32 = 80;
/// Rows of text a menu screen may use.
pub const MENU_ROWS: i32 = 16;
/// The frame around the menu grid stands this far outside it.
pub const MENU_PAD: i32 = 2 * CELL.0;

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

    /// The same rectangle moved by an offset.
    #[must_use]
    pub const fn shifted(self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
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
/// The viewport as text cells: the fight screens paint on this grid.
pub const VIEWPORT_COLUMNS: i32 = VIEWPORT.w as i32 / CELL.0;
/// The viewport as text rows.
pub const VIEWPORT_ROWS: i32 = VIEWPORT.h as i32 / CELL.1;
/// The box framed around the menu grid.
pub const MENU_BOX: Rect = Rect::new(
    cell(MENU_ORIGIN.0, MENU_ORIGIN.1).0 - MENU_PAD,
    cell(MENU_ORIGIN.0, MENU_ORIGIN.1).1 - MENU_PAD,
    (MENU_COLUMNS * CELL.0 + 2 * MENU_PAD) as u32,
    (MENU_ROWS * CELL.1 + 2 * MENU_PAD) as u32,
);
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
        cell(VIEWPORT_COLUMNS, 0).0,
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
/// The bottom band: the message line, the roster, the event log, the help line (`band.rs`).
pub const BAND: Rect = Rect::new(
    0,
    VIEWPORT.bottom(),
    CANVAS_WIDTH,
    CANVAS_HEIGHT - VIEWPORT.bottom() as u32,
);

/// The canvas pixel of a text cell in the menu area: one pixel in from the left edge so the
/// first glyph has a margin, and the column past the viewport lands one pixel clear of it.
#[must_use]
pub const fn cell(column: i32, row: i32) -> (i32, i32) {
    (1 + column * CELL.0, row * CELL.1)
}

/// The canvas pixel of a cell of the menu grid.
#[must_use]
pub const fn menu_cell(column: i32, row: i32) -> (i32, i32) {
    cell(MENU_ORIGIN.0 + column, MENU_ORIGIN.1 + row)
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

/// A window's physical size class: what `--window` accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeClass {
    /// 1280×720: the canvas at 1×.
    Small,
    /// 2560×1440: 2×.
    Medium,
    /// 3840×2160, a 4K monitor: 3×, the display target (PRD D20).
    Large,
    /// 7680×2160, an 8K ultrawide: 3× with bars at the sides.
    Huge,
}

/// The classes with their names on the command line and their physical sizes.
pub const SIZE_CLASSES: [(SizeClass, &str, (u32, u32)); 4] = [
    (SizeClass::Small, "small", (1280, 720)),
    (SizeClass::Medium, "medium", (2560, 1440)),
    (SizeClass::Large, "large", (3840, 2160)),
    (SizeClass::Huge, "huge", (7680, 2160)),
];

impl SizeClass {
    /// The class named on the command line.
    #[must_use]
    pub fn parse(name: &str) -> Option<SizeClass> {
        SIZE_CLASSES
            .iter()
            .find(|(_, n, _)| *n == name)
            .map(|(class, _, _)| *class)
    }

    /// The window's size in physical pixels.
    #[must_use]
    pub fn physical(self) -> (u32, u32) {
        SIZE_CLASSES
            .iter()
            .find(|(class, _, _)| *class == self)
            .map_or((CANVAS_WIDTH, CANVAS_HEIGHT), |(_, _, size)| *size)
    }
}

/// How the canvas fits a window's physical pixels: the whole multiple it is shown at and
/// the letterbox that centres it, in whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fit {
    /// Physical pixels per canvas pixel, at least 1.
    pub scale: u32,
    /// The physical pixel of the canvas's top-left corner; negative when the window is
    /// smaller than the canvas.
    pub offset: (i32, i32),
}

/// The fit for a window of this physical size: the largest whole multiple that fits both
/// ways, at least 1, centred to a whole pixel.
#[must_use]
pub const fn fit(physical: (u32, u32)) -> Fit {
    let h = physical.0 / CANVAS_WIDTH;
    let v = physical.1 / CANVAS_HEIGHT;
    let scale = if h < v { h } else { v };
    let scale = if scale < 1 { 1 } else { scale };
    let shown = (
        (CANVAS_WIDTH * scale) as i64,
        (CANVAS_HEIGHT * scale) as i64,
    );
    Fit {
        scale,
        offset: (
            ((physical.0 as i64 - shown.0) / 2) as i32,
            ((physical.1 as i64 - shown.1) / 2) as i32,
        ),
    }
}

/// A canvas rectangle as physical pixels `(left, top, width, height)`.
#[must_use]
pub fn canvas_rect_to_physical(rect: (i32, i32, u32, u32), fit: Fit) -> (i32, i32, u32, u32) {
    let s = fit.scale as i32;
    (
        fit.offset.0 + rect.0 * s,
        fit.offset.1 + rect.1 * s,
        rect.2 * fit.scale,
        rect.3 * fit.scale,
    )
}

/// The canvas pixel under a physical position (origin top-left), or `None` in the letterbox
/// or outside the window.
#[must_use]
pub fn physical_to_canvas(cursor: (f32, f32), fit: Fit) -> Option<(i32, i32)> {
    let s = fit.scale as f32;
    let x = ((cursor.0 - fit.offset.0 as f32) / s).floor();
    let y = ((cursor.1 - fit.offset.1 as f32) / s).floor();
    if x < 0.0 || y < 0.0 || x >= CANVAS_WIDTH as f32 || y >= CANVAS_HEIGHT as f32 {
        return None;
    }
    Some((x as i32, y as i32))
}

/// Where the canvas sprite's centre sits in world units (canvas pixels, y up) so that its
/// corner lands on the fit's whole-pixel offset: zero for even letterbox bars, half a
/// physical pixel for odd ones.
#[must_use]
pub fn canvas_translation(fit: Fit, physical: (u32, u32)) -> (f32, f32) {
    let s = fit.scale as f32;
    let centre = (
        fit.offset.0 as f32 + CANVAS_WIDTH as f32 * s / 2.0,
        fit.offset.1 as f32 + CANVAS_HEIGHT as f32 * s / 2.0,
    );
    (
        (centre.0 - physical.0 as f32 / 2.0) / s,
        (physical.1 as f32 / 2.0 - centre.1) / s,
    )
}

/// A window's logical size and scale factor as physical pixels, rounded.
#[must_use]
pub fn physical_size(logical: (f32, f32), scale_factor: f32) -> (u32, u32) {
    (
        (logical.0 * scale_factor).round() as u32,
        (logical.1 * scale_factor).round() as u32,
    )
}

/// A canvas rectangle as logical window pixels `(left, top, width, height)` for a window of
/// this logical size and scale factor.
#[must_use]
pub fn canvas_rect_to_window(
    rect: (i32, i32, u32, u32),
    logical: (f32, f32),
    scale_factor: f32,
) -> (f32, f32, f32, f32) {
    let fit = fit(physical_size(logical, scale_factor));
    let (x, y, w, h) = canvas_rect_to_physical(rect, fit);
    (
        x as f32 / scale_factor,
        y as f32 / scale_factor,
        w as f32 / scale_factor,
        h as f32 / scale_factor,
    )
}

/// The canvas pixel under a logical window position, or `None` in the letterbox or outside
/// the window.
#[must_use]
pub fn window_to_canvas(
    cursor: (f32, f32),
    logical: (f32, f32),
    scale_factor: f32,
) -> Option<(i32, i32)> {
    let fit = fit(physical_size(logical, scale_factor));
    physical_to_canvas((cursor.0 * scale_factor, cursor.1 * scale_factor), fit)
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

    #[test]
    fn size_classes_fit_at_whole_physical_multiples() {
        let fits: Vec<(SizeClass, Fit)> = SIZE_CLASSES
            .iter()
            .map(|(class, _, size)| (*class, fit(*size)))
            .collect();
        let at = |scale, offset| Fit { scale, offset };
        assert_eq!(
            fits,
            [
                (SizeClass::Small, at(1, (0, 0))),
                (SizeClass::Medium, at(2, (0, 0))),
                (SizeClass::Large, at(3, (0, 0))),
                (SizeClass::Huge, at(3, (1920, 0))),
            ]
        );
        assert_eq!(SizeClass::parse("large"), Some(SizeClass::Large));
        assert_eq!(SizeClass::parse("Large"), None);
        assert_eq!(SizeClass::Huge.physical(), (7680, 2160));
        // The ultrawide shows the canvas centred with bars at the sides.
        let huge = fit(SizeClass::Huge.physical());
        assert_eq!(
            canvas_rect_to_physical(CANVAS.tuple(), huge),
            (1920, 0, 3840, 2160)
        );
        assert_eq!(physical_to_canvas((1919.0, 0.0), huge), None);
        assert_eq!(physical_to_canvas((1920.0, 0.0), huge), Some((0, 0)));
        assert_eq!(
            physical_to_canvas((5759.9, 2159.9), huge),
            Some((1279, 719))
        );
        assert_eq!(physical_to_canvas((5760.0, 0.0), huge), None);
    }

    #[test]
    fn a_two_x_panel_shows_three_physical_pixels_per_canvas_pixel() {
        // A 4K panel driven at 2x logical: 1920x1080 logical, 3840x2160 physical.
        let logical = (1920.0, 1080.0);
        assert_eq!(physical_size(logical, 2.0), (3840, 2160));
        let three = fit((3840, 2160));
        assert_eq!(
            three,
            Fit {
                scale: 3,
                offset: (0, 0)
            }
        );
        assert_eq!(
            window_to_canvas((960.0, 540.0), logical, 2.0),
            Some((640, 360))
        );
        assert_eq!(window_to_canvas((0.4, 0.4), logical, 2.0), Some((0, 0)));
        assert_eq!(
            window_to_canvas((1919.9, 1079.9), logical, 2.0),
            Some((1279, 719))
        );
        assert_eq!(window_to_canvas((1920.0, 0.0), logical, 2.0), None);
        assert_eq!(
            canvas_rect_to_window(CANVAS.tuple(), logical, 2.0),
            (0.0, 0.0, 1920.0, 1080.0)
        );
        assert_eq!(
            canvas_rect_to_window(PAD.tuple(), logical, 2.0),
            (
                PAD.x as f32 * 1.5,
                PAD.y as f32 * 1.5,
                PAD.w as f32 * 1.5,
                PAD.h as f32 * 1.5
            )
        );
        // The same panel at 1x logical is the same physical fit.
        assert_eq!(
            window_to_canvas((1920.0, 1080.0), (3840.0, 2160.0), 1.0),
            Some((640, 360))
        );
    }

    #[test]
    fn odd_letterboxes_snap_to_whole_pixels() {
        let even = fit((1282, 722));
        assert_eq!(
            even,
            Fit {
                scale: 1,
                offset: (1, 1)
            }
        );
        assert_eq!(canvas_translation(even, (1282, 722)), (0.0, 0.0));
        let odd = fit((1281, 721));
        assert_eq!(
            odd,
            Fit {
                scale: 1,
                offset: (0, 0)
            }
        );
        assert_eq!(canvas_translation(odd, (1281, 721)), (-0.5, 0.5));
        let three = fit((3841, 2160));
        assert_eq!(
            three,
            Fit {
                scale: 3,
                offset: (0, 0)
            }
        );
        let (x, y) = canvas_translation(three, (3841, 2160));
        assert!((x + 0.5 / 3.0).abs() < 1e-6 && y == 0.0);
        // Never a fraction of a physical pixel at the canvas's corner.
        for size in [(1920, 1200), (2462, 1284), (5000, 3000)] {
            let f = fit(size);
            let (x, y, _, _) = canvas_rect_to_physical((0, 0, 1, 1), f);
            assert_eq!((x, y), f.offset);
            assert!(f.scale >= 1 && (x as u32) * 2 <= size.0 && (y as u32) * 2 <= size.1);
        }
    }

    #[test]
    fn windows_smaller_than_the_canvas_still_map_the_centre() {
        let small = fit((640, 360));
        assert_eq!(
            small,
            Fit {
                scale: 1,
                offset: (-320, -180)
            }
        );
        assert_eq!(physical_to_canvas((320.0, 180.0), small), Some((640, 360)));
        assert_eq!(physical_to_canvas((0.0, 0.0), small), Some((320, 180)));
        assert_eq!(
            canvas_rect_to_physical((320, 180, 2, 2), small),
            (0, 0, 2, 2)
        );
        assert_eq!(fit((100, 50)).scale, 1, "never below one");
        // Just under 2x: rounding gave 2 and a canvas larger than the window.
        assert_eq!(fit((2559, 1440)).scale, 1);
        assert_eq!(fit((2560, 1439)).scale, 1);
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
        assert_eq!(cell(0, 0), (1, 0));
        assert_eq!(
            cell(VIEWPORT_COLUMNS, 0).0,
            VIEWPORT.right() + 1,
            "the column past the viewport grid clears it by a pixel"
        );
        assert!(VIEWPORT.encloses(MENU_BOX), "{MENU_BOX:?}");
        let (x, y) = menu_cell(0, 0);
        let area = Rect::new(
            x,
            y,
            (MENU_COLUMNS * CELL.0) as u32,
            (MENU_ROWS * CELL.1) as u32,
        );
        assert!(
            MENU_BOX.encloses(area) && x - MENU_BOX.x >= 5,
            "room for the marker"
        );
        let (w, h) = (CANVAS_WIDTH as i32, CANVAS_HEIGHT as i32);
        assert!(COLUMNS * CELL.0 <= w && (COLUMNS + 1) * CELL.0 > w);
        assert!(ROWS * CELL.1 <= h && (ROWS + 1) * CELL.1 > h);
        assert!(
            HUD_LINES[2].1 + CELL.1 <= PAD.y,
            "the location lines end above the pad"
        );
    }
}
