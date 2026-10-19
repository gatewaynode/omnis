//! What the loaded packs make of a save, before it loads (M8.5 7h4). `examine` reports a verdict
//! for every save that holds together; `restore` loads what was examined. On other packs the
//! places a save depends on are checked: a broken one loads and warns (owner, 2026-10-19), and
//! no item is lost. The edited maps are the loaded `Data` changed in memory, its packs' versions
//! raised, so the names stay the same and the verdict is `Content`.

use crate::by_name::{Disk, SAVE, after, before, look, read, write};
use crate::common::inside;
use omnis_core::{Facing, ItemId, Level, MapId};
use omnis_data::Data;
use omnis_data::placement::Pile;
use omnis_sim::objects::ObjectState;
use omnis_sim::save::{
    Examined, Manifest, NameKind, PackFit, PlaceFix, Spot, Verdict, examine, load_save, restore,
    world_id,
};
use omnis_sim::{LoadError, World};

fn examined(disk: &Disk, data: &Data) -> Examined {
    examine(&disk[SAVE], data, &mut |p| {
        disk.get(p).cloned().ok_or_else(|| format!("no file {p}"))
    })
    .unwrap_or_else(|e| panic!("{e}"))
}

/// `data` with every pack's version raised: other packs, the same names.
fn edited(data: &Data) -> Data {
    let mut data = data.clone();
    for pack in &mut data.fingerprints {
        pack.version.push_str("-edited");
    }
    data
}

fn yard(data: &Data) -> MapId {
    data.registry.maps.get("objects:map:yard").unwrap()
}

/// Take the ground from a cell of the yard's own level.
fn rock(data: &mut Data, x: u16, y: u16) {
    let yard = yard(data);
    let map = data.maps.get_mut(&yard).unwrap();
    let width = usize::from(map.def.width);
    let level = map
        .levels
        .iter_mut()
        .find(|l| l.level == Level::GROUND)
        .unwrap();
    level.cells[usize::from(y) * width + usize::from(x)] = None;
}

fn spot(x: u16, y: u16) -> Spot {
    Spot {
        level: Level::GROUND,
        x,
        y,
    }
}

fn dagger(data: &Data) -> ItemId {
    data.registry.items.get("base:item:dagger").unwrap()
}

/// Every dagger the yard holds on its ground, the map's piles where the world has none.
fn daggers_on_the_ground(world: &World, data: &Data) -> u32 {
    let yard = yard(data);
    let saved = world
        .maps
        .get(&yard)
        .map(|m| m.ground.clone())
        .unwrap_or_default();
    let mut ground = data.maps[&yard].ground.clone();
    ground.extend(saved);
    ground
        .values()
        .flat_map(|p| &p.items)
        .filter(|(i, _)| *i == dagger(data))
        .map(|(_, n)| u32::from(*n))
        .sum()
}

fn saved_in_the_yard(x: u16, y: u16) -> (World, Data, Disk) {
    let data = before();
    let mut world = crate::objects::in_yard(&data, 0, x, y, Facing::East);
    look(&mut world, &data);
    omnis_sim::save::fold(&mut world, &data);
    let mut disk = Disk::new();
    write(&world, &data, &mut disk);
    (world, data, disk)
}

#[test]
fn the_same_packs_report_same() {
    let (world, data, disk) = saved_in_the_yard(1, 2);
    let examined = examined(&disk, &data);
    let report = examined.report().clone();
    assert_eq!(report.verdict, Verdict::Same);
    assert!(
        report
            .packs
            .iter()
            .all(|p| p.saved == p.loaded && p.saved.is_some())
    );
    assert_eq!((report.moved, report.trimmed), (0, 0));
    assert!(report.missing.is_empty() && report.places.is_empty());
    let (back, restored) = restore(examined, &data, false).unwrap();
    assert_eq!(restored.fit, PackFit::Same);
    assert_eq!(back.position, world.position);
}

