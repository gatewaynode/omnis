//! Items on the ground and in containers on the yard fixture (M8.5 step 2a, item 11): Drop,
//! PickUp, TakeOut and PutIn, the piles and contents the save keeps, and every refusal.

use crate::objects::{in_yard, objects, yard};
use omnis_core::{CharacterId, Facing, ItemId, Level};
use omnis_data::Data;
use omnis_data::placement::Pile;
use omnis_sim::event::ItemPlace;
use omnis_sim::ground::pile_at;
use omnis_sim::objects::{ObjectState, Refusal};
use omnis_sim::ops::{load_text, save_text};
use omnis_sim::{Command, Event, ItemCommand, Rejection, World, apply};

const DAGGER: &str = "base:item:dagger";
const ARROWS: &str = "base:item:arrows";

fn item(data: &Data, id: &str) -> ItemId {
    data.registry.items.get(id).expect(id)
}

/// A party of one at `(x, y)` facing `facing`, carrying `n` daggers.
fn armed(data: &Data, x: u16, y: u16, facing: Facing, n: u16) -> World {
    let mut world = in_yard(data, 0, x, y, facing);
    let dagger = item(data, DAGGER);
    let kit = &mut world.party.members[0].equipment;
    kit.retain(|(i, _)| *i != dagger);
    kit.push((dagger, n));
    world
}

fn carried(world: &World, data: &Data, id: &str) -> u16 {
    let id = item(data, id);
    world.party.members[0]
        .equipment
        .iter()
        .filter(|(i, _)| *i == id)
        .map(|(_, n)| *n)
        .sum()
}

fn pile(world: &World, data: &Data, x: u16, y: u16) -> Option<Pile> {
    pile_at(world, data, yard(data), (Level(0), x, y)).cloned()
}

fn saved_ground(world: &World, data: &Data) -> usize {
    world.maps.get(&yard(data)).map_or(0, |s| s.ground.len())
}

fn drop(member: CharacterId, id: &str, count: u16) -> Command {
    Command::Item(ItemCommand::Drop {
        member,
        item: id.into(),
        count,
    })
}

fn pick_up(member: CharacterId, id: &str, count: u16) -> Command {
    Command::Item(ItemCommand::PickUp {
        member,
        item: id.into(),
        count,
    })
}

fn take_out(member: CharacterId, object: &str, id: &str, count: u16) -> Command {
    Command::Item(ItemCommand::TakeOut {
        member,
        object: object.into(),
        item: id.into(),
        count,
    })
}

fn put_in(member: CharacterId, object: &str, id: &str, count: u16) -> Command {
    Command::Item(ItemCommand::PutIn {
        member,
        object: object.into(),
        item: id.into(),
        count,
    })
}

fn moved(events: &[Event]) -> Option<(ItemPlace, ItemPlace)> {
    events.iter().find_map(|e| match e {
        Event::ItemMoved { from, to, .. } => Some((from.clone(), to.clone())),
        _ => None,
    })
}

/// Apply a command that must be refused, and check the world did not change.
fn refused(world: &mut World, data: &Data, command: Command) -> Rejection {
    let before = world.clone();
    let rejection = apply(world, data, command).expect_err("refused");
    assert_eq!(*world, before, "a refusal changes nothing");
    rejection
}

/// The world read back from its save is the same, but for the log and the journal a save does
/// not keep.
fn round_trip(world: &World, data: &Data) {
    let text = save_text(world).expect("saves");
    let mut back = load_text(&text, data, false).expect("loads");
    back.log.clone_from(&world.log);
    back.journal.clone_from(&world.journal);
    assert_eq!(back, *world);
}

fn set(world: &mut World, data: &Data, id: &str, state: ObjectState) {
    world.map_state(yard(data)).objects.insert(id.into(), state);
}

fn open_chest(world: &mut World, data: &Data) {
    let open = ObjectState {
        state: Some("open".into()),
        ..ObjectState::default()
    };
    set(world, data, "chest_1", open);
}

/// `n` kinds of item no pack defines, one of each: they only fill room.
fn others(n: u32) -> Vec<(ItemId, u16)> {
    (0..n).map(|i| (ItemId(100_000 + i), 1)).collect()
}

