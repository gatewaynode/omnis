//! Subjective time (M8, PRD §7.8): the party's age never reverses; each minute counts toward its
//! shared time at the company of where it was lived; a settlement catches up by that shared time
//! and sets the party's date; a wild region catches up by the lived time; coupled regions follow;
//! the jitter comes from the pair's own stream; routes in different orders replay.

mod common;

use common::data;
use omnis_core::{Direction, HolderId, StreamName};
use omnis_data::Data;
use omnis_sim::{Command, DevCommand, Event, PARTY, Replay, Settings, World, apply};

const YEAR: i64 = 360 * 1440;

fn dev_world(data: &Data, seed: u64) -> World {
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    World::new(data, seed, settings).expect("entry map")
}

/// A dev teleport to `map`'s start.
fn to(data: &Data, map: &str) -> Command {
    let id = data.registry.maps.get(map).expect(map);
    let (x, y, facing) = data.maps[&id].def.start;
    Command::Dev(DevCommand::Teleport {
        map: map.into(),
        x,
        y,
        facing,
    })
}

fn region(data: &Data, name: &str) -> HolderId {
    HolderId::Region(data.registry.regions.get(name).expect(name))
}

/// Every reconciliation in `events`: `(a, b, delta_a, delta_b)`.
/// The holder an event names: `party:0`, or a region's string id.
fn holder(data: &Data, name: &str) -> HolderId {
    if name == "party:0" {
        PARTY
    } else {
        region(data, name)
    }
}

fn reconciled(data: &Data, events: &[Event]) -> Vec<(HolderId, HolderId, i64, i64)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Reconciled {
                a,
                b,
                delta_a,
                delta_b,
                ..
            } => Some((holder(data, a), holder(data, b), *delta_a, *delta_b)),
            _ => None,
        })
        .collect()
}

fn run(world: &mut World, data: &Data, commands: &[Command]) -> Vec<Event> {
    commands
        .iter()
        .flat_map(|c| apply(world, data, c.clone()).unwrap_or_else(|r| panic!("{c:?}: {r:?}")))
        .collect()
}

/// Steps out and back on the meadow: lived time in the wilds.
fn pace(steps: usize) -> Vec<Command> {
    (0..steps)
        .map(|i| {
            Command::Step(if i % 2 == 0 {
                Direction::Forward
            } else {
                Direction::Back
            })
        })
        .collect()
}

#[test]
fn years_in_the_wilds_are_months_in_the_town_and_the_date_snaps_back() {
    let data = data();
    let (town, crossroads) = (
        region(&data, "test:region:town"),
        region(&data, "test:region:crossroads"),
    );
    for seed in 1..=20 {
        let mut world = dev_world(&data, seed);
        run(&mut world, &data, &[to(&data, "test:map:meadow")]);
        // Two years in the meadow (company 100): what steps would do, at scale.
        world.clocks.get_mut(&PARTY).unwrap().elapsed += 2 * YEAR;
        world.party_time.shared_milli += 2 * YEAR * 100;
        world.party_time.date += 2 * YEAR;
        let events = run(&mut world, &data, &[to(&data, "test:map:town")]);
        let r = reconciled(&data, &events);
        let (_, b, lived, delta) = r[0];
        assert_eq!((b, lived), (town, 2 * YEAR), "seed {seed}: {r:?}");
        let shared = 2 * YEAR / 10;
        assert!(
            (delta - shared).abs() <= shared * 20 / 1000,
            "seed {seed}: about 2.4 months, within the town's 2%: {delta} of {shared}"
        );
        let town_clock = world.clocks[&town].elapsed;
        assert_eq!(town_clock, delta);
        assert_eq!(
            world.party_time.date, town_clock,
            "the party's date is the town's"
        );
        assert_eq!(
            world.party_clock().elapsed,
            2 * YEAR,
            "the party's age does not go back"
        );
        let (a, b, lived_c, delta_c) = r[1];
        assert_eq!((a, b, lived_c), (town, crossroads, delta), "the road");
        assert!(
            (delta_c - delta).abs() <= delta * 50 / 1000,
            "the crossroads follows within its 5%"
        );
        assert_eq!(
            r.len(),
            2,
            "coupling depth one: the crossroads passes nothing on"
        );
    }
}

