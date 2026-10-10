//! An RGBA picture in memory, for the pictures the shell uploads as images: the minimap and
//! the fight's picture window. Bevy-free.

/// An RGBA picture, row-major, four bytes a pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Pixels a cell (the minimap's grid; 1 for a picture with no grid).
    pub scale: u32,
    /// The bytes.
    pub rgba: Vec<u8>,
}

impl Raster {
    /// A picture of one colour.
    #[must_use]
    pub fn filled(width: u32, height: u32, scale: u32, colour: [u8; 4]) -> Raster {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..width * height {
            rgba.extend_from_slice(&colour);
        }
        Raster {
            width,
            height,
            scale,
            rgba,
        }
    }

    /// The pixel at `(x, y)`, if inside.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y * self.width + x) * 4) as usize;
        self.rgba.get(i..i + 4).map(|p| [p[0], p[1], p[2], p[3]])
    }

    /// Set the pixel at `(x, y)`; outside is ignored.
    pub fn put(&mut self, x: i64, y: i64, colour: [u8; 4]) {
        self.blend(x, y, colour, false);
    }

    /// Brighten the pixel at `(x, y)` to `colour`, channel by channel, so overlapping strokes
    /// keep the brightest; outside is ignored.
    pub fn lighten(&mut self, x: i64, y: i64, colour: [u8; 4]) {
        self.blend(x, y, colour, true);
    }

    fn blend(&mut self, x: i64, y: i64, colour: [u8; 4], max: bool) {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return;
        };
        if x >= self.width || y >= self.height {
            return;
        }
        let i = ((y * self.width + x) * 4) as usize;
        let pixel = &mut self.rgba[i..i + 4];
        for (p, c) in pixel.iter_mut().zip(colour) {
            *p = if max { (*p).max(c) } else { c };
        }
    }

    /// A stroke from `a` to `b` in pixels, `radius` pixels either side of the line, lightening.
    pub fn stroke(&mut self, a: (f32, f32), b: (f32, f32), radius: f32, colour: [u8; 4]) {
        let length = (b.0 - a.0).hypot(b.1 - a.1);
        // Half-pixel steps leave no gaps.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (length * 2.0).ceil().max(1.0) as u32;
        #[allow(clippy::cast_possible_truncation)]
        let r = radius.ceil() as i64;
        for i in 0..=steps {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f32 / steps as f32;
            let (cx, cy) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            #[allow(clippy::cast_possible_truncation)]
            let (px, py) = (cx.round() as i64, cy.round() as i64);
            for dy in -r..=r {
                for dx in -r..=r {
                    #[allow(clippy::cast_precision_loss)]
                    if ((dx * dx + dy * dy) as f32) <= radius * radius {
                        self.lighten(px + dx, py + dy, colour);
                    }
                }
            }
        }
    }
}
