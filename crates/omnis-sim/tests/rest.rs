//! Resting outside a service (M7 step 5): the short rest's hit dice, the long rest's food and
//! restoration, the once-a-day rule shared with the inn, the ambush, the map's rest events,
//! and the refusals, each leaving the world as it was.

mod common;

use common::{data, encounter, party_of, word as parse_word};
use omnis_core::{CharacterId, Facing, MapId, Position};
use omnis_data::{Ability, Data, Disposition, RestKind};
use omnis_rules::modifier;
use omnis_sim::rest::HitDiceSpend;
use omnis_sim::{
    Command, EncounterSource, Event, Mode, Rejection, RestCommand, ServiceCommand, Settings, World,
    apply,
};

const CON: usize = Ability::Constitution.index();

fn map(data: &Data, name: &str) -> MapId {
    data.registry.maps.get(&format!("test:map:{name}")).unwrap()
}

/// A new game with `members` of the six, placed on a map in the explore mode.
fn at(data: &Data, seed: u64, members: usize, name: &str, x: u16, y: u16) -> World {
    let mut world = World::new(data, seed, Settings::default()).unwrap();
    party_of(&mut world, data, members);
    assert!(
        world
            .party
            .ids()
            .iter()
            .enumerate()
            .all(|(slot, id)| id == &slot_id(slot)),
        "a new party's ids follow its slots, as `short` names them"
    );
    world.position = Position {
        map: map(data, name),
        x,
        y,
        facing: Facing::North,
    };
    world
}

fn rest(world: &mut World, data: &Data, command: RestCommand) -> Vec<Event> {
    apply(world, data, Command::Rest(command.clone()))
        .unwrap_or_else(|r| panic!("{command:?}: {r}"))
}

/// The identity of the member in `slot` of a party made by [`at`] (checked there).
fn slot_id(slot: usize) -> CharacterId {
    CharacterId(u32::try_from(slot).unwrap())
}

/// A short rest spending `dice[slot]` of each member's hit dice, as `short-rest-A-B-…` reads.
fn short(dice: &[u8]) -> RestCommand {
    RestCommand::Short {
        dice: dice
            .iter()
            .enumerate()
            .map(|(slot, &count)| HitDiceSpend {
                member: slot_id(slot),
                count,
            })
            .collect(),
    }
}

/// The command is refused with `why`, and nothing changed, streams included.
fn refused(world: &mut World, data: &Data, command: RestCommand, why: Rejection) {
    let before = world.clone();
    assert_eq!(
        apply(world, data, Command::Rest(command.clone())),
        Err(why),
        "{command:?}"
    );
    assert_eq!(*world, before, "{command:?} changed the world");
}

fn minutes(world: &World) -> i64 {
    world.party_clock().elapsed
}

fn kill(world: &mut World, data: &Data, index: usize) {
    let dead = omnis_rules::condition_id(data, "dead").unwrap();
    let member = &mut world.party.members[index];
    member.hp = 0;
    member.conditions.push(dead);
}

#[test]
fn a_short_rest_spends_hit_dice_and_heals_by_the_die_and_constitution() {
    let data = data();
    let mut world = at(&data, 5, 2, "meadow", 16, 16);
    world.party.members[0].hp = 1;
    let con = modifier(world.party.members[0].scores[CON]);
    let start = minutes(&world);
    let events = rest(&mut world, &data, short(&[1]));
    assert_eq!(minutes(&world), start + 60);
    let brenna = world.party.members[0].id;
    let rested = events
        .iter()
        .position(|e| {
            *e == Event::Rested {
                long: false,
                minutes: 60,
                food: 0,
            }
        })
        .expect("rested");
    assert_eq!(
        events[rested + 1],
        Event::HitDiceSpent {
            member: brenna,
            dice: 1
        }
    );
    let Event::Healed { rolls, amount, .. } = &events[rested + 2] else {
        panic!("{:?}", events[rested + 2]);
    };
    assert_eq!(rolls.len(), 1);
    assert_eq!(rolls[0].dice.sides, 10, "a fighter's d10");
    assert_eq!(*amount, (i64::from(rolls[0].total) + con).max(0));
    assert_eq!(world.party.members[0].hit_dice_spent, 1);
    assert_eq!(
        world.party.members[0].hp,
        (1 + i32::try_from(*amount).unwrap()).min(world.party.members[0].hp_max)
    );
}

