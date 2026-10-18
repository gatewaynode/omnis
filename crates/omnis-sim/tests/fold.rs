//! The fold (PRD D28, M8.5 7e): the world as it stands becomes the new bases, the journal goes to
//! the archive, and older saves keep the bases they name, so each loads at its own moment. A
//! safe point (a long rest, the inn's room, coming into a settlement) folds once the journal
//! holds `fold_interactions`.

use crate::common::{self, data, party_of};
use crate::objects::{in_yard, objects, yard};
use omnis_core::{Facing, Level, Position};
use omnis_data::{Data, Verb};
use omnis_sim::save::{
    Archived, FOLD_INTERACTIONS, FOLD_RULE, Restored, archive_path, fold_at, load_save, save_files,
    split,
};
use omnis_sim::{
    Command, DevCommand, Event, Journal, LoadError, Mode, Rejection, RestCommand, ServiceCommand,
    Settings, World, apply,
};
use std::collections::BTreeMap;

type Disk = BTreeMap<String, String>;

/// Save `world` at `path` on `disk`, keeping files already there that may be kept.
fn save(world: &World, data: &Data, disk: &mut Disk, path: &str) {
    for file in save_files(world, data, path).unwrap_or_else(|e| panic!("{e}")) {
        if !(file.keep && disk.contains_key(&file.path)) {
            disk.insert(file.path, file.text);
        }
    }
}

fn load(disk: &Disk, data: &Data, path: &str) -> Result<(World, Restored), LoadError> {
    load_save(&disk[path], data, false, &mut |p| {
        disk.get(p).cloned().ok_or_else(|| format!("no file {p}"))
    })
}

fn dev(command: DevCommand) -> Command {
    Command::Dev(command)
}

fn act(object: &str, verb: Verb) -> Command {
    Command::Act {
        object: object.into(),
        verb,
        option: 0,
    }
}

/// The yard with the lever found, the party before it, devtools on.
fn at_the_lever(data: &Data) -> World {
    let mut world = in_yard(data, 0, 0, 1, Facing::North);
    world.settings.devtools = true;
    world.known.notice(yard(data), "lever_1");
    world
}

fn state(world: &World, data: &Data, id: &str) -> Option<String> {
    world
        .maps
        .get(&yard(data))
        .and_then(|s| s.objects.get(id))
        .and_then(|s| s.state.clone())
}

fn folded(events: &[Event]) -> Option<u32> {
    events.iter().find_map(|e| match e {
        Event::Folded { interactions } => Some(*interactions),
        _ => None,
    })
}

#[test]
fn a_fold_makes_the_world_its_bases_and_archives_the_journal() {
    let data = objects();
    let mut world = at_the_lever(&data);
    apply(&mut world, &data, act("lever_1", Verb::Pull)).unwrap();
    let journal = world.journal.clone();
    assert_eq!(journal.len(), 1);
    let events = apply(&mut world, &data, dev(DevCommand::Fold)).unwrap();
    assert_eq!(folded(&events), Some(1));
    assert!(world.journal.is_empty(), "the fold itself journals nothing");
    assert_eq!(world.archive, [Archived::Held(journal)]);
    assert_eq!(world.origin, Some(split(&world, &data).into()));
    assert_eq!(state(&world, &data, "lever_1").as_deref(), Some("down"));
    let again = apply(&mut world, &data, dev(DevCommand::Fold)).unwrap();
    assert_eq!(folded(&again), Some(0));
    assert_eq!(world.archive.len(), 1, "an empty journal archives nothing");
}

#[test]
fn a_fold_is_refused_in_a_fight() {
    let data = data();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/replays/fight.ron");
    let replay: omnis_sim::Replay = omnis_data::ron_io::read_ron(&path, &path).unwrap();
    let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
    for command in replay.commands {
        apply(&mut world, &data, command).unwrap_or_else(|r| panic!("{r}"));
        if matches!(world.mode, Mode::Combat(_)) {
            break;
        }
    }
    assert!(
        matches!(world.mode, Mode::Combat(_)),
        "the golden fight fights"
    );
    world.settings.devtools = true;
    let before = world.clone();
    assert_eq!(
        apply(&mut world, &data, dev(DevCommand::Fold)),
        Err(Rejection::WrongMode)
    );
    assert_eq!(world, before);
}

