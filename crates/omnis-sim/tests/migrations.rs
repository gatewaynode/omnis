//! Pack migrations on load and reload (M8.5 7h6). The `chain` fixture pack in two versions: a
//! save is made on 0.1.0 and loaded on 0.3.0, whose two migrations rename the old sword twice,
//! replace the old bolt with the base's fire bolt, remove the junk and the fading spell, move a
//! cell of the hall and rename its chest and its rats. A save the steps find something of asks
//! first (`NeedsMigration`, owner 2026-10-19) and loads with the player's consent; one they find
//! nothing of loads as it would, the files recorded as applied.

use crate::by_name::{Disk, look, named, views, write};
use crate::common::{base_pack, party_of, test_pack};
use omnis_core::{Facing, Level, MapId, Position};
use omnis_data::migration::{Kind, Step};
use omnis_data::{Data, Pile, load_packs};
use omnis_sim::objects::ObjectState;
use omnis_sim::omnis_rules::tactics::{ActionRef, Criteria, CriteriaSet, Predicate, Trigger, Who};
use omnis_sim::ops::reload;
use omnis_sim::save::{
    Migrated, NameKind, PackFit, Restored, SaveCheck, Verdict, examine, fold, load_save_with,
    restore,
};
use omnis_sim::{LoadError, Settings, World};
use std::path::PathBuf;

/// The mark on the automap tile of the party's cell in the hall.
const SEEN: i64 = 12_345;

fn packs(version: &str) -> Data {
    let fixture = |name: &str| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/packs")
            .join(name)
    };
    load_packs(&[
        &base_pack(),
        &test_pack(),
        &fixture("objects"),
        &fixture(&format!("chain/{version}")),
    ])
    .unwrap_or_else(|r| panic!("{r}"))
}

fn id<T>(found: Option<T>, what: &str) -> T {
    found.unwrap_or_else(|| panic!("{what}"))
}

fn hall(data: &Data) -> MapId {
    id(data.registry.maps.get("chain:map:hall"), "hall")
}

fn item(data: &Data, name: &str) -> omnis_core::ItemId {
    id(data.registry.items.get(name), name)
}

fn spell(data: &Data, name: &str) -> omnis_core::SpellId {
    id(data.registry.spells.get(name), name)
}

/// The wizard's place in the party.
fn wizard(world: &World, data: &Data) -> usize {
    let class = id(data.registry.classes.get("base:class:wizard"), "wizard");
    id(
        world.party.members.iter().position(|m| m.class == class),
        "a wizard",
    )
}

/// A party of six on 0.1.0 in the hall at (2, 1), holding something of every step: the old
/// sword in its stores and on the ground beside it, the junk in its stores and in the opened
/// chest, the old bolt and the fading spell known to the wizard, the rats cleared, the chest
/// noticed, the automap knowing the cells around.
fn in_the_hall(data: &Data) -> World {
    let mut world = World::new(data, 7, Settings::default()).expect("entry map");
    party_of(&mut world, data, 6);
    let hall = hall(data);
    world.position = Position {
        map: hall,
        level: Level(0),
        x: 2,
        y: 1,
        facing: Facing::East,
    };
    let (sword, junk) = (
        item(data, "chain:item:old_sword"),
        item(data, "chain:item:junk"),
    );
    world.party.inventory.push((sword, 1));
    world.party.inventory.push((junk, 2));
    let w = wizard(&world, data);
    let member = &mut world.party.members[w];
    member
        .known_spells
        .push(spell(data, "chain:spell:old_bolt"));
    member.known_spells.push(spell(data, "chain:spell:fading"));
    let dagger = item(data, "base:item:dagger");
    let state = world.maps.entry(hall).or_default();
    state.cleared.insert("rats".into());
    state.objects.insert(
        "chest_1".into(),
        ObjectState {
            state: Some("open".into()),
            contents: Some(vec![(dagger, 1), (junk, 1)]),
            ..ObjectState::default()
        },
    );
    state.ground.insert(
        (Level(0), 2, 1),
        Pile {
            pos: (0, 0),
            items: vec![(sword, 1)],
        },
    );
    world.known.noticed.insert((hall, "chest_1".into()));
    look(&mut world, data);
    // The automap's tile of the party's cell, marked so its move can be seen.
    if let Some(known) = world
        .automap
        .maps
        .get_mut(&hall)
        .and_then(|t| t.get_mut(&(2, 1)))
    {
        known.seen_at = SEEN;
    }
    // Set by hand, not by play: folded, so the bases hold it as the journal would have.
    fold(&mut world, data);
    world
}

fn read(disk: &Disk, data: &Data, migrate: bool) -> Result<(World, Restored), LoadError> {
    load_save_with(
        &disk[crate::by_name::SAVE],
        data,
        false,
        migrate,
        &mut |p| disk.get(p).cloned().ok_or_else(|| format!("no file {p}")),
    )
}