#[test]
fn a_stable_member_at_zero_wakes_on_a_short_rest() {
    let data = data();
    let mut world = at(&data, 5, 2, "meadow", 16, 16);
    let unconscious = omnis_rules::condition_id(&data, "unconscious").unwrap();
    world.party.members[0].hp = 0;
    world.party.members[0].conditions.push(unconscious);
    assert!(modifier(world.party.members[0].scores[CON]) >= 0);
    rest(&mut world, &data, short(&[1]));
    let brenna = &world.party.members[0];
    assert!(brenna.hp >= 1, "a d10 plus a Constitution of at least 0");
    assert!(!brenna.conditions.contains(&unconscious), "awake");
}

#[test]
fn short_rest_refusals_change_nothing() {
    let data = data();
    let mut world = at(&data, 5, 2, "meadow", 16, 16);
    world.party.members[0].hp = 1;
    refused(
        &mut world,
        &data,
        short(&[0, 0, 1]),
        Rejection::NoSuchMember { member: slot_id(2) },
    );
    refused(
        &mut world,
        &data,
        short(&[0, 1]),
        Rejection::NothingToTreat { member: slot_id(1) },
    );
    refused(
        &mut world,
        &data,
        short(&[2]),
        Rejection::NoHitDice {
            member: slot_id(0),
            left: 1,
        },
    );
    kill(&mut world, &data, 1);
    refused(
        &mut world,
        &data,
        short(&[0, 1]),
        Rejection::MemberDead { member: slot_id(1) },
    );
    rest(&mut world, &data, short(&[1]));
    world.party.members[0].hp = 1;
    refused(
        &mut world,
        &data,
        short(&[1]),
        Rejection::NoHitDice {
            member: slot_id(0),
            left: 0,
        },
    );
    let start = minutes(&world);
    rest(&mut world, &data, short(&[]));
    assert_eq!(
        minutes(&world),
        start + 60,
        "an hour with no dice still passes"
    );
}

#[test]
fn hit_dice_roll_in_marching_order_whatever_the_list_order() {
    let data = data();
    let hurt = || {
        let mut world = at(&data, 5, 3, "meadow", 16, 16);
        for member in &mut world.party.members {
            member.hp = 1;
        }
        world
    };
    let spend = |slots: &[usize]| RestCommand::Short {
        dice: slots
            .iter()
            .map(|&slot| HitDiceSpend {
                member: slot_id(slot),
                count: 1,
            })
            .collect(),
    };
    let (mut forward, mut backward) = (hurt(), hurt());
    rest(&mut forward, &data, spend(&[0, 1, 2]));
    rest(&mut backward, &data, spend(&[2, 0, 1]));
    assert_eq!(
        forward, backward,
        "the same spends in another order roll the same dice for the same members"
    );
    let hp: Vec<i32> = forward.party.members.iter().map(|m| m.hp).collect();
    assert!(hp.iter().all(|&hp| hp > 1), "each member healed: {hp:?}");
}

#[test]
fn a_member_named_twice_is_refused_whatever_they_spend() {
    let data = data();
    let mut world = at(&data, 5, 2, "meadow", 16, 16);
    world.party.members[0].hp = 1;
    for counts in [[1, 1], [0, 1], [1, 0], [0, 0]] {
        let twice = RestCommand::Short {
            dice: counts
                .iter()
                .map(|&count| HitDiceSpend {
                    member: slot_id(0),
                    count,
                })
                .collect(),
        };
        refused(
            &mut world,
            &data,
            twice,
            Rejection::MemberTwice { member: slot_id(0) },
        );
    }
}

