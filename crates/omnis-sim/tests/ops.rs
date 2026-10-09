//! The client protocol answers every simulation op on real pack data and refuses what it must.

mod common;

use common::{data, script_for_six, word, world};
use omnis_core::{Direction, Facing, Position, Rotation};
use omnis_data::ron_io::{parse, to_string};
use omnis_sim::ops::{MAX_SCRIPT, ShotTarget};
use omnis_sim::{Command, Event, Op, OpError, Reply, dispatch, parse_script};

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
    let mut layout = data.maps[&dungeon].def.layout.clone();
    // The way up at (0, 0) and the way down at (23, 23) are portals: rows and columns 1 and 47
    // of the layout.
    layout[1].replace_range(1..2, "*");
    layout[47].replace_range(47..48, "*");
    assert_eq!(
        text(&mut world, &data, Some("test:map:dungeon")),
        layout.join("\n") + "\n",
        "no party there and no door touched: the file's own layout and its portal"
    );
    let town = text(&mut world, &data, Some("test:map:town"));
    let street: Vec<char> = town.lines().nth(2 * 2 + 1).unwrap().chars().collect();
    assert_eq!(street[2 * 11 + 1], '*', "the town gate: {town}");
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
        Op::Screenshot {
            path: None,
            target: ShotTarget::Canvas,
        },
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
            target: ShotTarget::Window,
        },
        Op::ScreenText,
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
        Op::CombatGet,
        Op::ServiceGet,
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
    for service in ["smith", "temple"] {
        let mut town = common::inside(&data, service);
        for op in [Op::ServiceGet, Op::PartyGet, Op::GameStatus] {
            let reply = dispatch(&mut town, &data, &op).unwrap();
            assert_eq!(parse::<Reply>(&to_string(&reply).unwrap()).unwrap(), reply);
        }
    }

    let script = script_for_six("forward, turn-left  # to the west\n\nuse back\n");
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
        assert_eq!(word(command.word()), Some(command));
    }
    assert_eq!(word("party"), None, "party commands carry data");
}

/// `time.clocks` names every clock and contact; `time.reconcile` is a dev command that a plain
/// world refuses and a dev world answers with `Reconciled`; `game.status` carries the date
/// (M8).
#[test]
fn time_ops_list_the_clocks_and_reconcile_as_a_dev_command() {
    let data = data();
    let mut world = omnis_sim::World::new(&data, 3, omnis_sim::Settings::default()).unwrap();
    let reconcile = Op::TimeReconcile {
        region: "test:region:town".into(),
    };
    assert_eq!(
        dispatch(&mut world, &data, &reconcile).unwrap_err(),
        OpError::Rejected {
            rejection: omnis_sim::Rejection::DevOnly
        }
    );
    world.settings.devtools = true;
    world.clocks.get_mut(&omnis_sim::PARTY).unwrap().elapsed = 3000;
    world.party_time.shared_milli = 3000 * 900;
    let got = events(dispatch(&mut world, &data, &reconcile).unwrap());
    assert!(
        got.iter()
            .filter(|e| matches!(e, Event::Reconciled { .. }))
            .count()
            == 2,
        "the town and the crossroads by road: {got:?}"
    );
    let unknown = Op::TimeReconcile {
        region: "test:region:none".into(),
    };
    assert!(dispatch(&mut world, &data, &unknown).is_err());
    let Reply::Time { time } = dispatch(&mut world, &data, &Op::TimeClocks).unwrap() else {
        panic!("a time view")
    };
    assert_eq!((time.age, time.shared), (3000, 2700));
    let holders: Vec<&str> = time.clocks.iter().map(|c| c.holder.as_str()).collect();
    assert_eq!(
        holders,
        ["party:0", "test:region:crossroads", "test:region:town"]
    );
    assert_eq!(time.date.minutes, time.clocks[2].elapsed, "the town's date");
    assert_eq!(time.contacts.len(), 4, "both sides of two meetings");
    let Reply::Status(status) = dispatch(&mut world, &data, &Op::GameStatus).unwrap() else {
        panic!("a status")
    };
    assert_eq!(status.date, time.date);
    assert_eq!(status.clock.elapsed, 3000, "the clock is the party's age");
}

/// The host ops' rules, shared by the dev socket and the headless driver (ARCHITECTURE.md §4.9):
/// the save rule decides `save.write`, a save reads back as the world it was, a reload must keep
/// the party's tile, and `rules.set` bounds and checks its source.
#[test]
fn the_host_ops_rules_hold_for_every_host() {
    use omnis_sim::ops::{check_reload, load_text, rules_set, save_text};
    use omnis_sim::{SaveRule, Settings, World};
    let data = data();
    let world = world(&data);
    let text = save_text(&world).unwrap();
    assert_eq!(load_text(&text, &data, false).unwrap(), world);
    let strict = Settings {
        save_rule: SaveRule::InnOnly,
        ..Settings::default()
    };
    let outside = World::new(&data, 1, strict).unwrap();
    assert!(
        matches!(save_text(&outside), Err(OpError::Failed { .. })),
        "only at an inn"
    );

    assert_eq!(check_reload(&world, &data), Ok(()));
    let mut gone = data.clone();
    gone.maps.remove(&world.position.map);
    assert!(
        matches!(check_reload(&world, &gone), Err(OpError::Failed { .. })),
        "the party's map is gone"
    );

    let mut rules = data.clone();
    let long = "1 + ".repeat(2000) + "1";
    assert_eq!(
        rules_set(&mut rules, "spell_points.pool", &long),
        Err(OpError::TooLong {
            limit: omnis_data::limits::MAX_STRING_BYTES
        })
    );
    assert!(matches!(
        rules_set(&mut rules, "spell_points.pool", "level +"),
        Err(OpError::BadRequest { .. })
    ));
    assert_eq!(
        rules_set(&mut rules, "no.such.slot", "1"),
        Err(OpError::UnknownSlot {
            slot: "no.such.slot".to_owned()
        }),
        "as rules.get answers it (B5)"
    );
    assert_eq!(rules, data, "a refused swap changes nothing");
    let Ok(Reply::Rule { rule }) = rules_set(&mut rules, "spell_points.pool", "level * 10") else {
        panic!("a good formula swaps");
    };
    assert_eq!(rule.source, "level * 10");
}
