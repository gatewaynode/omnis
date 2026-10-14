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

/// The crest lines seen from `eye`, within a reach that covers the fixture.
fn crests(ground: &Ground, eye: [f32; 3]) -> Vec<([f32; 3], [f32; 3])> {
    ground.crests(eye, 20.0)
}

fn touches(lines: &[([f32; 3], [f32; 3])], p: [f32; 3]) -> bool {
    let near = |q: [f32; 3]| (0..3).all(|i| (q[i] - p[i]).abs() < 1e-4);
    lines.iter().any(|&(a, b)| near(a) || near(b))
}

#[test]
fn flat_ground_has_no_crest() {
    let data = data();
    for m in data.maps.values() {
        let ground = Ground::from_map(m, omnis_sim::omnis_core::Level::GROUND);
        let eye = [1.5, 0.5, 1.5];
        assert!(crests(&ground, eye).is_empty(), "{}", m.def.id);
    }
}

#[test]
fn the_terrace_s_lip_is_drawn_from_above_and_below() {
    let (_, ground) = hills();
    // The ramp's top cell (1, 2): its centre at 2.0, its west corners sunk toward the ramp's
    // foot, so the ground folds over at the centre.
    let lip = [1.5, 2.0, 2.5];
    let above = crests(&ground, [2.5, 2.5, 2.5]);
    assert!(touches(&above, lip), "from the terrace: {above:?}");
    let below = crests(&ground, [0.5, 0.5, 2.5]);
    assert!(touches(&below, lip), "from the ramp's foot: {below:?}");
    // The terrace's flat middle has none.
    let inside = |&(a, b): &([f32; 3], [f32; 3])| {
        let mid = [(a[0] + b[0]) * 0.5, (a[2] + b[2]) * 0.5];
        mid[0] > 4.0 && mid[0] < 7.0 && mid[1] > 2.0 && mid[1] < 3.0
    };
    assert!(!above.iter().any(inside), "{above:?}");
}

#[test]
fn a_crest_can_run_along_a_cell_s_side() {
    let (_, ground) = hills();
    // Below the ramp's foot, the side between (0, 3) and (1, 3) folds: its north end rises
    // with the ramp's corner and its south end lies on the flat.
    let (a, b) = ([1.0, 0.5, 3.0], [1.0, 0.0, 4.0]);
    let lines = crests(&ground, [0.25, 1.0, 2.75]);
    let near = |p: [f32; 3], q: [f32; 3]| (0..3).all(|i| (p[i] - q[i]).abs() < 1e-4);
    assert!(
        lines
            .iter()
            .any(|&(p, q)| (near(p, a) && near(q, b)) || (near(p, b) && near(q, a))),
        "{lines:?}"
    );
}

#[test]
fn no_crest_runs_along_a_split() {
    let (_, ground) = hills();
    let h = |p: [f32; 3]| (p[0], p[1], p[2]);
    for ex in 0..18u8 {
        for ez in 0..10u8 {
            for ey in [0.6, 1.0, 2.5, 3.0] {
                let eye = [f32::from(ex) * 0.5 + 0.25, ey, f32::from(ez) * 0.5 + 0.25];
                for (a, b) in crests(&ground, eye) {
                    // A side runs along x or z on whole numbers; a diagonal does not.
                    let (along_z, along_x) = (
                        a[0] == b[0] && a[0].fract() == 0.0,
                        a[2] == b[2] && a[2].fract() == 0.0,
                    );
                    if !(along_z || along_x) {
                        continue;
                    }
                    // Both cells either side carry the side at the same heights.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let (cx, cz) = (a[0].min(b[0]) as u16, a[2].min(b[2]) as u16);
                    let (here, there, facing) = if along_z {
                        ((cx - 1, cz), (cx, cz), Facing::East)
                    } else {
                        ((cx, cz - 1), (cx, cz), Facing::South)
                    };
                    let p = patch(&ground, here.0, here.1).side(facing);
                    let q = patch(&ground, there.0, there.1).side(facing.opposite());
                    assert_eq!(p, q, "{eye:?}: {:?} {:?}", h(a), h(b));
                }
            }
        }
    }
}
