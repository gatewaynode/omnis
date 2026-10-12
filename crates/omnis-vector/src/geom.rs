//! World-space constants and conversions (presentation-ARCHITECTURE.md §4).
//!
//! The ground is the X–Z plane, Y is up. Cell `(x, y)` covers `x ≤ X < x+1`, `y ≤ Z < y+1`;
//! north (−y) is −Z. Yaw 0 looks north and positive yaw turns counter-clockwise seen from above,
//! so +90° looks west, matching Bevy's `Quat::from_rotation_y`.

use core::f32::consts::{FRAC_PI_2, PI, TAU};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::Terrain;

/// The camera's height above the floor, in cells.
pub const EYE_HEIGHT: f32 = 0.5;

/// How tall a wall stands on this terrain. 1.0 for every terrain for now (owner, 2026-10-02);
/// the one place to vary it later.
#[must_use]
pub fn wall_height(_terrain: &Terrain) -> f32 {
    1.0
}

/// The centre of cell `(x, y)` on the ground plane, as `(X, Z)`.
#[must_use]
pub fn cell_centre(x: u16, y: u16) -> (f32, f32) {
    (f32::from(x) + 0.5, f32::from(y) + 0.5)
}

/// The yaw that looks straight along `facing`.
#[must_use]
pub fn yaw_of(facing: Facing) -> f32 {
    match facing {
        Facing::North => 0.0,
        Facing::West => FRAC_PI_2,
        Facing::South => PI,
        Facing::East => -FRAC_PI_2,
    }
}

/// The unit forward vector `(dX, dZ)` for a yaw.
#[must_use]
pub fn forward(yaw: f32) -> (f32, f32) {
    (-yaw.sin(), -yaw.cos())
}

/// The unit right vector `(dX, dZ)` for a yaw.
#[must_use]
pub fn right(yaw: f32) -> (f32, f32) {
    (yaw.cos(), -yaw.sin())
}

/// An angle wrapped into `(-π, π]`.
#[must_use]
pub fn wrap(angle: f32) -> f32 {
    let a = angle.rem_euclid(TAU);
    if a > PI { a - TAU } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5
    }

    #[test]
    fn forward_follows_each_facing() {
        assert!(close(forward(yaw_of(Facing::North)), (0.0, -1.0)));
        assert!(close(forward(yaw_of(Facing::East)), (1.0, 0.0)));
        assert!(close(forward(yaw_of(Facing::South)), (0.0, 1.0)));
        assert!(close(forward(yaw_of(Facing::West)), (-1.0, 0.0)));
    }

    #[test]
    fn right_is_a_quarter_turn_clockwise_of_forward() {
        for facing in Facing::ALL {
            let yaw = yaw_of(facing);
            assert!(close(right(yaw), forward(yaw_of(facing.right()))));
        }
    }

    #[test]
    fn wrap_stays_in_the_half_open_range() {
        assert!((wrap(3.0 * PI) - PI).abs() < 1e-5);
        assert!((wrap(-PI) - PI).abs() < 1e-5);
        assert!((wrap(TAU + 0.5) - 0.5).abs() < 1e-5);
    }
}