#[test]
fn ahead_reports_renamed_and_how_many_moved() {
    let (_, _, disk) = saved_in_the_yard(1, 2);
    let new = after();
    let report = examined(&disk, &new).into_report();
    assert_eq!(report.verdict, Verdict::Renamed);
    assert!(report.moved > 10, "{}", report.moved);
    let ahead = report.packs.iter().find(|p| p.id == "ahead").unwrap();
    assert!(ahead.saved.is_none() && ahead.loaded.is_some());
    assert!(report.places.is_empty(), "{:?}", report.places);
    let (_, restored) = read(&disk, &new).unwrap();
    assert_eq!(restored.fit, PackFit::Renumbered);
}

#[test]
fn a_name_the_packs_lack_reports_unresolved_and_restore_refuses() {
    let (old, new) = (before(), after());
    let mut world = inside(&new, "smith");
    let dagger = new.registry.items.get("ahead:item:dagger").unwrap();
    world.party.inventory.push((dagger, 1));
    let mut disk = Disk::new();
    write(&world, &new, &mut disk);
    let examined = examined(&disk, &old);
    let report = examined.report().clone();
    assert_eq!(report.verdict, Verdict::Unresolved);
    assert!(!report.verdict.loads());
    assert!(
        report.missing.iter().any(|m| m.kind == NameKind::Item
            && m.name == "ahead:item:dagger"
            && m.at == "party.inventory"),
        "{:?}",
        report.missing
    );
    let gone = report.packs.iter().find(|p| p.id == "ahead").unwrap();
    assert!(gone.saved.is_some() && gone.loaded.is_none());
    assert!(matches!(
        restore(examined, &old, false),
        Err(LoadError::Unresolved(m)) if m == report.missing
    ));
    assert!(
        matches!(
            restore(self::examined(&disk, &old), &old, true),
            Err(LoadError::Unresolved(_))
        ),
        "force never passes a missing name"
    );
}

/// A save whose manifest keeps no names, read on other packs.
#[test]
fn a_save_without_names_reports_unnamed_and_loads_only_forced() {
    let (_, data, disk) = saved_in_the_yard(1, 2);
    let path = disk
        .keys()
        .find(|p| p.ends_with("manifest.ron"))
        .unwrap()
        .clone();
    let mut manifest: Manifest = omnis_data::ron_io::parse(&disk[&path]).unwrap();
    let was = world_id(&manifest).unwrap();
    manifest.names = omnis_sim::save::NameTable::default();
    let id = world_id(&manifest).unwrap();
    let mut bare = Disk::new();
    for (p, text) in &disk {
        let text = if *p == path {
            omnis_data::ron_io::to_string(&manifest).unwrap()
        } else {
            text.replace(&was, &id)
        };
        bare.insert(p.replace(&was, &id), text);
    }
    let other = edited(&data);
    let examined = examined(&bare, &other);
    assert_eq!(examined.report().verdict, Verdict::Unnamed);
    assert!(examined.report().packs.iter().all(|p| p.saved != p.loaded));
    assert!(matches!(
        restore(examined, &other, false),
        Err(LoadError::PackMismatch)
    ));
    let (_, restored) = load_save(&bare[SAVE], &other, true, &mut |p| {
        bare.get(p).cloned().ok_or_else(|| format!("no file {p}"))
    })
    .unwrap();
    assert_eq!(restored.fit, PackFit::Forced);
}

#[test]
fn restore_refuses_packs_other_than_the_ones_examined() {
    let (_, data, disk) = saved_in_the_yard(1, 2);
    let examined = examined(&disk, &data);
    assert!(matches!(
        restore(examined, &edited(&data), false),
        Err(LoadError::PackMismatch)
    ));
}

