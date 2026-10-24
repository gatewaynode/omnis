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

/// The pixels of the outline of cell `(col, row)`'s square at `scale` pixels a cell, `inset`
/// pixels in from each side.
#[must_use]
pub fn rect_pixels(scale: u32, (col, row): (i64, i64), inset: i64) -> Vec<(i64, i64)> {
    let s = i64::from(scale);
    let (left, top) = (col * s + inset, row * s + inset);
    let (right, bottom) = (col * s + s - 1 - inset, row * s + s - 1 - inset);
    if right <= left || bottom <= top {
        return vec![(col * s + s / 2, row * s + s / 2)];
    }
    let mut out = Vec::new();
    out.extend(line_pixels((left, top), (right, top)));
    out.extend(line_pixels((right, top), (right, bottom)));
    out.extend(line_pixels((right, bottom), (left, bottom)));
    out.extend(line_pixels((left, bottom), (left, top)));
    out
}

/// The pixels of a circle of `radius` about `centre` (the midpoint algorithm); a point when the
/// radius is nought.
#[must_use]
pub fn circle_pixels((cx, cy): (i64, i64), radius: i64) -> Vec<(i64, i64)> {
    if radius <= 0 {
        return vec![(cx, cy)];
    }
    let mut out = Vec::new();
    let (mut x, mut y, mut d) = (radius, 0_i64, 1 - radius);
    while x >= y {
        for (dx, dy) in [
            (x, y),
            (y, x),
            (-y, x),
            (-x, y),
            (-x, -y),
            (-y, -x),
            (y, -x),
            (x, -y),
        ] {
            out.push((cx + dx, cy + dy));
        }
        y += 1;
        if d < 0 {
            d += 2 * y + 1;
        } else {
            x -= 1;
            d += 2 * (y - x) + 1;
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The pixels of a portal's mark on cell `(col, row)`: a rectangle a quarter in from each side
/// with a circle in the middle (owner, 2026-10-24; a door keeps its cross).
#[must_use]
pub fn portal_pixels(scale: u32, cell: (i64, i64)) -> Vec<(i64, i64)> {
    let s = i64::from(scale);
    let mut out = rect_pixels(scale, cell, s / 4);
    let centre = (cell.0 * s + s / 2, cell.1 * s + s / 2);
    out.extend(circle_pixels(centre, (s / 4).max(1)));
    out
}

/// The pixels of a filled triangle with its apex at `apex` and its base from `left` to `right`
/// on one row (`left.1 == right.1`), scanline by scanline.
#[must_use]
pub fn triangle_pixels(apex: (i64, i64), left: (i64, i64), right: (i64, i64)) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let rows = left.1 - apex.1;
    if rows == 0 {
        return line_pixels(left, right);
    }
    for y in apex.1.min(left.1)..=apex.1.max(left.1) {
        let t = (y - apex.1) * 1000 / rows;
        let x0 = apex.0 + (left.0 - apex.0) * t / 1000;
        let x1 = apex.0 + (right.0 - apex.0) * t / 1000;
        for x in x0.min(x1)..=x0.max(x1) {
            out.push((x, y));
        }
    }
    out
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

    /// An icon of the set drawn with the round brush: its strokes scaled to `size` pixels a side
    /// with their top left at `(left, top)`, the brush's radius the icon's stroke width (owner,
    /// 2026-10-24).
    pub fn icon(
        &mut self,
        icon: &omnis_icon::Icon,
        (left, top): (i64, i64),
        size: u32,
        colour: [u8; 4],
    ) {
        #[allow(clippy::cast_precision_loss)]
        let (size, left, top) = (size as f32, left as f32, top as f32);
        let radius = (omnis_icon::Icon::width(size) / 2.0).max(0.5);
        for [a, b] in icon.segments(size) {
            self.stroke(
                (left + a[0], top + a[1]),
                (left + b[0], top + b[1]),
                radius,
                colour,
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_icon_draws_inside_its_box_with_the_round_brush() {
        let icon = omnis_icon::get("square").expect("the square icon");
        let mut raster = Raster::filled(40, 40, 1, [0, 0, 0, 255]);
        raster.icon(&icon, (8, 8), 24, [255, 255, 255, 255]);
        let lit = |x: u32, y: u32| raster.pixel(x, y) == Some([255, 255, 255, 255]);
        let count = (0..40)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter(|&(x, y)| lit(x, y))
            .count();
        assert!(count > 40, "{count}");
        // Nothing outside the icon's box plus its brush, nothing in the box's hollow middle.
        for y in 0..40 {
            for x in 0..40 {
                if lit(x, y) {
                    assert!((6..=34).contains(&x) && (6..=34).contains(&y), "({x}, {y})");
                }
            }
        }
        assert!(!lit(20, 20), "the square is hollow");
    }
}
