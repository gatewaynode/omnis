//! A canvas-sized RGBA buffer the UI paints into: fills, one-pixel frames, glyphs, text, and
//! the selection marker, all clipped to the canvas. Bevy-free so a whole screen can be checked
//! pixel by pixel in tests; the plugin uploads the bytes into a sprite.

use crate::font::{self, GLYPH_HEIGHT, GLYPH_WIDTH};
use crate::layout::{CANVAS_HEIGHT, CANVAS_WIDTH, CELL, Rect};

/// An RGB colour.
pub type Rgb = (u8, u8, u8);

/// The buffer; transparent where nothing was painted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA bytes, row-major from the top-left.
    pub rgba: Vec<u8>,
}

impl Default for Raster {
    fn default() -> Self {
        Raster::new(CANVAS_WIDTH, CANVAS_HEIGHT)
    }
}

impl Raster {
    /// A transparent buffer of the given size.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Raster {
        Raster {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    /// Everything transparent again.
    pub fn clear(&mut self) {
        self.rgba.fill(0);
    }

    /// The pixel's RGBA, or `None` outside the buffer.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        Some([
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ])
    }

    /// Paint one opaque pixel; outside the buffer is ignored.
    pub fn set(&mut self, x: i32, y: i32, color: Rgb) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&[color.0, color.1, color.2, 255]);
    }

    /// Fill a rectangle.
    pub fn fill(&mut self, rect: Rect, color: Rgb) {
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                self.set(x, y, color);
            }
        }
    }

    /// A one-pixel frame just inside the rectangle.
    pub fn stroke(&mut self, rect: Rect, color: Rgb) {
        if rect.w == 0 || rect.h == 0 {
            return;
        }
        for x in rect.x..rect.right() {
            self.set(x, rect.y, color);
            self.set(x, rect.bottom() - 1, color);
        }
        for y in rect.y..rect.bottom() {
            self.set(rect.x, y, color);
            self.set(rect.right() - 1, y, color);
        }
    }

    /// One glyph with its top-left at `(x, y)`.
    pub fn glyph(&mut self, c: char, x: i32, y: i32, color: Rgb) {
        let glyph = font::glyph(c);
        for row in 0..GLYPH_HEIGHT {
            for column in 0..GLYPH_WIDTH {
                if font::ink(glyph, column, row) {
                    self.set(x + column, y + row, color);
                }
            }
        }
    }

    /// Text from `(x, y)`, one cell per character.
    pub fn text(&mut self, x: i32, y: i32, text: &str, color: Rgb) {
        for (i, c) in text.chars().enumerate() {
            if c != ' ' {
                self.glyph(c, x + i as i32 * CELL.0, y, color);
            }
        }
    }

    /// The selection marker: a 3×5 triangle pointing right, top-left at `(x, y)`.
    pub fn marker(&mut self, x: i32, y: i32, color: Rgb) {
        for (row, width) in [1, 2, 3, 2, 1].into_iter().enumerate() {
            for column in 0..width {
                self.set(x + column, y + row as i32, color);
            }
        }
    }

    /// A stable hash of the bytes, for goldens.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        omnis_sim::omnis_core::fnv1a64(&self.rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INK: Rgb = (200, 100, 50);

    fn painted(r: &Raster) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for y in 0..r.height as i32 {
            for x in 0..r.width as i32 {
                if r.get(x, y).unwrap()[3] != 0 {
                    out.push((x, y));
                }
            }
        }
        out
    }

    #[test]
    fn glyphs_paint_five_by_seven_inside_a_six_by_eight_cell() {
        let mut r = Raster::new(12, 16);
        r.glyph('A', 0, 0, INK);
        let cells = painted(&r);
        assert_eq!(cells.len(), 18, "A has 18 ink pixels");
        assert!(cells.iter().all(|&(x, y)| x < 5 && y < 7));
        assert_eq!(r.get(2, 0), Some([200, 100, 50, 255]));
        assert_eq!(r.get(5, 3), Some([0, 0, 0, 0]));
        assert_eq!(r.get(0, 7), Some([0, 0, 0, 0]));
    }

    #[test]
    fn text_advances_one_cell_and_clips_at_the_edge() {
        let mut r = Raster::new(320, 8);
        r.text(0, 0, "I I", INK);
        let cells = painted(&r);
        assert!(cells.iter().any(|&(x, _)| x == 2), "first I at column 2");
        assert!(
            cells.iter().all(|&(x, _)| !(5..12).contains(&x)),
            "the space is blank"
        );
        assert!(cells.iter().any(|&(x, _)| x == 14), "third glyph at 12 + 2");
        let mut edge = Raster::new(320, 8);
        edge.text(316, 0, "HH", INK);
        assert!(painted(&edge).iter().all(|&(x, _)| (316..320).contains(&x)));
        assert!(!painted(&edge).is_empty());
    }

    #[test]
    fn strokes_touch_only_the_border_and_fills_cover_the_rectangle() {
        let mut r = Raster::new(10, 10);
        r.stroke(Rect::new(2, 3, 4, 3), INK);
        let cells = painted(&r);
        assert_eq!(cells.len(), 4 + 4 + 1 + 1);
        assert!(r.get(3, 4).unwrap()[3] == 0, "the inside is untouched");
        assert!(r.get(2, 3).unwrap()[3] == 255 && r.get(5, 5).unwrap()[3] == 255);
        let mut f = Raster::new(10, 10);
        f.fill(Rect::new(8, 8, 5, 5), INK);
        assert_eq!(painted(&f).len(), 4, "clipped to the buffer");
        f.clear();
        assert!(painted(&f).is_empty());
        r.stroke(Rect::new(0, 0, 0, 5), INK);
        assert_eq!(painted(&r).len(), 10, "an empty rectangle paints nothing");
    }

    #[test]
    fn the_marker_is_a_three_by_five_triangle() {
        let mut r = Raster::new(8, 8);
        r.marker(1, 1, INK);
        assert_eq!(
            painted(&r),
            vec![
                (1, 1),
                (1, 2),
                (2, 2),
                (1, 3),
                (2, 3),
                (3, 3),
                (1, 4),
                (2, 4),
                (1, 5)
            ]
        );
    }

    #[test]
    fn fingerprints_change_with_the_pixels() {
        let mut a = Raster::default();
        let b = a.clone();
        assert_eq!(a.fingerprint(), b.fingerprint());
        a.set(319, 179, INK);
        assert_ne!(a.fingerprint(), b.fingerprint());
        assert_eq!(a.get(320, 0), None);
    }
}
