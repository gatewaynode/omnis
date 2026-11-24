//! Doorways that fall in (owner, 2026-11-24: "many door frames are major load bearing
//! structures and destruction of the door could compromise the frame and cause a collapse").
//! A door that breaks may bury its doorway, by the map's `collapse_percent` (25 unless the map
//! says); rubble blocks the step and the sight until it is cleared, which takes time and
//! leaves the doorway open for good.

use crate::breaking_doors::{
    BOLT, FIGHTER, GATE, HAND, TORCH, attack, door, rows, spell, the_bolt, until_broken,
};
use crate::objects::{objects, yard};
use crate::opening_strike::at;
use omnis_core::{Direction, Facing, Level, StreamName};
use omnis_data::Data;
use omnis_sim::query::EdgeView;
use omnis_sim::{BlockReason, Chosen, Command, Event, Rejection, World, apply, query};

/// The yard with doorways that fall in `percent` times in a hundred.
fn falling(percent: u8) -> Data {
    let mut data = objects();
    let yard = yard(&data);
    data.maps
        .get_mut(&yard)
        .unwrap()
        .def
        .door_make
        .collapse_percent = percent;
    data
}

/// `(broken, collapsed)` of a door of the yard.
fn fallen(world: &World, data: &Data, (x, y, facing): (u16, u16, Facing)) -> (bool, bool) {
    let wear = world
        .maps
        .get(&yard(data))
        .map(|s| s.door_wear(Level::GROUND, x, y, facing))
        .unwrap_or_default();
    (wear.broken, wear.collapsed)
}

/// Before the torch's door, which the fighter then breaks: the events of the last blow.
fn broken_at(data: &Data, seed: Option<u64>) -> (World, Vec<Event>) {
    let mut world = at(data, 6, 3, Facing::South);
    if let Some(seed) = seed {
        world.seed = seed;
        world.rngs.clear();
    }
    let command = attack(&world, FIGHTER, TORCH);
    for _ in 0..200 {
        let events = apply(&mut world, data, command.clone()).expect("a blow");
        if events.iter().any(|e| matches!(e, Event::DoorBroken { .. })) {
            return (world, events);
        }
    }
    panic!("the door never broke");
}

/// The d100 a breaking door rolled for its frame, if it rolled.
fn frame(events: &[Event]) -> Option<(i64, bool)> {
    events.iter().find_map(|e| match e {
        Event::DoorFrame {
            roll, collapsed, ..
        } => Some((i64::from(roll.total), *collapsed)),
        _ => None,
    })
}

fn step(world: &mut World, data: &Data) -> Vec<Event> {
    apply(world, data, Command::Step(Direction::Forward)).expect("a step is a turn")
}

