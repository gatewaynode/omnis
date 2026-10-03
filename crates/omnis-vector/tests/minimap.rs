//! The minimap's pixels on the real test maps (alt-ARCHITECTURE.md §8): only what the automap
//! knows is painted, the party is marked where the pose is, and walls appear once seen.

mod common;

use common::{data, drive, session};
use omnis_vector::minimap::{BACKGROUND, MARKER, Raster, WALL, paint, scale_for};
use omnis_vector::pose::Pose;

fn painted(raster: &Raster) -> usize {
    raster
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| **p != BACKGROUND)
        .count()
}

#[test]
fn cells_scale_to_whole_pixels_within_the_side() {
    assert_eq!(scale_for(24, 24), 8);
    assert_eq!(scale_for(32, 20), 6);
    assert_eq!(scale_for(500, 10), 2, "never below two pixels a cell");
}

#[test]
fn a_fresh_party_sees_only_its_surroundings_and_is_marked() {
    let data = data();
    let (world, _) = session(&data);
    let p = world.position;
    let map = &data.maps[&p.map];
    let pose = Pose::at(p);
    let raster = paint(map, world.automap.map(p.map), pose);
    let s = raster.scale;
    assert_eq!(raster.width, u32::from(map.def.width) * s);
    assert_eq!(raster.height, u32::from(map.def.height) * s);
    // The marker sits on the pose; the party's cell is known and painted around it.
    let (cx, cy) = (u32::from(p.x) * s, u32::from(p.y) * s);
    assert_eq!(raster.pixel(cx + s / 2, cy + s / 2), Some(MARKER));
    assert_ne!(raster.pixel(cx, cy + s - 1), Some(BACKGROUND));
    // A corner far from the party is unknown.
    assert_eq!(raster.pixel(0, raster.height - 1), Some(BACKGROUND));
    assert!(
        painted(&raster) < raster.rgba.len() / 4,
        "not everything is known"
    );
}

#[test]
fn walking_reveals_more_and_the_dungeon_walls_show() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let start = world.position;
    let before = paint(
        &data.maps[&start.map],
        world.automap.map(start.map),
        Pose::at(start),
    );
    // North up the road and through the portal into the dungeon.
    let (pose, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        Pose::at(start),
        (0.0, -0.05),
        0.0,
        240,
    );
    let meadow = paint(
        &data.maps[&start.map],
        world.automap.map(start.map),
        Pose::at(start),
    );
    assert!(painted(&meadow) > painted(&before), "the road was seen");
    let here = world.position;
    assert_ne!(here.map, start.map, "in the dungeon");
    let dungeon = paint(&data.maps[&here.map], world.automap.map(here.map), pose);
    assert!(
        dungeon.rgba.as_chunks::<4>().0.contains(&WALL),
        "the dungeon's walls are drawn once seen"
    );
}

#[test]
fn the_marker_points_along_the_yaw() {
    let data = data();
    let (world, _) = session(&data);
    let p = world.position;
    let map = &data.maps[&p.map];
    for facing in omnis_sim::omnis_core::Facing::ALL {
        let pose = Pose::at(omnis_sim::omnis_core::Position { facing, ..p });
        let raster = paint(map, None, pose);
        let s = raster.scale as f32;
        let (fx, fz) = omnis_vector::geom::forward(pose.yaw);
        // How far ahead of the pose, in cells, each marker pixel lies.
        let mut along = Vec::new();
        for y in 0..raster.height {
            for x in 0..raster.width {
                if raster.pixel(x, y) == Some(MARKER) {
                    let (dx, dz) = ((x as f32 + 0.5) / s - pose.x, (y as f32 + 0.5) / s - pose.z);
                    along.push(dx * fx + dz * fz);
                }
            }
        }
        let ahead = along.iter().copied().fold(f32::MIN, f32::max);
        let behind = along.iter().copied().fold(f32::MAX, f32::min);
        // The tip reaches well ahead of the pose; the base stays close behind it.
        assert!(ahead > 0.5, "the tip is ahead, {facing:?}: {ahead}");
        assert!(
            ahead > -behind + 0.2,
            "tip beyond base, {facing:?}: {ahead} {behind}"
        );
    }
}

#[test]
fn an_edge_recorded_on_one_side_is_drawn_once_known() {
    use omnis_sim::Known;
    use omnis_sim::omnis_core::{Edges, Facing};
    let data = data();
    let (world, _) = session(&data);
    let map = &data.maps[&world.position.map];
    let tile = |walls: Edges| Known {
        terrain: 0,
        walls,
        doors: Edges::NONE,
        layers: 0,
        seen_at: 0,
    };
    // Cell (5, 5) knows a wall on its east side; (6, 5) knows none on its west.
    let mut known = std::collections::BTreeMap::new();
    known.insert((5, 5), tile(Edges::NONE.with(Facing::East)));
    known.insert((6, 5), tile(Edges::NONE));
    let far = Pose {
        x: 20.5,
        z: 20.5,
        yaw: 0.0,
    };
    let raster = paint(map, Some(&known), far);
    let s = raster.scale;
    for dy in 0..s {
        assert_eq!(raster.pixel(6 * s, 5 * s + dy), Some(WALL));
    }
    // The west side of (5, 5) has no wall.
    assert_ne!(raster.pixel(5 * s, 5 * s + 1), Some(WALL));
}
