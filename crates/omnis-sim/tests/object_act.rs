//! `Command::Act` on the yard fixture (M8.5 step 2a, item 9): reach, refusals, checks, items
//! used up, `once`, and every effect.

use crate::common::six;
use crate::objects::{in_yard, objects, yard};
use omnis_core::{Facing, ItemId, StreamName};
use omnis_data::{Data, Verb};
use omnis_sim::event::{CheckKind, ItemPlace};
use omnis_sim::objects::{ObjectState, Refusal};
use omnis_sim::{Command, Event, PartyCommand, Rejection, World, apply, query};

fn act(object: &str, verb: Verb, option: u8) -> Command {
    Command::Act {
        object: object.into(),
        verb,
        option,
    }
}

fn refused(reason: Refusal) -> Result<Vec<Event>, Rejection> {
    Err(Rejection::VerbRefused { reason })
}

fn no_such(object: &str) -> Result<Vec<Event>, Rejection> {
    Err(Rejection::NoSuchObject {
        object: object.into(),
    })
}

/// Mark a hidden placement found, as Search or a sharp eye will (item 13).
fn notice(world: &mut World, data: &Data, id: &str) {
    world.known.notice(yard(data), id);
}

/// Add the `n`th of the six standard drafts to the party.
fn join(world: &mut World, data: &Data, n: usize) {
    let draft = six().swap_remove(n);
    apply(world, data, Command::Party(PartyCommand::Create(draft))).expect("a valid draft");
}

fn item(data: &Data, id: &str) -> ItemId {
    data.registry.items.get(id).expect(id)
}

fn saved<'w>(world: &'w World, data: &Data, id: &str) -> Option<&'w ObjectState> {
    world.maps.get(&yard(data)).and_then(|s| s.objects.get(id))
}

fn minutes(events: &[Event]) -> u32 {
    events
        .iter()
        .filter_map(|e| match e {
            Event::TimeAdvanced { minutes, .. } => Some(*minutes),
            _ => None,
        })
        .sum()
}

fn used(events: &[Event]) -> Option<bool> {
    events.iter().find_map(|e| match e {
        Event::ObjectUsed { success, .. } => Some(*success),
        _ => None,
    })
}

fn said(events: &[Event], key: &str) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::ObjectMessage { key: k, .. } if k == key))
}

/// Act until the attempt succeeds, the world as it stood before each failure kept apart.
fn until_success(world: &mut World, data: &Data, command: &Command) -> (usize, Vec<Event>) {
    for failures in 0..40 {
        let events = apply(world, data, command.clone()).expect("offered");
        if used(&events) == Some(true) {
            return (failures, events);
        }
    }
    panic!("forty failures in a row");
}

#[test]
fn examining_the_crate_says_its_text_and_spends_a_minute() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 1, Facing::East);
    let before = world.party_clock().elapsed;
    let events = apply(&mut world, &data, act("crate_1", Verb::Examine, 0)).expect("in reach");
    assert!(events.contains(&Event::ObjectUsed {
        object: "crate_1".into(),
        verb: Verb::Examine,
        success: true,
    }));
    assert!(said(&events, "objects:text:crate.examine"));
    assert_eq!(minutes(&events), 1, "the interact cost, as a door");
    assert_eq!(world.party_clock().elapsed, before + 1);
    assert_eq!(
        saved(&world, &data, "crate_1"),
        None,
        "nothing changed, so the save remembers nothing"
    );
}

#[test]
fn an_object_not_in_reach_is_no_such_object_and_changes_nothing() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 1, Facing::East);
    let before = world.clone();
    for id in ["statue_1", "nothing", "crate_2"] {
        assert_eq!(
            apply(&mut world, &data, act(id, Verb::Examine, 0)),
            no_such(id),
            "{id}: too far, not in the map, or on the loft"
        );
    }
    assert_eq!(world, before, "a rejection leaves the world as it was");

    let mut world = in_yard(&data, 0, 1, 2, Facing::South);
    assert_eq!(
        apply(&mut world, &data, act("fountain_1", Verb::Drink, 0)),
        no_such("fountain_1"),
        "the hedge stands between"
    );

    let mut world = in_yard(&data, 0, 0, 1, Facing::North);
    assert_eq!(
        apply(&mut world, &data, act("lever_1", Verb::Pull, 0)),
        no_such("lever_1"),
        "a hidden lever is not there until found"
    );
    notice(&mut world, &data, "lever_1");
    assert!(apply(&mut world, &data, act("lever_1", Verb::Pull, 0)).is_ok());
    world.position.facing = Facing::South;
    assert_eq!(
        apply(&mut world, &data, act("lever_1", Verb::Pull, 0)),
        no_such("lever_1"),
        "the party faces away from its cell"
    );
}

