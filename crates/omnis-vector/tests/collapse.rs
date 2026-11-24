//! Doorways that fall in, as the player meets them (owner, 2026-11-24): the break and the fall
//! are told, the view is drawn anew, the walk stops at the rubble as the engine's step does,
//! and the square offers the clearing by name with its time, never as the default action.

use crate::common;
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::Data;
use omnis_sim::{BlockReason, Command, Event, Target};
use omnis_vector::collide;
use omnis_vector::rolllog::Names;
use omnis_vector::screen::actions::{self, Does};
use omnis_vector::shell::session::reshapes;
use omnis_vector::shell::text;
use omnis_vector::sim::SimWorld;

/// The yard with doorways that fall in `percent` times in a hundred.
fn falling(percent: u8) -> Data {
    let mut data = common::objects_data();
    let yard = data.registry.maps.get("objects:map:yard").unwrap();
    data.maps
        .get_mut(&yard)
        .unwrap()
        .def
        .door_make
        .collapse_percent = percent;
    data
}

/// Before the torch's door, broken by the fighter: the world and every line told of the door.
fn broken(data: &Data) -> (SimWorld, Vec<String>) {
    let mut world = crate::loot::yard(data, 6, 3, Facing::South);
    let views = common::views(&world, data);
    let mut names = Names::default();
    names.observe(&views.party);
    names.locate(views.position.map);
    let command = Command::Attack {
        member: views.party.members[0].member,
        target: Target::Door {
            x: 6,
            y: 3,
            facing: Facing::South,
        },
    };
    for _ in 0..200 {
        let events = world.apply(data, command.clone()).expect("a blow");
        if !events.iter().any(|e| matches!(e, Event::DoorBroken { .. })) {
            continue;
        }
        let of_the_door =
            |e: &&Event| matches!(e, Event::DoorBroken { .. } | Event::DoorFrame { .. });
        let told = events.iter().filter(of_the_door);
        let lines = told
            .map(|e| {
                assert!(reshapes(e), "the view is drawn anew: {e:?}");
                text::describe(e, &names, data).expect("told")
            })
            .collect();
        return (world, lines);
    }
    panic!("the door never broke");
}

/// The lines of the torch's doorway itself and the triangles of the level's solid faces.
fn picture(world: &SimWorld, data: &Data) -> (usize, usize) {
    use omnis_vector::geometry::SegKind;
    let map = data.registry.maps.get("objects:map:yard").unwrap();
    let scene = omnis_vector::scene::Scene::build(data, map, Level::GROUND, world);
    let near = |p: [f32; 3]| (p[2] - 4.0).abs() < 0.3 && (6.0..=7.0).contains(&p[0]);
    let own = scene.own.iter();
    let lines = own
        .filter(|s| s.kind == SegKind::Door && near(s.a) && near(s.b))
        .count();
    (lines, scene.fills.len())
}

/// The doorway's pictures: shut, a crossed frame in a whole wall; broken, wreckage about an
/// opening cut in the wall; buried, the mound in a wall with no opening.
#[test]
fn the_view_shows_wreckage_in_an_opening_and_a_mound_in_a_closed_wall() {
    let data = falling(100);
    let shut = picture(&crate::loot::yard(&data, 6, 3, Facing::South), &data);
    assert_eq!(shut.0, 5);
    let buried = picture(&broken(&data).0, &data);
    assert_eq!(buried.0, 28, "posts, mound, boulders, lintel, stones");
    assert_eq!(
        buried.1, shut.1,
        "nothing is seen through rubble: no opening is cut"
    );
    let data = falling(0);
    let open = picture(&broken(&data).0, &data);
    assert_eq!(open.0, 10, "posts, what hangs, planks");
    assert!(open.1 > shut.1, "the opening is cut");
    // Cleared, the rubble is gone and the doorway is the broken one.
    let data = falling(100);
    let (mut world, _) = broken(&data);
    world.apply(&data, Command::Clear).unwrap();
    assert_eq!(picture(&world, &data), open);
}

/// The minimap's pixels along the torch's door: its bar, and the pixel two below its middle
/// (the party's own mark lies above it).
fn bar(world: &SimWorld, data: &Data) -> (Vec<[u8; 4]>, [u8; 4]) {
    use omnis_vector::minimap::{corner, marks, paint_marked};
    const S: u32 = 8;
    let map_id = data.registry.maps.get("objects:map:yard").unwrap();
    let known = world.automap(map_id, Level::GROUND);
    let worn = marks(known, |x, y, facing| {
        (
            world.door_broken(map_id, Level::GROUND, x, y, facing),
            world.door_collapsed(map_id, Level::GROUND, x, y, facing),
        )
    });
    let pose = omnis_vector::pose::Pose {
        x: 6.5,
        z: 3.5,
        yaw: 0.0,
    };
    let raster = paint_marked(&data.maps[&map_id], Level::GROUND, known, pose, S, &worn);
    let (cx, cy) = corner(S, pose);
    let at = |x: i64, y: i64| {
        let (px, py) = (
            u32::try_from(x - cx).unwrap(),
            u32::try_from(y - cy).unwrap(),
        );
        raster.pixel(px, py).unwrap()
    };
    // The edge between rows 3 and 4 of column 6, a quarter of a cell in from each end.
    let along = (6 * 8 + 2..7 * 8 - 2).map(|x| at(x, 32)).collect();
    (along, at(6 * 8 + 4, 34))
}

