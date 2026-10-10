//! Pure grid math: which cell a point is in, which facing a yaw means, and which boundaries a
//! movement crosses, in order (alt-ARCHITECTURE.md §5.1–5.3).

use crate::geom::{wrap, yaw_of};
use core::f32::consts::FRAC_PI_4;
use omnis_sim::omnis_core::{Direction, Facing, Rotation};

/// The cell containing the ground point `(X, Z)`.
#[must_use]
pub fn cell_of(x: f32, z: f32) -> (i32, i32) {
    // Map coordinates fit in u16, so the floor of any reachable point fits in i32.
    #[allow(clippy::cast_possible_truncation)]
    (x.floor() as i32, z.floor() as i32)
}

/// The cardinal facing a yaw means, keeping `current` until the yaw is more than 45° plus
/// `margin` (radians) away from it, so a yaw near a diagonal does not flip back and forth.
#[must_use]
pub fn facing_of(yaw: f32, current: Facing, margin: f32) -> Facing {
    if wrap(yaw - yaw_of(current)).abs() <= FRAC_PI_4 + margin {
        return current;
    }
    nearest(yaw)
}

/// The cardinal facing nearest a yaw.
#[must_use]
pub fn nearest(yaw: f32) -> Facing {
    let mut best = Facing::North;
    let mut gap = f32::MAX;
    for facing in Facing::ALL {
        let d = wrap(yaw - yaw_of(facing)).abs();
        if d < gap {
            gap = d;
            best = facing;
        }
    }
    best
}

/// The step direction that moves toward the absolute `heading` while facing `facing`.
#[must_use]
pub fn relative(heading: Facing, facing: Facing) -> Direction {
    if heading == facing {
        Direction::Forward
    } else if heading == facing.opposite() {
        Direction::Back
    } else if heading == facing.left() {
        Direction::Left
    } else {
        Direction::Right
    }
}

/// The single rotation from `from` to `to`, or `None` when they are equal.
#[must_use]
pub fn rotation(from: Facing, to: Facing) -> Option<Rotation> {
    if from == to {
        None
    } else if to == from.left() {
        Some(Rotation::Left)
    } else if to == from.right() {
        Some(Rotation::Right)
    } else {
        Some(Rotation::Around)
    }
}

/// The boundaries of `cell` that the movement `from → to` passes beyond by more than `margin`,
/// ordered by where along the movement each is met. An exact tie puts north and south first.
#[must_use]
pub fn crossings(from: (f32, f32), to: (f32, f32), cell: (i32, i32), margin: f32) -> Vec<Facing> {
    #[allow(clippy::cast_precision_loss)]
    let (lo_x, lo_z) = (cell.0 as f32 - margin, cell.1 as f32 - margin);
    let (hi_x, hi_z) = (lo_x + 1.0 + 2.0 * margin, lo_z + 1.0 + 2.0 * margin);
    let mut found: Vec<(f32, u8, Facing)> = Vec::with_capacity(2);
    let t = |a: f32, b: f32, bound: f32| {
        if (b - a).abs() < f32::EPSILON {
            0.0
        } else {
            (bound - a) / (b - a)
        }
    };
    if to.1 < lo_z {
        found.push((t(from.1, to.1, lo_z), 0, Facing::North));
    } else if to.1 >= hi_z {
        found.push((t(from.1, to.1, hi_z), 0, Facing::South));
    }
    if to.0 < lo_x {
        found.push((t(from.0, to.0, lo_x), 1, Facing::West));
    } else if to.0 >= hi_x {
        found.push((t(from.0, to.0, hi_x), 1, Facing::East));
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    found.into_iter().map(|(_, _, f)| f).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::{FRAC_PI_2, PI};

    #[test]
    fn cell_of_floors_boundaries_and_negatives() {
        assert_eq!(cell_of(0.0, 0.0), (0, 0));
        assert_eq!(cell_of(0.999, 1.0), (0, 1));
        assert_eq!(cell_of(-0.01, 2.5), (-1, 2));
    }

    #[test]
    fn facing_holds_inside_the_margin_and_flips_beyond_it() {
        let margin = 5f32.to_radians();
        let near_diagonal = FRAC_PI_4 + 3f32.to_radians();
        assert_eq!(
            facing_of(near_diagonal, Facing::North, margin),
            Facing::North
        );
        assert_eq!(facing_of(near_diagonal, Facing::West, margin), Facing::West);
        let past = FRAC_PI_4 + 6f32.to_radians();
        assert_eq!(facing_of(past, Facing::North, margin), Facing::West);
        assert_eq!(facing_of(PI, Facing::North, margin), Facing::South);
        assert_eq!(facing_of(-FRAC_PI_2, Facing::North, margin), Facing::East);
    }

    #[test]
    fn relative_covers_all_sixteen_pairs() {
        for facing in Facing::ALL {
            assert_eq!(relative(facing, facing), Direction::Forward);
            assert_eq!(relative(facing.opposite(), facing), Direction::Back);
            assert_eq!(relative(facing.left(), facing), Direction::Left);
            assert_eq!(relative(facing.right(), facing), Direction::Right);
        }
    }

    #[test]
    fn rotation_matches_the_core_turn() {
        for from in Facing::ALL {
            for to in Facing::ALL {
                match rotation(from, to) {
                    None => assert_eq!(from, to),
                    Some(r) => assert_eq!(from.rotated(r), to),
                }
            }
        }
    }

    #[test]
    fn crossings_need_the_margin() {
        let m = 0.15;
        assert!(crossings((0.5, 0.5), (1.1, 0.5), (0, 0), m).is_empty());
        assert_eq!(
            crossings((0.5, 0.5), (1.2, 0.5), (0, 0), m),
            vec![Facing::East]
        );
        assert_eq!(
            crossings((0.5, 0.5), (0.5, -0.2), (0, 0), m),
            vec![Facing::North]
        );
    }

    #[test]
    fn diagonal_crossings_come_in_the_order_they_are_met() {
        let m = 0.15;
        // Mostly east: the east boundary is met first.
        assert_eq!(
            crossings((0.9, 0.8), (1.3, 1.2), (0, 0), m),
            vec![Facing::East, Facing::South]
        );
        // Mostly south: the south boundary first.
        assert_eq!(
            crossings((0.8, 0.9), (1.2, 1.3), (0, 0), m),
            vec![Facing::South, Facing::East]
        );
        // An exact corner: north and south first.
        assert_eq!(
            crossings((0.85, 0.85), (1.25, 1.25), (0, 0), m),
            vec![Facing::South, Facing::East]
        );
    }
}
