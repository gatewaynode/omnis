//! Links (owner, 2026-11-14; `tasks/plans/targeting.md` step 8): "special link is an advanced
//! targeting option for magically connected or otherwise always targetable objects or beings."
//! A member's links are saved with the party; an option whose range is `Link` reaches exactly
//! what its user is linked to, wherever it is, seen or not. No spell forges a link yet: a dev
//! command does, so the rule can be tested.

use crate::common::{self, party_of};
use crate::objects::{objects, yard};
use omnis_core::{CharacterId, Facing, Level};
use omnis_data::{Data, Range, Targeting};
use omnis_sim::save::{join, split};
use omnis_sim::{
    Chosen, Command, DevCommand, Mode, Pay, Rejection, Settings, Sight, Target, TargetView, World,
    apply, targets,
};

const CLERIC: usize = 1;
const WIZARD: usize = 2;
const HAND: &str = "base:spell:mage_hand";
const CURE: &str = "base:spell:cure_wounds";
const BOLT: &str = "base:spell:fire_bolt";

/// The data with each of `spells` reaching by link alone.
fn linked(mut data: Data, spells: &[&str]) -> Data {
    for spell in spells {
        let id = data.registry.spells.get(spell).unwrap();
        data.spells.get_mut(&id).unwrap().targeting = Some(Targeting::of(Range::Link));
    }
    data
}

fn at(data: &Data, map: &str, x: u16, y: u16, facing: Facing) -> World {
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    let mut world = common::new_world(data, 3, settings);
    party_of(&mut world, data, 6);
    let teleport = DevCommand::Teleport {
        map: map.into(),
        level: Level::GROUND,
        x,
        y,
        facing,
    };
    apply(&mut world, data, Command::Dev(teleport)).expect("a cell of the map");
    world
}

fn link(world: &mut World, data: &Data, member: usize, target: Target, linked: bool) {
    let member = world.party.members[member].id;
    let forge = DevCommand::Link {
        member,
        target,
        linked,
    };
    apply(world, data, Command::Dev(forge)).expect("a link");
}

fn cast(world: &World, member: usize, spell: &str, target: Target) -> Command {
    Command::Cast {
        caster: world.party.members[member].id,
        spell: spell.into(),
        target,
    }
}

/// The spell's list for `member`, each row checked against the command on a copy of the world.
fn listed(world: &World, data: &Data, member: usize, spell: &str) -> Vec<TargetView> {
    let id = world.party.members[member].id;
    let chosen = Chosen::Spell {
        spell: spell.into(),
        pay: Pay::Action,
    };
    let rows = targets(world, data, id, &chosen).expect("a list");
    for row in &rows {
        let mut copy = world.clone();
        let result = apply(
            &mut copy,
            data,
            cast(world, member, spell, row.target.clone()),
        );
        assert_eq!(result.as_ref().err(), row.refusal.as_ref(), "{row:?}");
    }
    rows
}

fn refused(world: &World, data: &Data, member: usize, spell: &str, target: Target) -> Rejection {
    let mut copy = world.clone();
    let result = apply(&mut copy, data, cast(world, member, spell, target));
    assert_eq!(&copy, world, "a refusal changes nothing");
    result.expect_err("refused")
}

fn door(x: u16, y: u16, facing: Facing) -> Target {
    Target::Door { x, y, facing }
}