/// The map's marks: a whole door is a bar, a broken one a bar with a gap, and a buried doorway
/// a crossed bar in the danger colour; cleared, it is the broken one's.
#[test]
fn the_minimap_marks_a_broken_door_and_a_buried_doorway() {
    use omnis_vector::minimap::{DOOR, RUBBLE};
    let data = falling(100);
    // The party has looked about it: a turn away and back.
    let mut fresh = crate::loot::yard(&data, 6, 3, Facing::South);
    for way in [
        omnis_sim::omnis_core::Rotation::Left,
        omnis_sim::omnis_core::Rotation::Right,
    ] {
        fresh.apply(&data, Command::Turn(way)).unwrap();
    }
    let (whole, off) = bar(&fresh, &data);
    assert_eq!(whole, [DOOR; 4]);
    assert_ne!(off, DOOR);
    let (mut world, _) = broken(&data);
    let (buried, cross) = bar(&world, &data);
    assert_eq!(buried, [RUBBLE; 4]);
    assert_eq!(cross, RUBBLE, "crossed");
    world.apply(&data, Command::Clear).unwrap();
    let (gapped, off) = bar(&world, &data);
    assert_eq!((gapped[0], gapped[3]), (DOOR, DOOR));
    assert!(gapped[1] != DOOR && gapped[2] != DOOR, "a gap: {gapped:?}");
    assert!(off != RUBBLE && off != DOOR);
}

fn labels(world: &SimWorld, data: &Data) -> Vec<String> {
    let offers = actions::available(world, data);
    offers.into_iter().map(|o| o.label).collect()
}

fn blocks(world: &SimWorld, data: &Data, y: u16, heading: Facing) -> Option<BlockReason> {
    let map = data.registry.maps.get("objects:map:yard").unwrap();
    collide::blocks(data, world, map, Level::GROUND, 6, y, heading)
}

#[test]
fn a_buried_doorway_is_told_stops_the_walk_and_is_cleared_by_name() {
    let data = falling(100);
    let (mut world, lines) = broken(&data);
    assert_eq!(
        lines,
        [
            "The door breaks",
            "The frame gives way: the doorway is buried"
        ]
    );
    // The walk stops at it from either side, as rubble and not as a shut door.
    assert_eq!(
        blocks(&world, &data, 3, Facing::South),
        Some(BlockReason::Rubble)
    );
    assert_eq!(
        blocks(&world, &data, 4, Facing::North),
        Some(BlockReason::Rubble)
    );
    let names = Names::default();
    let bump = Event::Blocked {
        reason: BlockReason::Rubble,
    };
    assert_eq!(
        text::describe(&bump, &names, &data).as_deref(),
        Some("Blocked: rubble fills the doorway")
    );
    // The square offers the clearing with its time; Shift+Space never starts an hour's work.
    let offered = labels(&world, &data);
    assert!(
        offered.contains(&"Clear the rubble (60 min)".to_owned()),
        "{offered:?}"
    );
    assert!(!offered.iter().any(|l| l.contains("door")), "{offered:?}");
    assert_ne!(
        actions::default_action(&world, &data),
        Some(Does::Command(Command::Clear))
    );
    let clear = actions::available(&world, &data)
        .into_iter()
        .find(|o| o.label.starts_with("Clear"))
        .unwrap();
    let Does::Command(command) = clear.does else {
        panic!("a command");
    };
    let events = world.apply(&data, command).expect("cleared");
    let cleared: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::DoorCleared { .. }))
        .collect();
    assert_eq!(cleared.len(), 1);
    assert!(reshapes(cleared[0]));
    assert_eq!(
        text::describe(cleared[0], &names, &data).as_deref(),
        Some("The rubble is cleared")
    );
    assert_eq!(blocks(&world, &data, 3, Facing::South), None);
    let offered = labels(&world, &data);
    assert!(!offered.iter().any(|l| l.contains("rubble")), "{offered:?}");
}

#[test]
fn a_frame_that_holds_is_told_and_nothing_is_offered() {
    let mut data = falling(100);
    data.rules.insert_value("rubble_clear_minutes", 25);
    let (world, _) = broken(&data);
    assert!(labels(&world, &data).contains(&"Clear the rubble (25 min)".to_owned()));
    // One in a hundred: the frame's roll is told and it holds (seeds differ, so some do).
    let data = falling(1);
    let (world, lines) = broken(&data);
    assert_eq!(lines, ["The door breaks", "The frame holds"]);
    assert_eq!(blocks(&world, &data, 3, Facing::South), None);
    assert!(!labels(&world, &data).iter().any(|l| l.contains("rubble")));
    // And a map whose doorways never fall tells no roll.
    let data = falling(0);
    assert_eq!(broken(&data).1, ["The door breaks"]);
}
