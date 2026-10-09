//! The town mode (M7 step 4b): a step onto a site goes inside, `Interact` on one goes in again,
//! a step out leaves, a turn stays, `Leave` leaves in place, and an inn is where inn-only rules
//! allow a save.

mod common;

use common::{data, interact, party_of, step, turn, word as parse_word};
use omnis_core::{CharacterId, Direction, Facing, Position, Rotation, ServiceId};
use omnis_data::{Data, ServiceKind};
use omnis_sim::{
    BlockReason, Command, Event, LoadError, Mode, ModeKind, SaveRule, ServiceCommand, ServiceState,
    Settings, World, apply, query,
};

fn service(data: &Data, name: &str) -> ServiceId {
    data.registry.services.get(name).unwrap()
}

fn town(data: &Data, x: u16, y: u16, facing: Facing) -> Position {
    let map = data.registry.maps.get("test:map:town").unwrap();
    Position { map, x, y, facing }
}

/// From the new game's start, down the street to the inn's door, through it, and inside.
fn walk_into_the_inn(world: &mut World, data: &Data) -> Vec<Event> {
    for _ in 0..9 {
        step(world, data);
    }
    assert_eq!(world.position, town(data, 1, 2, Facing::West));
    turn(world, data, Rotation::Right);
    interact(world, data);
    step(world, data)
}

fn new_game(data: &Data, save_rule: SaveRule) -> World {
    let settings = Settings {
        save_rule,
        ..Settings::default()
    };
    World::new(data, 7, settings).unwrap()
}

#[test]
fn a_step_onto_a_site_goes_inside_with_no_encounter_roll() {
    let mut data = data();
    // Give the town a table that fires on every step, so a roll on the site would show.
    let meadow = data.registry.maps.get("test:map:meadow").unwrap();
    let mut table = data.maps[&meadow].random.clone().unwrap();
    table.chance_percent = 100;
    let town_map = data.registry.maps.get("test:map:town").unwrap();
    data.maps.get_mut(&town_map).unwrap().random = Some(table);

    let mut world = new_game(&data, SaveRule::Anywhere);
    let inn = service(&data, "base:service:inn");
    // Out of the inn's doorway and back: the street rolls, the site does not.
    world.position = town(&data, 1, 1, Facing::South);
    world.mode = Mode::Town(ServiceState {
        service: inn,
        kind: ServiceKind::Inn,
    });
    let door = apply(&mut world, &data, Command::Interact);
    assert!(door.is_err(), "inside, Interact is not a door: {door:?}");
    world.mode = Mode::Explore;
    let pos = world.position;
    world
        .map_state(pos.map)
        .open_doors
        .insert(omnis_sim::world::door_key(1, 1, Facing::South));
    let out = step(&mut world, &data);
    assert!(
        out.iter()
            .any(|e| matches!(e, Event::EncounterCheck { .. })),
        "the street rolls: {out:?}"
    );
    world.mode = Mode::Explore;
    turn(&mut world, &data, Rotation::Around);
    let inside = step(&mut world, &data);
    assert!(inside.contains(&Event::ServiceEntered { service: inn }));
    assert!(
        !inside
            .iter()
            .any(|e| matches!(e, Event::EncounterCheck { .. })),
        "no roll on a site: {inside:?}"
    );
    assert_eq!(world.mode.kind(), ModeKind::Town);
}

