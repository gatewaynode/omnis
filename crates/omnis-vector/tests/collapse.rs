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
