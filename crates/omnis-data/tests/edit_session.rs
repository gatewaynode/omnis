//! `edit::Session` (M5.2) on a scratch copy of `crates/omnis-sim/tests/packs/test`: open by id, save refused while the
//! loader would refuse, save then reseal (the patch version rises) so the pack loads again with
//! the map as the document holds it; every other file read, checked and written as typed; the
//! notes store written at once and never sealed.

use crate::common::{base_pack, copy_pack, load_test_packs, test_pack};
use omnis_core::{Facing, Level};
use omnis_data::edit::{Brush, EdgeKind, EditError, Place, Session};
use omnis_data::{Data, RestEventDef, RestKind, load_packs, seal};
use std::path::{Path, PathBuf};

fn session(name: &str) -> (Session, PathBuf, Data) {
    let scratch = copy_pack(&test_pack(), name);
    let roots = vec![base_pack(), scratch.clone()];
    let data = load_packs(&[&base_pack(), &scratch]).unwrap_or_else(|r| panic!("{r}"));
    (Session::new(roots).unwrap(), scratch, data)
}

fn reload(scratch: &Path) -> Result<Data, String> {
    load_packs(&[&base_pack(), scratch]).map_err(|r| r.to_string())
}

/// The version `pack.ron` names now.
fn version_of(root: &Path) -> String {
    seal::status(root).unwrap().version
}

/// The version the next save or write seals at, the patch one higher: read from the pack,
/// not pinned, since the owner's own saves through the editor move it (M5.10).
fn next_patch(root: &Path) -> String {
    let version = version_of(root);
    let (rest, patch) = version.rsplit_once('.').unwrap();
    format!("{rest}.{}", patch.parse::<u32>().unwrap() + 1)
}

#[test]
fn the_pack_lists_its_maps_and_files_and_opens_by_id() {
    let (mut session, _, _) = session("edit-session-list");
    assert_eq!(session.pack(), "test");
    let maps = session.maps().unwrap();
    let ids: Vec<&str> = maps.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "test:map:depths",
            "test:map:dungeon",
            "test:map:meadow",
            "test:map:town"
        ]
    );
    assert!(maps.iter().all(|m| m.error.is_none()));
    let files = session.files().unwrap();
    assert_eq!(files[0], "pack.ron");
    assert!(files.contains(&"data/maps/town.ron".to_owned()));
    assert!(files.iter().any(|f| f.starts_with("text/en/")));
    assert_eq!(
        session.open("test:map:nowhere"),
        Err(EditError::NoSuchTarget("map 'test:map:nowhere'".into()))
    );
    session.open("test:map:town").unwrap();
    assert_eq!(session.document().unwrap().file(), "data/maps/town.ron");
}

#[test]
fn a_save_is_refused_while_the_loader_would_refuse_and_nothing_is_written() {
    let (mut session, scratch, data) = session("edit-session-refused");
    session.open("test:map:town").unwrap();
    let before = std::fs::read_to_string(scratch.join("data/maps/town.ron")).unwrap();
    let seal_before = std::fs::read_to_string(scratch.join("pack.ron")).unwrap();
    session
        .document_mut()
        .unwrap()
        .place(Place::RestEvent(RestEventDef {
            terrain: "street".into(),
            rest: RestKind::Short,
            chance: 10,
            text: "test:text:rest.nowhere".into(),
        }))
        .unwrap();
    let status = session.status(&data);
    assert!(status.dirty);
    assert_eq!(status.errors.len(), 1, "{:?}", status.errors);
    assert!(
        status.errors[0].contains("test:text:rest.nowhere"),
        "{:?}",
        status.errors
    );
    match session.save(&data) {
        Err(EditError::Invalid(errors)) => assert_eq!(errors.len(), 1),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(scratch.join("data/maps/town.ron")).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read_to_string(scratch.join("pack.ron")).unwrap(),
        seal_before
    );
    assert!(session.document().unwrap().dirty());
}

