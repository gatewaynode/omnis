//! Canvas geometry shared by the renderer and the draw planner. Internal resolution is
//! provisional (PRD §14): 320×180 with a 240×135 viewport in the top-left, a right column for
//! the party panel (M3), and a bottom band for messages.

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
/// Where the large automap overlay sits on the canvas.
pub const OVERLAY_MAP_ORIGIN: (i32, i32) = (8, 8);
/// Pixels per tile in the large automap overlay.
pub const OVERLAY_MAP_SCALE: i32 = 4;

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