#[test]
fn a_doorway_that_falls_in_blocks_the_step_and_is_nothing_to_open_strike_or_hand() {
    let data = falling(100);
    let (mut world, events) = broken_at(&data, None);
    // Told in order: broken, then the frame's roll and what came of it.
    let told: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            Event::DoorBroken { .. } => Some(0),
            Event::DoorFrame { .. } => Some(1),
            _ => None,
        })
        .collect();
    assert_eq!(told, [0, 1]);
    assert!(matches!(frame(&events), Some((1..=100, true))));
    assert_eq!(fallen(&world, &data, TORCH), (true, true));
    assert_eq!(fallen(&world, &data, (6, 4, Facing::North)), (true, true));
    let buried = EdgeView::Door {
        open: false,
        damage: 18,
        broken: true,
        collapsed: true,
    };
    let view = query::viewport(&world, &data).unwrap();
    assert_eq!(view.tiles[0].front, buried);
    let yard = yard(&data);
    assert!(!query::door_open(
        &world,
        yard,
        Level::GROUND,
        6,
        4,
        Facing::North
    ));
    assert!(query::door_collapsed(
        &world,
        yard,
        Level::GROUND,
        6,
        4,
        Facing::North
    ));
    assert!(query::door_collapsed(
        &world,
        yard,
        Level::GROUND,
        6,
        3,
        Facing::South
    ));
    assert!(!query::door_collapsed(
        &world,
        yard,
        Level::GROUND,
        6,
        2,
        Facing::East
    ));
    // The step is a turn taken against rubble, not against a shut door.
    let events = step(&mut world, &data);
    assert_eq!((world.position.x, world.position.y), (6, 3));
    let rubble = Event::Blocked {
        reason: BlockReason::Rubble,
    };
    assert!(events.contains(&rubble), "{events:?}");
    // Nothing to open, shut, strike or hand.
    assert_eq!(
        apply(&mut world, &data, Command::Door),
        Err(Rejection::Broken)
    );
    let blow = attack(&world, FIGHTER, TORCH);
    assert_eq!(apply(&mut world, &data, blow), Err(Rejection::Broken));
    for name in [BOLT, HAND] {
        let cast = spell(&world, name, TORCH);
        assert_eq!(
            apply(&mut world, &data, cast),
            Err(Rejection::Broken),
            "{name}"
        );
    }
    for chosen in [the_bolt(), Chosen::Attack] {
        let listed = rows(&world, &data, &chosen);
        assert!(!listed.contains(&door(TORCH)), "{chosen:?}: {listed:?}");
    }
    // Use does not start an hour's work unasked: it is asked for by name.
    let was = world.clone();
    let events = apply(&mut world, &data, Command::Interact).expect("nothing here");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::DoorCleared { .. }))
    );
    assert_eq!(world.party_clock().elapsed, was.party_clock().elapsed);
    assert_eq!(fallen(&world, &data, TORCH), (true, true));
}

#[test]
fn rubble_hides_what_is_beyond_it() {
    let sees = |world: &World, data: &Data| {
        let view = query::viewport(world, data).unwrap();
        view.tiles.iter().any(|t| (t.x, t.y) == (7, 2))
    };
    for (percent, seen) in [(0, true), (100, false)] {
        let data = falling(percent);
        let mut world = at(&data, 6, 2, Facing::East);
        let command = attack(&world, FIGHTER, GATE);
        until_broken(&mut world, &data, &command);
        assert_eq!(sees(&world, &data), seen, "{percent}%");
    }
}

#[test]
fn a_map_whose_doorways_never_fall_rolls_nothing() {
    let strikes = |world: &World| {
        let strike = world.rngs.get(&StreamName::new("strike"));
        strike.map_or(0, omnis_core::Pcg32::draws)
    };
    let (sure, events) = broken_at(&falling(0), None);
    assert_eq!(frame(&events), None, "no roll is told");
    assert_eq!(fallen(&sure, &falling(0), TORCH), (true, false));
    let (risky, _) = broken_at(&falling(100), None);
    assert_eq!(
        strikes(&risky),
        strikes(&sure) + 1,
        "one d100, and none at 0"
    );
    // Unbroken by the fall, the way is open.
    let data = falling(0);
    let mut world = sure;
    step(&mut world, &data);
    assert_eq!((world.position.x, world.position.y), (6, 4));
}