/// Examined on other packs, then restored on yet others: refused, though both sets keep the
/// save's names.
#[test]
fn restore_refuses_other_packs_than_the_content_ones_examined() {
    let (_, data, disk) = saved_in_the_yard(1, 2);
    let examined = examined(&disk, &edited(&data));
    assert_eq!(examined.report().verdict, Verdict::Content);
    let third = edited(&edited(&data));
    assert!(matches!(
        restore(examined, &third, false),
        Err(LoadError::PackMismatch)
    ));
}

#[test]
fn other_packs_with_the_same_names_report_content() {
    let (_, data, disk) = saved_in_the_yard(1, 2);
    let other = edited(&data);
    let report = examined(&disk, &other).into_report();
    assert_eq!(report.verdict, Verdict::Content);
    assert_eq!(report.moved, 0);
    assert!(report.places.is_empty());
    let (_, restored) = read(&disk, &other).unwrap();
    assert_eq!(restored.fit, PackFit::Content);
}

/// Load on `data`, save again and load that: the fixed world is the one its save records.
fn round_trip(disk: &Disk, data: &Data) -> World {
    let (back, _) = read(disk, data).unwrap_or_else(|e| panic!("{e}"));
    let mut again = Disk::new();
    write(&back, data, &mut again);
    let (twice, restored) = read(&again, data).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(restored.fit, PackFit::Same);
    assert_eq!(twice.position, back.position);
    assert_eq!(twice.maps, back.maps);
    back
}

#[test]
fn a_party_in_rock_moves_to_the_nearest_cell_it_can_stand_on() {
    let (_, data, disk) = saved_in_the_yard(2, 2);
    let mut other = edited(&data);
    rock(&mut other, 2, 2);
    let report = examined(&disk, &other).into_report();
    assert_eq!(report.verdict, Verdict::Content);
    assert_eq!(report.places.len(), 1, "{:?}", report.places);
    let place = &report.places[0];
    // Ring 1 in row order: (1, 1) first, but a crate stands across its centre.
    assert_eq!(
        place.fix,
        PlaceFix::PartyMoved {
            from: spot(2, 2),
            to: spot(2, 1)
        }
    );
    assert_eq!(place.map, "objects:map:yard");
    assert_eq!(
        place.to_string(),
        "the party could not stand at objects:map:yard (2, 2); moved to (2, 1)"
    );
    let back = round_trip(&disk, &other);
    assert_eq!((back.position.x, back.position.y), (2, 1));
    assert_eq!(back.position.facing, Facing::East);
}

#[test]
fn a_party_stands_only_where_the_terrain_lets_it() {
    let (_, data, disk) = saved_in_the_yard(2, 2);
    let mut other = edited(&data);
    let yard = yard(&other);
    for t in &mut other.maps.get_mut(&yard).unwrap().def.terrains {
        t.passable = false;
    }
    let report = examined(&disk, &other).into_report();
    let start = data.maps[&yard].def.start;
    assert_eq!(
        report.places[0].fix,
        PlaceFix::PartyMoved {
            from: spot(2, 2),
            to: spot(start.0, start.1)
        },
        "nowhere to stand: the map's start"
    );
}

#[test]
fn an_open_door_no_longer_a_door_is_forgotten() {
    let data = before();
    let yard = yard(&data);
    let map = &data.maps[&yard];
    let (x, y, facing) = (0..map.def.height)
        .flat_map(|y| (0..map.def.width).map(move |x| (x, y)))
        .find_map(|(x, y)| {
            let cell = map.cell(Level::GROUND, x, y)?;
            [Facing::East, Facing::South]
                .into_iter()
                .find(|f| cell.doors.has(*f))
                .map(|f| (x, y, f))
        })
        .expect("the yard has a door");
    let mut world = crate::objects::in_yard(&data, 0, 1, 2, Facing::East);
    world.map_state(yard).open_doors.insert((x, y, facing));
    omnis_sim::save::fold(&mut world, &data);
    let mut disk = Disk::new();
    write(&world, &data, &mut disk);
    let mut other = edited(&data);
    {
        let map = other.maps.get_mut(&yard).unwrap();
        let width = usize::from(map.def.width);
        let level = map
            .levels
            .iter_mut()
            .find(|l| l.level == Level::GROUND)
            .unwrap();
        let cell = level.cells[usize::from(y) * width + usize::from(x)]
            .as_mut()
            .unwrap();
        cell.doors = cell.doors.without(facing);
    }
    let report = examined(&disk, &other).into_report();
    assert_eq!(report.places.len(), 1, "{:?}", report.places);
    assert_eq!(
        report.places[0].fix,
        PlaceFix::DoorForgotten {
            at: spot(x, y),
            facing
        }
    );
    let back = round_trip(&disk, &other);
    assert!(back.maps[&yard].open_doors.is_empty());
}

