//! The ground under the view (presentation-ARCHITECTURE.md §8.4, M8.5 step 4) on the engine's
//! elevation fixture: smooth where the party can walk, split where it cannot, each centre at its
//! cell's own height; and flat on every shipped map.

use crate::common;

use common::{data, heights_data, map};
use omnis_sim::omnis_core::Facing;
use omnis_vector::ground::{Ground, NE, NW, eye, joined};
use omnis_vector::pose::Pose;

fn hills() -> (omnis_sim::omnis_data::Data, Ground) {
    let data = heights_data();
    let ground = Ground::from_map(
        &data.maps[&map(&data, "heights:map:hills")],
        omnis_sim::omnis_core::Level::GROUND,
    );
    (data, ground)
}

fn patch(ground: &Ground, x: u16, y: u16) -> omnis_vector::ground::Patch {
    ground.patch(x, y).expect("on the map")
}

#[test]
fn the_shipped_maps_are_flat() {
    let data = data();
    for m in data.maps.values() {
        let ground = Ground::from_map(m, omnis_sim::omnis_core::Level::GROUND);
        for y in 0..m.def.height {
            for x in 0..m.def.width {
                let p = patch(&ground, x, y);
                assert_eq!((p.centre, p.corners), (0.0, [0.0; 4]), "{}", m.def.id);
            }
        }
    }
}

#[test]
fn a_slope_shares_its_corners_and_a_cliff_splits_them() {
    let (_, ground) = hills();
    // Row 0 climbs 0, 2, 4 in eighths, then a cliff up to 12.
    let (a, b, c, d) = (
        patch(&ground, 1, 0),
        patch(&ground, 2, 0),
        patch(&ground, 3, 0),
        patch(&ground, 0, 0),
    );
    assert_eq!(d.corners[NE], a.corners[NW]);
    assert_eq!(a.corners[NE], b.corners[NW]);
    assert!((a.corners[NE] - 0.375).abs() < 1e-6, "the mean of 2 and 4");
    assert_eq!(
        (b.corners[NE], c.corners[NW]),
        (0.5, 1.5),
        "the cliff splits"
    );
}

#[test]
fn a_ramp_joins_and_a_one_way_edge_and_a_wall_do_not() {
    let (data, ground) = hills();
    let hills = &data.maps[&map(&data, "heights:map:hills")];
    assert!(
        joined(
            hills,
            omnis_sim::omnis_core::Level::GROUND,
            0,
            2,
            Facing::East
        ),
        "the ramp"
    );
    assert!(
        !joined(
            hills,
            omnis_sim::omnis_core::Level::GROUND,
            0,
            1,
            Facing::East
        ),
        "scree to stairs goes one way"
    );
    assert!(
        !joined(
            hills,
            omnis_sim::omnis_core::Level::GROUND,
            1,
            1,
            Facing::West
        ),
        "and not back"
    );
    assert!(
        !joined(
            hills,
            omnis_sim::omnis_core::Level::GROUND,
            3,
            4,
            Facing::East
        ),
        "the wall"
    );
    assert!(
        !joined(
            hills,
            omnis_sim::omnis_core::Level::GROUND,
            8,
            0,
            Facing::East
        ),
        "the map's edge"
    );
    let (low, high) = (patch(&ground, 0, 2), patch(&ground, 1, 2));
    assert_eq!(
        low.corners[NE], high.corners[NW],
        "the ramp's corner is shared"
    );
}

#[test]
fn each_centre_is_at_its_cell_s_height() {
    let (_, ground) = hills();
    assert_eq!(ground.height(1.5, 2.5), 2.0, "the ramp's top");
    assert_eq!(ground.height(2.5, 4.5), 2.0, "the hill");
    assert_eq!(ground.height(1.5, 0.5), 0.25, "the slope");
}

#[test]
fn the_ground_runs_on_across_a_slope_and_jumps_at_a_cliff() {
    let (_, ground) = hills();
    let e = 1e-3;
    let slope = (ground.height(1.0 - e, 0.5), ground.height(1.0 + e, 0.5));
    assert!((slope.0 - slope.1).abs() < 0.01, "{slope:?}");
    let cliff = (ground.height(3.0 - e, 0.5), ground.height(3.0 + e, 0.5));
    assert!(cliff.1 - cliff.0 > 0.9, "{cliff:?}");
}

#[test]
fn the_eye_stands_on_the_hill() {
    let (_, ground) = hills();
    let top = Pose {
        x: 2.5,
        z: 4.5,
        yaw: 0.0,
    };
    assert_eq!(eye(&ground, top), 2.5);
}
