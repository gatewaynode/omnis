//! `pack.reload` by name (M8.5 7h5): a reload is a load of the running world. The packs it ran
//! on give the names behind its numbers, and the fresh packs are fitted by those names, as a save
//! made on the old packs would be: the same packs change nothing, other content records them,
//! moved names renumber the world and fold it, places the maps no longer allow load and warn,
//! and a name the fresh packs lack refuses the reload with nothing changed.

use crate::by_name::{Disk, after, armed, before, look, named, read, views, write};
use crate::common::{inside, party_of, play, walk_to_the_rats, world};
use crate::save_check::{edited, rock, saved_in_the_yard, spot};
use omnis_core::Facing;
use omnis_data::Data;
use omnis_sim::ops::reload;
use omnis_sim::save::{Archived, NameKind, PackFit, PlaceFix, SaveCheck, Verdict};
use omnis_sim::{Command, EncounterChoice, ModeKind, OpError, World, apply};

/// Reload `world`, running on `old`, onto `new`, which must take it.
fn reloaded(world: &World, old: &Data, new: &Data, what: &str) -> (World, SaveCheck) {
    let mut now = world.clone();
    let check = reload(&mut now, old, new).unwrap_or_else(|r| panic!("{what}: {}", r.error));
    (now, check)
}

/// Saved on `data` and loaded back, `world` is the same world.
fn saves_and_loads(world: &World, data: &Data, what: &str) {
    let mut disk = Disk::new();
    write(world, data, &mut disk);
    let (back, restored) = read(&disk, data).unwrap_or_else(|e| panic!("{what}: {e}"));
    assert_eq!(restored.fit, PackFit::Same, "{what}");
    let mut kept = world.clone();
    kept.log.retain(|l| omnis_sim::told(&l.event));
    assert_eq!(back.fingerprint(), kept.fingerprint(), "{what}");
}

/// Reload `world`, running on base, test and objects, with `ahead` added: every number moves,
/// and every view and every named thing reads the same; the world folded, its journal held
/// for the old packs' folder; saved, it loads back as the same world.
fn proves(world: &World, what: &str) {
    let (old, new) = (before(), after());
    let (now, check) = reloaded(world, &old, &new, what);
    assert_eq!(check.verdict, Verdict::Renamed, "{what}");
    assert!(check.moved > 10, "{what}: {}", check.moved);
    assert!(
        check.missing.is_empty() && check.places.is_empty(),
        "{what}"
    );
    assert_eq!(now.packs, new.fingerprints, "{what}");
    assert_eq!(views(&now, &new), views(world, &old), "{what}");
    assert_eq!(named(&now, &new), named(world, &old), "{what}: by name");
    assert!(now.journal.is_empty(), "{what}: folded");
    if !world.journal.is_empty() {
        let Some(Archived::Held {
            from: Some(from), ..
        }) = now.archive.last()
        else {
            panic!("{what}: the journal held for its old folder");
        };
        assert_eq!(from.packs, old.fingerprints, "{what}");
    }
    saves_and_loads(&now, &new, what);
}

#[test]
fn a_reload_that_adds_definitions_keeps_every_view_exploring() {
    let data = before();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    play(&mut world, &data, &walk_to_the_rats());
    assert_eq!(world.mode.kind(), ModeKind::Explore);
    armed(&mut world, &data);
    assert!(!world.journal.is_empty(), "the fight's interactions");
    proves(&world, "exploring");
}

#[test]
fn a_reload_mid_fight_keeps_every_view() {
    let data = before();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    for command in walk_to_the_rats() {
        if world.mode.kind() != ModeKind::Explore {
            break;
        }
        apply(&mut world, &data, command).unwrap_or_else(|r| panic!("{r}"));
    }
    if world.mode.kind() == ModeKind::Encounter {
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
    }
    assert_eq!(world.mode.kind(), ModeKind::Combat);
    armed(&mut world, &data);
    proves(&world, "mid-fight");
}

#[test]
fn a_reload_in_town_keeps_every_view() {
    let data = before();
    for service in ["smith", "bank", "inn"] {
        let mut world = inside(&data, service);
        armed(&mut world, &data);
        proves(&world, service);
    }
}

