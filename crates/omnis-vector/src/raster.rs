//! An RGBA picture in memory, for the pictures the shell uploads as images: the minimap, the
//! fight's picture window and the editor's plan. Bevy-free. The pixel helpers draw a grid's
//! edges, ramps and joins at a whole number of pixels a cell; the minimap and the plan share
//! them (M5.4).

use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Join;

/// An edge of a grid: horizontal (a north side) or vertical (a west side), by the column and
/// row of the cell whose north or west side it is, so an edge shared by two cells has one key.
/// The column and row are in the picture's cells, which may start before the map's.
pub type EdgeKey = (bool, i64, i64);

/// The key of the `facing` side of cell `(x, y)`.
#[must_use]
pub fn edge_key(x: i64, y: i64, facing: Facing) -> EdgeKey {
    match facing {
        Facing::North => (true, x, y),
        Facing::South => (true, x, y + 1),
        Facing::West => (false, x, y),
        Facing::East => (false, x + 1, y),
    }
}

/// The pixels of a tick across the middle of an edge, half a cell long, at `scale` pixels a cell.
#[must_use]
pub fn tick_pixels(scale: u32, (horizontal, col, row): EdgeKey) -> Vec<(i64, i64)> {
    let s = i64::from(scale);
    let (mx, my) = if horizontal {
        (col * s + s / 2, row * s)
    } else {
        (col * s, row * s + s / 2)
    };
    (-s / 4..=s / 4)
        .map(|i| {
            if horizontal {
                (mx, my + i)
            } else {
                (mx + i, my)
            }
        })
        .collect()
}

/// The pixels of a straight line from `a` to `b`, both ends included.
#[must_use]
pub fn line_pixels(a: (i64, i64), b: (i64, i64)) -> Vec<(i64, i64)> {
    let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
    (0..=n)
        .map(|i| (a.0 + (b.0 - a.0) * i / n, a.1 + (b.1 - a.1) * i / n))
        .collect()
}

/// The pixels of a join's mark on cell `(col, row)`, drawn like its glyph: a chevron pointing
/// the way a stair rises (`rise`), with a bar across the foot for a stair down; a ladder's
/// rails and rungs; a hole's cross.
#[must_use]
pub fn join_pixels(
    scale: u32,
    (col, row): (i64, i64),
    join: Join,
    rise: Facing,
) -> Vec<(i64, i64)> {
    let s = i64::from(scale);
    let (x0, y0) = (col * s, row * s);
    let at = |fx: i64, fy: i64| (x0 + s * fx / 8, y0 + s * fy / 8);
    // A stair's strokes are written rising north, then turned to its rise.
    let turned = |fx: i64, fy: i64| match rise {
        Facing::North => at(fx, fy),
        Facing::East => at(8 - fy, fx),
        Facing::South => at(8 - fx, 8 - fy),
        Facing::West => at(fy, 8 - fx),
    };
    let chevron = [(turned(2, 5), turned(4, 3)), (turned(4, 3), turned(6, 5))];
    let strokes = match join {
        Join::Up => chevron.to_vec(),
        Join::Down => [chevron.as_slice(), &[(turned(2, 6), turned(6, 6))]].concat(),
        Join::Ladder => vec![
            (at(3, 1), at(3, 7)),
            (at(5, 1), at(5, 7)),
            (at(3, 2), at(5, 2)),
            (at(3, 4), at(5, 4)),
            (at(3, 6), at(5, 6)),
        ],
        Join::Hole => vec![(at(2, 2), at(6, 6)), (at(6, 2), at(2, 6))],
    };
    strokes
        .into_iter()
        .flat_map(|(a, b)| line_pixels(a, b))
        .collect()
}

/// The pixels of an edge, `inset` pixels in from each end, in a picture `width × height`
/// pixels at `scale` a cell. An edge on the picture's far side is drawn on the last pixel row
/// or column.
#[must_use]
pub fn edge_pixels(
    scale: u32,
    (width, height): (u32, u32),
    (horizontal, col, row): EdgeKey,
    inset: i64,
) -> Vec<(i64, i64)> {
    let s = i64::from(scale);
    let (x0, y0) = (
        (col * s).min(i64::from(width) - 1),
        (row * s).min(i64::from(height) - 1),
    );
    (inset..s - inset)
        .map(|i| {
            if horizontal {
                (col * s + i, y0)
            } else {
                (x0, row * s + i)
            }
        })
        .collect()
}

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
