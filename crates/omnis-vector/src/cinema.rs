//! The fight's picture window: a scene drawn in glowing vector lines above the choices
//! (presentation-ARCHITECTURE.md §9). Bevy-free; the shell uploads the pixels into an image.
//!
//! A stub for now. The window opens on the enemy, and every monster is drawn as the placeholder
//! rat. Later, scenes for the fight's events (a swing, a hit landing, a spell, a death) are
//! queued from the events as they come, and drawings per monster replace the rat.

use crate::raster::Raster;
use omnis_sim::omnis_core::MonsterId;
use omnis_sim::{Mode, World};

/// The window's size in logical pixels, to fit the fight panel's width.
pub const SIZE: (u32, u32) = (492, 200);

/// The window's ground: opaque, so the 3D view never shows through it.
pub const BACKGROUND: [u8; 4] = [0, 8, 3, 255];
/// The lines' core.
pub const LINE: [u8; 4] = [140, 255, 170, 255];
/// The glow around the lines.
pub const GLOW: [u8; 4] = [20, 90, 40, 255];

/// What the window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    /// The enemy faced: the first stack still standing.
    Enemy(MonsterId),
}

/// The scene a fight opens on, or `None` outside a fight or when no stack stands.
#[must_use]
pub fn opening(world: &World) -> Option<Scene> {
    let encounter = match &world.mode {
        Mode::Explore | Mode::Town(_) => return None,
        Mode::Encounter(e) => e,
        Mode::Combat(c) => &c.encounter,
    };
    encounter
        .stacks
        .iter()
        .find(|s| s.alive())
        .map(|s| Scene::Enemy(s.monster))
}

/// A drawing: polylines in a unit square, x to the right and y down.
pub type Drawing = Vec<Vec<(f32, f32)>>;

/// The scene's drawing. Every monster is the placeholder rat for now.
#[must_use]
pub fn drawing(scene: Scene) -> Drawing {
    match scene {
        Scene::Enemy(_) => rat(),
    }
}

/// An ellipse as a closed polyline of `n` points.
fn ellipse(c: (f32, f32), r: (f32, f32), n: u16) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let a = f32::from(i) / f32::from(n) * core::f32::consts::TAU;
            (c.0 + r.0 * a.cos(), c.1 + r.1 * a.sin())
        })
        .collect()
}

/// A quadratic curve from `a` to `b` bent toward `k`, in `n` pieces.
fn curve(a: (f32, f32), k: (f32, f32), b: (f32, f32), n: u16) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let t = f32::from(i) / f32::from(n);
            let u = 1.0 - t;
            (
                u * u * a.0 + 2.0 * u * t * k.0 + t * t * b.0,
                u * u * a.1 + 2.0 * u * t * k.1 + t * t * b.1,
            )
        })
        .collect()
}

/// The placeholder: a rat in profile, facing left toward the party, on a ground line.
fn rat() -> Drawing {
    vec![
        // Body.
        ellipse((0.52, 0.62), (0.22, 0.13), 32),
        // Head: a wedge to the snout.
        vec![(0.33, 0.53), (0.12, 0.64), (0.33, 0.72)],
        // Ear.
        ellipse((0.33, 0.50), (0.035, 0.045), 12),
        // Eye.
        ellipse((0.23, 0.6), (0.01, 0.01), 6),
        // Whiskers.
        vec![(0.13, 0.63), (0.04, 0.57)],
        vec![(0.13, 0.64), (0.03, 0.645)],
        vec![(0.13, 0.65), (0.05, 0.71)],
        // Legs.
        vec![(0.38, 0.73), (0.36, 0.83), (0.32, 0.83)],
        vec![(0.64, 0.74), (0.66, 0.83), (0.62, 0.83)],
        // Tail.
        curve((0.74, 0.64), (0.92, 0.80), (0.97, 0.48), 16),
        // Ground.
        vec![(0.02, 0.84), (0.98, 0.84)],
    ]
}

/// Pixels kept clear around the drawing.
pub const MARGIN: f32 = 12.0;