#[test]
fn a_verb_the_object_does_not_offer_now_is_refused() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 1, Facing::East);
    assert_eq!(
        apply(&mut world, &data, act("crate_1", Verb::Pull, 0)),
        refused(Refusal::NotOffered)
    );
    assert_eq!(
        apply(&mut world, &data, act("crate_1", Verb::Examine, 1)),
        refused(Refusal::NotOffered),
        "one Examine, so no second option"
    );
    let mut world = in_yard(&data, 0, 5, 4, Facing::North);
    assert_eq!(
        apply(&mut world, &data, act("chest_1", Verb::Close, 0)),
        refused(Refusal::NotOffered),
        "Close is offered only when open"
    );
    assert_eq!(
        apply(&mut world, &data, act("chest_1", Verb::Open, 2)),
        refused(Refusal::NotOffered),
        "two ways to open, 0 and 1"
    );
}

#[test]
fn the_lever_sets_the_flag_that_lets_the_gate_open_and_the_party_through() {
    let data = objects();
    let mut world = in_yard(&data, 0, 3, 2, Facing::South);
    assert_eq!(
        apply(&mut world, &data, act("gate_1", Verb::Push, 0)),
        refused(Refusal::Flag {
            flag: "objects:flag:lever".into()
        })
    );
    world.position = world.position.at(0, 1);
    world.position.facing = Facing::North;
    notice(&mut world, &data, "lever_1");
    let events = apply(&mut world, &data, act("lever_1", Verb::Pull, 0)).expect("found");
    assert!(events.contains(&Event::ObjectState {
        object: "lever_1".into(),
        state: "down".into(),
    }));
    assert!(said(&events, "objects:text:lever.pulled"));
    assert!(query::flags(&world, &data).contains(&("objects:flag:lever".into(), 1)));
    assert_eq!(
        saved(&world, &data, "lever_1").and_then(|s| s.state.as_deref()),
        Some("down")
    );

    world.position = world.position.at(3, 2);
    world.position.facing = Facing::South;
    let pushed = apply(&mut world, &data, act("gate_1", Verb::Push, 0)).expect("unlocked");
    assert!(pushed.contains(&Event::ObjectState {
        object: "gate_1".into(),
        state: "open".into(),
    }));
    let through = apply(
        &mut world,
        &data,
        Command::Step(omnis_core::Direction::Forward),
    )
    .expect("explore");
    assert!(
        !through.iter().any(|e| matches!(e, Event::Blocked { .. })),
        "the open gate lets the party through"
    );
    assert_eq!((world.position.x, world.position.y), (3, 3));
    apply(&mut world, &data, act("gate_1", Verb::Pull, 0)).expect("the gate is in reach");
    assert_eq!(
        saved(&world, &data, "gate_1"),
        None,
        "shut again, as it started: the save forgets it"
    );
}

#[test]
fn a_once_interaction_succeeds_once() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 2, Facing::East);
    let gem = item(&data, "base:item:gem");
    let events = apply(&mut world, &data, act("stone_1", Verb::Take, 0)).expect("in the cell");
    assert!(events.contains(&Event::ItemFound {
        item: "base:item:gem".into(),
        count: 1,
    }));
    assert!(events.contains(&Event::ObjectRemoved {
        object: "stone_1".into()
    }));
    assert!(world.party.inventory.contains(&(gem, 1)));
    assert_eq!(
        apply(&mut world, &data, act("stone_1", Verb::Take, 0)),
        no_such("stone_1"),
        "taken away for good"
    );

    let mut world = in_yard(&data, 0, 5, 4, Facing::North);
    world.map_state(yard(&data)).objects.insert(
        "chest_1".into(),
        ObjectState {
            state: Some("open".into()),
            ..ObjectState::default()
        },
    );
    let gold = world.party.gold;
    let events = apply(&mut world, &data, act("chest_1", Verb::Search, 0)).expect("open");
    assert!(events.contains(&Event::GoldFound { amount: 2500 }));
    assert_eq!(world.party.gold, gold + 2500);
    assert_eq!(
        apply(&mut world, &data, act("chest_1", Verb::Search, 0)),
        refused(Refusal::Spent)
    );
    assert_eq!(world.party.gold, gold + 2500);
}