#[test]
fn shared_time_is_lived_time_at_the_company_of_where_it_was_lived() {
    let data = data();
    let mut world = dev_world(&data, 1);
    run(&mut world, &data, &[Command::Step(Direction::Forward)]);
    let town_minutes = world.party_clock().elapsed;
    assert_eq!(world.party_time.shared_milli, town_minutes * 900);
    run(&mut world, &data, &[to(&data, "test:map:meadow")]);
    run(&mut world, &data, &pace(6));
    let meadow_minutes = world.party_clock().elapsed - town_minutes;
    assert!(meadow_minutes > 0);
    assert_eq!(
        world.party_time.shared_milli,
        town_minutes * 900 + meadow_minutes * 100
    );
    assert_eq!(
        world.party_time.date,
        world.party_clock().elapsed,
        "in the wilds the party's reckoning runs on"
    );
}

#[test]
fn a_wild_region_catches_up_by_lived_time_and_never_sets_the_date() {
    let data = data();
    let mut world = dev_world(&data, 3);
    run(&mut world, &data, &pace(4));
    let age = world.party_clock().elapsed;
    let date = world.party_time.date;
    let events = run(&mut world, &data, &[to(&data, "test:map:dungeon")]);
    let (a, b, lived, delta) = reconciled(&data, &events)[0];
    assert_eq!(
        (a, b, lived),
        (PARTY, region(&data, "test:region:dungeon"), age),
        "never met: counted from the origin"
    );
    assert!(
        (delta - age).abs() <= age * 400 / 1000,
        "within 40%: {delta}"
    );
    assert_eq!(world.party_time.date, date, "a wild region sets no date");
    let again = run(&mut world, &data, &[to(&data, "test:map:depths")]);
    assert!(
        reconciled(&data, &again).is_empty(),
        "the depths are the dungeon's region"
    );
}

#[test]
fn the_jitter_is_drawn_from_the_pair_and_no_other_stream_moves() {
    let data = data();
    let mut world = dev_world(&data, 4);
    run(&mut world, &data, &pace(2));
    let before = world.rngs.clone();
    run(&mut world, &data, &[to(&data, "test:map:meadow")]);
    let pair = StreamName::time(PARTY, region(&data, "test:region:meadow"));
    let changed: Vec<&StreamName> = world
        .rngs
        .iter()
        .filter(|(name, rng)| before.get(*name) != Some(*rng))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(changed, [&pair]);
    let id = data.registry.regions.get("test:region:meadow").unwrap().0;
    assert_eq!(pair.0, format!("time:party:0:region:{id}"));
}