/// Two folds, a save after each: both load at their own moment, because the first save's
/// bases are kept when the second writes its own.
#[test]
fn each_save_loads_at_its_own_moment_across_folds() {
    let data = objects();
    let mut world = at_the_lever(&data);
    let mut disk = Disk::new();
    apply(&mut world, &data, act("lever_1", Verb::Pull)).unwrap();
    apply(&mut world, &data, dev(DevCommand::Fold)).unwrap();
    save(&world, &data, &mut disk, ".omnis/first.ron");
    world.position = world.position.at(3, 2);
    world.position.facing = Facing::South;
    apply(&mut world, &data, act("gate_1", Verb::Push)).unwrap();
    apply(&mut world, &data, dev(DevCommand::Fold)).unwrap();
    save(&world, &data, &mut disk, ".omnis/second.ron");
    let regions = disk.keys().filter(|p| p.contains("/regions/")).count();
    assert!(regions >= 2, "both moments' yard bases are kept: {disk:?}");
    let (first, _) = load(&disk, &data, ".omnis/first.ron").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(state(&first, &data, "lever_1").as_deref(), Some("down"));
    assert_eq!(
        state(&first, &data, "gate_1"),
        None,
        "the gate still shut then"
    );
    let (second, report) =
        load(&disk, &data, ".omnis/second.ron").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(state(&second, &data, "gate_1").as_deref(), Some("open"));
    assert_eq!(report.applied, 0);
    let mut kept = world.clone();
    kept.log.clear();
    kept.archive = second.archive.clone();
    assert_eq!(second, kept, "the folded world, saved and loaded");
}

#[test]
fn a_save_names_its_archive_and_the_segment_holds_the_folded_interactions() {
    let data = objects();
    let mut world = at_the_lever(&data);
    apply(&mut world, &data, act("lever_1", Verb::Pull)).unwrap();
    let journal = world.journal.clone();
    apply(&mut world, &data, dev(DevCommand::Fold)).unwrap();
    let mut disk = Disk::new();
    save(&world, &data, &mut disk, ".omnis/quick.ron");
    let (back, _) = load(&disk, &data, ".omnis/quick.ron").unwrap_or_else(|e| panic!("{e}"));
    let [Archived::Named(name)] = back.archive.as_slice() else {
        panic!("one segment, by name: {:?}", back.archive);
    };
    let segment = disk
        .iter()
        .find(|(p, _)| p.ends_with(&format!("/archive/{name:016x}.ron")))
        .map(|(_, text)| text)
        .expect("the segment is on disk");
    let read: Journal = omnis_data::ron_io::parse(segment).unwrap();
    assert_eq!(read, journal);
    let id = disk
        .keys()
        .find_map(|p| p.strip_suffix("/manifest.ron"))
        .and_then(|p| p.rsplit('/').next())
        .unwrap();
    assert!(disk.contains_key(&archive_path(id, *name)));
    let mut again = Disk::new();
    save(&back, &data, &mut again, ".omnis/quick.ron");
    assert!(
        !again.keys().any(|p| p.contains("/archive/")),
        "a segment known by name is not written again"
    );
    assert!(again[".omnis/quick.ron"].contains(&format!("archive:[{name}]")));
}

#[test]
fn the_threshold_defaults_in_code() {
    assert_eq!(FOLD_INTERACTIONS, 5000);
    assert_eq!(
        fold_at(&data()),
        FOLD_INTERACTIONS,
        "no shipped pack sets it"
    );
}

/// `data` with the fold threshold at `n`.
fn folding_at(n: i64) -> Data {
    let mut data = data();
    data.rules.insert_value(FOLD_RULE, n);
    data
}

/// A party camped in the meadow with one interaction journaled (the meadow met), devtools on.
fn camped(data: &Data) -> World {
    let mut world = World::new(
        data,
        5,
        Settings {
            devtools: true,
            ..Settings::default()
        },
    )
    .unwrap();
    party_of(&mut world, data, 3);
    world.party.food = 10;
    world.position = Position {
        map: data.registry.maps.get("test:map:meadow").unwrap(),
        level: Level::GROUND,
        x: 16,
        y: 16,
        facing: Facing::North,
    };
    let meet = dev(DevCommand::Reconcile {
        region: "test:region:meadow".into(),
    });
    apply(&mut world, data, meet).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(
        world.journal.len(),
        1,
        "meeting the meadow is an interaction"
    );
    world
}

fn long_rest(world: &mut World, data: &Data) -> Vec<Event> {
    let events = apply(world, data, Command::Rest(RestCommand::Long)).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Rested { long: true, .. })),
        "the night passed unambushed: {events:?}"
    );
    events
}

#[test]
fn a_long_rest_folds_once_the_journal_reaches_the_threshold() {
    let data = folding_at(1);
    let mut world = camped(&data);
    let events = long_rest(&mut world, &data);
    assert_eq!(folded(&events), Some(1));
    assert!(world.journal.is_empty());
    assert_eq!(world.log.last(), events.last(), "the fold is news");
    let data = folding_at(2);
    let mut world = camped(&data);
    assert_eq!(
        folded(&long_rest(&mut world, &data)),
        None,
        "one short of it"
    );
}

