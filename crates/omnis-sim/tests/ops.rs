//! The client protocol answers every simulation op on real pack data and refuses what it must.

mod common;

use common::{data, world};
use omnis_core::{Direction, Facing, Position, Rotation};
use omnis_data::ron_io::{parse, to_string};
use omnis_sim::command::parse_script;
use omnis_sim::ops::MAX_SCRIPT;
use omnis_sim::{Command, Event, Op, OpError, Reply, dispatch};

fn events(reply: Reply) -> Vec<Event> {
    match reply {
        Reply::Events { events } => events,
        other => panic!("not events: {other:?}"),
    }
}

#[test]
fn status_reports_the_new_game() {
    let data = data();
    let mut world = world(&data);
    let Reply::Status(status) = dispatch(&mut world, &data, &Op::GameStatus).unwrap() else {
        panic!("not a status")
    };
    assert_eq!(status.map, "test:map:meadow");
    assert_eq!(
        (status.turn, status.clock.day, status.clock.minute),
        (0, 0, 0)
    );
    assert_eq!(status.position, world.position);
    assert_eq!(status.packs, data.fingerprints);
    assert_eq!(
        status.fingerprint,
        format!("{:016x}", world.fingerprint().unwrap())
    );
}

#[test]
fn commands_scripts_and_the_tail() {
    let data = data();
    let mut world = world(&data);
    let step = Op::SimCommand {
        command: Command::Step(Direction::Forward),
    };
    let first = events(dispatch(&mut world, &data, &step).unwrap());
    assert!(matches!(first[0], Event::Moved { .. }), "{first:?}");
    let script = Op::SimScript {
        commands: vec![
            Command::Turn(Rotation::Left),
            Command::Step(Direction::Forward),
        ],
    };
    let Reply::Script {
        applied,
        events: scripted,
        rejected,
    } = dispatch(&mut world, &data, &script).unwrap()
    else {
        panic!("not a script reply")
    };
    assert_eq!((applied, rejected), (2, None));
    assert_eq!((world.position.x, world.position.y), (15, 15));
    let tail = events(dispatch(&mut world, &data, &Op::EventsTail { count: 2 }).unwrap());
    assert_eq!(tail, scripted[scripted.len() - 2..]);
    assert!(events(dispatch(&mut world, &data, &Op::EventsTail { count: 0 }).unwrap()).is_empty());
    let query = |world: &mut omnis_sim::World, path: &str| match dispatch(
        world,
        &data,
        &Op::WorldQuery {
            path: path.to_owned(),
        },
    ) {
        Ok(Reply::Value { value }) => value,
        other => panic!("{other:?}"),
    };
    assert_eq!(query(&mut world, "position.x").as_deref(), Some("15"));
    assert_eq!(query(&mut world, "nope"), None);
}

fn text(world: &mut omnis_sim::World, data: &omnis_data::Data, map: Option<&str>) -> String {
    match dispatch(
        world,
        data,
        &Op::MapText {
            map: map.map(str::to_owned),
        },
    ) {
        Ok(Reply::Text { text }) => text,
        other => panic!("{other:?}"),
    }
}

#[test]
fn map_text_is_the_layout_plus_the_party_and_door_state() {
    let data = data();
    let mut world = world(&data);
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    let layout = data.maps[&dungeon].def.layout.join("\n") + "\n";
    assert_eq!(
        text(&mut world, &data, Some("test:map:dungeon")),
        layout,
        "no party there and no door touched: the file's own layout"
    );
    let meadow = text(&mut world, &data, None);
    let lines: Vec<&str> = meadow.lines().collect();
    assert_eq!(lines.len(), 65);
    assert_eq!(lines[2 * 16 + 1].chars().nth(2 * 16 + 1), Some('^'));

    world.position = Position {
        map: dungeon,
        x: 5,
        y: 3,
        facing: Facing::East,
    };
    dispatch(
        &mut world,
        &data,
        &Op::SimCommand {
            command: Command::Interact,
        },
    )
    .unwrap();
    let after = text(&mut world, &data, None);
    let row: Vec<char> = after.lines().nth(2 * 3 + 1).unwrap().chars().collect();
    assert_eq!((row[2 * 5 + 1], row[2 * 5 + 2]), ('>', '\''), "{after}");
    assert_eq!(
        dispatch(
            &mut world,
            &data,
            &Op::MapText {
                map: Some("nope".into())
            }
        )
        .unwrap_err(),
        OpError::UnknownMap { map: "nope".into() }
    );
}

