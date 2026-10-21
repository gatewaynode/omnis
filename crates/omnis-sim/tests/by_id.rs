//! Fixed encounters by id (M8.5 7h2): a save names a map's fixed encounter by its id, not its
//! row in the map file. A schema 8 save, frozen from the 7h1 build in `tests/saves/v8-cleared/`,
//! names rows in a region base, its journal, its party's encounter and its log; it loads with
//! each row turned into the id that row has, and a row the map does not have refuses the load.
//! The fixture was written once by a test in a worktree of `3f1e56f` (7h1): the golden fight
//! (the rats, row 0, cleared) saved as `journal.ron`; then a fold, a dev teleport to (3, 13) of
//! the dungeon facing south and a step onto the goblins (row 3), saved as `folded.ron`.

use crate::common::data;
use omnis_data::ron_io::{list_under, read_save_under, read_under};
use omnis_sim::journal::Change;
use omnis_sim::save::{Restored, load_save, save_files, tidy_list};
use omnis_sim::{EncounterSource, Event, LoadError, Mode, World};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The rats the golden fight clears, and the goblins the folded save meets: rows 0 and 3.
const RATS: &str = "giant_rat_3_8";
const GOBLINS: &str = "goblin_3_14";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/saves/v8-cleared")
}

/// A fixture save's text, and a reader of its world folder's files by their `.omnis/` path.
fn fixture(name: &str) -> (String, impl FnMut(&str) -> Result<String, String>) {
    let root = root();
    let text = read_save_under(&root, &format!("saves/{name}.ron")).unwrap();
    let read = move |p: &str| {
        let p = p.strip_prefix(".omnis/").ok_or("not under .omnis")?;
        read_under(&root, p).map_err(|e| e.to_string())
    };
    (text, read)
}

/// Loaded forced: its packs are 7h1's, and a save made before 7h loads on other packs only so.
fn load(name: &str) -> Result<(World, Restored), LoadError> {
    let (text, mut read) = fixture(name);
    load_save(&text, &data(), true, &mut read)
}

fn dungeon_cleared(world: &World) -> Vec<String> {
    let dungeon = data().registry.maps.get("test:map:dungeon").unwrap();
    world
        .maps
        .get(&dungeon)
        .map(|s| s.cleared.iter().cloned().collect())
        .unwrap_or_default()
}

#[test]
fn a_schema_8_journal_clears_the_rats_by_id() {
    let (world, report) = load("journal").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.converted_from, Some(8));
    assert_eq!(dungeon_cleared(&world), [RATS]);
    let ids: Vec<&str> = world
        .journal
        .entries()
        .iter()
        .flat_map(|e| &e.changes)
        .filter_map(|c| match c {
            Change::Cleared { id, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ids, [RATS], "the journal's change, by id");
}

#[test]
fn a_schema_8_base_and_encounter_load_by_id() {
    let (world, report) = load("folded").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.converted_from, Some(8));
    assert_eq!(dungeon_cleared(&world), [RATS], "the base's row");
    let origin = world.origin.as_ref().expect("the fold's bases");
    assert!(
        origin
            .regions
            .values()
            .flat_map(|r| r.maps.values())
            .any(|m| m.cleared.contains(RATS)),
        "the base itself, by id"
    );
    let Mode::Encounter(state) = &world.mode else {
        panic!("meeting the goblins: {:?}", world.mode);
    };
    assert_eq!(state.source, EncounterSource::Fixed(GOBLINS.into()));
    let told: Vec<&EncounterSource> = world
        .log
        .iter()
        .filter_map(|l| match &l.event {
            Event::EncounterStarted { source, .. } => Some(source),
            _ => None,
        })
        .collect();
    assert_eq!(
        told,
        [
            &EncounterSource::Fixed(RATS.into()),
            &EncounterSource::Fixed(GOBLINS.into())
        ],
        "the log retells them by id"
    );
}