/// A hand that reaches by link: nothing until a link is forged, then the linked door from four
/// rooms off, never seen; the link is its user's alone, and is unforged as it was forged.
#[test]
fn a_linked_door_is_reached_from_anywhere_by_its_holder_alone() {
    let data = linked(common::data(), &[HAND]);
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    let mut world = at(&data, "test:map:dungeon", 22, 0, Facing::North);
    let south = door(9, 5, Facing::South);
    assert!(
        listed(&world, &data, WIZARD, HAND).is_empty(),
        "no link, no target"
    );
    assert_eq!(
        refused(&world, &data, WIZARD, HAND, south.clone()),
        Rejection::BeyondRange
    );
    // Forged by the edge's other name: one door, one link.
    link(&mut world, &data, WIZARD, door(9, 6, Facing::North), true);
    let wizard = world.party.members[WIZARD].id;
    assert_eq!(world.links[&wizard].iter().collect::<Vec<_>>(), [&south]);
    let rows = listed(&world, &data, WIZARD, HAND);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].target.clone(), rows[0].distance, rows[0].sight),
        (south.clone(), 13, Sight::Obscured)
    );
    assert_eq!(rows[0].refusal, None, "wherever it is, seen or not");
    // Sent by the edge's other name it is the same linked door.
    let other = cast(&world, WIZARD, HAND, door(9, 6, Facing::North));
    assert!(apply(&mut world.clone(), &data, other).is_ok());
    let command = cast(&world, WIZARD, HAND, south.clone());
    apply(&mut world, &data, command).unwrap();
    assert!(world.maps[&dungeon].door_open(Level::GROUND, 9, 5, Facing::South));
    // A link to an edge with no door reaches nothing.
    link(&mut world, &data, WIZARD, door(9, 2, Facing::South), true);
    assert_eq!(
        refused(&world, &data, WIZARD, HAND, door(9, 2, Facing::South)),
        Rejection::NoDoor
    );
    // Unforged: gone from the list, and the world keeps no empty entry.
    link(&mut world, &data, WIZARD, door(9, 2, Facing::South), false);
    link(&mut world, &data, WIZARD, south.clone(), false);
    assert!(listed(&world, &data, WIZARD, HAND).is_empty());
    assert!(world.links.is_empty());
    // A member not in the party holds no link.
    let stranger = DevCommand::Link {
        member: CharacterId(99),
        target: south,
        linked: true,
    };
    assert_eq!(
        apply(&mut world, &data, Command::Dev(stranger)),
        Err(Rejection::NoSuchMember {
            member: CharacterId(99)
        })
    );
}

/// A heal by link reaches the member linked and no other, on the road and in a fight; a hidden
/// object linked is worked though never found; a group linked is struck though not seen.
#[test]
fn a_link_reaches_a_member_an_object_and_a_group() {
    let data = linked(objects(), &[HAND, CURE, BOLT]);
    let mut world = at(&data, "objects:map:yard", 5, 4, Facing::North);
    let ids = world.party.ids();
    let brenna = Target::Member(ids[0]);
    link(&mut world, &data, CLERIC, brenna.clone(), true);
    world.party.members[0].hp = 1;
    let rows = listed(&world, &data, CLERIC, CURE);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].target.clone(), rows[0].refusal.clone()),
        (brenna, None)
    );
    assert_eq!(
        refused(&world, &data, CLERIC, CURE, Target::Member(ids[3])),
        Rejection::BeyondRange,
        "not linked"
    );
    // The wizard's links are not the cleric's.
    assert!(listed(&world, &data, WIZARD, HAND).is_empty());
    // The hidden lever, never found, across the yard.
    let lever = Target::Object {
        object: "lever_1".into(),
    };
    link(&mut world, &data, WIZARD, lever.clone(), true);
    let rows = listed(&world, &data, WIZARD, HAND);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].target.clone(), rows[0].refusal.clone()),
        (lever.clone(), None)
    );
    let command = cast(&world, WIZARD, HAND, lever);
    apply(&mut world, &data, command).unwrap();
    let saved = world.maps[&yard(&data)].objects["lever_1"].state.clone();
    assert_eq!(saved.as_deref(), Some("down"));
    // The rats behind the hedge, unseen from here: linked, they are struck.
    let rat = Target::Being {
        being: "rat".into(),
    };
    assert!(listed(&world, &data, WIZARD, BOLT).is_empty());
    link(&mut world, &data, WIZARD, rat.clone(), true);
    let rows = listed(&world, &data, WIZARD, BOLT);
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].target.clone(), rows[0].distance), (rat.clone(), 3));
    let command = cast(&world, WIZARD, BOLT, rat);
    apply(&mut world, &data, command).unwrap();
    assert!(matches!(world.mode, Mode::Combat(_)));
}

/// Links are the party's and are saved with it; a world with none writes nothing new.
#[test]
fn links_are_saved_with_the_party() {
    let data = common::data();
    let mut world = at(&data, "test:map:dungeon", 9, 2, Facing::South);
    let plain = world.to_ron().unwrap();
    assert!(!plain.contains("links"), "no link, nothing written");
    let ids = world.party.ids();
    link(&mut world, &data, WIZARD, door(9, 5, Facing::South), true);
    link(&mut world, &data, WIZARD, Target::Member(ids[0]), true);
    link(
        &mut world,
        &data,
        CLERIC,
        Target::Object { object: "x".into() },
        true,
    );
    let text = world.to_ron().unwrap();
    assert!(text.contains("links"));
    let back = World::from_ron(&text, &data, false).unwrap();
    assert_eq!(back.links, world.links);
    assert_eq!(back.links[&ids[WIZARD]].len(), 2);
    let joined = join(split(&world, &data), &data, false).unwrap();
    assert_eq!(joined.links, world.links);
    assert_eq!(joined.fingerprint(), world.fingerprint());
}
