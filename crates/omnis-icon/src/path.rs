//! SVG path data to polylines: every command of the specification (`M L H V C S Q T A Z`, each in
//! both cases), implicit repeats and the compact syntax (`00-2`, `.01`, `1.124.992z`). Curves are
//! flattened to pieces short enough for a 24-unit icon drawn at any size the game uses; arcs by the
//! specification's endpoint-to-centre conversion (implementation notes F.6.5).

use crate::Error;
use crate::icon::Stroke;

/// A point in the icon's units.
pub type Point = [f32; 2];

/// Flattening: a curve's pieces, from the control polygon's length over this.
const PIECE: f32 = 0.25;
/// The most pieces one curve is cut into.
const PIECES_MAX: usize = 16;
/// An arc's step, in degrees.
const ARC_STEP_DEG: f32 = 15.0;

/// Parse `d` into strokes, one per subpath; a subpath closed by `Z` is a closed stroke.
///
/// # Errors
/// A command the specification lacks, a number that does not read, or a command short of its
/// numbers.
pub fn parse(d: &str) -> Result<Vec<Stroke>, Error> {
    let mut reader = Reader {
        s: d.as_bytes(),
        at: 0,
    };
    let mut out: Vec<Stroke> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let mut pen: Point = [0.0, 0.0];
    let mut start: Point = [0.0, 0.0];
    // The last control point, for the smooth curves.
    let mut last_cubic: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut command: Option<u8> = None;
    loop {
        reader.skip_separators();
        let Some(&b) = reader.s.get(reader.at) else {
            break;
        };
        if b.is_ascii_alphabetic() {
            command = Some(b);
            reader.at += 1;
            if b == b'Z' || b == b'z' {
                if !current.is_empty() {
                    out.push(Stroke {
                        points: core::mem::take(&mut current),
                        closed: true,
                    });
                }
                pen = start;
                last_cubic = None;
                last_quad = None;
                continue;
            }
        } else if command.is_none() {
            return Err(Error::Path(format!(
                "a number before any command at byte {}",
                reader.at
            )));
        }
        let Some(c) = command else {
            break;
        };
        let relative = c.is_ascii_lowercase();
        let rel = |p: Point, pen: Point| {
            if relative {
                [pen[0] + p[0], pen[1] + p[1]]
            } else {
                p
            }
        };
        let mut moved_cubic = None;
        let mut moved_quad = None;
        match c.to_ascii_uppercase() {
            b'M' => {
                let p = rel(reader.point()?, pen);
                if !current.is_empty() {
                    out.push(Stroke {
                        points: core::mem::take(&mut current),
                        closed: false,
                    });
                }
                current.push(p);
                pen = p;
                start = p;
                // The numbers after a move are lines.
                command = Some(if relative { b'l' } else { b'L' });
            }
            b'L' => {
                let p = rel(reader.point()?, pen);
                line_to(&mut current, pen, p);
                pen = p;
            }
            b'H' => {
                let x = reader.number()?;
                let p = [if relative { pen[0] + x } else { x }, pen[1]];
                line_to(&mut current, pen, p);
                pen = p;
            }
            b'V' => {
                let y = reader.number()?;
                let p = [pen[0], if relative { pen[1] + y } else { y }];
                line_to(&mut current, pen, p);
                pen = p;
            }
            b'C' => {
                let (c1, c2, p) = (
                    rel(reader.point()?, pen),
                    rel(reader.point()?, pen),
                    rel(reader.point()?, pen),
                );
                cubic(&mut current, pen, c1, c2, p);
                moved_cubic = Some(c2);
                pen = p;
            }
            b'S' => {
                let c1 = last_cubic.map_or(pen, |c| reflect(c, pen));
                let (c2, p) = (rel(reader.point()?, pen), rel(reader.point()?, pen));
                cubic(&mut current, pen, c1, c2, p);
                moved_cubic = Some(c2);
                pen = p;
            }
            b'Q' => {
                let (c1, p) = (rel(reader.point()?, pen), rel(reader.point()?, pen));
                quad(&mut current, pen, c1, p);
                moved_quad = Some(c1);
                pen = p;
            }
            b'T' => {
                let c1 = last_quad.map_or(pen, |c| reflect(c, pen));
                let p = rel(reader.point()?, pen);
                quad(&mut current, pen, c1, p);
                moved_quad = Some(c1);
                pen = p;
            }
            b'A' => {
                let (rx, ry, rotation) = (reader.number()?, reader.number()?, reader.number()?);
                let (large, sweep) = (reader.flag()?, reader.flag()?);
                let p = rel(reader.point()?, pen);
                arc(&mut current, pen, p, [rx, ry], rotation, large, sweep);
                pen = p;
            }
            other => {
                return Err(Error::Path(format!("unknown command '{}'", other as char)));
            }
        }
        last_cubic = moved_cubic;
        last_quad = moved_quad;
    }
    if !current.is_empty() {
        out.push(Stroke {
            points: current,
            closed: false,
        });
    }
    Ok(out)
}

fn line_to(current: &mut Vec<Point>, pen: Point, p: Point) {
    if current.is_empty() {
        current.push(pen);
    }
    current.push(p);
}

/// The control point mirrored through the pen.
fn reflect(control: Point, pen: Point) -> Point {
    [2.0 * pen[0] - control[0], 2.0 * pen[1] - control[1]]
}

fn distance(a: Point, b: Point) -> f32 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

/// How many pieces a curve with this control polygon length is cut into.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn pieces(length: f32) -> usize {
    ((length / PIECE).ceil() as usize).clamp(2, PIECES_MAX)
}

