//! Saves round-trip through RON text with equal fingerprints, refuse the wrong packs or
//! schema, and replays reproduce the golden fingerprint under `tests/replays/`.

mod common;

use common::{data, interact, step, turn, world};
use omnis_core::{Direction, Facing, Position, Rotation};
use omnis_data::ron_io::{read_ron, write_ron};
use omnis_sim::omnis_rules::DeathSaves;
use omnis_sim::{
    Command, LoadError, Mode, PartyCommand, Replay, ReplayError, SaveRule, Settings, World, apply,
    query,
};
use std::path::PathBuf;

fn replay_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/replays")
        .join(format!("{name}.ron"))
}

/// The command script behind `tests/replays/walk.ron`: down the road, into the dungeon,
/// through the first door, and a bump against a pillar.
fn walk() -> Vec<Command> {
    let mut commands = vec![Command::Step(Direction::Forward); 14];
    commands.extend([
        Command::Turn(Rotation::Left),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Turn(Rotation::Around),
        Command::Interact,
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Turn(Rotation::Right),
        Command::Step(Direction::Back),
        Command::Interact,
    ]);
    commands
}

#[test]
fn save_round_trip_keeps_the_fingerprint() {
    let data = data();
    let mut world = world(&data);
    for _ in 0..12 {
        step(&mut world, &data);
    }
    turn(&mut world, &data, Rotation::Left);
    interact(&mut world, &data);
    let text = world.to_ron().unwrap();
    let loaded = World::from_ron(&text, &data, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(loaded.fingerprint().unwrap(), world.fingerprint().unwrap());
    assert_eq!(loaded.position, world.position);
    assert_eq!(loaded.automap, world.automap);
    assert!(loaded.log.is_empty(), "the log starts empty after a load");
    let mut a = loaded.clone();
    let mut b = world.clone();
    let ea = step(&mut a, &data);
    let eb = step(&mut b, &data);
    assert_eq!(ea, eb, "a loaded world continues identically");
}

#[test]
fn loads_are_checked() {
    let data = data();
    let world = world(&data);
    let text = world.to_ron().unwrap();

    let other = text.replacen("schema: 3", "schema: 7", 1);
    assert_eq!(
        World::from_ron(&other, &data, false).unwrap_err(),
        LoadError::Schema(7)
    );

    let mut foreign = world.clone();
    foreign.packs[0].hash ^= 1;
    let foreign_text = foreign.to_ron().unwrap();
    assert_eq!(
        World::from_ron(&foreign_text, &data, false).unwrap_err(),
        LoadError::PackMismatch
    );
    assert!(
        World::from_ron(&foreign_text, &data, true).is_ok(),
        "force skips only the pack check"
    );

    let mut lost = world.clone();
    lost.position = Position {
        map: omnis_core::MapId(99),
        x: 0,
        y: 0,
        facing: Facing::North,
    };
    let lost_text = lost.to_ron().unwrap();
    assert_eq!(
        World::from_ron(&lost_text, &data, true).unwrap_err(),
        LoadError::BadPosition(lost.position)
    );

    assert!(matches!(
        World::from_ron("(", &data, false).unwrap_err(),
        LoadError::Parse(_)
    ));
}

#[test]
fn path_query_reads_the_world() {
    let data = data();
    let mut world = world(&data);
    step(&mut world, &data);
    assert_eq!(query::path(&world, "position.x").as_deref(), Some("16"));
    assert_eq!(query::path(&world, "position.y").as_deref(), Some("15"));
    assert_eq!(
        query::path(&world, "position.facing").as_deref(),
        Some("North")
    );
    assert_eq!(
        query::path(&world, "clocks.Party(0).elapsed").as_deref(),
        Some("1")
    );
    assert_eq!(query::path(&world, "turn").as_deref(), Some("1"));
    assert_eq!(query::path(&world, "mode").as_deref(), Some("Explore"));
    assert_eq!(query::path(&world, "packs.0.id").as_deref(), Some("base"));
    assert_eq!(query::path(&world, "packs.1.id").as_deref(), Some("test"));
    let map = world.position.map.0;
    assert_eq!(
        query::path(&world, &format!("automap.maps.{map}.16,15.layers")).as_deref(),
        Some("7")
    );
    assert_eq!(query::path(&world, "party.gold").as_deref(), Some("0"));
    assert_eq!(
        query::path(&world, "party.members.0.hp"),
        None,
        "no members yet"
    );
    assert_eq!(
        query::path(&world, "settings.save_rule").as_deref(),
        Some("Anywhere")
    );
}

/// A schema-1 save (captured from the M2 build's `schema dump`) loads through the migration.
#[test]
fn a_schema_1_save_migrates() {
    let data = data();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/saves/v1.ron");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("schema: 1") && text.contains("save_anywhere: true"));
    assert_eq!(
        World::from_ron(&text, &data, false).unwrap_err(),
        LoadError::PackMismatch,
        "the fixture names an example pack"
    );
    let world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(world.schema, 3);
    assert!(world.party.members.is_empty());
    assert_eq!(world.settings, Settings::default());
    assert_eq!(world.to_ron().unwrap().matches("schema: 3").count(), 1);
    let inn_only = text.replace("save_anywhere: true", "save_anywhere: false");
    let world = World::from_ron(&inn_only, &data, true).unwrap();
    assert_eq!(world.settings.save_rule, SaveRule::InnOnly);
    assert!(!world.may_save());
}

/// A schema-2 save (captured from the M3 build, `capture_schema_2_fixture`) loads through the
/// migration: the mode, cleared encounters, and death saves default.
#[test]
fn a_schema_2_save_migrates() {
    let data = data();
    let path = save_path("v2");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("schema: 2") && !text.contains("death_saves"));
    assert_eq!(
        World::from_ron(&text, &data, false).unwrap_err(),
        LoadError::PackMismatch,
        "the fixture's base pack predates the combat rules"
    );
    let world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(world.schema, 3);
    assert_eq!(world.mode, Mode::Explore);
    assert_eq!(world.party.members.len(), 1);
    assert_eq!(world.party.members[0].death_saves, DeathSaves::default());
    assert!(!world.party.members[0].is_down());
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    assert!(world.maps[&dungeon].door_open(5, 3, Facing::East));
    assert!(world.maps[&dungeon].cleared.is_empty());
    let text = world.to_ron().unwrap();
    assert_eq!(text.matches("schema: 3").count(), 1);
    assert_eq!(
        World::from_ron(&text, &data, true).unwrap(),
        world,
        "written back at schema 3, still on the fixture's pack hashes"
    );
}