#[test]
fn a_long_rest_eats_for_the_living_and_restores_once_a_day() {
    let data = data();
    let mut world = at(&data, 5, 3, "meadow", 16, 16);
    world.party.food = 5;
    kill(&mut world, &data, 2);
    for member in &mut world.party.members[..2] {
        member.hp = 1;
        member.spell_points = 0;
        member.hit_dice_spent = 1;
    }
    let start = minutes(&world);
    let events = rest(&mut world, &data, RestCommand::Long);
    assert!(events.contains(&Event::Rested {
        long: true,
        minutes: 480,
        food: 2
    }));
    assert_eq!(world.party.food, 3, "the dead do not eat");
    assert_eq!(minutes(&world), start + 480);
    assert_eq!(world.party.last_long_rest, Some(start + 480));
    for member in &world.party.members[..2] {
        assert_eq!(member.hp, member.hp_max);
        assert_eq!(member.spell_points, member.spell_points_max);
        assert_eq!(member.hit_dice_spent, 0);
    }
    assert_eq!(world.party.members[2].hp, 0, "the dead stay dead");

    refused(
        &mut world,
        &data,
        RestCommand::Long,
        Rejection::RestTooSoon { minutes: 960 },
    );
    world.party.last_long_rest = None;
    world.party.food = 1;
    refused(
        &mut world,
        &data,
        RestCommand::Long,
        Rejection::NoFood { need: 2, have: 1 },
    );
}

#[test]
fn a_rest_in_the_field_counts_against_the_inn_and_the_inn_against_the_field() {
    let data = data();
    // The inn's tile, outside it: the party rests on the street, then goes in.
    let mut world = at(&data, 5, 2, "town", 1, 1);
    world.party.food = 10;
    world.party.gold = 5000;
    rest(&mut world, &data, RestCommand::Long);
    apply(&mut world, &data, Command::Interact).unwrap();
    assert!(matches!(world.mode, Mode::Town(_)));
    assert_eq!(
        apply(&mut world, &data, Command::Service(ServiceCommand::Room)),
        Err(Rejection::RestTooSoon { minutes: 960 }),
        "the night on the street counts at the inn"
    );
    assert_eq!(
        apply(&mut world, &data, Command::Rest(RestCommand::Long)),
        Err(Rejection::WrongMode),
        "no rest inside a service"
    );
    apply(&mut world, &data, Command::Service(ServiceCommand::Leave)).unwrap();
    world.party.last_long_rest = None;
    apply(&mut world, &data, Command::Interact).unwrap();
    apply(&mut world, &data, Command::Service(ServiceCommand::Room)).unwrap();
    apply(&mut world, &data, Command::Service(ServiceCommand::Leave)).unwrap();
    assert!(matches!(
        apply(&mut world, &data, Command::Rest(RestCommand::Long)),
        Err(Rejection::RestTooSoon { .. })
    ));
}

/// The first seed from 1 under which the rest is ambushed in the dungeon, and its events.
fn ambushed(data: &Data, command: &RestCommand, limit: u64) -> (World, World, Vec<Event>) {
    for seed in 1..limit {
        let mut world = at(data, seed, 2, "dungeon", 1, 0);
        world.party.food = 10;
        for member in &mut world.party.members {
            member.hp = 1;
        }
        let before = world.clone();
        let events = rest(&mut world, data, command.clone());
        if events
            .iter()
            .any(|e| matches!(e, Event::RestInterrupted { .. }))
        {
            return (before, world, events);
        }
    }
    panic!("no seed under {limit} ambushes {command:?}");
}

#[test]
fn an_ambush_comes_after_part_of_the_rest_and_restores_nothing() {
    let data = data();
    for (command, step, most) in [(RestCommand::Long, 60, 480), (short(&[1]), 10, 60)] {
        let (before, world, events) = ambushed(&data, &command, 5000);
        let Some(Event::RestInterrupted { minutes: after }) = events
            .iter()
            .find(|e| matches!(e, Event::RestInterrupted { .. }))
        else {
            unreachable!()
        };
        assert!(
            after % step == 0 && (step..=most).contains(after),
            "{command:?}: {after}"
        );
        assert_eq!(minutes(&world), minutes(&before) + i64::from(*after));
        assert_eq!(world.party.food, before.party.food, "no food eaten");
        assert_eq!(world.party.members, {
            let mut members = before.party.members.clone();
            for (m, now) in members.iter_mut().zip(&world.party.members) {
                m.effects.clone_from(&now.effects);
            }
            members
        });
        assert_eq!(world.party.last_long_rest, None);
        assert!(!events.iter().any(|e| matches!(e, Event::Rested { .. })));
        let Mode::Encounter(state) = &world.mode else {
            panic!(
                "{command:?}: surprise is off, so the choice waits: {:?}",
                world.mode
            );
        };
        assert_eq!(state.source, EncounterSource::Ambush);
        assert_eq!(
            state.retreat,
            Position {
                facing: Facing::South,
                ..before.position
            },
            "Run turns the party away where it lay"
        );
    }
}

