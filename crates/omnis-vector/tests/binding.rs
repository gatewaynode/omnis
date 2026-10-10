//! The grid under free movement (presentation-PRD §4, presentation-ARCHITECTURE.md §10.3), driven headless on
//! the real test maps: at rest the camera's cell is the simulation's position; boundaries,
//! corners, walls, a portal and a placed encounter behave; the log replays.

use crate::common;

use common::{data, drive, map, place, session, steps, town_session_seeded};
use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use omnis_sim::omnis_core::{Direction, Facing};
use omnis_sim::{Command, Event, Mode, Settings};
use omnis_vector::geom::yaw_of;
use omnis_vector::grid::cell_of;
use omnis_vector::pose::Pose;

/// 3 cells a second at 60 frames a second.
const STEP: f32 = 0.05;

fn at_rest(
    world: &omnis_sim::World,
    binder: &omnis_vector::bind::Binder,
    pose: Pose,
) -> (i32, i32) {
    let rest = binder.settle(world, pose);
    cell_of(rest.x, rest.z)
}

fn position(world: &omnis_sim::World) -> (i32, i32) {
    (i32::from(world.position.x), i32::from(world.position.y))
}

#[test]
fn walking_north_five_cells_ends_on_the_simulations_cell() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let start = Pose::at(world.position);
    let (pose, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        start,
        (0.0, -STEP),
        0.0,
        100,
    );
    assert_eq!(position(&world), (16, 11));
    assert_eq!(at_rest(&world, &binder, pose), position(&world));
    assert_eq!((binder.refusals, binder.disagreements), (0, 0));
}

#[test]
fn strafing_is_a_sidestep() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let start = Pose::at(world.position);
    let (pose, _, _) = drive(&mut binder, &mut world, &data, start, (STEP, 0.0), 0.0, 60);
    assert_eq!(position(&world), (19, 16));
    assert_eq!(world.position.facing, Facing::North);
    assert!(binder.log.contains(&Command::Step(Direction::Right)));
    assert_eq!(at_rest(&world, &binder, pose), position(&world));
}

#[test]
fn jitter_inside_the_margin_emits_no_step() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let before = steps(&binder.log);
    let mut pose = Pose {
        x: 16.5,
        z: 16.0,
        yaw: 0.0,
    };
    for i in 0..100 {
        let z = if i % 2 == 0 { 15.9 } else { 16.1 };
        let to = Pose { z, ..pose };
        pose = binder
            .advance(&mut world, &data, pose, to)
            .pose
            .expect("a pose");
    }
    assert_eq!(steps(&binder.log), before);
    assert_eq!(position(&world), (16, 16));
}

#[test]
fn a_diagonal_walk_alternates_steps_and_ends_where_the_simulation_is() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let start = Pose::at(world.position);
    let d = STEP * FRAC_PI_4.cos();
    // Yaw −45° looks north-east; held inside the facing margin, so the facing stays north.
    let (pose, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        start,
        (d, -d),
        -FRAC_PI_4,
        85,
    );
    assert_eq!(position(&world), (19, 13));
    assert_eq!(steps(&binder.log), 6);
    assert_eq!(at_rest(&world, &binder, pose), position(&world));
    assert_eq!((binder.refusals, binder.disagreements), (0, 0));
}

#[test]
fn the_hedge_stops_the_pose_without_a_step() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let meadow = world.position.map;
    let start = place(&mut world, meadow, 20, 2, Facing::North);
    let (pose, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        start,
        (0.0, -STEP),
        0.0,
        100,
    );
    assert_eq!(position(&world), (20, 0));
    assert!(
        pose.z >= 0.2 - 1e-5,
        "the pose keeps the body radius from the hedge"
    );
    assert_eq!((binder.refusals, binder.disagreements), (0, 0));
}

#[test]
fn water_is_impassable_and_the_pose_slides_along_it() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let meadow = world.position.map;
    let start = place(&mut world, meadow, 8, 26, Facing::North);
    let (pose, _, _) = drive(&mut binder, &mut world, &data, start, (0.0, -STEP), 0.0, 60);
    assert_eq!(position(&world), (8, 25));
    // Pushing north-east along the shore slides east.
    let (_, _, _) = drive(&mut binder, &mut world, &data, pose, (STEP, -STEP), 0.0, 60);
    assert_eq!(world.position.y, 25);
    assert!(world.position.x > 10);
    assert_eq!(binder.disagreements, 0);
}

