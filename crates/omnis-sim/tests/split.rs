//! The world in its parts (PRD D28, M8.5 7b): `join(split(w))` is `w` but for the log and the
//! journal, along every command of both goldens; each part holds what it should; a new world's
//! parts are empty but for the party, so base 0 is the packs.

use crate::common;
use common::{data, world};
use omnis_core::{ActorId, Clock, Contact, EraId, HolderId, MapId, Pcg32, StreamName};
use omnis_data::Data;
use omnis_data::ron_io::read_ron;
use omnis_sim::save::{JoinError, join, split, split_report};
use omnis_sim::{LoadError, MapState, Mode, ModeKind, PARTY, Replay, World, apply};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn golden(name: &str) -> Replay {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/replays")
        .join(format!("{name}.ron"));
    read_ron(&path, &path).unwrap_or_else(|e| panic!("{e}"))
}

/// The world as its parts keep it: without the log and the journal.
fn kept(world: &World) -> World {
    let mut kept = world.clone();
    kept.log.clear();
    kept.journal = Default::default();
    kept
}

/// `join(split(w))` is `w` as its parts keep it, to the fingerprint.
fn round_trip(world: &World, data: &Data, at: &str) {
    let back = join(split(world, data), data, false).unwrap_or_else(|e| panic!("{at}: {e}"));
    assert_eq!(back, kept(world), "{at}");
    assert_eq!(back.fingerprint(), world.fingerprint(), "{at}");
}

#[test]
fn a_new_world_splits_into_the_party_alone() {
    let data = data();
    let fresh = World::new(&data, 5, Default::default()).unwrap();
    let parts = split(&fresh, &data);
    assert!(
        parts.regions.is_empty(),
        "no region has a base: {:?}",
        parts.regions
    );
    assert!(parts.world.is_empty(), "{:?}", parts.world);
    assert_eq!(parts.manifest.seed, 5);
    assert_eq!(parts.manifest.packs, data.fingerprints);
    assert_eq!(
        join(parts, &data, false).unwrap(),
        fresh,
        "base 0 is the packs"
    );
}

#[test]
fn both_goldens_round_trip_at_every_command() {
    let data = data();
    let mut modes = BTreeSet::new();
    let mut fight_bus = false;
    for name in ["walk", "fight"] {
        let replay = golden(name);
        let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
        let start = world.bus.clone();
        round_trip(&world, &data, name);
        for (at, command) in replay.commands.iter().enumerate() {
            apply(&mut world, &data, command.clone()).unwrap_or_else(|r| panic!("{r}"));
            modes.insert(world.mode.kind());
            fight_bus |= world.bus != start;
            round_trip(&world, &data, &format!("{name} after command {at}"));
        }
    }
    assert_eq!(
        modes,
        BTreeSet::from([ModeKind::Explore, ModeKind::Encounter, ModeKind::Combat]),
        "the goldens explore, meet and fight; a service is split below"
    );
    assert!(
        fight_bus,
        "a fight's own subscriptions were split and joined"
    );
}

#[test]
fn a_world_inside_a_service_round_trips() {
    let data = data();
    for name in ["inn", "temple", "smith"] {
        let world = common::inside(&data, name);
        round_trip(&world, &data, name);
    }
}

#[test]
fn each_part_holds_its_own() {
    let data = data();
    let replay = golden("walk");
    let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
    for command in &replay.commands {
        apply(&mut world, &data, command.clone()).unwrap_or_else(|r| panic!("{r}"));
    }
    world.map_state(world.position.map);
    let here = data.maps[&world.position.map].region;
    let region = HolderId::Region(here);
    let parts = split(&world, &data);
    let base = &parts.regions[&here];
    assert!(base.maps.contains_key(&world.position.map), "its map");
    assert!(base.holdings.clocks.contains_key(&region), "its clock");
    assert!(
        base.holdings.contacts.contains_key(&(region, PARTY)),
        "its side of meeting the party"
    );
    let stream = StreamName::time(PARTY, region);
    assert!(
        base.holdings.streams.contains_key(&stream),
        "the stream it shares with the party: {:?}",
        base.holdings.streams.keys()
    );
    let party = &parts.checkpoint.holdings;
    assert!(party.clocks.contains_key(&PARTY));
    assert!(
        party.contacts.contains_key(&(PARTY, region)),
        "the party's side"
    );
    assert!(!party.clocks.contains_key(&region));
    assert!(!party.streams.contains_key(&stream));
    assert!(
        party.streams.keys().all(|s| !s.0.starts_with("time:")),
        "{:?}",
        party.streams.keys()
    );
    for (id, other) in &parts.regions {
        if *id != here {
            assert!(!other.maps.contains_key(&world.position.map));
        }
    }
}