#[test]
fn walking_in_stepping_out_turning_and_leaving_in_place() {
    let data = data();
    let mut world = new_game(&data, SaveRule::Anywhere);
    let inn = service(&data, "base:service:inn");
    let events = walk_into_the_inn(&mut world, &data);
    assert!(events.contains(&Event::ServiceEntered { service: inn }));
    assert_eq!(
        world.mode,
        Mode::Town(ServiceState {
            service: inn,
            kind: ServiceKind::Inn
        })
    );
    assert_eq!(world.position, town(&data, 1, 1, Facing::North));

    // A turn stays inside; a step against a closed door keeps the party inside.
    turn(&mut world, &data, Rotation::Left);
    assert_eq!(world.mode.kind(), ModeKind::Town);
    turn(&mut world, &data, Rotation::Right);
    let map = world.position.map;
    world
        .map_state(map)
        .open_doors
        .remove(&omnis_sim::world::door_key(1, 1, Facing::South));
    let bump = apply(&mut world, &data, Command::Step(Direction::Back)).unwrap();
    assert!(bump.contains(&Event::Blocked {
        reason: BlockReason::ClosedDoor
    }));
    assert!(!bump.iter().any(|e| matches!(e, Event::ServiceLeft { .. })));
    assert_eq!(world.mode.kind(), ModeKind::Town);

    // A step inside the building leaves the site.
    let out = apply(&mut world, &data, Command::Step(Direction::Right)).unwrap();
    let left = out
        .iter()
        .position(|e| *e == Event::ServiceLeft { service: inn })
        .expect("left");
    let moved = out
        .iter()
        .position(|e| matches!(e, Event::Moved { .. }))
        .expect("moved");
    assert!(left < moved, "the party leaves, then moves: {out:?}");
    assert_eq!(world.mode, Mode::Explore);
    assert_eq!(world.position, town(&data, 2, 1, Facing::North));

    // Back onto the site goes in; Leave leaves in place; Interact goes in again.
    apply(&mut world, &data, Command::Step(Direction::Left)).unwrap();
    assert_eq!(world.mode.kind(), ModeKind::Town);
    let left = apply(&mut world, &data, Command::Service(ServiceCommand::Leave)).unwrap();
    assert!(left.contains(&Event::ServiceLeft { service: inn }));
    assert_eq!(world.mode, Mode::Explore);
    assert_eq!(world.position, town(&data, 1, 1, Facing::North));
    let before = world.party_clock().elapsed;
    let back = interact(&mut world, &data);
    assert!(back.contains(&Event::ServiceEntered { service: inn }));
    assert_eq!(world.mode.kind(), ModeKind::Town);
    assert_eq!(
        world.party_clock().elapsed,
        before,
        "going in takes no time"
    );
}

#[test]
fn a_service_command_outside_or_in_the_wrong_service_is_refused() {
    let data = data();
    let mut world = new_game(&data, SaveRule::Anywhere);
    party_of(&mut world, &data, 2);
    let outside = apply(&mut world, &data, Command::Service(ServiceCommand::Room));
    assert_eq!(outside, Err(omnis_sim::Rejection::WrongMode));
    walk_into_the_inn(&mut world, &data);
    let before = world.clone();
    let wrong = apply(
        &mut world,
        &data,
        Command::Service(ServiceCommand::Deposit { amount: 100 }),
    );
    assert_eq!(wrong, Err(omnis_sim::Rejection::NotOffered));
    assert_eq!(world, before, "a refusal changes nothing");
    // The party's own commands work inside.
    let ids = world.party.ids();
    apply(
        &mut world,
        &data,
        Command::Party(omnis_sim::PartyCommand::Reorder {
            order: vec![ids[1], ids[0]],
        }),
    )
    .expect("a reorder inside a service");
}

#[test]
fn inn_only_saves_are_allowed_in_an_inn_and_nowhere_else() {
    let data = data();
    let mut world = new_game(&data, SaveRule::InnOnly);
    assert!(!world.may_save(), "on the street");
    walk_into_the_inn(&mut world, &data);
    assert!(world.may_save(), "in the inn");
    let save = world.to_ron().unwrap();
    let back = World::from_ron(&save, &data, false).unwrap();
    assert_eq!(
        back.mode, world.mode,
        "a save made in the inn loads in the inn"
    );
    assert_eq!(back.to_ron(), world.to_ron());
    apply(&mut world, &data, Command::Service(ServiceCommand::Leave)).unwrap();
    assert!(!world.may_save(), "outside again");

    // Another service is not an inn.
    world.position = town(&data, 1, 3, Facing::South);
    interact(&mut world, &data);
    assert_eq!(
        world.mode,
        Mode::Town(ServiceState {
            service: service(&data, "base:service:smith"),
            kind: ServiceKind::Smith
        })
    );
    assert!(!world.may_save(), "at the smith");
}

