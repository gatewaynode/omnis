//! Pack migrations (M8.5 7h6): a pack's `migrations/NNNN.ron`, sealed with its content, each
//! naming the version it leads to. The `chain` fixture (in `omnis-sim`'s test packs) in two
//! versions: 0.1.0, as a save was made on it, and 0.3.0 with its two migrations. Every check is
//! a refusal with its row, on a scratch copy of 0.3.0 changed one way and resealed.

use crate::common::{base_pack, scratch, test_pack};
use omnis_data::migration::{Kind, Target, chain, compare, names, version};
use omnis_data::seal::seal;
use omnis_data::{Data, load_packs};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../omnis-sim/tests/packs")
}

fn load(chain: &Path) -> Result<Data, Vec<String>> {
    let objects = fixtures().join("objects");
    load_packs(&[&base_pack(), &test_pack(), &objects, chain]).map_err(|r| r.messages())
}

fn v3() -> PathBuf {
    fixtures().join("chain/v3")
}

#[test]
fn both_versions_load_and_only_the_later_has_migrations() {
    let one = load(&fixtures().join("chain/v1")).unwrap();
    assert!(one.migrations.is_empty());
    let three = load(&v3()).unwrap();
    let files: Vec<(&str, &str)> = three.migrations["chain"]
        .iter()
        .map(|m| (m.file.as_str(), m.to.as_str()))
        .collect();
    assert_eq!(files, [("0001.ron", "0.2.0"), ("0002.ron", "0.3.0")]);
    assert_eq!(three.migrations["chain"][1].steps.len(), 5);
}

/// A save made at `v` is offered every file with `v < to <= the pack's version`; versions
/// between files need none (owner, 2026-10-19).
#[test]
fn a_save_is_offered_the_files_after_its_version() {
    let data = load(&v3()).unwrap();
    let offered = |saved: &str| -> Vec<&str> {
        chain(&data, "chain", saved)
            .iter()
            .map(|m| m.file.as_str())
            .collect()
    };
    assert_eq!(offered("0.1.0"), ["0001.ron", "0002.ron"]);
    assert_eq!(offered("0.1.9"), ["0001.ron", "0002.ron"]);
    assert_eq!(offered("0.2.0"), ["0002.ron"]);
    assert_eq!(offered("0.2.7"), ["0002.ron"]);
    assert!(offered("0.3.0").is_empty());
    assert!(
        offered("0.4.0").is_empty(),
        "a later save is offered nothing"
    );
    assert!(offered("one").is_empty(), "a version that is not x.y.z");
    assert!(chain(&data, "base", "0.0.1").is_empty(), "a pack with none");
    assert!(
        chain(&data, "gone", "0.0.1").is_empty(),
        "a pack not loaded"
    );
    // A file past the pack's version is refused at load; were one there, it is not offered.
    let mut later = data.clone();
    let files = later.migrations.get_mut("chain").unwrap();
    let mut past = files[1].clone();
    past.to = "0.4.0".into();
    past.file = "0003.ron".into();
    files.push(past);
    let offered: Vec<&str> = chain(&later, "chain", "0.1.0")
        .iter()
        .map(|m| m.file.as_str())
        .collect();
    assert_eq!(offered, ["0001.ron", "0002.ron"]);
}

#[test]
fn versions_compare_part_by_part() {
    assert_eq!(version("0.10.2"), Some((0, 10, 2)));
    assert_eq!(compare("0.10.0", "0.9.9"), Some(Ordering::Greater));
    for bad in [
        "", "1", "1.2", "1.2.3.4", "1..3", "a.b.c", "1.2.-3", "1.2.3 ",
    ] {
        assert_eq!(version(bad), None, "{bad:?}");
    }
}

/// The name steps composed: a name renamed twice ends where the second leads, and the middle
/// name too; removed names leave.
#[test]
fn a_chain_composes_its_names() {
    let data = load(&v3()).unwrap();
    let all = names(&chain(&data, "chain", "0.1.0"));
    let to = |s: &str| Target::To(s.into());
    let expected = [
        ((Kind::Item, "chain:item:junk"), Target::Removed),
        ((Kind::Item, "chain:item:mid_sword"), to("chain:item:sword")),
        ((Kind::Item, "chain:item:old_sword"), to("chain:item:sword")),
        ((Kind::Spell, "chain:spell:fading"), Target::Removed),
        (
            (Kind::Spell, "chain:spell:old_bolt"),
            to("base:spell:fire_bolt"),
        ),
    ];
    let got: Vec<((Kind, &str), Target)> = all
        .iter()
        .map(|((k, n), t)| ((*k, n.as_str()), t.clone()))
        .collect();
    assert_eq!(got, expected);
    let later = names(&chain(&data, "chain", "0.2.0"));
    assert_eq!(later.len(), 3, "the second file's alone: {later:?}");
    assert_eq!(
        later[&(Kind::Item, "chain:item:mid_sword".into())],
        to("chain:item:sword")
    );
}

/// A scratch copy of 0.3.0 with `file` written as `text`, resealed at 0.3.0.
fn variant(name: &str, file: &str, text: &str) -> PathBuf {
    let root = scratch(&format!("migrations-{name}"));
    copy(&v3(), &root);
    std::fs::write(root.join(file), text).unwrap();
    seal(&root, Some("0.3.0")).unwrap();
    root
}

