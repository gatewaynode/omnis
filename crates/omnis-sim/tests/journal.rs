//! The journal (PRD D28, owner 2026-10-17): the interactions since the world's base, each with
//! the changes it made to the world. What changes only the party (a step, a turn, its purse) is
//! not journaled: the party is saved as it stands. Not fingerprinted; the character save carries
//! it.

use crate::common;
use crate::objects::{in_yard, objects, yard};
use common::{data, step, turn};
use omnis_core::{Facing, Rotation};
use omnis_data::Verb;
use omnis_data::ron_io::read_ron;
use omnis_sim::api::{Bases, Change, apply_changes, split};
use omnis_sim::{
    Command, DevCommand, EncounterChoice, Replay, Settings, World, apply, journal_len,
};
use std::path::PathBuf;

fn golden(name: &str) -> Replay {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/replays")
        .join(format!("{name}.ron"));
    read_ron(&path, &path).unwrap_or_else(|e| panic!("{e}"))
}

fn dev(command: DevCommand) -> Command {
    Command::Dev(command)
}

/// The yard with the lever found and the party before it, devtools on.
fn at_the_lever() -> (omnis_data::Data, World) {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 1, Facing::North);
    world.settings.devtools = true;
    world.known.notice(yard(&data), "lever_1");
    (data, world)
}

fn pull() -> Command {
    Command::Act {
        object: "lever_1".into(),
        verb: Verb::Pull,
        option: 0,
    }
}

#[test]
fn a_step_a_turn_and_a_purse_journal_nothing() {
    let data = data();
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    let mut world = common::new_world(&data, 7, settings);
    common::party_of(&mut world, &data, 1);
    step(&mut world, &data);
    turn(&mut world, &data, Rotation::Left);
    apply(&mut world, &data, dev(DevCommand::SetGold { gold: 5 })).unwrap();
    assert_eq!(journal_len(&world), 0, "{:?}", world.journal);
}

/// Entering another region is an interaction: the region's time catches up with the party, and
/// that is the world's. Walking inside one region is not.
#[test]
fn entering_another_region_is_one_interaction_with_its_time() {
    let (data, mut world) = at_the_lever();
    apply(&mut world, &data, Command::Turn(Rotation::Around)).unwrap();
    assert_eq!(journal_len(&world), 0, "inside the yard");
    let town = dev(DevCommand::Teleport {
        map: "test:map:town".into(),
        level: omnis_core::Level::GROUND,
        x: 1,
        y: 1,
        facing: Facing::North,
    });
    apply(&mut world, &data, town).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(journal_len(&world), 1);
    let town = data.registry.regions.get("test:region:town").unwrap();
    let changes = &world.journal.entries()[0].changes;
    assert!(
        changes.iter().any(|c| matches!(
            c,
            Change::Clock { holder: omnis_core::HolderId::Region(r), clock: Some(_) } if *r == town
        )),
        "{changes:?}"
    );
}

#[test]
fn pulling_the_lever_is_one_interaction_with_the_world_flag_and_the_lever() {
    let (data, mut world) = at_the_lever();
    apply(&mut world, &data, pull()).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(journal_len(&world), 1);
    let entry = &world.journal.entries()[0];
    assert_eq!(entry.command, pull());
    assert_eq!(entry.at, world.position);
    assert_eq!(entry.minute, world.party_clock().elapsed);
    let lever = data.registry.flags.get("objects:flag:lever").unwrap();
    assert!(entry.changes.contains(&Change::Flag {
        flag: lever,
        value: Some(1)
    }));
    assert!(entry.changes.iter().any(|c| matches!(
        c,
        Change::Object { map, id, state: Some(_) } if *map == yard(&data) && id == "lever_1"
    )));
}

#[test]
fn a_party_flag_journals_nothing_and_a_world_flag_one_change() {
    let (data, mut world) = at_the_lever();
    let set = |flag: &str| {
        dev(DevCommand::SetFlag {
            flag: flag.into(),
            value: 3,
        })
    };
    apply(&mut world, &data, set("objects:flag:debt")).unwrap();
    assert_eq!(journal_len(&world), 0, "the debt is the party's");
    apply(&mut world, &data, set("objects:flag:lever")).unwrap();
    let lever = data.registry.flags.get("objects:flag:lever").unwrap();
    assert_eq!(
        world.journal.entries()[0].changes,
        [Change::Flag {
            flag: lever,
            value: Some(3)
        }]
    );
}