#[test]
fn a_pile_whose_ground_went_away_is_lost() {
    let data = before();
    let yard = yard(&data);
    let mut world = crate::objects::in_yard(&data, 0, 1, 2, Facing::East);
    world.map_state(yard).ground.insert(
        (Level::GROUND, 4, 1),
        Pile {
            pos: (0, 0),
            items: vec![(dagger(&data), 2)],
        },
    );
    omnis_sim::save::fold(&mut world, &data);
    let mut disk = Disk::new();
    write(&world, &data, &mut disk);
    let before_count = daggers_on_the_ground(&world, &data);
    let mut other = edited(&data);
    rock(&mut other, 4, 1);
    let report = examined(&disk, &other).into_report();
    assert_eq!(report.places.len(), 1, "{:?}", report.places);
    assert_eq!(
        report.places[0].fix,
        PlaceFix::PileLost {
            at: spot(4, 1),
            items: 2
        }
    );
    assert_eq!(
        report.places[0].to_string(),
        "the ground at objects:map:yard (4, 1) is gone, and the 2 items lying there with it"
    );
    let back = round_trip(&disk, &other);
    assert!(!back.maps[&yard].ground.contains_key(&(Level::GROUND, 4, 1)));
    assert_eq!(daggers_on_the_ground(&back, &other), before_count - 2);
    assert_eq!(
        back.party.inventory, world.party.inventory,
        "the party's stores are its own"
    );
}

#[test]
fn what_an_object_the_map_no_longer_places_held_is_lost() {
    let data = before();
    let yard = yard(&data);
    let mut world = crate::objects::in_yard(&data, 0, 1, 2, Facing::East);
    world.map_state(yard).objects.insert(
        "chest_1".into(),
        ObjectState {
            state: Some("open".into()),
            contents: Some(vec![(dagger(&data), 3)]),
            ..ObjectState::default()
        },
    );
    omnis_sim::save::fold(&mut world, &data);
    let mut disk = Disk::new();
    write(&world, &data, &mut disk);
    let before_count = daggers_on_the_ground(&world, &data);
    let mut other = edited(&data);
    {
        let map = other.maps.get_mut(&yard).unwrap();
        let at = map.objects.iter().position(|p| p.id == "chest_1").unwrap();
        map.objects.remove(at);
        // The cells' indices follow the list.
        let at = u16::try_from(at).unwrap();
        for list in map.object_cells.values_mut() {
            list.retain(|&i| i != at);
            for i in list.iter_mut() {
                if *i > at {
                    *i -= 1;
                }
            }
        }
    }
    let report = examined(&disk, &other).into_report();
    assert_eq!(report.places.len(), 1, "{:?}", report.places);
    assert_eq!(
        report.places[0].fix,
        PlaceFix::ObjectForgotten {
            id: "chest_1".into(),
            items: 3
        }
    );
    assert_eq!(
        report.places[0].to_string(),
        "objects:map:yard no longer places 'chest_1'; the 3 items left in it are lost"
    );
    let back = round_trip(&disk, &other);
    assert!(!back.maps[&yard].objects.contains_key("chest_1"));
    assert_eq!(
        daggers_on_the_ground(&back, &other),
        before_count,
        "none laid"
    );
}

