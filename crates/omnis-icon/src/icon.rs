//! An icon as strokes in its 24-unit space, read from its SVG text, and its lines at any size.

use crate::Error;
use crate::{path, shape, svg};

/// The side of every icon's box, in its units.
pub const SIDE: f32 = 24.0;
/// Every stroke's width, in the icon's units; round caps and joins.
pub const WIDTH: f32 = 2.0;

/// One polyline of an icon.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// Its points, in the icon's units (0 to 24 each way, y down).
    pub points: Vec<[f32; 2]>,
    /// Whether the last point joins the first.
    pub closed: bool,
}

/// An icon: its name and its strokes.
#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    /// Its name in the set (`door-open`).
    pub name: String,
    /// Its strokes, in the file's order.
    pub strokes: Vec<Stroke>,
}

impl Icon {
    /// Read `text`, an SVG of the set's subset, as the icon `name`.
    ///
    /// # Errors
    /// The text is outside the subset or does not read (`Error`).
    pub fn parse(name: &str, text: &str) -> Result<Icon, Error> {
        let mut strokes = Vec::new();
        for element in svg::read(text)? {
            if element.name == "path" {
                let d = element.attr("d").unwrap_or("");
                strokes.extend(path::parse(d)?);
            } else {
                strokes.extend(shape::strokes(&element)?);
            }
        }
        Ok(Icon {
            name: name.to_owned(),
            strokes,
        })
    }

    /// The icon's lines at `size` pixels a side: each stroke's consecutive points, and the
    /// closing line of a closed stroke, scaled from the 24-unit box.
    pub fn segments(&self, size: f32) -> impl Iterator<Item = [[f32; 2]; 2]> + '_ {
        let k = size / SIDE;
        self.strokes.iter().flat_map(move |s| {
            let n = s.points.len();
            let pairs = if s.closed && n > 2 {
                n
            } else {
                n.saturating_sub(1)
            };
            (0..pairs).map(move |i| {
                let a = s.points[i];
                let b = s.points[(i + 1) % n];
                [[a[0] * k, a[1] * k], [b[0] * k, b[1] * k]]
            })
        })
    }

    /// The stroke width at `size` pixels a side.
    #[must_use]
    pub fn width(size: f32) -> f32 {
        WIDTH * size / SIDE
    }

    /// The box the strokes' points lie in: `[min x, min y, max x, max y]`; none for no points.
    #[must_use]
    pub fn bounds(&self) -> Option<[f32; 4]> {
        let mut points = self.strokes.iter().flat_map(|s| s.points.iter());
        let first = *points.next()?;
        Some(
            points.fold([first[0], first[1], first[0], first[1]], |b, p| {
                [
                    b[0].min(p[0]),
                    b[1].min(p[1]),
                    b[2].max(p[0]),
                    b[3].max(p[1]),
                ]
            }),
        )
    }
}
