//! The predictive collision mirror (presentation-ARCHITECTURE.md §5.4, VA3).
//!
//! `blocks` copies the refusal conditions of `omnis_sim::apply`'s private `move`, in the same
//! order, so the pose slides along walls instead of walking into them. The simulation stays
//! authoritative; `tests/agreement.rs` holds the copy to it on every edge of the test maps.

use omnis_sim::omnis_core::{Facing, MapId, Position};
use omnis_sim::omnis_data::Data;
use omnis_sim::{BlockReason, World};

/// Why a step from `(x, y)` on `map` toward `heading` would be refused, or `None` when the
/// simulation would take it. Mirrors `omnis_sim::apply::move`.
#[must_use]
pub fn blocks(
    data: &Data,
    world: &World,
    map: MapId,
    x: u16,
    y: u16,
    heading: Facing,
) -> Option<BlockReason> {
    let Some(map_data) = data.maps.get(&map) else {
        return Some(BlockReason::MapEdge);
    };
    let Some(cell) = map_data.cell(x, y) else {
        return Some(BlockReason::MapEdge);
    };
    if cell.walls.has(heading) {
        return Some(BlockReason::Wall);
    }
    if cell.doors.has(heading)
        && !world
            .maps
            .get(&map)
            .is_some_and(|s| s.door_open(x, y, heading))
    {
        return Some(BlockReason::ClosedDoor);
    }
    let here = Position {
        map,
        x,
        y,
        facing: heading,
    };
    let Some((tx, ty)) = here.neighbour(heading) else {
        return Some(BlockReason::MapEdge);
    };
    let Some(target) = map_data.cell(tx, ty) else {
        return Some(BlockReason::MapEdge);
    };
    if !map_data.terrain(target).passable {
        return Some(BlockReason::Impassable);
    }
    None
}

/// Keep the ground point `(X, Z)` at least `radius` inside every side of `cell` that `blocked`
/// reports as closed. Open sides are left alone, so movement through them continues.
#[must_use]
pub fn clamp(
    point: (f32, f32),
    cell: (u16, u16),
    radius: f32,
    blocked: impl Fn(Facing) -> bool,
) -> (f32, f32) {
    let (lo_x, lo_z) = (f32::from(cell.0), f32::from(cell.1));
    let (mut x, mut z) = point;
    if blocked(Facing::West) {
        x = x.max(lo_x + radius);
    }
    if blocked(Facing::East) {
        x = x.min(lo_x + 1.0 - radius);
    }
    if blocked(Facing::North) {
        z = z.max(lo_z + radius);
    }
    if blocked(Facing::South) {
        z = z.min(lo_z + 1.0 - radius);
    }
    (x, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_holds_closed_sides_and_frees_open_ones() {
        let walls = |f: Facing| matches!(f, Facing::North | Facing::East);
        assert_eq!(clamp((3.95, 1.0), (3, 2), 0.2, walls), (3.8, 2.2));
        // West and south are open: the point may pass them.
        assert_eq!(clamp((2.9, 3.1), (3, 2), 0.2, walls), (2.9, 3.1));
    }
}