#[test]
fn a_save_writes_the_map_reseals_at_the_next_patch_and_the_pack_loads_again() {
    let (mut session, scratch, data) = session("edit-session-save");
    session.open("test:map:dungeon").unwrap();
    let wall = Brush::Edge {
        side: Facing::South,
        kind: EdgeKind::Door,
    };
    session
        .document_mut()
        .unwrap()
        .paint(Level::GROUND, 1, 1, &wall, false)
        .unwrap();
    let def = session.document().unwrap().def();
    let next = next_patch(&scratch);
    let sealed = session.save(&data).unwrap();
    assert_eq!(
        sealed.version, next,
        "the owner's choice: a save raises the patch"
    );
    assert!(sealed.changed);
    assert!(!session.document().unwrap().dirty());
    let again = reload(&scratch).unwrap_or_else(|e| panic!("{e}"));
    let id = again.registry.maps.get("test:map:dungeon").unwrap();
    assert_eq!(again.maps[&id].def, def);
    assert_eq!(again.fingerprints[1].version, next);
    let text = std::fs::read_to_string(scratch.join("pack.ron")).unwrap();
    assert!(text.starts_with("//"), "pack.ron keeps its comment: {text}");
    // Saving again without a change writes nothing new.
    let sealed = session.save(&data).unwrap();
    assert!(!sealed.changed);
    assert_eq!(sealed.version, next);
}

#[test]
fn a_new_map_takes_its_pack_and_a_free_file() {
    let (mut session, scratch, data) = session("edit-session-new");
    let town = {
        session.open("test:map:town").unwrap();
        session.document().unwrap().def()
    };
    let new = |id: &str| omnis_data::edit::NewMap {
        id: id.into(),
        name: "test:text:map.town.name".into(),
        kind: town.kind,
        tileset: town.tileset.clone(),
        width: 4,
        height: 3,
        wall: town.wall.clone(),
        door: town.door.clone(),
        door_open: town.door_open.clone(),
        terrain: town.terrains[0].clone(),
    };
    assert_eq!(
        session.new_map(new("base:map:hamlet")),
        Err(EditError::BadId("base:map:hamlet".into()))
    );
    assert_eq!(
        session.new_map(new("test:item:hamlet")),
        Err(EditError::BadId("test:item:hamlet".into()))
    );
    assert_eq!(
        session.new_map(new("test:map:town")),
        Err(EditError::Exists("data/maps/town.ron".into()))
    );
    session.new_map(new("test:map:hamlet")).unwrap();
    let doc = session.document().unwrap();
    assert!(doc.dirty(), "a new map is unsaved");
    assert_eq!(doc.def().layout.len(), 7);
    let status = session.status(&data);
    assert_eq!(
        status.errors,
        ["data/maps/hamlet.ron: map 'test:map:hamlet' belongs to no region"],
        "the loader's words, file first"
    );
    // Name it in a region through the RON view, then the save goes through and the pack loads.
    let regions = session.read("data/regions/town.ron").unwrap();
    assert!(regions.contains("test:map:town"));
    let named = regions.replace(
        "\"test:map:town\"",
        "\"test:map:town\", \"test:map:hamlet\"",
    );
    assert_eq!(
        session
            .check("data/regions/town.ron", &named, &data)
            .unwrap(),
        vec![]
    );
    session
        .write("data/regions/town.ron", &named, &data)
        .unwrap();
    let data = reload(&scratch).expect_err("the region names a map that has no file yet");
    assert!(data.contains("test:map:hamlet"), "{data}");
    let data = load_test_packs();
    let errors = session.status(&data).errors;
    assert_eq!(
        errors,
        ["data/maps/hamlet.ron: map 'test:map:hamlet' belongs to no region"],
        "against the shipped data"
    );
    let _ = scratch;
}