fn report(disk: &Disk, data: &Data) -> SaveCheck {
    examine(&disk[crate::by_name::SAVE], data, &mut |p| {
        disk.get(p).cloned().ok_or_else(|| format!("no file {p}"))
    })
    .unwrap_or_else(|e| panic!("{e}"))
    .into_report()
}

/// The world with only the log lines a save keeps.
fn told(world: &World) -> World {
    let mut kept = world.clone();
    kept.log.retain(|l| omnis_sim::told(&l.event));
    kept
}

fn saved(world: &World, data: &Data) -> Disk {
    let mut disk = Disk::new();
    write(world, data, &mut disk);
    disk
}

fn applied() -> Vec<Migrated> {
    ["0001.ron", "0002.ron"]
        .map(|file| Migrated {
            pack: "chain".into(),
            file: file.into(),
        })
        .to_vec()
}

/// What each step did to the world, as a client reads it by name.
fn proves_the_steps(world: &World, data: &Data) {
    let hall = hall(data);
    let (sword, dagger) = (
        item(data, "chain:item:sword"),
        item(data, "base:item:dagger"),
    );
    assert!(
        world.party.inventory.contains(&(sword, 1)),
        "the old sword is the sword: {:?}",
        world.party.inventory
    );
    assert_eq!(
        world.party.inventory.len(),
        1,
        "the junk is gone: {:?}",
        world.party.inventory
    );
    let w = wizard(world, data);
    let member = &world.party.members[w];
    let fire_bolt = spell(data, "base:spell:fire_bolt");
    assert_eq!(
        member
            .known_spells
            .iter()
            .filter(|s| **s == fire_bolt)
            .count(),
        1,
        "the old bolt is the fire bolt, known once"
    );
    assert!(
        member.known_spells.iter().all(|s| data
            .registry
            .spells
            .name(*s)
            .is_some_and(|n| n.starts_with("base:"))),
        "the fading spell is gone"
    );
    let p = &world.position;
    assert_eq!(
        (p.map, p.level, p.x, p.y),
        (hall, Level(0), 3, 1),
        "the party moved with its cell"
    );
    let state = &world.maps[&hall];
    assert_eq!(
        state.ground.get(&(Level(0), 3, 1)).map(|p| p.items.clone()),
        Some(vec![(sword, 1)]),
        "the pile moved with its cell, the sword renamed"
    );
    assert!(!state.ground.contains_key(&(Level(0), 2, 1)));
    assert_eq!(
        state.objects.get("coffer").and_then(|o| o.contents.clone()),
        Some(vec![(dagger, 1)]),
        "the chest is the coffer, the junk gone from it"
    );
    assert!(!state.objects.contains_key("chest_1"));
    assert!(state.cleared.contains("vermin") && !state.cleared.contains("rats"));
    assert!(world.known.noticed.contains(&(hall, "coffer".into())));
    let tiles = &world.automap.maps[&hall];
    assert!(!tiles.contains_key(&(2, 1)), "the old cell's tile moved");
    assert_eq!(
        tiles[&(3, 1)].seen_at,
        SEEN,
        "the tile the party stood on moved with its cell"
    );
    assert_eq!(world.migrated, applied());
}

/// The save the steps find something of asks first: the report offers both files, in order,
/// and nothing is missing; unmigrated it is refused with the files named.
#[test]
fn a_save_the_migrations_change_asks_first() {
    let (old, new) = (packs("v1"), packs("v3"));
    let disk = saved(&in_the_hall(&old), &old);
    let check = report(&disk, &new);
    assert_eq!(check.verdict, Verdict::NeedsMigration);
    assert!(check.missing.is_empty(), "{:?}", check.missing);
    let offered: Vec<(&str, &str, &str, usize)> = check
        .migrations
        .iter()
        .map(|m| {
            (
                m.pack.as_str(),
                m.file.as_str(),
                m.to.as_str(),
                m.steps.len(),
            )
        })
        .collect();
    assert_eq!(
        offered,
        [
            ("chain", "0001.ron", "0.2.0", 3),
            ("chain", "0002.ron", "0.3.0", 5)
        ]
    );
    assert!(
        check.places.is_empty(),
        "the moved cell moved the party: {:?}",
        check.places
    );
    match read(&disk, &new, false) {
        Err(LoadError::NeedsMigration(files)) => assert_eq!(
            files,
            [
                ("chain".to_owned(), "0001.ron".to_owned()),
                ("chain".to_owned(), "0002.ron".to_owned())
            ]
        ),
        other => panic!("{:?}", other.map(|(_, r)| r)),
    }
}

