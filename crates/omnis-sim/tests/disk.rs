//! The character save and the world folder (PRD D28, M8.5 7d): a save is the party's checkpoint at
//! the world's last base, the journal since, and the names of the bases, which sit in a folder of
//! their own. Loading joins the bases and replays the journal. Here the files are a map in memory,
//! as a host would write and read them.

use crate::common;
use common::{data, step, world};
use omnis_data::Data;
use omnis_data::ron_io::read_ron;
use omnis_sim::save::{MAX_JOURNAL, Restored, load_save, save_files};
use omnis_sim::{Command, LoadError, Mode, Replay, SaveRule, Settings, World, apply};
use std::collections::BTreeMap;
use std::path::PathBuf;

const SAVE: &str = ".omnis/saves/one.ron";

fn golden(name: &str) -> Replay {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/replays")
        .join(format!("{name}.ron"));
    read_ron(&path, &path).unwrap_or_else(|e| panic!("{e}"))
}

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/saves")
        .join(format!("{name}.ron"));
    omnis_data::ron_io::read_text(&path, &path).unwrap()
}

/// The files a save writes, by path, in the order written.
fn write(world: &World, data: &Data, disk: &mut BTreeMap<String, String>) -> Vec<String> {
    let files = save_files(world, data, SAVE).unwrap_or_else(|e| panic!("{e}"));
    let order = files.iter().map(|f| f.path.clone()).collect();
    for file in files {
        if !(file.keep && disk.contains_key(&file.path)) {
            disk.insert(file.path, file.text);
        }
    }
    order
}

fn read(
    disk: &BTreeMap<String, String>,
    data: &Data,
    force: bool,
) -> Result<(World, Restored), LoadError> {
    let text = disk.get(SAVE).expect("the character save");
    load_save(text, data, force, &mut |path| {
        disk.get(path)
            .cloned()
            .ok_or_else(|| format!("no file {path}"))
    })
}

/// `world` made the base its journal starts from, as a fold will (7e): the test helpers place a
/// party and fill a purse without commands, which no journal could replay.
fn based(mut world: World, data: &Data) -> World {
    world.origin = Some(omnis_sim::save::split(&world, data));
    world.journal = Default::default();
    world
}

/// The world as a load gives it back: everything but the log of events already seen.
fn kept(world: &World) -> World {
    let mut kept = world.clone();
    kept.log.clear();
    kept
}

fn round_trip(world: &World, data: &Data, at: &str) -> Restored {
    let mut disk = BTreeMap::new();
    write(world, data, &mut disk);
    let (back, report) = read(&disk, data, false).unwrap_or_else(|e| panic!("{at}: {e}"));
    assert_eq!(back, kept(world), "{at}");
    assert_eq!(back.fingerprint(), world.fingerprint(), "{at}");
    report
}

#[test]
fn a_new_games_save_names_no_bases_and_replays_its_journal() {
    let data = data();
    let mut world = World::new(&data, 3, Settings::default()).unwrap();
    common::party_of(&mut world, &data, 2);
    let mut disk = BTreeMap::new();
    let order = write(&world, &data, &mut disk);
    assert_eq!(order.len(), 2, "the manifest, then the save: {order:?}");
    assert!(order[0].starts_with(".omnis/worlds/") && order[0].ends_with("/manifest.ron"));
    assert_eq!(order[1], SAVE, "the character save last");
    let text = &disk[SAVE];
    assert!(!text.contains('\n'), "the character save is compact RON");
    assert!(text.contains("schema:8"), "{text}");
    let (back, report) = read(&disk, &data, false).unwrap();
    assert_eq!(back, kept(&world));
    assert_eq!(report.replayed, 2, "the two drafts");
    assert_eq!(report.converted_from, None);
    assert_eq!(back.journal, world.journal, "rebuilt by the replay");
    assert!(back.log.is_empty(), "the replay's events are not news");
}

#[test]
fn both_goldens_save_and_load_along_the_way() {
    let data = data();
    let mut modes = std::collections::BTreeSet::new();
    for name in ["walk", "fight"] {
        let replay = golden(name);
        let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
        for (at, command) in replay.commands.iter().enumerate() {
            apply(&mut world, &data, command.clone()).unwrap_or_else(|r| panic!("{r}"));
            modes.insert(world.mode.kind());
            round_trip(&world, &data, &format!("{name} after command {at}"));
        }
    }
    assert_eq!(modes.len(), 3, "explore, encounter and combat: {modes:?}");
}

#[test]
fn a_save_in_a_service_and_mid_fight_load_back() {
    let data = data();
    let mut inn = based(common::inside(&data, "inn"), &data);
    round_trip(&inn, &data, "at the inn");
    apply(
        &mut inn,
        &data,
        Command::Service(omnis_sim::ServiceCommand::Leave),
    )
    .unwrap();
    round_trip(&inn, &data, "out of the inn, a command after its base");
    let replay = golden("fight");
    let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
    for command in &replay.commands {
        apply(&mut world, &data, command.clone()).unwrap();
        if matches!(world.mode, Mode::Combat(_)) {
            round_trip(&world, &data, "mid-fight");
            return;
        }
    }
    panic!("the golden fight fights");
}

