//! The party's log (M8.5 7f): each event stamped with the party's clock and map when it was said,
//! the newest `LOG_CAPACITY` kept, carried by the character save and left be by a fold, so a
//! client reads its full log back after a load.

use crate::common::data;
use omnis_data::Data;
use omnis_data::ron_io::read_ron;
use omnis_sim::ops::{Op, Reply, dispatch};
use omnis_sim::save::{load_save, save_files};
use omnis_sim::{Event, LOG_CAPACITY, Logged, Replay, World, apply, history, told};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const SAVE: &str = ".omnis/one.ron";

fn golden(name: &str) -> Replay {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/replays")
        .join(format!("{name}.ron"));
    read_ron(&path, &path).unwrap_or_else(|e| panic!("{e}"))
}

/// The golden walk, played.
fn walked(data: &Data) -> World {
    let replay = golden("walk");
    let mut world = World::new(data, replay.seed, replay.settings).unwrap();
    for command in replay.commands {
        apply(&mut world, data, command).unwrap_or_else(|r| panic!("{r}"));
    }
    world
}

fn save_and_load(world: &World, data: &Data) -> World {
    let disk: BTreeMap<String, String> = save_files(world, data, SAVE)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|f| (f.path, f.text))
        .collect();
    load_save(&disk[SAVE], data, false, &mut |p| {
        disk.get(p).cloned().ok_or_else(|| format!("no file {p}"))
    })
    .unwrap_or_else(|e| panic!("{e}"))
    .0
}

/// `count` more lines of news, said now.
fn chatter(world: &mut World, count: usize) {
    let news: Vec<Event> = (0..count).map(|_| Event::PartyChanged).collect();
    world.record(&news);
}

#[test]
fn each_event_is_stamped_with_the_clock_and_the_map_it_was_said_on() {
    let data = data();
    let replay = golden("walk");
    let mut world = World::new(&data, replay.seed, replay.settings).unwrap();
    for command in replay.commands {
        let said = world.log.len();
        let events = apply(&mut world, &data, command).unwrap_or_else(|r| panic!("{r}"));
        let new = &history(&world)[said..];
        assert_eq!(
            new.iter().map(|l| &l.event).collect::<Vec<_>>(),
            events.iter().collect::<Vec<_>>()
        );
        for logged in new {
            assert_eq!(logged.minute, world.party_clock().elapsed);
            assert_eq!(logged.map, world.position.map);
        }
    }
    let log = history(&world);
    let minutes: BTreeSet<i64> = log.iter().map(|l| l.minute).collect();
    let maps: BTreeSet<_> = log.iter().map(|l| l.map).collect();
    assert!(minutes.len() > 10, "the clock moves on: {minutes:?}");
    assert!(maps.len() >= 2, "the walk leaves the town: {maps:?}");
}

#[test]
fn the_log_keeps_the_newest_capacity() {
    let data = data();
    let mut world = walked(&data);
    chatter(&mut world, LOG_CAPACITY);
    assert_eq!(world.log.len(), LOG_CAPACITY);
    let last = world.log.last().cloned();
    world.record(&[Event::Folded { interactions: 7 }]);
    assert_eq!(world.log.len(), LOG_CAPACITY, "one more drops the oldest");
    assert_eq!(world.log[LOG_CAPACITY - 2], last.unwrap());
    let Ok(Reply::Events { events }) = dispatch(
        &mut world,
        &data,
        &Op::EventsTail {
            count: LOG_CAPACITY + 10,
        },
    ) else {
        panic!("events.tail answers with events");
    };
    assert_eq!(events.len(), LOG_CAPACITY);
    assert_eq!(events.last(), Some(&Event::Folded { interactions: 7 }));
}