#[test]
fn routes_in_different_orders_give_different_deltas_and_each_replays() {
    let data = data();
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    let (meadow, dungeon, town) = (
        to(&data, "test:map:meadow"),
        to(&data, "test:map:dungeon"),
        to(&data, "test:map:town"),
    );
    let mut a = vec![meadow.clone()];
    a.extend(pace(8));
    a.extend([dungeon.clone(), town.clone()]);
    let mut b = vec![dungeon, meadow];
    b.extend(pace(8));
    b.push(town);
    let deltas = |commands: &[Command]| {
        let mut world = World::new(&data, 9, settings).unwrap();
        reconciled(&data, &run(&mut world, &data, commands))
    };
    let (da, db) = (deltas(&a), deltas(&b));
    assert_ne!(
        da, db,
        "the dungeon is met after the meadow's time, or before it"
    );
    assert_eq!(da, deltas(&a), "the same route, the same deltas");
    for commands in [a, b] {
        let replay = Replay::record(&data, 9, settings, commands).unwrap();
        replay.check(&data).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn a_schema_6_save_counts_its_past_in_full() {
    let data = data();
    let mut world = dev_world(&data, 5);
    run(&mut world, &data, &[to(&data, "test:map:meadow")]);
    run(&mut world, &data, &pace(4));
    let age = world.party_clock().elapsed;
    let mut old = world.clone();
    old.contacts.clear();
    old.party_time = Default::default();
    old.bus = Default::default();
    let text = old.to_ron().unwrap().replacen("schema: 7", "schema: 6", 1);
    let loaded = World::from_ron(&text, &data, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(loaded.schema, 7);
    assert_eq!(
        (loaded.party_time.shared_milli, loaded.party_time.date),
        (age * 1000, age)
    );
    assert_eq!(loaded.bus, omnis_sim::time::subscriptions(&data));
    assert!(loaded.contacts.is_empty());
}

#[test]
fn a_second_visit_counts_only_the_time_since_the_last() {
    let data = data();
    let town = region(&data, "test:region:town");
    let mut world = dev_world(&data, 6);
    run(&mut world, &data, &[to(&data, "test:map:meadow")]);
    run(&mut world, &data, &pace(4));
    let first = reconciled(
        &data,
        &run(&mut world, &data, &[to(&data, "test:map:town")]),
    );
    let at_first = world.party_clock().elapsed;
    assert_eq!((first[0].1, first[0].2), (town, at_first));
    run(&mut world, &data, &[to(&data, "test:map:meadow")]);
    run(&mut world, &data, &pace(6));
    let second = reconciled(
        &data,
        &run(&mut world, &data, &[to(&data, "test:map:town")]),
    );
    assert_eq!(
        (second[0].1, second[0].2),
        (town, world.party_clock().elapsed - at_first),
        "both sides remember the first visit"
    );
    assert_eq!(
        world.contacts[&(PARTY, town)].self_elapsed,
        world.party_clock().elapsed
    );
    assert_eq!(
        world.contacts[&(town, PARTY)].other_elapsed,
        world.party_clock().elapsed
    );
}

#[test]
fn one_reconciliation_catches_up_at_most_the_cap() {
    let data = data();
    let cap = data.rules.value("time_cap_minutes").unwrap();
    let mut at_cap = 0;
    for seed in 1..=20 {
        let mut world = dev_world(&data, seed);
        run(&mut world, &data, &[to(&data, "test:map:meadow")]);
        world.clocks.get_mut(&PARTY).unwrap().elapsed += 20 * YEAR;
        let (_, _, lived, delta) = reconciled(
            &data,
            &run(&mut world, &data, &[to(&data, "test:map:dungeon")]),
        )[0];
        assert_eq!(lived, 20 * YEAR);
        assert!(
            (cap * 6 / 10..=cap).contains(&delta),
            "seed {seed}: ten years at most: {delta}"
        );
        at_cap += usize::from(delta == cap);
    }
    assert!(at_cap > 0, "some seeds drift up into the cap");
}

#[test]
fn the_day_rolls_on_the_date_not_the_age() {
    let data = data();
    let rolled = |age: i64, date: i64| {
        let mut world = dev_world(&data, 8);
        world.clocks.get_mut(&PARTY).unwrap().elapsed = age;
        world.party_time.date = date;
        run(&mut world, &data, &[Command::Step(Direction::Forward)])
            .iter()
            .any(|e| {
                matches!(
                    e,
                    Event::TimeAdvanced {
                        day_rolled: true,
                        ..
                    }
                )
            })
    };
    assert!(rolled(5, 1439));
    assert!(!rolled(1439, 5));
    assert!(
        !rolled(1439, 3 * 1440 + 5),
        "a date days ahead of the age rolls nothing"
    );
}

#[test]
fn a_reconciliation_names_the_party_and_each_region_by_id() {
    let data = data();
    let mut world = dev_world(&data, 2);
    run(&mut world, &data, &[to(&data, "test:map:meadow")]);
    let events = run(&mut world, &data, &[to(&data, "test:map:town")]);
    let named: Vec<(&str, &str)> = events
        .iter()
        .filter_map(|e| match e {
            Event::Reconciled { a, b, .. } => Some((a.as_str(), b.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        named,
        [
            ("party:0", "test:region:town"),
            ("test:region:town", "test:region:crossroads"),
        ],
        "the party meets the town, and the town its coupling"
    );
    let step = run(&mut world, &data, &pace(1));
    assert!(
        step.iter()
            .any(|e| matches!(e, Event::TimeAdvanced { holder, .. } if holder == "party:0")),
        "the party's clock is named as the party: {step:?}"
    );
}