#[test]
fn a_short_rest_is_not_a_safe_point() {
    let data = folding_at(1);
    let mut world = camped(&data);
    let events = apply(
        &mut world,
        &data,
        Command::Rest(RestCommand::Short { spend: Vec::new() }),
    )
    .unwrap();
    assert_eq!(folded(&events), None);
    assert_eq!(world.journal.len(), 1);
}

#[test]
fn coming_into_a_settlement_folds_and_a_wild_region_does_not() {
    let data = folding_at(1);
    let mut world = camped(&data);
    let teleport = |map: &str| {
        dev(DevCommand::Teleport {
            map: map.into(),
            level: Level::GROUND,
            x: 1,
            y: 1,
            facing: Facing::North,
        })
    };
    let into_dungeon = apply(&mut world, &data, teleport("test:map:dungeon")).unwrap();
    assert_eq!(folded(&into_dungeon), None, "the dungeon is wild");
    assert_eq!(world.journal.len(), 2);
    let into_town = apply(&mut world, &data, teleport("test:map:town")).unwrap();
    assert_eq!(folded(&into_town), Some(3), "the town is a settlement");
}

#[test]
fn the_inns_room_folds() {
    let data = folding_at(0);
    let mut world = common::inside(&data, "inn");
    let events = apply(&mut world, &data, Command::Service(ServiceCommand::Room)).unwrap();
    assert!(events.iter().any(|e| matches!(e, Event::RoomTaken { .. })));
    assert_eq!(folded(&events), Some(0));
    let data = folding_at(1);
    let mut world = common::inside(&data, "inn");
    let events = apply(&mut world, &data, Command::Service(ServiceCommand::Room)).unwrap();
    assert_eq!(folded(&events), None, "an empty journal is under one");
}

/// Tidy lists the files no save names: a base and an archive segment the remaining save left
/// behind, and a whole world folder no save names. Nothing a save names, and nothing that is not
/// a world-folder file, is ever listed.
#[test]
fn tidy_lists_only_what_no_save_names() {
    let data = objects();
    let mut world = at_the_lever(&data);
    let mut disk = Disk::new();
    apply(&mut world, &data, act("lever_1", Verb::Pull)).unwrap();
    apply(&mut world, &data, dev(DevCommand::Fold)).unwrap();
    save(&world, &data, &mut disk, ".omnis/first.ron");
    world.position = world.position.at(3, 2);
    world.position.facing = Facing::South;
    apply(&mut world, &data, act("gate_1", Verb::Push)).unwrap();
    apply(&mut world, &data, dev(DevCommand::Fold)).unwrap();
    save(&world, &data, &mut disk, ".omnis/second.ron");
    let other = World::new(&data, 99, Settings::default()).unwrap();
    save(&other, &data, &mut disk, ".omnis/other.ron");
    disk.insert(".omnis/worlds/notes.ron".into(), "(mine)".into());
    let parse = |p: &str| omnis_data::ron_io::parse(&disk[p]).unwrap();
    let files: Vec<String> = disk
        .keys()
        .filter(|p| p.starts_with(".omnis/worlds/"))
        .cloned()
        .collect();
    let all: Vec<omnis_sim::save::CharacterSave> =
        [".omnis/first.ron", ".omnis/second.ron", ".omnis/other.ron"]
            .iter()
            .map(|p| parse(p))
            .collect();
    assert!(
        omnis_sim::save::unnamed(&all, &files).is_empty(),
        "every file is named"
    );
    let second_only = omnis_sim::save::unnamed(&all[1..2], &files);
    let first: omnis_sim::save::CharacterSave = parse(".omnis/first.ron");
    let first_base = first.bases.regions.values().next().copied().unwrap();
    assert!(
        second_only
            .iter()
            .any(|p| p.ends_with(&format!("{first_base:016x}.ron"))),
        "the first save's yard: {second_only:?}"
    );
    assert!(
        !second_only.iter().any(|p| p.contains("/archive/")),
        "the second save names both segments: {second_only:?}"
    );
    let other_id = &all[2].world;
    assert!(
        second_only.contains(&format!(".omnis/worlds/{other_id}/manifest.ron")),
        "a world no save names goes whole: {second_only:?}"
    );
    assert!(!second_only.iter().any(|p| p.ends_with("notes.ron")));
    let none = omnis_sim::save::unnamed(&[], &files);
    assert_eq!(
        none.len(),
        files.len() - 1,
        "all but the stranger: {none:?}"
    );
}