#[test]
fn repeated_commands_are_one_run() {
    let data = data();
    let mut world = based(world(&data), &data);
    for _ in 0..5 {
        apply(&mut world, &data, Command::Turn(omnis_core::Rotation::Left)).unwrap();
    }
    step(&mut world, &data);
    let mut disk = BTreeMap::new();
    write(&world, &data, &mut disk);
    let text = &disk[SAVE];
    assert_eq!(text.matches("Turn(Left)").count(), 1, "{text}");
    assert!(text.contains("times:5"), "{text}");
    let (back, report) = read(&disk, &data, false).unwrap();
    assert_eq!(report.replayed, 6);
    assert_eq!(back, kept(&world));
}

/// A save file edited to say something false is refused, and nothing is replayed past a lie.
#[test]
fn a_save_that_does_not_hold_together_is_refused() {
    let data = data();
    let mut world = based(world(&data), &data);
    step(&mut world, &data);
    let mut disk = BTreeMap::new();
    write(&world, &data, &mut disk);
    let edit = |from: &str, to: &str| {
        let mut bad = disk.clone();
        let text = bad[SAVE].replacen(from, to, 1);
        bad.insert(SAVE.to_owned(), text);
        bad
    };
    assert!(matches!(
        read(
            &edit("Step(Forward)", "Step(Forward),times:0"),
            &data,
            false
        ),
        Err(LoadError::BadSave(_))
    ));
    let huge = format!("Step(Forward),times:{}", MAX_JOURNAL + 1);
    assert!(matches!(
        read(&edit("Step(Forward)", &huge), &data, false),
        Err(LoadError::BadSave(_))
    ));
    assert!(matches!(
        read(&edit("Step(Forward)", "Encounter(Attack)"), &data, false),
        Err(LoadError::Replay { index: 0, .. })
    ));
    let fingerprint = world.fingerprint().unwrap().to_string();
    assert!(matches!(
        read(&edit(&fingerprint, "1"), &data, false),
        Err(LoadError::Diverged { .. })
    ));
    let id = disk
        .keys()
        .find(|p| p.ends_with("manifest.ron"))
        .and_then(|p| p.split('/').nth(2))
        .unwrap()
        .to_owned();
    let climbing = edit(&id, "../../etc");
    let mut asked = Vec::new();
    let refused = load_save(&climbing[SAVE], &data, false, &mut |path| {
        asked.push(path.to_owned());
        Err("nothing here".into())
    });
    assert!(matches!(refused, Err(LoadError::BadSave(_))));
    assert!(
        asked.is_empty(),
        "a world name that is not one is never read from: {asked:?}"
    );
}

#[test]
fn other_packs_are_refused_unless_forced() {
    let data = data();
    let mut world = based(world(&data), &data);
    step(&mut world, &data);
    let mut disk = BTreeMap::new();
    write(&world, &data, &mut disk);
    let objects = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/packs/objects");
    let more = omnis_data::load_packs(&[&common::base_pack(), &common::test_pack(), &objects])
        .unwrap_or_else(|r| panic!("{r}"));
    assert!(matches!(
        read(&disk, &more, false),
        Err(LoadError::PackMismatch)
    ));
    assert!(read(&disk, &more, true).is_ok(), "force passes the packs");
}

#[test]
fn a_schema_7_save_converts_and_its_bases_are_written() {
    let data = data();
    // Unforced, as the window loads: the fixture was made on today's base and test packs.
    let (world, report) = load_save(&fixture("v7"), &data, false, &mut |p| {
        Err(format!("a schema-7 save reads no other file, not {p}"))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.converted_from, Some(7));
    assert_eq!(report.replayed, 0);
    assert!(world.journal.is_empty());
    assert!(world.origin.is_some(), "the old world is the new base");
    let mut disk = BTreeMap::new();
    let order = write(&world, &data, &mut disk);
    let regions = order.iter().filter(|p| p.contains("/regions/")).count();
    assert!(
        regions >= 2,
        "the walk's regions and the dungeon's: {order:?}"
    );
    assert_eq!(order.last().map(String::as_str), Some(SAVE));
    let manifest_at = order
        .iter()
        .position(|p| p.ends_with("manifest.ron"))
        .unwrap();
    assert!(
        order[..manifest_at]
            .iter()
            .all(|p| p.contains("/regions/") || p.contains("/world/")),
        "bases, then the manifest, then the save: {order:?}"
    );
    let (back, report) = read(&disk, &data, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.converted_from, None);
    assert_eq!(back, kept(&world), "the converted world, saved and loaded");
    let mut tampered = disk.clone();
    let base = order.iter().find(|p| p.contains("/regions/")).unwrap();
    let text = tampered[base].clone() + " ";
    tampered.insert(base.clone(), text);
    assert!(matches!(
        read(&tampered, &data, true),
        Err(LoadError::BadBase(_))
    ));
    let mut missing = disk.clone();
    missing.remove(base);
    assert!(matches!(
        read(&missing, &data, true),
        Err(LoadError::BadBase(_))
    ));
}

#[test]
fn every_older_fixture_converts() {
    let data = data();
    for schema in 1..=7 {
        let (world, report) = load_save(&fixture(&format!("v{schema}")), &data, true, &mut |p| {
            Err(format!("no {p}"))
        })
        .unwrap_or_else(|e| panic!("v{schema}: {e}"));
        assert_eq!(report.converted_from, Some(schema), "v{schema}");
        assert!(world.origin.is_some());
    }
}

#[test]
fn the_save_rule_still_holds() {
    let data = data();
    let settings = Settings {
        save_rule: SaveRule::InnOnly,
        ..Settings::default()
    };
    let world = common::new_world(&data, 1, settings);
    assert!(save_files(&world, &data, SAVE).is_err());
}