#[test]
fn a_drop_lies_ahead_stacks_and_survives_a_save() {
    let data = objects();
    let mut world = armed(&data, 0, 1, Facing::East, 2);
    let m = world.party.members[0].id;
    let dagger = item(&data, DAGGER);
    let member = world.party.members[0].id;
    let events = apply(&mut world, &data, drop(m, DAGGER, 1)).expect("carried");
    assert_eq!(
        moved(&events),
        Some((ItemPlace::Member(member), ItemPlace::Ground))
    );
    assert_eq!(carried(&world, &data, DAGGER), 1);
    let laid = Pile {
        pos: (2, 0),
        items: vec![(dagger, 1)],
    };
    assert_eq!(pile(&world, &data, 0, 1), Some(laid));
    world.position.facing = Facing::South;
    apply(&mut world, &data, drop(m, DAGGER, 1)).expect("the other one");
    assert_eq!(
        pile(&world, &data, 0, 1),
        Some(Pile {
            pos: (2, 0),
            items: vec![(dagger, 2)],
        }),
        "a pile already there takes it where it lies"
    );
    round_trip(&world, &data);
}

#[test]
fn dropping_the_last_worn_one_takes_it_off() {
    let data = objects();
    let mut world = armed(&data, 0, 1, Facing::East, 1);
    let m = world.party.members[0].id;
    let member = world.party.members[0].id;
    let wield = Command::Item(ItemCommand::Equip {
        member,
        item: DAGGER.into(),
    });
    apply(&mut world, &data, wield).expect("a dagger in hand");
    let events = apply(&mut world, &data, drop(m, DAGGER, 1)).expect("carried");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Unequipped { member: m, .. } if *m == member))
    );
    assert!(
        !world.party.members[0]
            .equipped
            .values()
            .any(|i| *i == item(&data, DAGGER))
    );
}

#[test]
fn the_placed_pile_is_picked_up_and_put_back_as_it_was() {
    let data = objects();
    let mut world = armed(&data, 2, 2, Facing::North, 0);
    let m = world.party.members[0].id;
    let member = world.party.members[0].id;
    let events = apply(&mut world, &data, pick_up(m, ARROWS, 4)).expect("ten lie here");
    assert_eq!(
        moved(&events),
        Some((ItemPlace::Ground, ItemPlace::Member(member)))
    );
    assert_eq!(carried(&world, &data, ARROWS), 4);
    let arrows = item(&data, ARROWS);
    assert_eq!(
        pile(&world, &data, 2, 2).map(|p| p.items),
        Some(vec![(arrows, 6)])
    );
    apply(&mut world, &data, drop(m, ARROWS, 4)).expect("back");
    assert_eq!(
        saved_ground(&world, &data),
        0,
        "the pile is the map's again, so the save keeps nothing"
    );
    apply(&mut world, &data, pick_up(m, ARROWS, 10)).expect("all of them");
    assert_eq!(pile(&world, &data, 2, 2), None);
    assert_eq!(
        saved_ground(&world, &data),
        1,
        "an emptied placed pile is remembered"
    );
    apply(&mut world, &data, drop(m, ARROWS, 1)).expect("one back");
    assert_eq!(
        pile(&world, &data, 2, 2),
        Some(Pile {
            pos: (0, -2),
            items: vec![(arrows, 1)],
        }),
        "a new pile, ahead of the party"
    );
}

#[test]
fn picking_up_what_is_not_there_is_refused() {
    let data = objects();
    let mut world = armed(&data, 2, 2, Facing::North, 0);
    let m = world.party.members[0].id;
    let refusals = [
        (pick_up(m, ARROWS, 0), Rejection::ZeroCount),
        (
            pick_up(m, ARROWS, 11),
            Rejection::NotEnough {
                item: ARROWS.into(),
                have: 10,
            },
        ),
        (
            pick_up(m, DAGGER, 1),
            Rejection::NotOnGround {
                item: DAGGER.into(),
            },
        ),
        (
            drop(m, DAGGER, 1),
            Rejection::UnknownItem {
                item: DAGGER.into(),
            },
        ),
    ];
    for (command, rejection) in refusals {
        assert_eq!(refused(&mut world, &data, command), rejection);
    }
    world.position.x = 1;
    assert_eq!(
        refused(&mut world, &data, pick_up(m, ARROWS, 1)),
        Rejection::NotOnGround {
            item: ARROWS.into()
        },
        "the pile is in the next cell"
    );
}

#[test]
fn a_full_pile_takes_more_of_what_it_holds_but_no_new_kind() {
    let data = objects();
    let mut world = armed(&data, 0, 1, Facing::East, 2);
    let m = world.party.members[0].id;
    world.party.members[0]
        .equipment
        .push((item(&data, ARROWS), 1));
    let pile = Pile {
        pos: (0, 0),
        items: others(31),
    };
    world
        .map_state(yard(&data))
        .ground
        .insert((Level(0), 0, 1), pile);
    apply(&mut world, &data, drop(m, DAGGER, 1)).expect("the 32nd kind");
    assert_eq!(
        refused(&mut world, &data, drop(m, ARROWS, 1)),
        Rejection::PileFull
    );
    apply(&mut world, &data, drop(m, DAGGER, 1)).expect("a kind it holds");
}