#[test]
fn the_portal_snaps_the_pose_into_the_dungeon() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let start = Pose::at(world.position);
    let (pose, _, outcomes) = drive(
        &mut binder,
        &mut world,
        &data,
        start,
        (0.0, -STEP),
        0.0,
        240,
    );
    assert_eq!(world.position.map, map(&data, "test:map:dungeon"));
    // The snap frame puts the pose at the destination's centre, looking along its facing.
    let snap = outcomes
        .iter()
        .find(|o| o.snapped)
        .and_then(|o| o.pose)
        .expect("a snap");
    assert_eq!((snap.x, snap.z), (1.5, 0.5));
    assert!((snap.yaw - yaw_of(Facing::South)).abs() < 1e-5);
    // This driver keeps asking for yaw 0 afterwards (a real shell integrates from the snapped
    // yaw), so the party turns north into the map edge; the pose stays in the simulation's cell.
    assert_eq!(at_rest(&world, &binder, pose), position(&world));
    assert_eq!(binder.disagreements, 0);
}

#[test]
fn entering_a_placed_encounter_freezes_motion() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let dungeon = map(&data, "test:map:dungeon");
    let start = place(&mut world, dungeon, 3, 6, Facing::South);
    let (pose, _, outcomes) = drive(&mut binder, &mut world, &data, start, (0.0, STEP), PI, 80);
    assert!(matches!(world.mode, Mode::Encounter(_)));
    assert_eq!(position(&world), (3, 8));
    assert!(outcomes.iter().any(|o| o.frozen));
    assert_eq!(at_rest(&world, &binder, pose), position(&world));
}

#[test]
fn the_random_table_rolls_once_per_cell_never_per_frame() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let start = Pose::at(world.position);
    // Slowly: 0.01 a frame, 500 frames, five cells east.
    let (_, events, _) = drive(&mut binder, &mut world, &data, start, (0.01, 0.0), 0.0, 500);
    let checks = events
        .iter()
        .filter(|e| matches!(e, Event::EncounterCheck { .. }))
        .count();
    assert_eq!(position(&world), (21, 16));
    assert_eq!(checks, 5);
}

#[test]
fn a_free_movement_session_replays_to_the_same_fingerprint() {
    let data = data();
    // A seed whose walk from the town meets no random encounter before the placed group (most
    // do; the shared seed 7 meets one in the dungeon's first room).
    let seed = 1;
    let (mut world, mut binder) = town_session_seeded(&data, seed);
    // From the game's start, east through the signpost onto the meadow, north up the road into
    // the portal, then through the dungeon door to the placed group.
    let pose = Pose::at(world.position);
    let (_, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        pose,
        (STEP, 0.0),
        yaw_of(Facing::East),
        20,
    );
    assert_eq!(world.position.map, map(&data, "test:map:meadow"));
    let pose = Pose::at(world.position);
    let (pose, _, _) = drive(&mut binder, &mut world, &data, pose, (0.0, -STEP), 0.0, 520);
    assert_eq!(world.position.map, map(&data, "test:map:dungeon"));
    // Face south (a turn from the yaw), walk to (1, 5), then east to (3, 5).
    let (pose, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        pose,
        (0.0, STEP),
        yaw_of(Facing::South),
        100,
    );
    let (pose, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        pose,
        (STEP, 0.0),
        yaw_of(Facing::South),
        40,
    );
    assert_eq!(position(&world), (3, 5));
    binder.interact(&mut world, &data);
    let (_, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        pose,
        (0.0, STEP),
        yaw_of(Facing::South),
        80,
    );
    assert!(
        matches!(world.mode, Mode::Encounter(_)),
        "the door opened and the group was met"
    );
    // A spin of the yaw in place emits turns, which are free and logged.
    let turns_before = binder
        .log
        .iter()
        .filter(|c| matches!(c, Command::Turn(_)))
        .count();
    let still = binder.settle(&world, Pose::at(world.position));
    let _ = binder.advance(
        &mut world,
        &data,
        still,
        Pose {
            yaw: FRAC_PI_2,
            ..still
        },
    );
    let turns_after = binder
        .log
        .iter()
        .filter(|c| matches!(c, Command::Turn(_)))
        .count();
    assert_eq!(turns_before, turns_after, "no turns while frozen");
    let replay = binder
        .replay(&data, seed, Settings::default())
        .expect("the log replays");
    assert_eq!(
        replay.fingerprint,
        world.fingerprint().expect("a fingerprint")
    );
    assert_eq!(binder.refusals, 0);
}