#[test]
fn a_refused_command_leaves_the_journal_as_it_was() {
    let (data, mut world) = at_the_lever();
    apply(&mut world, &data, pull()).unwrap();
    let before = world.journal.clone();
    for command in [pull(), Command::Encounter(EncounterChoice::Attack)] {
        let refused = apply(&mut world, &data, command.clone());
        assert!(refused.is_err(), "{command:?} is refused here");
        assert_eq!(world.journal, before, "{command:?}");
    }
}

#[test]
fn the_journal_is_not_in_the_world_snapshot_or_the_fingerprint() {
    let (data, mut world) = at_the_lever();
    apply(&mut world, &data, pull()).unwrap();
    assert_eq!(journal_len(&world), 1);
    let mut emptied = world.clone();
    emptied.journal = Default::default();
    let text = world.to_ron().unwrap();
    assert!(
        !text.contains("journal"),
        "the world's text (the fingerprint's) has no journal; the character save carries it"
    );
    assert_eq!(world.fingerprint(), emptied.fingerprint());
    let loaded = World::from_ron(&text, &data, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        journal_len(&loaded),
        0,
        "a world read from a snapshot starts a new journal"
    );
}

/// What the world holds, apart from the party.
fn held(world: &World, data: &omnis_data::Data) -> Bases {
    split(world, data).into()
}

/// The start, with every entry's changes laid over it in order.
fn rebuilt(start: &World, now: &World) -> World {
    let mut world = start.clone();
    for entry in now.journal.entries() {
        apply_changes(&mut world, &entry.changes);
    }
    world
}

/// No place that writes to the world is missed: along both replays and the yard's lever and
/// gate, after every command, the journal laid over the start holds what the world holds.
#[test]
fn the_journal_laid_over_the_start_rebuilds_what_the_world_holds() {
    let data = data();
    let mut kinds = std::collections::BTreeSet::new();
    for name in ["walk", "fight"] {
        let replay = golden(name);
        let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
        let start = world.clone();
        for (at, command) in replay.commands.iter().enumerate() {
            apply(&mut world, &data, command.clone()).unwrap_or_else(|r| panic!("{r}"));
            assert_eq!(
                held(&rebuilt(&start, &world), &data),
                held(&world, &data),
                "{name} after command {at}"
            );
        }
        for entry in world.journal.entries() {
            assert!(
                !entry.changes.is_empty(),
                "{name}: an entry changes something"
            );
            for change in &entry.changes {
                let kind = format!("{change:?}");
                kinds.insert(kind[..kind.find(' ').unwrap_or(kind.len())].to_owned());
            }
        }
        assert!(
            world.journal.len() < replay.commands.len(),
            "{name}: moving is not journaled"
        );
    }
    let (yard_data, mut world) = at_the_lever();
    let start = world.clone();
    let gate = Command::Act {
        object: "gate_1".into(),
        verb: Verb::Push,
        option: 0,
    };
    let unset = dev(DevCommand::SetFlag {
        flag: "objects:flag:lever".into(),
        value: 0,
    });
    let shut = Command::Act {
        object: "gate_1".into(),
        verb: Verb::Pull,
        option: 0,
    };
    for command in [pull(), gate.clone(), shut.clone(), unset] {
        if command == gate {
            world.position = world.position.at(3, 2);
            world.position.facing = Facing::South;
        }
        apply(&mut world, &yard_data, command).unwrap_or_else(|r| panic!("{r}"));
        assert_eq!(
            held(&rebuilt(&start, &world), &yard_data),
            held(&world, &yard_data)
        );
    }
    assert_eq!(
        world.journal.len(),
        4,
        "the lever, the gate opened and shut, and the flag"
    );
    let gate_back = world.journal.entries()[2]
        .changes
        .iter()
        .any(|c| matches!(c, Change::Object { id, state: None, .. } if id == "gate_1"));
    assert!(gate_back, "the gate shut is back as placed");
    let (_, mut world) = at_the_lever();
    let start = world.clone();
    let open = dev(DevCommand::SetFlag {
        flag: "objects:flag:lever".into(),
        value: 1,
    });
    apply(&mut world, &yard_data, open).unwrap();
    world.position = world.position.at(3, 2);
    world.position.facing = Facing::South;
    for command in [gate, shut] {
        apply(&mut world, &yard_data, command).unwrap_or_else(|r| panic!("{r}"));
    }
    assert!(
        !world.maps.contains_key(&yard(&yard_data)),
        "a map back as written holds nothing"
    );
    assert_eq!(
        held(&rebuilt(&start, &world), &yard_data),
        held(&world, &yard_data)
    );
    assert!(
        kinds.contains("Door") && kinds.contains("Clock"),
        "the replays open doors and meet regions: {kinds:?}"
    );
}