#[test]
fn a_save_carries_the_log_and_a_load_gives_it_back() {
    let data = data();
    let world = walked(&data);
    assert!(world.log.len() > 50);
    let back = save_and_load(&world, &data);
    let told: Vec<&Logged> = history(&world).iter().filter(|l| told(&l.event)).collect();
    assert!(told.len() < world.log.len(), "the walk saw and rolled");
    assert_eq!(
        history(&back).iter().collect::<Vec<_>>(),
        told,
        "events and stamps alike, but for the view and the dice of each step"
    );
    assert!(history(&back).iter().all(|l| !matches!(
        l.event,
        Event::Visible { .. } | Event::EncounterCheck { .. } | Event::TimeAdvanced { .. }
    )));
}

#[test]
fn a_save_holding_more_than_the_capacity_loads_its_newest() {
    let data = data();
    let mut world = walked(&data);
    chatter(&mut world, LOG_CAPACITY);
    let extra = Logged {
        minute: -1,
        map: world.position.map,
        event: Event::PartyChanged,
    };
    let mut over = world.clone();
    over.log.splice(0..0, vec![extra.clone(); 5]);
    let back = save_and_load(&over, &data);
    assert_eq!(back.log, world.log, "the five oldest are dropped");
}

#[test]
fn a_save_without_a_log_loads_with_none() {
    let data = data();
    let mut world = walked(&data);
    world.log.clear();
    let text = save_files(&world, &data, SAVE).unwrap().pop().unwrap().text;
    assert!(!text.contains("log:"), "an empty log is left out: {text}");
    assert!(save_and_load(&world, &data).log.is_empty());
}

/// The golden fight, played.
fn fought(data: &Data) -> World {
    let replay = golden("fight");
    let mut world = World::new(data, replay.seed, replay.settings).unwrap();
    for command in replay.commands {
        apply(&mut world, data, command).unwrap_or_else(|r| panic!("{r}"));
    }
    world
}

#[test]
fn a_full_log_and_a_full_journal_fit_in_a_save() {
    let data = data();
    let walk = walked(&data);
    let mut world = fought(&data);
    // The fight's events are the wordiest; the walk's interactions are typical ones.
    let said: Vec<Logged> = world
        .log
        .iter()
        .filter(|l| told(&l.event))
        .cloned()
        .collect();
    let entries: Vec<_> = walk.journal.entries().to_vec();
    assert!(!said.is_empty() && !entries.is_empty());
    world.log = said.iter().cycle().take(LOG_CAPACITY).cloned().collect();
    let full: Vec<_> = entries
        .iter()
        .cycle()
        .take(omnis_sim::save::FOLD_INTERACTIONS)
        .cloned()
        .collect();
    world.journal = full.into();
    let log_bytes = omnis_data::ron_io::to_compact(&world.log).unwrap().len();
    let journal_bytes = omnis_data::ron_io::to_compact(&world.journal)
        .unwrap()
        .len();
    let mut files = save_files(&world, &data, SAVE).unwrap();
    let save = files.pop().unwrap().text.len();
    println!(
        "log {log_bytes} B ({} B an event), journal {journal_bytes} B ({} B an interaction), save {save} B",
        log_bytes / LOG_CAPACITY,
        journal_bytes / omnis_sim::save::FOLD_INTERACTIONS
    );
    assert!(
        (save as u64) < omnis_data::limits::MAX_SAVE_BYTES * 3 / 5,
        "{save} bytes leaves less than two fifths of the save's room spare"
    );
}

#[test]
fn a_save_over_its_room_is_not_written() {
    let data = data();
    let mut world = walked(&data);
    let long = Event::Reconciled {
        a: "x".repeat(4096),
        b: "y".into(),
        delta_a: 0,
        delta_b: 0,
        era_b: omnis_core::EraId(0),
    };
    let size = omnis_data::ron_io::to_compact(&long).unwrap().len();
    let lines = (omnis_data::limits::MAX_SAVE_BYTES as usize / size + 1).min(LOG_CAPACITY);
    world.record(&vec![long; lines]);
    let refused = save_files(&world, &data, SAVE).map(|_| ()).unwrap_err();
    assert!(refused.to_string().contains("a save may hold"), "{refused}");
}
