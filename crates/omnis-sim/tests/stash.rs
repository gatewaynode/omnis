//! Store and destroy (owner, 2026-11-12): `ItemCommand::Stash` puts what lies on the ground in
//! reach, or in an open container, into the party's stores; `ItemCommand::Destroy` ends it for
//! good. Both on the yard fixture: ten arrows lie on (2, 2), `chest_1` at (4, 3) holds a dagger.

use crate::ground::{
    ARROWS, DAGGER, armed, item, listed, open_chest, pile, refused, round_trip, saved_ground,
};
use crate::objects::{objects, yard};
use omnis_core::Facing;
use omnis_data::Data;
use omnis_sim::event::ItemPlace;
use omnis_sim::objects::Refusal;
use omnis_sim::{Command, Event, ItemCommand, Rejection, World, apply};

fn stash(from: ItemPlace, id: &str, count: u16) -> Command {
    Command::Item(ItemCommand::Stash {
        from,
        item: id.into(),
        count,
    })
}

fn destroy(from: ItemPlace, id: &str, count: u16) -> Command {
    Command::Item(ItemCommand::Destroy {
        from,
        item: id.into(),
        count,
    })
}

fn chest() -> ItemPlace {
    ItemPlace::Object("chest_1".into())
}

fn stored(world: &World, data: &Data, id: &str) -> u16 {
    let id = item(data, id);
    world
        .party
        .inventory
        .iter()
        .filter(|(i, _)| *i == id)
        .map(|(_, n)| *n)
        .sum()
}

fn in_chest(world: &World, data: &Data) -> Option<Vec<(omnis_core::ItemId, u16)>> {
    world
        .maps
        .get(&yard(data))
        .and_then(|s| s.objects.get("chest_1"))
        .and_then(|s| s.contents.clone())
}

#[test]
fn a_pile_in_reach_goes_into_the_stores_and_the_save_keeps_both() {
    let data = objects();
    // Beside the pile, facing it: reach is the faced cell too.
    let mut world = armed(&data, 1, 2, Facing::East, 0);
    let before = stored(&world, &data, ARROWS);
    let events = apply(&mut world, &data, stash(ItemPlace::Ground, ARROWS, 4)).expect("in reach");
    assert_eq!(
        events[..1],
        [Event::ItemMoved {
            item: ARROWS.into(),
            count: 4,
            from: ItemPlace::Ground,
            to: ItemPlace::Stores,
        }]
    );
    assert_eq!(stored(&world, &data, ARROWS), before + 4);
    assert_eq!(listed(&world, &data), [(ARROWS.to_owned(), 6)]);
    assert_eq!(
        saved_ground(&world, &data),
        1,
        "the pile is saved as changed"
    );
    round_trip(&world, &data);
    // The rest, with nobody able to carry: the stores need no hands.
    world.party.members[0].hp = 0;
    let kit = world.party.members[0].equipment.clone();
    apply(&mut world, &data, stash(ItemPlace::Ground, ARROWS, 6)).expect("the rest");
    assert_eq!(stored(&world, &data, ARROWS), before + 10);
    assert_eq!(pile(&world, &data, 2, 2), None);
    assert_eq!(world.party.members[0].equipment, kit, "nobody carried it");
    round_trip(&world, &data);
}

#[test]
fn an_open_container_gives_to_the_stores_and_a_shut_one_does_not() {
    let data = objects();
    let mut world = armed(&data, 5, 3, Facing::West, 0);
    for command in [stash(chest(), DAGGER, 1), destroy(chest(), DAGGER, 1)] {
        assert_eq!(
            refused(&mut world, &data, command),
            Rejection::VerbRefused {
                reason: Refusal::Closed
            }
        );
    }
    open_chest(&mut world, &data);
    let events = apply(&mut world, &data, stash(chest(), DAGGER, 1)).expect("open now");
    assert_eq!(
        events[..1],
        [Event::ItemMoved {
            item: DAGGER.into(),
            count: 1,
            from: chest(),
            to: ItemPlace::Stores,
        }]
    );
    assert_eq!(stored(&world, &data, DAGGER), 1);
    assert_eq!(in_chest(&world, &data), Some(vec![]));
    round_trip(&world, &data);
}