/// A drawing's bounds as `(x, y, width, height)`, never zero-sized.
#[must_use]
pub fn bounds(lines: &Drawing) -> (f32, f32, f32, f32) {
    let points = || lines.iter().flatten();
    let (x0, x1) = points().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (y0, y1) = points().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    (
        x0,
        y0,
        (x1 - x0).max(f32::EPSILON),
        (y1 - y0).max(f32::EPSILON),
    )
}

/// Where a drawing's points land in the rectangle `(x, y, w, h)`: its bounds scaled evenly to
/// fill the rectangle inside `margin`, and centred.
pub fn place(
    lines: &Drawing,
    (x, y, w, h): (f32, f32, f32, f32),
    margin: f32,
) -> impl Fn((f32, f32)) -> (f32, f32) + use<> {
    let (x0, y0, bw, bh) = bounds(lines);
    let scale = ((w - 2.0 * margin) / bw).min((h - 2.0 * margin) / bh);
    let origin = (x + (w - bw * scale) / 2.0, y + (h - bh * scale) / 2.0);
    move |(px, py)| (origin.0 + (px - x0) * scale, origin.1 + (py - y0) * scale)
}

/// The scene drawn at `width × height` pixels, fitted and centred, each line a glow under a
/// bright core.
#[must_use]
pub fn paint(scene: Scene, width: u32, height: u32) -> Raster {
    let mut out = Raster::filled(width, height, 1, BACKGROUND);
    let lines = drawing(scene);
    #[allow(clippy::cast_precision_loss)]
    let at = place(&lines, (0.0, 0.0, width as f32, height as f32), MARGIN);
    for (radius, colour) in [(3.0, GLOW), (1.0, LINE)] {
        for line in &lines {
            for pair in line.windows(2) {
                out.stroke(at(pair[0]), at(pair[1]), radius, colour);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(raster: &Raster, x: u32, y: u32) -> bool {
        raster.pixel(x, y).is_some_and(|p| p == LINE)
    }

    #[test]
    fn the_rat_stays_inside_its_square() {
        for line in rat() {
            assert!(line.len() >= 2);
            for (x, y) in line {
                assert!((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y));
            }
        }
    }

    #[test]
    fn the_scene_fills_the_window_centred_on_an_opaque_ground() {
        let (w, h) = SIZE;
        let raster = paint(Scene::Enemy(MonsterId(0)), w, h);
        assert_eq!((raster.width, raster.height), SIZE);
        assert!(
            raster.rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255),
            "opaque"
        );
        let lit_columns: Vec<u32> = (0..w)
            .filter(|&x| (0..h).any(|y| lit(&raster, x, y)))
            .collect();
        let lit_rows: Vec<u32> = (0..h)
            .filter(|&y| (0..w).any(|x| lit(&raster, x, y)))
            .collect();
        let (left, right) = (lit_columns[0], w - 1 - lit_columns[lit_columns.len() - 1]);
        let (top, bottom) = (lit_rows[0], h - 1 - lit_rows[lit_rows.len() - 1]);
        // The margin is kept, give or take the line's width.
        let m = MARGIN as u32;
        assert!(left.min(right).min(top).min(bottom) >= m - 2);
        // Centred, and filling the window's height (the rat is wider than tall, but not as
        // wide as the window).
        assert!(left.abs_diff(right) <= 2 && top.abs_diff(bottom) <= 2);
        assert!(top <= m + 2 && bottom <= m + 2, "fills the height");
    }

    #[test]
    fn the_rat_faces_left() {
        let (w, h) = SIZE;
        let raster = paint(Scene::Enemy(MonsterId(0)), w, h);
        #[allow(clippy::cast_precision_loss)]
        let at = place(&rat(), (0.0, 0.0, w as f32, h as f32), MARGIN);
        // The middle whisker's tip, and the point mirrored across the drawing's middle.
        let (tip, mirrored) = (at((0.03, 0.645)), at((0.97, 0.645)));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let on = |(x, y): (f32, f32)| lit(&raster, x.round() as u32, y.round() as u32);
        assert!(on(tip), "the snout is drawn at the left");
        assert!(!on(mirrored), "nothing at the mirrored point");
    }
}