/// Converted and saved again, the save is schema 9 and loads on today's packs unforced, as the
/// world it was.
#[test]
fn a_converted_save_saves_as_schema_9_and_loads_back() {
    let data = data();
    let (mut world, _) = load("folded").unwrap();
    world.packs.clone_from(&data.fingerprints);
    let mut disk = BTreeMap::new();
    for file in save_files(&world, &data, ".omnis/again.ron").unwrap() {
        disk.insert(file.path, file.text);
    }
    assert!(disk[".omnis/again.ron"].starts_with("(schema:9,"));
    let (back, report) = load_save(&disk[".omnis/again.ron"], &data, false, &mut |p| {
        disk.get(p).cloned().ok_or_else(|| format!("no {p}"))
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.converted_from, None);
    assert_eq!(back.fingerprint(), world.fingerprint());
    assert_eq!(back.mode, world.mode);
    assert_eq!(dungeon_cleared(&back), [RATS]);
}

#[test]
fn a_row_the_map_does_not_have_refuses_the_load() {
    let (text, mut read) = fixture("journal");
    let bad = text.replacen("index:0,", "index:99,", 1);
    assert_ne!(bad, text);
    let refused = load_save(&bad, &data(), true, &mut read).unwrap_err();
    assert_eq!(
        refused,
        LoadError::BadSave("map 'test:map:dungeon' has no fixed encounter in row 99".into())
    );
}

/// Tidy counts a schema 8 save as a save: the files it names are never listed.
#[test]
fn tidy_keeps_what_a_schema_8_save_names() {
    let root = root();
    let mut all = Vec::new();
    for dir in ["saves", "worlds"] {
        all.extend(list_under(&root, dir).unwrap());
    }
    let all: Vec<String> = all.iter().map(|p| format!(".omnis/{p}")).collect();
    assert!(all.iter().any(|p| p.contains("/regions/")), "{all:?}");
    let (_, mut read) = fixture("journal");
    let listed = tidy_list(&all, &mut read).unwrap();
    assert!(listed.is_empty(), "{listed:?}");
}

/// The name table (M8.5 7h2): each kind's string ids in number order, the party's flags, each
/// map's terrains and each object kind's `once` interactions by row. A save writes it into its
/// world's manifest; a manifest without one (before 7h2) keeps its world's name.
#[test]
fn the_manifest_names_what_the_numbers_stand_for() {
    use omnis_sim::save::{Manifest, NameTable, world_id};
    let data = crate::objects::objects();
    let table = NameTable::of(&data);
    for (i, name) in table.maps.iter().enumerate() {
        let id = omnis_core::MapId(u32::try_from(i).unwrap());
        assert_eq!(data.registry.maps.name(id), Some(name.as_str()));
    }
    assert_eq!(table.spells.len(), data.registry.spells.len());
    assert_eq!(table.services.len(), data.registry.services.len());
    let party: Vec<&str> = table.party_flags.iter().map(String::as_str).collect();
    assert!(party.contains(&"objects:flag:debt"), "{party:?}");
    assert!(
        !party.contains(&"objects:flag:lever"),
        "a world flag: {party:?}"
    );
    let yard = &table.terrains["objects:map:yard"];
    let def = &data.maps[&data.registry.maps.get("objects:map:yard").unwrap()].def;
    assert_eq!(yard.len(), def.terrains.len());
    assert!(
        !table.interactions.is_empty(),
        "the yard's once interactions"
    );
    let world = World::new(&data, 7, Default::default()).unwrap();
    let manifest = Manifest::of(&world, &data);
    assert_eq!(manifest.names, table);
    let text = omnis_data::ron_io::to_string(&manifest).unwrap();
    assert_eq!(
        omnis_data::ron_io::parse::<Manifest>(&text).unwrap(),
        manifest
    );
    let bare = Manifest {
        names: NameTable::default(),
        ..manifest.clone()
    };
    assert!(
        !omnis_data::ron_io::to_string(&bare)
            .unwrap()
            .contains("names")
    );
    assert_ne!(world_id(&bare).unwrap(), world_id(&manifest).unwrap());
}

/// A whole world of schema 1 to 7 names its cleared encounters by row too: the v7 fixture, the
/// rats' row written into its dungeon's state, loads with the rats cleared by id.
#[test]
fn a_whole_world_save_clears_by_id() {
    let data = data();
    let path = root().join("../v7.ron");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert_eq!(
        data.registry.maps.name(omnis_core::MapId(1)),
        Some("test:map:dungeon")
    );
    let rows = text.replacen("cleared: [],", "cleared: [0],", 1);
    assert_ne!(rows, text);
    let (world, report) = load_save(&rows, &data, true, &mut |p| Err(format!("no {p}")))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.converted_from, Some(7));
    assert_eq!(dungeon_cleared(&world), [RATS]);
    let far = text.replacen("cleared: [],", "cleared: [99],", 1);
    assert!(matches!(
        load_save(&far, &data, true, &mut |p| Err(format!("no {p}"))),
        Err(LoadError::BadSave(_))
    ));
}

/// The fixture's files in memory under `.omnis/`, its manifest on today's packs: a schema 8
/// world as it would stand had the packs not changed since. The folder is renamed for the new
/// manifest, and the save names it.
fn on_todays_packs(name: &str) -> (BTreeMap<String, String>, String) {
    use omnis_sim::save::{Manifest, world_id};
    let data = data();
    let root = root();
    let (text, _) = fixture(name);
    let save: omnis_sim::save::CharacterSave = omnis_data::ron_io::parse(&text).unwrap();
    let old = save.world.clone();
    let manifest_path = format!("worlds/{old}/manifest.ron");
    let mut manifest: Manifest =
        omnis_data::ron_io::parse(&read_under(&root, &manifest_path).unwrap()).unwrap();
    manifest.packs.clone_from(&data.fingerprints);
    let new = world_id(&manifest).unwrap();
    let mut disk = BTreeMap::new();
    for path in list_under(&root, &format!("worlds/{old}")).unwrap() {
        let moved = path.replacen(&old, &new, 1);
        disk.insert(format!(".omnis/{moved}"), read_under(&root, &path).unwrap());
    }
    disk.insert(
        format!(".omnis/worlds/{new}/manifest.ron"),
        omnis_data::ron_io::to_string(&manifest).unwrap(),
    );
    (disk, text.replacen(&old, &new, 1))
}

/// On its own packs a schema 8 save loads without `force`, though the world it rebuilds cannot
/// have the fingerprint it recorded: that one was taken of rows, not ids.
#[test]
fn a_schema_8_save_on_its_own_packs_loads_unforced() {
    let data = data();
    for name in ["journal", "folded"] {
        let (disk, text) = on_todays_packs(name);
        let (world, report) = load_save(&text, &data, false, &mut |p| {
            disk.get(p).cloned().ok_or_else(|| format!("no {p}"))
        })
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(report.converted_from, Some(8));
        assert_eq!(dungeon_cleared(&world), [RATS], "{name}");
    }
}