#[test]
fn a_map_visited_and_unchanged_keeps_its_region_base() {
    let data = data();
    let mut world = world(&data);
    let map = world.position.map;
    world.map_state(map);
    let parts = split(&world, &data);
    let region = data.maps[&map].region;
    assert_eq!(
        parts.regions[&region].maps.get(&map),
        Some(&MapState::default()),
        "the empty entry is kept: it is in the fingerprint"
    );
    round_trip(&world, &data, "an empty map state");
}

#[test]
fn what_belongs_to_no_region_or_party_goes_to_the_world_base() {
    let data = data();
    let mut world = world(&data);
    let actor = HolderId::Actor(ActorId(7));
    world.clocks.insert(actor, Clock::new(EraId(0)));
    let contact = Contact {
        other: PARTY,
        self_elapsed: 3,
        other_elapsed: 4,
        self_shared: 0,
    };
    world.contacts.insert((actor, PARTY), contact);
    let stream = StreamName::time(PARTY, actor);
    world
        .rngs
        .insert(stream.clone(), Pcg32::for_stream(1, &stream));
    let stray = MapId(9_999);
    world.maps.insert(stray, MapState::default());
    let parts = split(&world, &data);
    let base = &parts.world;
    assert!(base.holdings.clocks.contains_key(&actor));
    assert!(base.holdings.contacts.contains_key(&(actor, PARTY)));
    assert!(base.holdings.streams.contains_key(&stream));
    assert!(base.strays.contains_key(&stray));
    assert!(parts.regions.values().all(|r| !r.maps.contains_key(&stray)));
    round_trip(&world, &data, "the catch-alls");
}

#[test]
fn join_refuses_a_part_held_twice_a_bad_place_and_other_packs() {
    let data = data();
    let mut world = world(&data);
    let map = world.position.map;
    world.map_state(map);
    let mut parts = split(&world, &data);
    parts.world.strays.insert(map, MapState::default());
    assert!(
        matches!(join(parts, &data, false), Err(JoinError::Duplicate(_))),
        "the same map twice"
    );
    let mut parts = split(&world, &data);
    parts.checkpoint.position.map = MapId(9_999);
    assert!(matches!(
        join(parts, &data, false),
        Err(JoinError::Load(LoadError::BadPosition(_)))
    ));
    let mut parts = split(&world, &data);
    parts.manifest.packs.clear();
    assert!(matches!(
        join(parts.clone(), &data, false),
        Err(JoinError::Load(LoadError::PackMismatch))
    ));
    assert!(join(parts, &data, true).is_ok(), "force passes the packs");
}

#[test]
fn the_report_sizes_each_part_and_checks_the_round_trip() {
    let data = data();
    let mut world = world(&data);
    world.map_state(world.position.map);
    let report = split_report(&world, &data).unwrap();
    assert!(report.round_trip);
    assert!(report.checkpoint_bytes > 0);
    assert_eq!(report.regions.len(), 1, "{:?}", report.regions);
    assert!(
        report.regions[0].id.contains(":region:"),
        "{:?}",
        report.regions
    );
    assert_eq!(report.regions[0].maps, 1);
    assert!(report.regions[0].bytes > 0);
    assert!(matches!(world.mode, Mode::Explore));
}

#[test]
fn the_report_says_so_when_the_parts_do_not_join() {
    let data = data();
    let mut world = world(&data);
    world.position.map = MapId(9_999);
    let report = split_report(&world, &data).unwrap();
    assert!(!report.round_trip, "a party nowhere does not join back");
}

#[test]
fn a_party_flag_travels_in_the_checkpoint_and_a_world_flag_stays() {
    let data = crate::objects::objects();
    let mut world = crate::objects::in_yard(&data, 0, 0, 1, omnis_core::Facing::East);
    let flag = |name: &str| data.registry.flags.get(name).unwrap();
    let (debt, lever) = (flag("objects:flag:debt"), flag("objects:flag:lever"));
    world.flags.insert(debt, 3);
    world.flags.insert(lever, 1);
    let undeclared = omnis_core::FlagId(4_000);
    world.flags.insert(undeclared, 7);
    let parts = split(&world, &data);
    assert_eq!(parts.checkpoint.flags.get(&debt), Some(&3), "the party's");
    assert_eq!(parts.world.flags.get(&lever), Some(&1), "the world's");
    assert_eq!(
        parts.world.flags.get(&undeclared),
        Some(&7),
        "undeclared: the world's"
    );
    assert!(!parts.world.flags.contains_key(&debt));
    assert!(!parts.checkpoint.flags.contains_key(&lever));
    round_trip(&world, &data, "flags of both scopes");
    let report = split_report(&world, &data).unwrap();
    assert_eq!((report.party_flags, report.world_flags), (1, 2));
    let mut twice = split(&world, &data);
    twice.world.flags.insert(debt, 3);
    assert!(
        matches!(join(twice, &data, false), Err(JoinError::Duplicate(_))),
        "a flag held by both"
    );
}