#[test]
fn replays_are_deterministic() {
    let data = data();
    let commands = walk();
    let settings = Settings::default();
    let a = omnis_sim::replay::run(&data, 1, settings, &commands).unwrap();
    let b = omnis_sim::replay::run(&data, 1, settings, &commands).unwrap();
    let c = omnis_sim::replay::run(&data, 2, settings, &commands).unwrap();
    let hard = Settings {
        save_rule: SaveRule::InnOnly,
        permadeath: true,
    };
    let d = omnis_sim::replay::run(&data, 1, hard, &commands).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c, "the seed is part of the world");
    assert_ne!(a, d, "the settings are part of the world");
    let recorded = Replay::record(&data, 1, settings, commands).unwrap();
    assert_eq!(recorded.check(&data), Ok(()));
    let mut wrong = recorded.clone();
    wrong.fingerprint ^= 1;
    assert!(matches!(
        wrong.check(&data),
        Err(ReplayError::Diverged { .. })
    ));
    let mut other_packs = recorded;
    other_packs.packs.clear();
    assert_eq!(other_packs.check(&data), Err(ReplayError::PackMismatch));
}

/// The golden replay. When `packs/test` or the simulation changes on purpose, re-baseline in
/// the same commit with `cargo test -p omnis-sim rebaseline -- --ignored`.
#[test]
fn golden_walk_replay_reproduces() {
    let data = data();
    let replay: Replay =
        read_ron(&replay_path("walk"), &replay_path("walk")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        replay.commands,
        walk(),
        "the script in this file is the recorded one"
    );
    replay.check(&data).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
#[ignore = "writes the golden file; run deliberately"]
fn rebaseline_walk_replay() {
    let data = data();
    let replay = Replay::record(&data, 0x0123_4567_89ab_cdef, Settings::default(), walk()).unwrap();
    write_ron(&replay_path("walk"), &replay).unwrap();
}

fn save_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/saves")
        .join(format!("{name}.ron"))
}

/// Captures `tests/saves/v2.ron` from a schema-2 build: one member and one open door, so the
/// migration test exercises every field a party and a map state carry. Run once, deliberately,
/// before the schema moves on.
#[test]
#[ignore = "writes the fixture; run deliberately on a schema-2 build"]
fn capture_schema_2_fixture() {
    let data = data();
    let mut world = world(&data);
    let draft = omnis_rules::Draft {
        name: "Brenna".to_owned(),
        race: "base:race:human".to_owned(),
        class: "base:class:fighter".to_owned(),
        background: "base:background:acolyte".to_owned(),
        alignment: omnis_data::Alignment::LawfulGood,
        scores: [15, 14, 13, 12, 10, 8],
        skills: vec![omnis_data::Skill::Athletics, omnis_data::Skill::Perception],
    };
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Create(draft)),
    )
    .unwrap();
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    world.position = Position {
        map: dungeon,
        x: 5,
        y: 3,
        facing: Facing::East,
    };
    interact(&mut world, &data);
    assert!(world.maps[&dungeon].door_open(5, 3, Facing::East));
    assert_eq!(world.schema, 2);
    write_ron(&save_path("v2"), &world).unwrap();
}
