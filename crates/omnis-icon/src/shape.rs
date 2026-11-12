//! The set's shapes other than paths, as strokes: a circle or an ellipse as a polygon of
//! `SIDES`, a rectangle with rounded corners through the path code's arcs, a line, a polyline
//! and a polygon.

use crate::Error;
use crate::icon::Stroke;
use crate::path::{self, Point};
use crate::svg::Element;

/// The sides a circle or an ellipse is drawn with.
pub const SIDES: usize = 32;

/// The strokes of a child element that is not a path.
///
/// # Errors
/// A number that does not read, or a points list of odd length.
pub fn strokes(element: &Element) -> Result<Vec<Stroke>, Error> {
    Ok(match element.name.as_str() {
        "circle" => {
            let r = element.number("r", 0.0)?;
            vec![ellipse(
                [element.number("cx", 0.0)?, element.number("cy", 0.0)?],
                [r, r],
            )]
        }
        "ellipse" => vec![ellipse(
            [element.number("cx", 0.0)?, element.number("cy", 0.0)?],
            [element.number("rx", 0.0)?, element.number("ry", 0.0)?],
        )],
        "rect" => {
            let (x, y) = (element.number("x", 0.0)?, element.number("y", 0.0)?);
            let (w, h) = (
                element.number("width", 0.0)?,
                element.number("height", 0.0)?,
            );
            let rx = element.number("rx", 0.0)?;
            let ry = element.number("ry", rx)?;
            let rx = rx.min(w / 2.0).max(0.0);
            let ry = ry.min(h / 2.0).max(0.0);
            if rx == 0.0 || ry == 0.0 {
                vec![Stroke {
                    points: vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
                    closed: true,
                }]
            } else {
                // The rounded rectangle as a path, so its corners are the arcs' own.
                let d = format!(
                    "M{} {}H{}A{rx} {ry} 0 0 1 {} {}V{}A{rx} {ry} 0 0 1 {} {}H{}A{rx} {ry} 0 0 1 {} {}V{}A{rx} {ry} 0 0 1 {} {}Z",
                    x + rx,
                    y,
                    x + w - rx,
                    x + w,
                    y + ry,
                    y + h - ry,
                    x + w - rx,
                    y + h,
                    x + rx,
                    x,
                    y + h - ry,
                    y + ry,
                    x + rx,
                    y,
                );
                path::parse(&d)?
            }
        }
        "line" => vec![Stroke {
            points: vec![
                [element.number("x1", 0.0)?, element.number("y1", 0.0)?],
                [element.number("x2", 0.0)?, element.number("y2", 0.0)?],
            ],
            closed: false,
        }],
        "polyline" | "polygon" => vec![Stroke {
            points: points(element.attr("points").unwrap_or(""))?,
            closed: element.name == "polygon",
        }],
        other => return Err(Error::UnknownElement(other.to_owned())),
    })
}

/// A full ellipse about `c` with radii `r`, as a closed polygon.
fn ellipse(c: Point, r: Point) -> Stroke {
    let points = (0..SIDES)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let a = core::f32::consts::TAU * i as f32 / SIDES as f32;
            [c[0] + r[0] * a.cos(), c[1] + r[1] * a.sin()]
        })
        .collect();
    Stroke {
        points,
        closed: true,
    }
}

/// A `points` list: numbers separated by spaces or commas, in pairs.
fn points(text: &str) -> Result<Vec<Point>, Error> {
    let numbers: Result<Vec<f32>, Error> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<f32>()
                .map_err(|_| Error::Malformed(format!("points: \"{s}\" is not a number")))
        })
        .collect();
    let numbers = numbers?;
    if numbers.len() % 2 != 0 {
        return Err(Error::Malformed(
            "points: an odd count of numbers".to_owned(),
        ));
    }
    Ok(numbers.chunks(2).map(|p| [p[0], p[1]]).collect())
}
