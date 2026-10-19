//! The character save and the world folder (PRD D28, M8.5 7d): a save is the party as it stands,
//! the journal of interactions since the world's last base with the changes each made, and the
//! names of the bases, which sit in a folder of their own. Loading joins the bases with the party
//! and lays the changes over them; no rule runs. Here the files are a map in memory, as a host
//! would write and read them.

use crate::common;
use common::{data, step, world};
use omnis_data::Data;
use omnis_data::ron_io::read_ron;
use omnis_sim::save::{Restored, load_save, save_files};
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

/// The world as a load gives it back: all of it, and of the party's log the events a full log
/// tells (M8.5 7f).
fn kept(world: &World) -> World {
    let mut kept = world.clone();
    kept.log.retain(|l| omnis_sim::told(&l.event));
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
fn a_new_games_save_names_no_bases_and_making_a_party_journals_nothing() {
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
    assert!(text.contains("schema:9"), "{text}");
    let (back, report) = read(&disk, &data, false).unwrap();
    assert_eq!(back, kept(&world));
    assert_eq!(report.applied, 0, "the drafts change only the party");
    assert_eq!(report.converted_from, None);
    assert!(back.journal.is_empty());
    assert!(!back.log.is_empty(), "the party's making is in its log");
    assert_eq!(
        back.log,
        kept(&world).log,
        "a load makes no news of its own"
    );
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
    let mut inn = common::inside(&data, "inn");
    round_trip(&inn, &data, "at the inn");
    apply(
        &mut inn,
        &data,
        Command::Service(omnis_sim::ServiceCommand::Leave),
    )
    .unwrap();
    round_trip(&inn, &data, "out of the inn");
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

/// A party set down by hand, then walking and turning: the party is saved as it stands, so
/// nothing of the walk is journaled.
#[test]
fn walking_journals_nothing_and_a_party_placed_by_hand_saves_as_it_stands() {
    let data = data();
    let mut world = world(&data);
    for n in 0..200 {
        if n % 3 == 0 {
            apply(&mut world, &data, Command::Turn(omnis_core::Rotation::Left)).unwrap();
        } else {
            step(&mut world, &data);
        }
    }
    assert!(world.journal.is_empty(), "{:?}", world.journal);
    let mut disk = BTreeMap::new();
    write(&world, &data, &mut disk);
    assert!(disk[SAVE].contains("journal:[]"), "{}", disk[SAVE]);
    let (back, report) = read(&disk, &data, false).unwrap();
    assert_eq!(report.applied, 0);
    assert_eq!(back, kept(&world));
}

/// A save made after the rules changed under it (the dev socket's `rules.set`) loads: no rule
/// runs on load.
#[test]
fn a_save_loads_after_the_rules_change() {
    let data = data();
    let replay = golden("walk");
    let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
    for command in &replay.commands {
        apply(&mut world, &data, command.clone()).unwrap();
    }
    let mut disk = BTreeMap::new();
    write(&world, &data, &mut disk);
    let mut changed = common::data();
    omnis_sim::ops::rules_set(&mut changed, "encounter.random", "true")
        .unwrap_or_else(|e| panic!("{e}"));
    let (back, _) = read(&disk, &changed, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, kept(&world));
}

/// A save file edited to say something false is refused.
#[test]
fn a_save_that_does_not_hold_together_is_refused() {
    let data = data();
    let replay = golden("walk");
    let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
    for command in &replay.commands {
        apply(&mut world, &data, command.clone()).unwrap();
    }
    assert!(!world.journal.is_empty(), "the walk changes the world");
    let mut disk = BTreeMap::new();
    write(&world, &data, &mut disk);
    let edit = |from: &str, to: &str| {
        let mut bad = disk.clone();
        let text = bad[SAVE].replacen(from, to, 1);
        bad.insert(SAVE.to_owned(), text);
        bad
    };
    assert!(
        matches!(
            read(
                &edit("changes:[", "changes:[Flag(flag:0,value:Some(9)),"),
                &data,
                false
            ),
            Err(LoadError::Diverged { .. })
        ),
        "a change the play never made"
    );
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
    let mut world = world(&data);
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
    // Forced: the fixture was made on the base and test packs before they were sealed (M8.5
    // 7h1), whose numbering is today's.
    let (world, report) = load_save(&fixture("v7"), &data, true, &mut |p| {
        Err(format!("a schema-7 save reads no other file, not {p}"))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.converted_from, Some(7));
    assert_eq!(report.applied, 0);
    assert!(world.journal.is_empty());
    assert!(world.origin.is_some(), "the old world is the new base");
    assert!(world.log.is_empty(), "an old save kept no log");
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
    // Still on the packs the fixture names (made before the seal), so still forced.
    let (back, report) = read(&disk, &data, true).unwrap_or_else(|e| panic!("{e}"));
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

/// A fixture's world: a golden replay, folded halfway, so its save names bases, an archive
/// segment and a journal, and carries a log. The schema 8 fixture is the walk's; the schema 9
/// fixture is the fight's, so its journal clears the rats by id.
fn fixture_world(data: &Data, name: &str) -> World {
    let replay = golden(name);
    let mut world = World::new(data, replay.seed, replay.settings).unwrap();
    world.settings.devtools = true;
    let half = replay.commands.len() / 2;
    for (at, command) in replay.commands.into_iter().enumerate() {
        if at == half {
            apply(&mut world, data, Command::Dev(omnis_sim::DevCommand::Fold))
                .unwrap_or_else(|r| panic!("{r}"));
        }
        apply(&mut world, data, command).unwrap_or_else(|r| panic!("{r}"));
    }
    world
}

/// A fixture's folder (`v8`, `v9`): a save's files as a host writes them under its root, but
/// without the `.omnis/` prefix, which git ignores.
fn fixture_root(schema: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/saves/{schema}"))
}

fn in_fixture(path: &str) -> &str {
    path.strip_prefix(".omnis/").expect("a path under .omnis")
}

/// Writes `tests/saves/v9/`: run with `--ignored` only when the schema moves on and a new frozen
/// save is wanted (into a new folder), never to make the tests below pass.
#[test]
#[ignore = "writes the schema 9 fixture"]
fn write_the_schema_9_fixture() {
    use omnis_data::ron_io::{list_under, remove_under, write_files};
    let data = data();
    let world = fixture_world(&data, "fight");
    let root = fixture_root("v9");
    for dir in ["saves", "worlds"] {
        for old in list_under(&root, dir).unwrap() {
            remove_under(&root, &old).unwrap();
        }
    }
    let files = save_files(&world, &data, SAVE).unwrap_or_else(|e| panic!("{e}"));
    let files: Vec<(&str, &str, bool)> = files
        .iter()
        .map(|f| (in_fixture(&f.path), f.text.as_str(), false))
        .collect();
    write_files(&root, &files).unwrap();
}

/// A frozen schema 8 save (M8.5 7g) still loads: its party, its journal laid over its base, its
/// archive by name and its log. On the packs it was made with it is the world that made it.
#[test]
fn the_schema_8_fixture_still_loads() {
    let data = data();
    let root = fixture_root("v8");
    let text = omnis_data::ron_io::read_save_under(&root, in_fixture(SAVE)).unwrap();
    // Forced, as the older fixtures are: a pack edit must not retire a frozen save.
    let (world, report) = load_save(&text, &data, true, &mut |p| {
        omnis_data::ron_io::read_under(&root, in_fixture(p)).map_err(|e| e.to_string())
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        report.converted_from,
        Some(8),
        "its rows become ids (M8.5 7h2)"
    );
    assert_eq!(report.applied, 1, "the interaction after the fold");
    assert_eq!(world.archive.len(), 1, "the segment the fold put away");
    assert!(world.origin.is_some(), "the bases the fold made");
    assert!(!world.log.is_empty());
    if world.packs == data.fingerprints {
        let made = fixture_world(&data, "walk");
        assert_eq!(world.fingerprint(), made.fingerprint());
        assert_eq!(world.journal, made.journal);
        assert_eq!(world.log, kept(&made).log);
    }
}

/// The frozen schema 9 save (M8.5 7h2) loads as written: no conversion, its world folder's
/// manifest naming what its numbers stand for, the rats cleared by id. On the packs it was made
/// with it is the world that made it.
#[test]
fn the_schema_9_fixture_still_loads() {
    let data = data();
    let root = fixture_root("v9");
    let text = omnis_data::ron_io::read_save_under(&root, in_fixture(SAVE)).unwrap();
    let save: omnis_sim::save::CharacterSave = omnis_data::ron_io::parse(&text).unwrap();
    assert_eq!(save.schema, 9);
    let manifest =
        omnis_data::ron_io::read_under(&root, &format!("worlds/{}/manifest.ron", save.world))
            .unwrap();
    let manifest: omnis_sim::save::Manifest = omnis_data::ron_io::parse(&manifest).unwrap();
    assert!(
        manifest.names.maps.iter().any(|m| m == "test:map:dungeon"),
        "{:?}",
        manifest.names.maps
    );
    let (world, report) = load_save(&text, &data, true, &mut |p| {
        omnis_data::ron_io::read_under(&root, in_fixture(p)).map_err(|e| e.to_string())
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.converted_from, None);
    assert_eq!(world.archive.len(), 1, "the segment the fold put away");
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    assert!(world.maps[&dungeon].cleared.contains("giant_rat_3_8"));
    assert!(
        text.contains("id:\"giant_rat_3_8\""),
        "the journal's change by id"
    );
    if world.packs == data.fingerprints {
        let made = fixture_world(&data, "fight");
        assert_eq!(world.fingerprint(), made.fingerprint());
        assert_eq!(world.journal, made.journal);
        assert_eq!(world.log, kept(&made).log);
    }
}