#[test]
fn no_ambush_where_the_map_has_no_table_or_a_chance_of_zero() {
    let data = data();
    for (name, x, y) in [("town", 5, 2), ("meadow", 16, 16)] {
        for seed in 1..200 {
            let mut world = at(&data, seed, 1, name, x, y);
            world.party.food = 1;
            let events = rest(&mut world, &data, RestCommand::Long);
            assert!(
                events.contains(&Event::Rested {
                    long: true,
                    minutes: 480,
                    food: 1
                }),
                "{name} under seed {seed}"
            );
        }
    }
}

#[test]
fn rest_events_roll_for_their_terrain_and_rest_and_change_nothing() {
    let mut data = data();
    let meadow = map(&data, "meadow");
    let placeholders = data.maps[&meadow].rest_events.clone();
    assert_eq!(placeholders.len(), 2, "road and grass");
    assert!(placeholders.iter().all(|e| e.chance == 0));

    // At chance 0 nothing is reported, but the roll is made: the stream moves.
    let mut with = at(&data, 5, 1, "meadow", 16, 16);
    with.party.food = 2;
    let mut without = with.clone();
    let events = rest(&mut with, &data, RestCommand::Long);
    assert!(!events.iter().any(|e| matches!(e, Event::RestEvent { .. })));
    let mut bare = data.clone();
    bare.maps.get_mut(&meadow).unwrap().rest_events.clear();
    rest(&mut without, &bare, RestCommand::Long);
    let stream = omnis_core::StreamName::new("encounter");
    assert_ne!(
        with.rngs[&stream], without.rngs[&stream],
        "a roll at chance 0"
    );

    // A sure event for the long rest on the road.
    let entry = &mut data.maps.get_mut(&meadow).unwrap().rest_events[0];
    (entry.chance, entry.rest) = (1000, RestKind::Long);
    let mut road = at(&data, 5, 1, "meadow", 16, 16);
    road.party.food = 2;
    let fresh = road.clone();
    let events = rest(&mut road, &data, RestCommand::Long);
    let at_rest = events
        .iter()
        .filter(|e| matches!(e, Event::RestEvent { .. }))
        .collect::<Vec<_>>();
    assert_eq!(
        at_rest,
        [&Event::RestEvent {
            map: "test:map:meadow".to_owned(),
            entry: 0
        }]
    );
    let mut quiet = fresh.clone();
    quiet.party.members[0].hp = 1;
    let events = rest(&mut quiet, &data, short(&[1]));
    assert!(
        !events.iter().any(|e| matches!(e, Event::RestEvent { .. })),
        "the entry is for the long rest"
    );
    let mut grass = fresh;
    grass.position.x = 15;
    let events = rest(&mut grass, &data, RestCommand::Long);
    assert!(
        !events.iter().any(|e| matches!(e, Event::RestEvent { .. })),
        "the grass beside the road"
    );
}

#[test]
fn rest_is_refused_in_a_fight_or_before_one() {
    let data = data();
    let mut world = at(&data, 5, 2, "meadow", 16, 16);
    let retreat = world.position;
    world.mode = Mode::Encounter(encounter(
        &data,
        &[("giant_rat", 1)],
        Disposition::Hostile,
        retreat,
    ));
    refused(&mut world, &data, RestCommand::Long, Rejection::WrongMode);
    refused(&mut world, &data, short(&[]), Rejection::WrongMode);
}

#[test]
fn the_rest_words_parse_and_print() {
    for (word, command) in [("rest", RestCommand::Long), ("short-rest", short(&[]))] {
        assert_eq!(parse_word(word), Some(Command::Rest(command.clone())));
        assert_eq!(Command::Rest(command).word(), word);
    }
    assert_eq!(Command::Rest(short(&[1, 2])).word(), "short-rest");
    assert_eq!(
        parse_word("short-rest-1-0-2"),
        Some(Command::Rest(short(&[1, 0, 2])))
    );
    assert_eq!(parse_word("short-rest-3"), Some(Command::Rest(short(&[3]))));
    for bad in [
        "short-rest-",
        "short-rest-1-",
        "short-rest-x",
        "short-rest-1-256",
    ] {
        assert_eq!(parse_word(bad), None, "{bad}");
    }
}