#[test]
fn clearing_the_rubble_takes_the_time_and_leaves_the_doorway_open_for_good() {
    let mut data = falling(100);
    let (mut world, _) = broken_at(&data, None);
    let before = world.party_clock().elapsed;
    let events = apply(&mut world, &data, Command::Clear).expect("rubble to clear");
    let cleared = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::DoorCleared {
                    x: 6,
                    y: 3,
                    facing: Facing::South,
                    ..
                }
            )
        })
        .count();
    assert_eq!(cleared, 1, "{events:?}");
    assert_eq!(
        world.party_clock().elapsed,
        before + 60,
        "an hour by the base rules"
    );
    assert_eq!(fallen(&world, &data, TORCH), (true, false));
    let open = EdgeView::Door {
        open: true,
        damage: 18,
        broken: true,
        collapsed: false,
    };
    assert_eq!(query::viewport(&world, &data).unwrap().tiles[0].front, open);
    // Once: there is nothing more to clear, and no door to shut.
    let was = world.clone();
    assert_eq!(
        apply(&mut world, &data, Command::Clear),
        Err(Rejection::NothingToClear)
    );
    assert_eq!(
        apply(&mut world, &data, Command::Door),
        Err(Rejection::Broken)
    );
    assert_eq!(world, was);
    step(&mut world, &data);
    assert_eq!((world.position.x, world.position.y), (6, 4));
    // From the far side too, and for the minutes the rules say.
    data.rules.insert_value("rubble_clear_minutes", 25);
    let (mut world, _) = broken_at(&data, None);
    let far = omnis_sim::DevCommand::Teleport {
        map: "objects:map:yard".into(),
        level: Level::GROUND,
        x: 6,
        y: 4,
        facing: Facing::North,
    };
    apply(&mut world, &data, Command::Dev(far)).expect("the far side");
    let before = world.party_clock().elapsed;
    apply(&mut world, &data, Command::Clear).expect("rubble to clear");
    assert_eq!(world.party_clock().elapsed, before + 25);
    assert_eq!(fallen(&world, &data, TORCH), (true, false));
}

#[test]
fn there_is_nothing_to_clear_at_a_wall_a_whole_door_or_a_doorway_that_stood() {
    let data = falling(0);
    let mut world = at(&data, 6, 3, Facing::South);
    let was = world.clone();
    assert_eq!(
        apply(&mut world, &data, Command::Clear),
        Err(Rejection::NothingToClear)
    );
    for facing in [Facing::North, Facing::East, Facing::West] {
        let mut world = at(&data, 6, 3, facing);
        assert_eq!(
            apply(&mut world, &data, Command::Clear),
            Err(Rejection::NothingToClear)
        );
    }
    assert_eq!(world, was);
    let (mut world, _) = broken_at(&data, None);
    assert_eq!(
        apply(&mut world, &data, Command::Clear),
        Err(Rejection::NothingToClear)
    );
}

#[test]
fn a_buried_doorway_comes_back_from_a_save_and_the_map_s_text_shows_it() {
    let data = falling(100);
    let (world, _) = broken_at(&data, None);
    crate::disk::round_trip(&world, &data, "a buried doorway");
    let text = |world: &World, data: &Data| {
        query::map_text(world, data, yard(data), Level::GROUND).unwrap()
    };
    let buried = text(&world, &data);
    let shut = text(&at(&data, 6, 3, Facing::South), &data);
    let (stood, _) = broken_at(&falling(0), None);
    let open = text(&stood, &falling(0));
    assert_ne!(buried, shut);
    assert_ne!(buried, open);
    assert_eq!(
        buried.matches('x').count(),
        open.matches('x').count() + 1,
        "{buried}"
    );
    let mut cleared = world;
    apply(&mut cleared, &data, Command::Clear).unwrap();
    crate::disk::round_trip(&cleared, &data, "a cleared doorway");
    assert_eq!(text(&cleared, &data), open);
}

/// The default make: one doorway in four falls in, by a d100 told with the break.
#[test]
fn by_default_one_doorway_in_four_falls_in() {
    let data = objects();
    let map = &data.maps[&yard(&data)].def;
    assert_eq!(map.door_make.collapse_percent, 25);
    let (mut fell, mut stood) = (0, 0);
    for seed in 0..80 {
        let (world, events) = broken_at(&data, Some(seed));
        let (roll, collapsed) = frame(&events).expect("a roll");
        assert_eq!(collapsed, roll <= 25, "seed {seed}: {roll}");
        assert_eq!(
            fallen(&world, &data, TORCH),
            (true, collapsed),
            "seed {seed}"
        );
        fell += usize::from(collapsed);
        stood += usize::from(!collapsed);
    }
    assert!(
        (10..=30).contains(&fell) && stood > 40,
        "{fell} fell, {stood} stood"
    );
}