/// Applied, every step did its part; the world folded with its journal held for the old
/// folder, and its next save loads as the same world with the migrations it went through.
#[test]
fn applied_the_migrations_rename_replace_remove_and_move() {
    let (old, new) = (packs("v1"), packs("v3"));
    let before = in_the_hall(&old);
    let picks = before.party.members[wizard(&before, &old)].spell_picks;
    let disk = saved(&before, &old);
    let (world, restored) = read(&disk, &new, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(restored.fit, PackFit::Migrated);
    proves_the_steps(&world, &new);
    assert_eq!(
        world.party.members[wizard(&world, &new)].spell_picks,
        picks + 2,
        "the fading spell, and the old bolt replaced by a spell already known, give picks back"
    );
    assert_eq!(world.packs, new.fingerprints);
    assert!(world.journal.is_empty(), "folded");
    let again = saved(&world, &new);
    let (back, restored) = read(&again, &new, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(restored.fit, PackFit::Same);
    assert_eq!(back.fingerprint(), world.fingerprint());
    assert_eq!(
        back.migrated,
        applied(),
        "the save keeps what it went through"
    );
}

/// A save the steps find nothing of loads as it would without them (renumbered by name, the
/// names differing), and still records the files: the versions moved on past them.
#[test]
fn a_save_the_migrations_do_not_touch_loads_and_records_them() {
    let (old, new) = (packs("v1"), packs("v3"));
    let mut world = World::new(&old, 7, Settings::default()).expect("entry map");
    party_of(&mut world, &old, 2);
    let disk = saved(&world, &old);
    let check = report(&disk, &new);
    assert_eq!(check.verdict, Verdict::Renamed);
    assert_eq!(check.migrations.len(), 2, "listed, as applied");
    let (back, restored) = read(&disk, &new, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(restored.fit, PackFit::Renumbered);
    assert_eq!(back.migrated, applied());
    assert_eq!(views(&back, &new), views(&told(&world), &old));
}

/// The same packs offer nothing, and a save made on 0.3.0 is offered nothing by 0.3.0.
#[test]
fn the_same_version_is_offered_nothing() {
    let new = packs("v3");
    let mut world = World::new(&new, 7, Settings::default()).expect("entry map");
    party_of(&mut world, &new, 1);
    let check = report(&saved(&world, &new), &new);
    assert_eq!(check.verdict, Verdict::Same);
    assert!(check.migrations.is_empty());
}

/// A removed name the world cannot do without (the map the party stands on) stays missing,
/// with why; one a list holds leaves it.
#[test]
fn a_removed_map_the_party_stands_on_is_missing() {
    let (old, mut new) = (packs("v1"), packs("v3"));
    let steps = &mut new.migrations.get_mut("chain").unwrap()[1].steps;
    steps.retain(|s| !matches!(s, Step::MovePlace { .. } | Step::RenameEncounter { .. }));
    steps.push(Step::Remove {
        kind: Kind::Map,
        id: "chain:map:hall".into(),
    });
    let disk = saved(&in_the_hall(&old), &old);
    let check = report(&disk, &new);
    assert_eq!(check.verdict, Verdict::Unresolved);
    assert!(
        check
            .missing
            .iter()
            .any(|m| m.kind == NameKind::Map && m.at == "position (removed by a migration)"),
        "{:?}",
        check.missing
    );
    assert!(
        check.missing.iter().all(|m| !m.at.starts_with("maps")),
        "the map's state leaves with it: {:?}",
        check.missing
    );
}

/// Tactics: an action naming a removed spell leaves its runbook; criteria naming one cannot
/// lose it, so it is missing, with where.
#[test]
fn tactics_lose_a_removed_spells_action_but_not_its_criteria() {
    let (old, new) = (packs("v1"), packs("v3"));
    let mut world = in_the_hall(&old);
    let fading = spell(&old, "chain:spell:fading");
    let w = wizard(&world, &old);
    let tactics = &mut world.party.members[w].tactics;
    tactics.runbooks[0]
        .entries
        .push((ActionRef::Spell(fading), 0));
    let entries = tactics.runbooks[0].entries.len();
    let disk = saved(&world, &old);
    let (back, _) = read(&disk, &new, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        back.party.members[wizard(&back, &new)].tactics.runbooks[0]
            .entries
            .len(),
        entries - 1,
        "the action leaves"
    );
    let tactics = &mut world.party.members[w].tactics;
    tactics.library.push(CriteriaSet {
        name: "fade".into(),
        action: ActionRef::Spell(fading),
        trigger: Trigger::SpellCast,
        when: Criteria::Is(Predicate::HasCondition {
            who: Who::Me,
            condition: id(old.registry.conditions.get("base:condition:prone"), "prone"),
        }),
    });
    let at = format!(
        "party.members[{w}].tactics.library[{}] (removed by a migration, named by its criteria)",
        tactics.library.len() - 1
    );
    let check = report(&saved(&world, &old), &new);
    assert_eq!(check.verdict, Verdict::Unresolved);
    assert!(
        check
            .missing
            .iter()
            .any(|m| m.name == "chain:spell:fading" && m.at == at),
        "{:?}",
        check.missing
    );
}

/// A reload runs the same chain from the running packs' versions: refused with the files
/// unmigrated, nothing changed; applied, the world is the one a load makes.
#[test]
fn a_reload_asks_first_and_then_applies_as_a_load_does() {
    let (old, new) = (packs("v1"), packs("v3"));
    let world = in_the_hall(&old);
    let mut running = world.clone();
    let refused = reload(&mut running, &old, &new, false).expect_err("asks first");
    assert_eq!(refused.check.verdict, Verdict::NeedsMigration);
    assert_eq!(running, world, "nothing changed");
    let check = reload(&mut running, &old, &new, true).unwrap_or_else(|r| panic!("{}", r.error));
    assert_eq!(check.verdict, Verdict::NeedsMigration);
    proves_the_steps(&running, &new);
    let (loaded, _) = read(&saved(&world, &old), &new, true).unwrap();
    assert_eq!(running.fingerprint(), loaded.fingerprint());
    assert_eq!(named(&told(&running), &new), named(&loaded, &new));
}

/// `examine` then `restore` with consent is the load with consent; without, the refusal.
#[test]
fn restore_needs_consent() {
    let (old, new) = (packs("v1"), packs("v3"));
    let disk = saved(&in_the_hall(&old), &old);
    let examined = || {
        examine(&disk[crate::by_name::SAVE], &new, &mut |p| {
            disk.get(p).cloned().ok_or_else(|| format!("no file {p}"))
        })
        .unwrap()
    };
    assert!(matches!(
        restore(examined(), &new, false, false),
        Err(LoadError::NeedsMigration(_))
    ));
    let (world, _) = restore(examined(), &new, false, true).unwrap();
    proves_the_steps(&world, &new);
}

/// Only a name step finds something of the save (the old sword carried, far from the hall):
/// it asks first all the same.
#[test]
fn a_name_step_alone_asks_first() {
    let (old, new) = (packs("v1"), packs("v3"));
    let mut world = World::new(&old, 7, Settings::default()).expect("entry map");
    party_of(&mut world, &old, 1);
    world
        .party
        .inventory
        .push((item(&old, "chain:item:old_sword"), 1));
    let check = report(&saved(&world, &old), &new);
    assert_eq!(check.verdict, Verdict::NeedsMigration);
}

/// Only a place step finds something of the save (the party in the hall, holding nothing of
/// `chain`'s): it asks first all the same.
#[test]
fn a_place_step_alone_asks_first() {
    let (old, new) = (packs("v1"), packs("v3"));
    let mut world = World::new(&old, 7, Settings::default()).expect("entry map");
    party_of(&mut world, &old, 1);
    world.position = Position {
        map: hall(&old),
        level: Level(0),
        x: 2,
        y: 1,
        facing: Facing::East,
    };
    let check = report(&saved(&world, &old), &new);
    assert_eq!(check.verdict, Verdict::NeedsMigration);
    let (back, _) = read(&saved(&world, &old), &new, true).unwrap();
    assert_eq!((back.position.x, back.position.y), (3, 1));
}

/// Packs whose names are all the same may still move a cell: the migration runs and asks.
#[test]
fn a_cell_moved_on_packs_with_the_same_names_asks_first() {
    let old = packs("v1");
    let mut new = old.clone();
    let chain = new.packs.iter_mut().find(|p| p.id == "chain").unwrap();
    chain.version = "0.2.0".into();
    let print = new
        .fingerprints
        .iter_mut()
        .find(|p| p.id == "chain")
        .unwrap();
    print.version = "0.2.0".into();
    print.hash ^= 1;
    new.migrations.insert(
        "chain".into(),
        vec![omnis_data::migration::PackMigration {
            file: "0001.ron".into(),
            to: "0.2.0".into(),
            steps: vec![Step::MovePlace {
                map: "chain:map:hall".into(),
                from: (0, 2, 1),
                to: (0, 3, 1),
            }],
        }],
    );
    let mut world = World::new(&old, 7, Settings::default()).expect("entry map");
    party_of(&mut world, &old, 1);
    world.position = Position {
        map: hall(&old),
        level: Level(0),
        x: 2,
        y: 1,
        facing: Facing::East,
    };
    let disk = saved(&world, &old);
    assert_eq!(report(&disk, &new).verdict, Verdict::NeedsMigration);
    let (back, restored) = read(&disk, &new, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(restored.fit, PackFit::Migrated);
    assert_eq!((back.position.x, back.position.y), (3, 1));
}