#[test]
fn a_reload_in_the_yard_keeps_every_view() {
    let data = before();
    let mut world = crate::objects::in_yard(&data, 0, 4, 3, Facing::East);
    look(&mut world, &data);
    armed(&mut world, &data);
    proves(&world, "the yard");
}

/// A reload whose packs lack a name the world holds is refused with the report, and nothing
/// changes.
#[test]
fn a_reload_that_lacks_a_carried_name_is_refused_and_changes_nothing() {
    let (old, new) = (after(), before());
    let mut world = inside(&old, "smith");
    let dagger = old.registry.items.get("ahead:item:dagger").unwrap();
    world.party.inventory.push((dagger, 1));
    let was = world.clone();
    let refused = reload(&mut world, &old, &new).expect_err("the dagger is gone");
    assert_eq!(world, was, "nothing changed");
    assert_eq!(refused.check.verdict, Verdict::Unresolved);
    assert!(
        refused
            .check
            .missing
            .iter()
            .any(|m| m.kind == NameKind::Item
                && m.name == "ahead:item:dagger"
                && m.at == "party.inventory"),
        "{:?}",
        refused.check.missing
    );
    let OpError::Failed { message } = &refused.error else {
        panic!("{:?}", refused.error);
    };
    assert!(
        message.contains("ahead:item:dagger") && message.ends_with("nothing changed"),
        "{message}"
    );
    assert_eq!(OpError::from(refused.clone()), refused.error);
}

#[test]
fn the_same_packs_reload_as_same_and_change_nothing() {
    let (world, data, _) = saved_in_the_yard(1, 2);
    let (now, check) = reloaded(&world, &data, &data, "same");
    assert_eq!(check.verdict, Verdict::Same);
    assert!(check.packs.iter().all(|p| p.saved == p.loaded));
    assert_eq!(now, world);
}

/// Other packs with every name the same: recorded, the journal kept, nothing else changed.
#[test]
fn other_content_with_the_same_names_records_the_packs_and_keeps_the_journal() {
    let data = before();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    play(&mut world, &data, &walk_to_the_rats());
    assert!(!world.journal.is_empty());
    let other = edited(&data);
    let (now, check) = reloaded(&world, &data, &other, "content");
    assert_eq!(check.verdict, Verdict::Content);
    assert_eq!(check.moved, 0);
    assert_eq!(now.packs, other.fingerprints);
    let mut expected = world.clone();
    expected.packs.clone_from(&other.fingerprints);
    assert_eq!(now, expected, "only the packs recorded; no fold");
    saves_and_loads(&now, &other, "content");
}

/// The fresh maps wall the party in: it moves to the nearest cell it can stand on, with the
/// warning, and the world folds, so its next save loads.
#[test]
fn a_reload_that_walls_the_party_in_moves_it_and_warns() {
    let (world, data, _) = saved_in_the_yard(2, 2);
    let mut other = edited(&data);
    rock(&mut other, 2, 2);
    let (now, check) = reloaded(&world, &data, &other, "walled in");
    assert_eq!(check.verdict, Verdict::Content);
    assert_eq!(check.places.len(), 1, "{:?}", check.places);
    assert_eq!(
        check.places[0].fix,
        PlaceFix::PartyMoved {
            from: spot(2, 2),
            to: spot(2, 1)
        }
    );
    assert_eq!((now.position.x, now.position.y), (2, 1));
    saves_and_loads(&now, &other, "walled in");
}

/// A refused fit reports what is missing and no places: the places of a world that cannot be
/// renumbered are not checked, since it is dropped half-renumbered.
#[test]
fn a_refused_reload_reports_no_places() {
    let old = after();
    let mut world = crate::objects::in_yard(&old, 0, 2, 2, Facing::East);
    let dagger = old.registry.items.get("ahead:item:dagger").unwrap();
    world.party.inventory.push((dagger, 1));
    let mut new = edited(&before());
    rock(&mut new, 2, 2);
    let refused = reload(&mut world.clone(), &old, &new).expect_err("the dagger is gone");
    assert_eq!(refused.check.verdict, Verdict::Unresolved);
    assert!(!refused.check.missing.is_empty());
    assert!(
        refused.check.places.is_empty() && refused.check.trimmed == 0,
        "{:?}",
        refused.check.places
    );
}