/// A cleared encounter the map no longer has is dropped with no warning: nothing was lost.
#[test]
fn a_cleared_encounter_the_map_lost_is_dropped() {
    let data = before();
    let yard = yard(&data);
    let mut world = crate::objects::in_yard(&data, 0, 1, 2, Facing::East);
    world.map_state(yard).cleared.insert("rat".into());
    omnis_sim::save::fold(&mut world, &data);
    let mut disk = Disk::new();
    write(&world, &data, &mut disk);
    let mut other = edited(&data);
    other.maps.get_mut(&yard).unwrap().encounters.clear();
    let report = examined(&disk, &other).into_report();
    assert!(report.places.is_empty(), "{:?}", report.places);
    let back = round_trip(&disk, &other);
    assert!(back.maps[&yard].cleared.is_empty());
    let (kept, _) = read(&disk, &edited(&data)).unwrap();
    assert!(
        kept.maps[&yard].cleared.contains("rat"),
        "kept while the map has it"
    );
}

/// `moved` counts a name only when its number changed.
#[test]
fn moved_counts_only_names_whose_number_changed() {
    let table = |items: &[&str]| omnis_sim::save::NameTable {
        items: items.iter().map(|s| (*s).to_owned()).collect(),
        ..omnis_sim::save::NameTable::default()
    };
    let old = table(&["a", "b", "gone"]);
    assert_eq!(
        old.moved(&table(&["a", "new", "b"])),
        1,
        "b moved; a stayed; gone is missing"
    );
    assert_eq!(old.moved(&old), 0);
}

#[test]
fn automap_tiles_outside_the_map_are_trimmed() {
    let data = before();
    let yard = yard(&data);
    let mut world = crate::objects::in_yard(&data, 0, 1, 2, Facing::East);
    look(&mut world, &data);
    let seen = *world.automap.maps[&yard].values().next().unwrap();
    world.automap.record(yard, Level::GROUND, 40, 1, seen);
    world.automap.record(yard, Level(5), 1, 1, seen);
    omnis_sim::save::fold(&mut world, &data);
    let mut disk = Disk::new();
    write(&world, &data, &mut disk);
    let other = edited(&data);
    let report = examined(&disk, &other).into_report();
    assert_eq!(report.trimmed, 2);
    assert!(report.places.is_empty());
    let back = round_trip(&disk, &other);
    assert!(!back.automap.maps[&yard].contains_key(&(40, 1)));
    assert!(!back.automap.levels.contains_key(&(yard, Level(5))));
}

#[test]
fn the_report_and_the_fixed_world_are_the_same_every_time() {
    let (_, data, disk) = saved_in_the_yard(2, 2);
    let mut other = edited(&data);
    rock(&mut other, 2, 2);
    rock(&mut other, 2, 1);
    let first = examined(&disk, &other).into_report();
    let second = examined(&disk, &other).into_report();
    assert_eq!(first, second);
    let (a, _) = read(&disk, &other).unwrap();
    let (b, _) = read(&disk, &other).unwrap();
    assert_eq!(a.fingerprint().unwrap(), b.fingerprint().unwrap());
    assert_eq!((a.position.x, a.position.y), (3, 1));
}

#[test]
fn a_report_reads_back_as_it_was_written() {
    let (old, new) = (before(), after());
    let mut world = inside(&new, "smith");
    world
        .party
        .inventory
        .push((new.registry.items.get("ahead:item:dagger").unwrap(), 1));
    let mut disk = Disk::new();
    write(&world, &new, &mut disk);
    let report = examined(&disk, &old).into_report();
    let text = omnis_data::ron_io::to_string(&report).unwrap();
    assert!(text.contains("verdict: unresolved"), "{text}");
    assert!(text.contains("kind: item"), "{text}");
    let back: omnis_sim::save::SaveCheck = omnis_data::ron_io::parse(&text).unwrap();
    assert_eq!(back, report);
}