fn cubic(current: &mut Vec<Point>, p0: Point, c1: Point, c2: Point, p3: Point) {
    if current.is_empty() {
        current.push(p0);
    }
    let n = pieces(distance(p0, c1) + distance(c1, c2) + distance(c2, p3));
    for i in 1..=n {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f32 / n as f32;
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        current.push([
            a * p0[0] + b * c1[0] + c * c2[0] + d * p3[0],
            a * p0[1] + b * c1[1] + c * c2[1] + d * p3[1],
        ]);
    }
}

fn quad(current: &mut Vec<Point>, p0: Point, c1: Point, p2: Point) {
    if current.is_empty() {
        current.push(p0);
    }
    let n = pieces(distance(p0, c1) + distance(c1, p2));
    for i in 1..=n {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f32 / n as f32;
        let u = 1.0 - t;
        let (a, b, c) = (u * u, 2.0 * u * t, t * t);
        current.push([
            a * p0[0] + b * c1[0] + c * p2[0],
            a * p0[1] + b * c1[1] + c * p2[1],
        ]);
    }
}

/// An elliptical arc from `p1` to `p2` (SVG implementation notes F.6.5 and F.6.6): the centre
/// found from the endpoints, the radii scaled up when too small, the sweep flattened by
/// `ARC_STEP_DEG`. A zero radius is a line, as the specification says.
pub fn arc(
    current: &mut Vec<Point>,
    p1: Point,
    p2: Point,
    [rx, ry]: [f32; 2],
    rotation_deg: f32,
    large: bool,
    sweep: bool,
) {
    if current.is_empty() {
        current.push(p1);
    }
    if rx == 0.0 || ry == 0.0 || p1 == p2 {
        current.push(p2);
        return;
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    let phi = rotation_deg.to_radians();
    let (sin, cos) = phi.sin_cos();
    // The endpoints in the ellipse's frame, about their midpoint.
    let dx = (p1[0] - p2[0]) / 2.0;
    let dy = (p1[1] - p2[1]) / 2.0;
    let x1 = cos * dx + sin * dy;
    let y1 = -sin * dx + cos * dy;
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        let k = lambda.sqrt();
        rx *= k;
        ry *= k;
    }
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let coef = sign * (num / den).max(0.0).sqrt();
    let cx1 = coef * rx * y1 / ry;
    let cy1 = coef * -ry * x1 / rx;
    let cx = cos * cx1 - sin * cy1 + (p1[0] + p2[0]) / 2.0;
    let cy = sin * cx1 + cos * cy1 + (p1[1] + p2[1]) / 2.0;
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let dot = ux * vx + uy * vy;
        let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let mut a = (dot / len).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 {
            a = -a;
        }
        a
    };
    let theta1 = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle(
        (x1 - cx1) / rx,
        (y1 - cy1) / ry,
        (-x1 - cx1) / rx,
        (-y1 - cy1) / ry,
    );
    let tau = core::f32::consts::TAU;
    if !sweep && delta > 0.0 {
        delta -= tau;
    } else if sweep && delta < 0.0 {
        delta += tau;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = ((delta.abs().to_degrees() / ARC_STEP_DEG).ceil() as usize).max(1);
    for i in 1..=n {
        #[allow(clippy::cast_precision_loss)]
        let t = theta1 + delta * i as f32 / n as f32;
        let (st, ct) = t.sin_cos();
        let ex = rx * ct;
        let ey = ry * st;
        current.push([cos * ex - sin * ey + cx, sin * ex + cos * ey + cy]);
    }
    // The last point is the endpoint exactly, whatever the rounding.
    if let Some(last) = current.last_mut() {
        *last = p2;
    }
}

struct Reader<'a> {
    s: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn skip_separators(&mut self) {
        while self.at < self.s.len()
            && (self.s[self.at].is_ascii_whitespace() || self.s[self.at] == b',')
        {
            self.at += 1;
        }
    }

    /// A number as path data writes it: an optional sign, digits, a fraction, an exponent; the
    /// next number may follow with no separator (`1.124.992`, `00-2`).
    fn number(&mut self) -> Result<f32, Error> {
        self.skip_separators();
        let start = self.at;
        if matches!(self.s.get(self.at), Some(b'+' | b'-')) {
            self.at += 1;
        }
        let mut digits = 0;
        while self.s.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
            digits += 1;
        }
        if self.s.get(self.at) == Some(&b'.') {
            self.at += 1;
            while self.s.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            return Err(Error::Path(format!("a number at byte {start}")));
        }
        if matches!(self.s.get(self.at), Some(b'e' | b'E')) {
            let save = self.at;
            self.at += 1;
            if matches!(self.s.get(self.at), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.s.get(self.at).is_some_and(u8::is_ascii_digit) {
                while self.s.get(self.at).is_some_and(u8::is_ascii_digit) {
                    self.at += 1;
                }
            } else {
                self.at = save;
            }
        }
        let text = core::str::from_utf8(&self.s[start..self.at])
            .map_err(|_| Error::Path("utf-8".into()))?;
        text.parse::<f32>()
            .map_err(|_| Error::Path(format!("\"{text}\" is not a number")))
    }

    /// An arc's flag: one digit, `0` or `1`, which the next number may follow at once.
    fn flag(&mut self) -> Result<bool, Error> {
        self.skip_separators();
        match self.s.get(self.at) {
            Some(b'0') => {
                self.at += 1;
                Ok(false)
            }
            Some(b'1') => {
                self.at += 1;
                Ok(true)
            }
            _ => Err(Error::Path(format!("an arc flag at byte {}", self.at))),
        }
    }

    fn point(&mut self) -> Result<Point, Error> {
        Ok([self.number()?, self.number()?])
    }
}