#[test]
fn destroyed_items_are_gone_from_the_place_and_reach_nobody() {
    let data = objects();
    let mut world = armed(&data, 2, 2, Facing::North, 0);
    let stores = world.party.inventory.clone();
    let kit = world.party.members[0].equipment.clone();
    let events = apply(&mut world, &data, destroy(ItemPlace::Ground, ARROWS, 3)).expect("here");
    assert_eq!(
        events[..1],
        [Event::ItemDestroyed {
            item: ARROWS.into(),
            count: 3,
            from: ItemPlace::Ground,
        }]
    );
    assert_eq!(listed(&world, &data), [(ARROWS.to_owned(), 7)]);
    assert_eq!(world.party.inventory, stores);
    assert_eq!(world.party.members[0].equipment, kit);
    round_trip(&world, &data);
    apply(&mut world, &data, destroy(ItemPlace::Ground, ARROWS, 7)).expect("the rest");
    assert_eq!(pile(&world, &data, 2, 2), None);
    assert_eq!(listed(&world, &data), []);
    round_trip(&world, &data);

    let mut world = armed(&data, 5, 3, Facing::West, 0);
    open_chest(&mut world, &data);
    let events = apply(&mut world, &data, destroy(chest(), DAGGER, 1)).expect("open");
    assert_eq!(
        events[..1],
        [Event::ItemDestroyed {
            item: DAGGER.into(),
            count: 1,
            from: chest(),
        }]
    );
    assert_eq!(in_chest(&world, &data), Some(vec![]));
    assert_eq!(stored(&world, &data, DAGGER), 0);
    round_trip(&world, &data);
}

#[test]
fn every_refusal_leaves_the_world_as_it_was() {
    let data = objects();
    type Make = fn(ItemPlace, &str, u16) -> Command;
    for make in [stash as Make, destroy as Make] {
        // On the pile: a count of zero, more than lie there, a kind that does not.
        let mut world = armed(&data, 2, 2, Facing::North, 1);
        let member = world.party.members[0].id;
        assert_eq!(
            refused(&mut world, &data, make(ItemPlace::Ground, ARROWS, 0)),
            Rejection::ZeroCount
        );
        assert_eq!(
            refused(&mut world, &data, make(ItemPlace::Ground, ARROWS, 11)),
            Rejection::NotEnough {
                item: ARROWS.into(),
                have: 10
            }
        );
        assert_eq!(
            refused(&mut world, &data, make(ItemPlace::Ground, DAGGER, 1)),
            Rejection::NotOnGround {
                item: DAGGER.into()
            }
        );
        // A kit and the stores are not places these take from, though the kit holds a dagger.
        for place in [ItemPlace::Member(member), ItemPlace::Stores] {
            assert_eq!(
                refused(&mut world, &data, make(place, DAGGER, 1)),
                Rejection::NotFromThere
            );
        }
        // A container that is not in reach, and one that holds no such thing.
        assert_eq!(
            refused(&mut world, &data, make(chest(), DAGGER, 1)),
            Rejection::NoSuchObject {
                object: "chest_1".into()
            }
        );
        // Facing away from the pile, it is out of reach.
        let mut world = armed(&data, 1, 2, Facing::West, 0);
        assert_eq!(
            refused(&mut world, &data, make(ItemPlace::Ground, ARROWS, 1)),
            Rejection::NotOnGround {
                item: ARROWS.into()
            }
        );
        let mut world = armed(&data, 5, 3, Facing::West, 0);
        open_chest(&mut world, &data);
        assert_eq!(
            refused(&mut world, &data, make(chest(), ARROWS, 1)),
            Rejection::NotInObject {
                item: ARROWS.into()
            }
        );
        assert_eq!(
            refused(&mut world, &data, make(chest(), DAGGER, 2)),
            Rejection::NotEnough {
                item: DAGGER.into(),
                have: 1
            }
        );
        // Not in a fight, a town service or any mode but exploring.
        let mut world = armed(&data, 2, 2, Facing::North, 0);
        world.mode = omnis_sim::Mode::Town(omnis_sim::ServiceState {
            service: omnis_core::ServiceId(0),
            kind: omnis_data::ServiceKind::Inn,
        });
        for place in [ItemPlace::Ground, chest()] {
            assert_eq!(
                refused(&mut world, &data, make(place, ARROWS, 1)),
                Rejection::WrongMode
            );
        }
    }
}
