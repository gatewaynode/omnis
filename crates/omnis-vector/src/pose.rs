//! The continuous pose and its integration (presentation-ARCHITECTURE.md §5).

use crate::geom::{cell_centre, forward, right, wrap, yaw_of};
use omnis_sim::omnis_core::Position;

/// Where the camera stands on the ground plane and which way it looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// World X.
    pub x: f32,
    /// World Z.
    pub z: f32,
    /// Yaw in radians; 0 looks north.
    pub yaw: f32,
}

impl Pose {
    /// Standing at the centre of the simulation's cell, looking along its facing.
    #[must_use]
    pub fn at(position: Position) -> Pose {
        let (x, z) = cell_centre(position.x, position.y);
        Pose {
            x,
            z,
            yaw: yaw_of(position.facing),
        }
    }

    /// The ground point.
    #[must_use]
    pub const fn ground(self) -> (f32, f32) {
        (self.x, self.z)
    }
}

/// What the player asks for this frame, each axis in `-1.0..=1.0`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Motion {
    /// Forward (+) or back (−).
    pub forward: f32,
    /// Right (+) or left (−).
    pub strafe: f32,
}

impl Motion {
    /// Whether nothing is asked for.
    #[must_use]
    pub fn is_idle(self) -> bool {
        self.forward == 0.0 && self.strafe == 0.0
    }
}

/// The pose after `dt` seconds of `motion` at `speed` cells a second, with the yaw changed by
/// `yaw_delta` radians. Diagonal input is normalised so it is no faster than straight.
#[must_use]
pub fn integrate(pose: Pose, motion: Motion, yaw_delta: f32, speed: f32, dt: f32) -> Pose {
    let yaw = wrap(pose.yaw + yaw_delta);
    let (fx, fz) = forward(yaw);
    let (rx, rz) = right(yaw);
    let (mut a, mut b) = (
        motion.forward.clamp(-1.0, 1.0),
        motion.strafe.clamp(-1.0, 1.0),
    );
    let len = a.hypot(b);
    if len > 1.0 {
        a /= len;
        b /= len;
    }
    let step = speed * dt;
    Pose {
        x: pose.x + (fx * a + rx * b) * step,
        z: pose.z + (fz * a + rz * b) * step,
        yaw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_core::{Facing, MapId};

    #[test]
    fn forward_from_a_south_facing_start_increases_z() {
        let start = Pose::at(Position {
            map: MapId(0),
            x: 1,
            y: 0,
            facing: Facing::South,
        });
        let moved = integrate(
            start,
            Motion {
                forward: 1.0,
                strafe: 0.0,
            },
            0.0,
            2.0,
            0.5,
        );
        assert!((moved.x - 1.5).abs() < 1e-5);
        assert!((moved.z - 1.5).abs() < 1e-5);
    }

    #[test]
    fn diagonal_input_is_not_faster() {
        let start = Pose {
            x: 0.0,
            z: 0.0,
            yaw: 0.0,
        };
        let moved = integrate(
            start,
            Motion {
                forward: 1.0,
                strafe: 1.0,
            },
            0.0,
            1.0,
            1.0,
        );
        assert!((moved.x.hypot(moved.z) - 1.0).abs() < 1e-5);
    }
}