#[test]
fn a_failed_check_spends_the_minutes_and_the_roll_but_not_the_attempt() {
    let data = objects();
    let force = act("chest_1", Verb::Open, 0);
    let stream = StreamName::new("object");
    let mut seen_failure = false;
    for seed in 0..20 {
        let mut world = in_yard(&data, 0, 5, 4, Facing::North);
        world.seed = seed;
        let events = apply(&mut world, &data, force.clone()).expect("offered");
        let check = events.iter().find_map(|e| match e {
            Event::Check { kind, success, .. } => Some((*kind, *success)),
            _ => None,
        });
        assert_eq!(
            check.map(|c| c.0),
            Some(CheckKind::Skill(omnis_data::Skill::Athletics))
        );
        assert!(world.rngs.contains_key(&stream), "the roll is kept");
        assert_eq!(minutes(&events), 2, "the kind's minutes, either way");
        if used(&events) == Some(false) {
            seen_failure = true;
            assert_eq!(check.map(|c| c.1), Some(false));
            assert!(said(&events, "objects:text:chest.stuck"));
            assert_eq!(saved(&world, &data, "chest_1"), None, "still closed");
            let (_, events) = until_success(&mut world, &data, &force);
            assert!(said(&events, "objects:text:chest.opened"));
            assert_eq!(
                saved(&world, &data, "chest_1").and_then(|s| s.state.as_deref()),
                Some("open")
            );
            break;
        }
    }
    assert!(seen_failure, "some seed fails Athletics 15 at +4");
}

#[test]
fn a_gem_is_needed_and_used_up_only_when_the_wish_succeeds() {
    let data = objects();
    let wish = act("fountain_1", Verb::Use, 0);
    let gem = item(&data, "base:item:gem");
    let mut world = in_yard(&data, 0, 0, 3, Facing::East);
    assert_eq!(
        apply(&mut world, &data, wish.clone()),
        refused(Refusal::Lacks {
            item: "base:item:gem".into()
        })
    );
    world.party.inventory.push((gem, 2));
    assert_eq!(
        apply(&mut world, &data, wish.clone()),
        refused(Refusal::Rule {
            rule: "objects.crowd".into()
        }),
        "a wish needs company"
    );
    assert!(
        !world.rngs.contains_key(&StreamName::new("object")),
        "a refusal keeps no roll"
    );
    join(&mut world, &data, 1);
    let (mut failed, mut succeeded) = (false, false);
    for seed in 0..20 {
        let mut world = world.clone();
        world.seed = seed;
        let events = apply(&mut world, &data, wish.clone()).expect("offered");
        if used(&events) == Some(false) {
            failed = true;
            assert!(said(&events, "objects:text:fountain.sinks"));
            assert!(!events.iter().any(|e| matches!(e, Event::ItemSpent { .. })));
            assert_eq!(world.party.inventory, [(gem, 2)], "the gem is kept");
        } else {
            succeeded = true;
            assert!(events.contains(&Event::ItemSpent {
                item: "base:item:gem".into(),
                count: 1,
                from: ItemPlace::Stores,
            }));
            assert!(events.contains(&Event::GoldFound { amount: 100 }));
            assert_eq!(world.party.inventory, [(gem, 1)]);
        }
    }
    assert!(
        failed && succeeded,
        "Wisdom 12 at +2 goes both ways over 20 seeds"
    );

    world.party.inventory.clear();
    world.party.members[1].equipment.push((gem, 1));
    let durin = world.party.members[1].id;
    let (_, events) = until_success(&mut world, &data, &wish);
    assert!(
        events.contains(&Event::ItemSpent {
            item: "base:item:gem".into(),
            count: 1,
            from: ItemPlace::Member(durin),
        }),
        "a spare in a kit serves when the stores hold none"
    );
}

#[test]
fn a_worn_item_is_not_a_spare_to_use_up() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 3, Facing::East);
    join(&mut world, &data, 1);
    let gem = item(&data, "base:item:gem");
    let brenna = &mut world.party.members[0];
    brenna.equipment.push((gem, 1));
    brenna.equipped.insert(omnis_data::EquipSlot::OffHand, gem);
    assert_eq!(
        apply(&mut world, &data, act("fountain_1", Verb::Use, 0)),
        refused(Refusal::Lacks {
            item: "base:item:gem".into()
        })
    );
}

#[test]
fn the_rune_needs_someone_who_knows_abjuration() {
    let data = objects();
    let mut world = in_yard(&data, 0, 2, 3, Facing::South);
    notice(&mut world, &data, "rune_1");
    let read = act("rune_1", Verb::Read, 0);
    assert_eq!(
        apply(&mut world, &data, read.clone()),
        refused(Refusal::School {
            school: omnis_data::School::Abjuration
        })
    );
    let shield = data
        .registry
        .spells
        .get("base:spell:shield")
        .expect("shield");
    world.party.members[0].known_spells.push(shield);
    let events = apply(&mut world, &data, read).expect("she knows Shield now");
    assert!(said(&events, "objects:text:rune.read"));
}

#[test]
fn nobody_who_can_act_cannot_act() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 1, Facing::East);
    world.party.members[0].hp = 0;
    assert_eq!(
        apply(&mut world, &data, act("crate_1", Verb::Examine, 0)),
        refused(Refusal::NoneCanAct)
    );
}
