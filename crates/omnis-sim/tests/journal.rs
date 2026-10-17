//! The journal (PRD D28, M8.5 7a): every accepted command since the world's base, with the
//! party's age after it. Not saved yet, so not fingerprinted; a replay rebuilds it.

use crate::common;
use common::{data, step, turn, world};
use omnis_core::{Direction, Rotation};
use omnis_data::ron_io::read_ron;
use omnis_sim::replay::replay_world;
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

#[test]
fn each_accepted_command_is_an_entry_stamped_with_the_party_age_after_it() {
    let data = data();
    let mut world = world(&data);
    assert_eq!(journal_len(&world), 0, "a new world has an empty journal");
    step(&mut world, &data);
    let after_step = world.party_clock().elapsed;
    turn(&mut world, &data, Rotation::Left);
    assert_eq!(journal_len(&world), 2);
    let entries = world.journal.entries();
    assert_eq!(entries[0].command, Command::Step(Direction::Forward));
    assert_eq!(entries[0].minute, after_step);
    assert!(
        after_step > 0,
        "a step takes time, so the stamp is after it"
    );
    assert_eq!(entries[1].command, Command::Turn(Rotation::Left));
    assert_eq!(entries[1].minute, world.party_clock().elapsed);
}

#[test]
fn a_refused_command_leaves_the_journal_as_it_was() {
    let data = data();
    let mut world = world(&data);
    step(&mut world, &data);
    let before = world.journal.clone();
    for command in [
        Command::Dev(DevCommand::SetGold { gold: 5 }),
        Command::Encounter(EncounterChoice::Attack),
    ] {
        let refused = apply(&mut world, &data, command.clone());
        assert!(refused.is_err(), "{command:?} is refused here");
        assert_eq!(world.journal, before, "{command:?}");
    }
}

#[test]
fn a_dev_command_is_an_entry_a_replay_repeats() {
    let data = data();
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    let mut world = common::new_world(&data, 7, settings);
    let gold = Command::Dev(DevCommand::SetGold { gold: 5 });
    apply(&mut world, &data, gold.clone()).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(journal_len(&world), 1);
    assert_eq!(world.journal.entries()[0].command, gold);
}

#[test]
fn the_journal_is_not_in_the_save_or_the_fingerprint() {
    let data = data();
    let mut world = world(&data);
    let fresh = world.clone();
    step(&mut world, &data);
    let mut emptied = world.clone();
    emptied.journal = fresh.journal.clone();
    let text = world.to_ron().unwrap();
    assert!(
        !text.contains("journal"),
        "the journal is not saved yet (7d)"
    );
    assert_eq!(world.fingerprint(), emptied.fingerprint());
    let loaded = World::from_ron(&text, &data, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        journal_len(&loaded),
        0,
        "a loaded world starts a new journal"
    );
}

#[test]
fn a_replay_rebuilds_the_journal_it_was_played_with() {
    let data = data();
    for name in ["walk", "fight"] {
        let replay = golden(name);
        let mut played = World::new(&data, replay.seed, replay.settings).unwrap();
        for command in &replay.commands {
            apply(&mut played, &data, command.clone()).unwrap_or_else(|r| panic!("{r}"));
        }
        let rebuilt = replay_world(&data, replay.seed, replay.settings, &replay.commands)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(rebuilt.journal, played.journal, "{name}");
        let commands: Vec<Command> = rebuilt
            .journal
            .entries()
            .iter()
            .map(|e| e.command.clone())
            .collect();
        assert_eq!(
            commands, replay.commands,
            "{name}: the journal is the script"
        );
        let minutes: Vec<i64> = rebuilt.journal.entries().iter().map(|e| e.minute).collect();
        assert!(minutes.windows(2).all(|w| w[0] <= w[1]), "{name}: in order");
        assert_eq!(rebuilt.fingerprint().unwrap(), replay.fingerprint, "{name}");
    }
}