#[test]
fn a_saved_service_must_be_the_one_on_the_party_tile() {
    let data = data();
    let mut world = new_game(&data, SaveRule::Anywhere);
    walk_into_the_inn(&mut world, &data);
    world.position = town(&data, 2, 1, Facing::North);
    let save = world.to_ron().unwrap();
    assert_eq!(
        World::from_ron(&save, &data, false),
        Err(LoadError::BadTown),
        "the tile has no site"
    );
    world.position = town(&data, 1, 1, Facing::North);
    world.mode = Mode::Town(ServiceState {
        service: service(&data, "base:service:inn"),
        kind: ServiceKind::Bank,
    });
    let save = world.to_ron().unwrap();
    assert_eq!(
        World::from_ron(&save, &data, false),
        Err(LoadError::BadTown),
        "the kind is not the service's"
    );
}

#[test]
fn the_queries_say_where_a_step_lands_and_which_service_it_goes_into() {
    let data = data();
    let mut world = new_game(&data, SaveRule::Anywhere);
    let inn = service(&data, "base:service:inn");
    world.position = town(&data, 1, 2, Facing::North);
    assert_eq!(query::step_lands(&world, &data, Direction::Forward), None);
    assert_eq!(
        query::site_ahead(&world, &data, Direction::Forward),
        None,
        "the door is shut"
    );
    interact(&mut world, &data);
    assert_eq!(
        query::step_lands(&world, &data, Direction::Forward),
        Some(town(&data, 1, 1, Facing::North))
    );
    assert_eq!(
        query::site_ahead(&world, &data, Direction::Forward),
        Some(inn)
    );
    assert_eq!(
        query::site_ahead(&world, &data, Direction::Right),
        None,
        "the street"
    );
    world.position = town(&data, 10, 2, Facing::West);
    let meadow = data.registry.maps.get("test:map:meadow").unwrap();
    assert_eq!(
        query::step_lands(&world, &data, Direction::Back),
        Some(Position {
            map: meadow,
            x: 16,
            y: 30,
            facing: Facing::North
        }),
        "through the gate, at the portal's far end"
    );
}

#[test]
fn the_service_words_parse_and_print() {
    for (word, command) in [
        ("leave", ServiceCommand::Leave),
        ("room", ServiceCommand::Room),
        ("rumor", ServiceCommand::Rumor),
    ] {
        assert_eq!(parse_word(word), Some(Command::Service(command)));
        assert_eq!(Command::Service(command).word(), word);
    }
    let numbered = [
        ("food-3", "food", ServiceCommand::BuyFood { count: 3 }),
        (
            "heal-1",
            "heal",
            ServiceCommand::Heal {
                member: CharacterId(1),
            },
        ),
        (
            "cure-0",
            "cure",
            ServiceCommand::Cure {
                member: CharacterId(0),
            },
        ),
        (
            "raise-5",
            "raise",
            ServiceCommand::Raise {
                member: CharacterId(5),
            },
        ),
        ("buy-2", "buy", ServiceCommand::Buy { item: 2, count: 1 }),
        ("buy-2-4", "buy", ServiceCommand::Buy { item: 2, count: 4 }),
        ("sell-0", "sell", ServiceCommand::Sell { item: 0, count: 1 }),
        (
            "sell-1-2",
            "sell",
            ServiceCommand::Sell { item: 1, count: 2 },
        ),
        (
            "deposit-250",
            "deposit",
            ServiceCommand::Deposit { amount: 250 },
        ),
        (
            "withdraw-9",
            "withdraw",
            ServiceCommand::Withdraw { amount: 9 },
        ),
    ];
    for (word, verb, command) in numbered {
        assert_eq!(parse_word(word), Some(Command::Service(command)), "{word}");
        assert_eq!(
            Command::Service(command).word(),
            verb,
            "commands with data log their verb"
        );
    }
    for bad in [
        "service",
        "buy",
        "buy-",
        "buy-x",
        "buy-1-",
        "buy-1-2-3",
        "heal-m1",
        "food--1",
        "sell-300",
        "bribe-1",
    ] {
        assert_eq!(parse_word(bad), None, "{bad}");
    }
}