#[test]
fn the_chest_gives_up_its_dagger_only_when_open_and_takes_it_back() {
    let data = objects();
    let mut world = armed(&data, 5, 3, Facing::West, 0);
    let m = world.party.members[0].id;
    let member = world.party.members[0].id;
    assert_eq!(
        refused(&mut world, &data, take_out(m, "chest_1", DAGGER, 1)),
        Rejection::VerbRefused {
            reason: Refusal::Closed
        }
    );
    open_chest(&mut world, &data);
    let events = apply(&mut world, &data, take_out(m, "chest_1", DAGGER, 1)).expect("open now");
    assert_eq!(
        moved(&events),
        Some((
            ItemPlace::Object("chest_1".into()),
            ItemPlace::Member(member)
        ))
    );
    assert_eq!(carried(&world, &data, DAGGER), 1);
    let state = |world: &World| {
        world
            .maps
            .get(&yard(&data))
            .and_then(|s| s.objects.get("chest_1"))
            .cloned()
    };
    assert_eq!(state(&world).and_then(|s| s.contents), Some(vec![]));
    round_trip(&world, &data);
    let events = apply(&mut world, &data, put_in(m, "chest_1", DAGGER, 1)).expect("back in");
    assert_eq!(
        moved(&events),
        Some((
            ItemPlace::Member(member),
            ItemPlace::Object("chest_1".into())
        ))
    );
    assert_eq!(
        state(&world).and_then(|s| s.contents),
        None,
        "as placed, so only its state is kept"
    );
}

#[test]
fn container_refusals() {
    let data = objects();
    let mut world = armed(&data, 5, 3, Facing::West, 1);
    let m = world.party.members[0].id;
    open_chest(&mut world, &data);
    assert_eq!(
        refused(&mut world, &data, take_out(m, "chest_1", ARROWS, 1)),
        Rejection::NotInObject {
            item: ARROWS.into()
        }
    );
    assert_eq!(
        refused(&mut world, &data, take_out(m, "chest_1", DAGGER, 2)),
        Rejection::NotEnough {
            item: DAGGER.into(),
            have: 1
        }
    );
    assert_eq!(
        refused(&mut world, &data, take_out(m, "crate_1", DAGGER, 1)),
        Rejection::NoSuchObject {
            object: "crate_1".into()
        },
        "not in reach"
    );
    let full = ObjectState {
        state: Some("open".into()),
        contents: Some(others(8)),
        ..ObjectState::default()
    };
    set(&mut world, &data, "chest_1", full);
    assert_eq!(
        refused(&mut world, &data, put_in(m, "chest_1", DAGGER, 1)),
        Rejection::VerbRefused {
            reason: Refusal::Full
        }
    );
    let mut room = others(7);
    room.push((item(&data, DAGGER), 1));
    let seven = ObjectState {
        state: Some("open".into()),
        contents: Some(room),
        ..ObjectState::default()
    };
    set(&mut world, &data, "chest_1", seven);
    apply(&mut world, &data, put_in(m, "chest_1", DAGGER, 1)).expect("a kind it holds");

    let mut world = armed(&data, 0, 1, Facing::East, 1);

    let m = world.party.members[0].id;
    assert_eq!(
        refused(&mut world, &data, put_in(m, "crate_1", DAGGER, 1)),
        Rejection::VerbRefused {
            reason: Refusal::NotOffered
        },
        "a crate is no container"
    );
}

#[test]
fn a_member_who_cannot_act_moves_nothing_and_not_outside_exploring() {
    let data = objects();
    let mut world = armed(&data, 2, 2, Facing::North, 1);
    let m = world.party.members[0].id;
    world.party.members[0].hp = 0;
    assert!(matches!(
        refused(&mut world, &data, pick_up(m, ARROWS, 1)),
        Rejection::MemberDead { .. } | Rejection::MemberDown { .. }
    ));
    assert!(matches!(
        refused(&mut world, &data, drop(m, DAGGER, 1)),
        Rejection::MemberDead { .. } | Rejection::MemberDown { .. }
    ));
    let mut world = armed(&data, 2, 2, Facing::North, 1);
    let m = world.party.members[0].id;
    world.mode = omnis_sim::Mode::Town(omnis_sim::ServiceState {
        service: omnis_core::ServiceId(0),
        kind: omnis_data::ServiceKind::Inn,
    });
    for command in [
        pick_up(m, ARROWS, 1),
        drop(m, DAGGER, 1),
        take_out(m, "chest_1", DAGGER, 1),
        put_in(m, "chest_1", DAGGER, 1),
    ] {
        assert_eq!(refused(&mut world, &data, command), Rejection::WrongMode);
    }
}