fn copy(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn one(to: &str, steps: &str) -> String {
    format!("(schema: 1, to: \"{to}\", steps: [{steps}])")
}

/// Each check refuses the pack with its row.
#[test]
fn every_check_is_a_row() {
    let rows: [(&str, &str, String, &str); 19] = [
        (
            "name",
            "migrations/first.ron",
            one("0.2.5", r#"Remove(kind: item, id: "chain:item:junk")"#),
            "migration file 'first.ron' is not NNNN.ron",
        ),
        (
            "to",
            "migrations/0003.ron",
            one("next", r#"Remove(kind: item, id: "chain:item:junk")"#),
            "to 'next' is not x.y.z",
        ),
        (
            "order",
            "migrations/0003.ron",
            one("0.2.5", r#"Remove(kind: item, id: "chain:item:junk")"#),
            "to '0.2.5' must be after the version the migration before leads to",
        ),
        (
            "equal",
            "migrations/0003.ron",
            one("0.3.0", r#"Remove(kind: item, id: "chain:item:junk")"#),
            "to '0.3.0' must be after the version the migration before leads to",
        ),
        (
            "past",
            "migrations/0003.ron",
            one("0.4.0", r#"Remove(kind: item, id: "chain:item:junk")"#),
            "to '0.4.0' is after the pack's version '0.3.0'",
        ),
        (
            "empty",
            "migrations/0001.ron",
            one("0.2.0", ""),
            "a migration takes 1 to 4096 steps",
        ),
        (
            "shape",
            "migrations/0001.ron",
            one("0.2.0", r#"Remove(kind: item, id: "chain:spell:fading")"#),
            "'chain:spell:fading' is not of the form pack:item:name",
        ),
        (
            "other",
            "migrations/0001.ron",
            one("0.2.0", r#"Remove(kind: item, id: "base:item:gem")"#),
            "'base:item:gem' is not this pack's: a migration names only its own pack's ids",
        ),
        (
            "race",
            "migrations/0001.ron",
            one("0.2.0", r#"Remove(kind: race, id: "chain:race:gone")"#),
            "race 'chain:race:gone' cannot be removed: rename or replace it",
        ),
        (
            "nothing",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"Rename(kind: item, from: "chain:item:a", to: "chain:item:a")"#,
            ),
            "a step from 'chain:item:a' changes nothing",
        ),
        (
            "local",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"RenameObject(map: "chain:map:hall", from: "Chest", to: "coffer")"#,
            ),
            "'Chest' is not [a-z0-9_]{1,32}",
        ),
        (
            "kept",
            "migrations/0001.ron",
            one("0.2.0", r#"Remove(kind: item, id: "chain:item:sword")"#),
            "item 'chain:item:sword' is still defined: a migration names only what is gone",
        ),
        (
            "renamed",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"Rename(kind: item, from: "chain:item:sword", to: "base:item:dagger")"#,
            ),
            "item 'chain:item:sword' is still defined: a migration names only what is gone",
        ),
        (
            "lands",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"Rename(kind: item, from: "chain:item:old_sword", to: "chain:item:nowhere")"#,
            ),
            "pack 'chain' renames item 'chain:item:old_sword' to 'chain:item:nowhere', which is \
             not defined by any loaded pack",
        ),
        (
            "map",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"MovePlace(map: "chain:map:cellar", from: (0, 0, 0), to: (0, 1, 0))"#,
            ),
            "map 'chain:map:cellar' is not defined by any loaded pack",
        ),
        (
            "cell",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"MovePlace(map: "chain:map:hall", from: (0, 0, 0), to: (0, 5, 0))"#,
            ),
            "(0, 5, 0) is not a cell of map 'chain:map:hall'",
        ),
        (
            "level",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"MovePlace(map: "chain:map:hall", from: (0, 0, 0), to: (1, 1, 0))"#,
            ),
            "(1, 1, 0) is not a cell of map 'chain:map:hall'",
        ),
        (
            "encounter",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"RenameEncounter(map: "chain:map:hall", from: "vermin", to: "rats")"#,
            ),
            "map 'chain:map:hall' has no encounter 'rats'",
        ),
        (
            "object",
            "migrations/0001.ron",
            one(
                "0.2.0",
                r#"RenameObject(map: "chain:map:hall", from: "coffer", to: "box")"#,
            ),
            "map 'chain:map:hall' has no object 'box'",
        ),
    ];
    for (name, file, text, row) in rows {
        let root = variant(name, file, &text);
        let refused = load(&root).expect_err(name);
        assert!(refused.iter().any(|m| m == row), "{name}: {refused:#?}");
    }
}

/// An object renamed to an id the map still has under its old name is refused twice over:
/// the old id is still there.
#[test]
fn a_rename_from_what_is_still_there_is_refused() {
    let root = variant(
        "still",
        "migrations/0001.ron",
        &one(
            "0.2.0",
            r#"RenameObject(map: "chain:map:hall", from: "coffer", to: "coffer_2")"#,
        ),
    );
    let refused = load(&root).unwrap_err();
    assert!(
        refused.contains(
            &"map 'chain:map:hall' still has object 'coffer': a migration names only what is gone"
                .to_owned()
        ),
        "{refused:#?}"
    );
}

/// Migrations are content: changing one without resealing is refused like any other file.
#[test]
fn a_migration_is_sealed_with_its_pack() {
    let root = scratch("migrations-sealed");
    copy(&v3(), &root);
    let file = root.join("migrations/0001.ron");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, text.replace("coffer", "coffer ")).unwrap();
    let refused = load(&root).unwrap_err();
    assert!(
        refused.iter().any(|m| m.contains("the content changed")),
        "{refused:#?}"
    );
}