#[test]
fn automap_get_lists_known_tiles_of_the_named_map() {
    let data = data();
    let mut world = world(&data);
    dispatch(
        &mut world,
        &data,
        &Op::SimCommand {
            command: Command::Step(Direction::Forward),
        },
    )
    .unwrap();
    let Reply::Automap { map, tiles } =
        dispatch(&mut world, &data, &Op::AutomapGet { map: None }).unwrap()
    else {
        panic!("not an automap")
    };
    assert_eq!(map, "test:map:meadow");
    assert!(tiles.len() > 10, "{}", tiles.len());
    assert!(tiles.iter().any(|t| (t.x, t.y) == (16, 15)));
    let Reply::Automap { tiles, .. } = dispatch(
        &mut world,
        &data,
        &Op::AutomapGet {
            map: Some("test:map:dungeon".into()),
        },
    )
    .unwrap() else {
        panic!("not an automap")
    };
    assert!(tiles.is_empty(), "never been there");
    assert!(matches!(
        dispatch(&mut world, &data, &Op::ViewportGet).unwrap(),
        Reply::Viewport(v) if v.tiles[0].depth == 0
    ));
}

#[test]
fn limits_and_host_ops_are_refused() {
    let data = data();
    let mut world = world(&data);
    let before = world.clone();
    let long = Op::SimScript {
        commands: vec![Command::Interact; MAX_SCRIPT + 1],
    };
    assert_eq!(
        dispatch(&mut world, &data, &long).unwrap_err(),
        OpError::TooMany { limit: MAX_SCRIPT }
    );
    let path = Op::WorldQuery {
        path: "x".repeat(5000),
    };
    assert!(matches!(
        dispatch(&mut world, &data, &path).unwrap_err(),
        OpError::TooLong { .. }
    ));
    for op in [
        Op::SaveWrite {
            path: "a.ron".into(),
        },
        Op::SaveRead {
            path: "a.ron".into(),
            force: false,
        },
        Op::PackReload,
        Op::Screenshot { path: None },
    ] {
        assert!(op.is_host());
        assert_eq!(
            dispatch(&mut world, &data, &op).unwrap_err(),
            OpError::HostOnly
        );
    }
    assert!(!Op::GameStatus.is_host());
    assert_eq!(world, before, "refusals change nothing");
}

#[test]
fn ops_and_replies_round_trip_and_scripts_parse() {
    let ops = [
        Op::GameStatus,
        Op::WorldQuery {
            path: "turn".into(),
        },
        Op::SimCommand {
            command: Command::Interact,
        },
        Op::SimScript {
            commands: vec![Command::Step(Direction::Back)],
        },
        Op::EventsTail { count: 5 },
        Op::ViewportGet,
        Op::MapText { map: None },
        Op::AutomapGet {
            map: Some("m".into()),
        },
        Op::SaveWrite {
            path: "s.ron".into(),
        },
        Op::SaveRead {
            path: "s.ron".into(),
            force: true,
        },
        Op::PackReload,
        Op::Screenshot {
            path: Some("p.png".into()),
        },
        Op::PartyGet,
        Op::PartyCreate {
            character: omnis_sim::omnis_rules::Draft {
                name: "Brenna".into(),
                race: "base:race:human".into(),
                class: "base:class:fighter".into(),
                background: "base:background:acolyte".into(),
                alignment: omnis_data::Alignment::NeutralGood,
                scores: [15, 14, 13, 12, 10, 8],
                skills: vec![],
            },
        },
        Op::RulesList,
        Op::RulesGet {
            slot: "spell_points.pool".into(),
        },
        Op::RulesSet {
            slot: "spell_points.pool".into(),
            source: "level".into(),
        },
    ];
    for op in &ops {
        let text = to_string(op).unwrap();
        assert!(text.contains("op:"), "{text}");
        assert_eq!(parse::<Op>(&text).unwrap(), *op);
    }
    let data = data();
    let mut world = world(&data);
    for op in [Op::GameStatus, Op::EventsTail { count: 1 }, Op::ViewportGet] {
        let reply = dispatch(&mut world, &data, &op).unwrap();
        assert_eq!(parse::<Reply>(&to_string(&reply).unwrap()).unwrap(), reply);
    }

    let script = parse_script("forward, turn-left  # to the west\n\nuse back\n").unwrap();
    assert_eq!(
        script,
        [
            Command::Step(Direction::Forward),
            Command::Turn(Rotation::Left),
            Command::Interact,
            Command::Step(Direction::Back),
        ]
    );
    let error = parse_script("forward\nfly").unwrap_err();
    assert_eq!((error.line, error.word.as_str()), (2, "fly"));
    for command in script {
        assert_eq!(Command::from_word(command.word()), Some(command));
    }
    assert_eq!(
        Command::from_word("party"),
        None,
        "party commands carry data"
    );
}