#[test]
fn every_file_reads_checks_clean_and_a_broken_one_is_refused_in_the_loaders_words() {
    let (mut session, scratch, data) = session("edit-session-files");
    for file in session.files().unwrap() {
        let text = session.read(&file).unwrap();
        assert_eq!(
            session.check(&file, &text, &data).unwrap(),
            vec![],
            "{file}"
        );
    }
    let meadow = session.read("data/maps/meadow.ron").unwrap();
    let broken = meadow.replacen("width: ", "widt: ", 1);
    let errors = session
        .check("data/maps/meadow.ron", &broken, &data)
        .unwrap();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].message.starts_with("parse error:"),
        "{}",
        errors[0]
    );
    assert!(errors[0].line.is_some(), "the line is named");
    match session.write("data/maps/meadow.ron", &broken, &data) {
        Err(EditError::Invalid(e)) => assert_eq!(e.len(), 1),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        session.read("data/maps/meadow.ron").unwrap(),
        meadow,
        "nothing written"
    );
    for bad in [
        "../x.ron",
        "/etc/x.ron",
        "data/maps/meadow.txt",
        "assets/x.ron",
        "data/nowhere/x.ron",
    ] {
        assert!(session.write(bad, "()", &data).is_err(), "{bad}");
        assert!(!scratch.join(bad).exists(), "{bad}");
    }
    // Written as typed: a comment survives, and the pack is resealed.
    let commented = format!("// A note the writer keeps.\n{meadow}");
    let next = next_patch(&scratch);
    let sealed = session
        .write("data/maps/meadow.ron", &commented, &data)
        .unwrap()
        .unwrap();
    assert_eq!(sealed.version, next);
    assert_eq!(session.read("data/maps/meadow.ron").unwrap(), commented);
    reload(&scratch).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn notes_are_written_at_once_never_sealed_and_orphans_are_told_apart() {
    let (mut session, scratch, data) = session("edit-session-notes");
    // The shipped pack carries notes (M5.10: the maps' comments, and the owner's on the
    // town's portal cell): the test counts from them, and its map note replaces the town's.
    let shipped = session.notes().len();
    let shipped_town = session.notes().with_prefix("test:map:town").len();
    assert!(session.notes().get("test:map:town").is_some());
    let version = version_of(&scratch);
    let seal_before = std::fs::read_to_string(scratch.join("pack.ron")).unwrap();
    assert_eq!(
        session.set_note("Not a key", Some("x")),
        Err(EditError::BadId("Not a key".into()))
    );
    assert_eq!(
        session.set_note("test:map:town", Some(&"n".repeat(4097))),
        Err(EditError::OutOfRange)
    );
    session
        .set_note(
            "test:map:town",
            Some("A street with seven services behind doors."),
        )
        .unwrap();
    session
        .set_note("test:map:town/0/1,1", Some("The inn's front room."))
        .unwrap();
    session
        .set_note("base:item:mace", Some("Every cleric starts with one."))
        .unwrap();
    session
        .set_note("test:map:town/0/40,1", Some("Outside the map."))
        .unwrap();
    session
        .set_note("test:monster:dragon", Some("Planned for the depths."))
        .unwrap();
    assert_eq!(session.notes().len(), shipped + 4);
    assert_eq!(
        session.notes().get("test:map:town/0/1,1"),
        Some("The inn's front room.")
    );
    assert_eq!(
        session.notes().with_prefix("test:map:town").len(),
        shipped_town + 2
    );
    assert!(
        scratch.join("notes/town.ron").is_file(),
        "a map's notes in its own file"
    );
    assert!(
        scratch.join("notes/item.ron").is_file(),
        "other kinds by kind"
    );
    assert_eq!(
        std::fs::read_to_string(scratch.join("pack.ron")).unwrap(),
        seal_before,
        "a note moves no seal"
    );
    assert_eq!(
        session.orphaned_notes(&data),
        ["test:map:town/0/40,1", "test:monster:dragon"]
    );
    let data = reload(&scratch).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        data.fingerprints[1].version, version,
        "the pack loads unchanged"
    );
    // A second session reads the files back; taking the last note of a file removes the file.
    let again = Session::new(vec![base_pack(), scratch.clone()]).unwrap();
    assert_eq!(again.notes().len(), shipped + 4);
    session.set_note("base:item:mace", None).unwrap();
    assert!(!scratch.join("notes/item.ron").exists());
    assert_eq!(session.notes().len(), shipped + 3);
    // The RON view writes a notes file as typed too, with no reseal.
    let text = session.read("notes/town.ron").unwrap();
    assert_eq!(session.write("notes/town.ron", &text, &data).unwrap(), None);
}
