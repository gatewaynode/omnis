//! Things of the map as targets (owner, 2026-11-14; PRD §7.3, §8.3; `tasks/plans/targeting.md`
//! step 6): a spell that works on the world lists the doors and objects its range reaches, with
//! their distance and whether the line to them is clear, and acts on the one chosen. Mage hand is
//! a hand: it works the chosen door, or the chosen object by the object's own interaction. What
//! the party neither sees nor knows of is not there for it.

use crate::common::{self, party_of};
use crate::objects::{objects, yard};
use omnis_core::{Facing, Level};
use omnis_data::{Data, Range, Targeting};
use omnis_sim::objects::Refusal;
use omnis_sim::{
    Chosen, Command, DevCommand, Event, Pay, Rejection, Settings, Sight, Target, TargetView, World,
    apply, targets,
};

/// Ilvara's slot in the six drafts, and Durin's.
const WIZARD: usize = 2;
const CLERIC: usize = 1;
const HAND: &str = "base:spell:mage_hand";

fn hand() -> Chosen {
    Chosen::Spell {
        spell: HAND.into(),
        pay: Pay::Action,
    }
}

/// The six, with the dev commands on, standing at `(x, y)` of `map`.
fn at(data: &Data, map: &str, x: u16, y: u16, facing: Facing) -> World {
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    let mut world = common::new_world(data, 3, settings);
    party_of(&mut world, data, 6);
    go(&mut world, data, map, x, y, facing);
    world
}

fn go(world: &mut World, data: &Data, map: &str, x: u16, y: u16, facing: Facing) {
    let teleport = DevCommand::Teleport {
        map: map.into(),
        level: Level::GROUND,
        x,
        y,
        facing,
    };
    apply(world, data, Command::Dev(teleport)).expect("a cell of the map");
}

const YARD: &str = "objects:map:yard";
const DUNGEON: &str = "test:map:dungeon";

fn door(x: u16, y: u16, facing: Facing) -> Target {
    Target::Door { x, y, facing }
}

fn object(id: &str) -> Target {
    Target::Object { object: id.into() }
}

fn cast(world: &World, member: usize, spell: &str, target: Target) -> Command {
    Command::Cast {
        caster: world.party.members[member].id,
        spell: spell.into(),
        target,
    }
}

/// The hand's list, each row checked against the command itself on a copy of the world: a row
/// with no refusal is accepted, a row with one is rejected with exactly it and changes nothing.
fn listed(world: &World, data: &Data) -> Vec<TargetView> {
    let id = world.party.members[WIZARD].id;
    let rows = targets(world, data, id, &hand()).expect("a list");
    for row in &rows {
        let mut copy = world.clone();
        let result = apply(
            &mut copy,
            data,
            cast(world, WIZARD, HAND, row.target.clone()),
        );
        assert_eq!(result.as_ref().err(), row.refusal.as_ref(), "{row:?}");
        if row.refusal.is_some() {
            assert_eq!(&copy, world, "a refusal changes nothing: {row:?}");
        }
    }
    rows
}

/// The wizard sends the hand at `target`.
fn send(world: &mut World, data: &Data, target: Target) -> Vec<Event> {
    let command = cast(world, WIZARD, HAND, target);
    apply(world, data, command).expect("accepted")
}

fn row<'r>(rows: &'r [TargetView], target: &Target) -> Option<&'r TargetView> {
    rows.iter().find(|r| r.target == *target)
}

/// The data with mage hand's targeting written over.
fn with_hand(mut data: Data, targeting: Targeting) -> Data {
    let id = data.registry.spells.get(HAND).unwrap();
    data.spells.get_mut(&id).unwrap().targeting = Some(targeting);
    data
}

fn refused(world: &World, data: &Data, target: Target) -> Rejection {
    let mut copy = world.clone();
    let result = apply(&mut copy, data, cast(world, WIZARD, HAND, target));
    assert_eq!(&copy, world, "a refusal changes nothing");
    result.expect_err("refused")
}

/// South of the hedge, at the gate: the hand's list is the gate, the chest, the fountain and the
/// door behind which the torch stands, nearest first, each with its distance. The crate, the
/// plaque and the torch offer nothing a hand does and are not listed; the rune is hidden.
#[test]
fn the_hand_lists_the_doors_and_objects_it_reaches_nearest_first() {
    let data = objects();
    let world = at(&data, YARD, 3, 3, Facing::East);
    let rows = listed(&world, &data);
    let seen: Vec<(Target, u16, Sight)> = rows
        .iter()
        .map(|r| (r.target.clone(), r.distance, r.sight))
        .collect();
    assert_eq!(
        seen,
        [
            (object("gate_1"), 1, Sight::Clear),
            (object("chest_1"), 2, Sight::Clear),
            (object("fountain_1"), 2, Sight::Clear),
            (door(6, 3, Facing::South), 3, Sight::Clear),
        ]
    );
    // The gate is shut until the lever is pulled: listed, with the object's own reason.
    assert_eq!(
        rows[0].refusal,
        Some(Rejection::VerbRefused {
            reason: Refusal::Flag {
                flag: "objects:flag:lever".into()
            }
        })
    );
    assert_eq!(rows[1].refusal, None);
    assert_eq!(rows[3].refusal, None);
    // A member's list is untouched by the map: a heal still offers the six, all at no distance.
    let cleric = world.party.members[CLERIC].id;
    let cure = Chosen::Spell {
        spell: "base:spell:cure_wounds".into(),
        pay: Pay::Action,
    };
    let members = targets(&world, &data, cleric, &cure).unwrap();
    assert_eq!(members.len(), 6);
    assert!(
        members
            .iter()
            .all(|r| matches!(r.target, Target::Member(_)) && r.distance == 0)
    );
}

/// The hand works the door chosen, whichever way the party faces, and either name of an edge
/// is the same door. It was "the first door straight ahead".
#[test]
fn the_hand_works_the_door_chosen_whichever_way_the_party_faces() {
    let data = common::data();
    let dungeon = data.registry.maps.get(DUNGEON).unwrap();
    let mut world = at(&data, DUNGEON, 9, 2, Facing::North);
    let south = door(9, 5, Facing::South);
    let rows = listed(&world, &data);
    let found = row(&rows, &south).expect("the door to the south");
    assert_eq!(
        (found.distance, found.sight, found.refusal.as_ref()),
        (3, Sight::Clear, None)
    );
    let events = send(&mut world, &data, south.clone());
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::Door {
                x: 9,
                y: 5,
                facing: Facing::South,
                open: true,
                ..
            }
        )),
        "{events:?}"
    );
    assert!(world.maps[&dungeon].door_open(Level::GROUND, 9, 5, Facing::South));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TimeAdvanced { minutes: 1, .. })),
        "a cast's minute"
    );
    // The same edge named from its other side closes it again.
    let other = door(9, 6, Facing::North);
    let events = send(&mut world, &data, other);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Door { open: false, .. }))
    );
    let open = |world: &World| {
        let state = world.maps.get(&dungeon);
        state.is_some_and(|s| s.door_open(Level::GROUND, 9, 5, Facing::South))
    };
    assert!(!open(&world));
    // An edge with no door on it, a member, and a stack are no targets of a hand.
    assert_eq!(
        refused(&world, &data, door(9, 2, Facing::South)),
        Rejection::NoDoor
    );
    let own = Target::Member(world.party.members[WIZARD].id);
    assert_eq!(refused(&world, &data, own), Rejection::WrongTarget);
    assert_eq!(
        refused(&world, &data, Target::Stack(0)),
        Rejection::WrongTarget
    );
    // A door is no target of a light, which is the party's...
    let light = cast(&world, WIZARD, "base:spell:light", south.clone());
    assert_eq!(
        apply(&mut world.clone(), &data, light),
        Err(Rejection::WrongTarget)
    );
    // ...nor of a heal.
    let mut copy = world.clone();
    let heal = cast(&world, CLERIC, "base:spell:cure_wounds", south);
    assert_eq!(apply(&mut copy, &data, heal), Err(Rejection::WrongTarget));
    assert_eq!(copy, world);
}

/// An object is worked by its own interaction, so its requirements hold for a hand as for a
/// hand of flesh: the lever is pulled from two tiles off, and then the gate gives way.
#[test]
fn the_hand_works_an_object_by_its_own_interaction() {
    let data = objects();
    let mut world = at(&data, YARD, 0, 2, Facing::East);
    world.known.notice(yard(&data), "lever_1");
    let rows = listed(&world, &data);
    let lever = row(&rows, &object("lever_1")).expect("the lever, found");
    assert_eq!(
        (lever.distance, lever.sight, lever.refusal.as_ref()),
        (2, Sight::Clear, None)
    );
    let gate = row(&rows, &object("gate_1")).expect("the gate");
    assert_eq!((gate.distance, gate.sight), (3, Sight::Clear));
    assert!(
        gate.refusal.is_some(),
        "shut fast until the lever is pulled"
    );
    let events = send(&mut world, &data, object("lever_1"));
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::ObjectUsed {
                object,
                verb: omnis_data::Verb::Pull,
                success: true
            } if object == "lever_1"
        )),
        "{events:?}"
    );
    let state = |world: &World, id: &str| {
        let saved = world.maps.get(&yard(&data)).and_then(|s| s.objects.get(id));
        saved.and_then(|s| s.state.clone())
    };
    assert_eq!(state(&world, "lever_1").as_deref(), Some("down"));
    // The lever offers nothing more; the gate is free now, and the hand pushes it open.
    let rows = listed(&world, &data);
    assert!(row(&rows, &object("lever_1")).is_none());
    assert_eq!(row(&rows, &object("gate_1")).unwrap().refusal, None);
    send(&mut world, &data, object("gate_1"));
    assert_eq!(state(&world, "gate_1").as_deref(), Some("open"));
    // An object that offers a hand nothing is not listed and refuses as an object does.
    assert!(row(&rows, &object("stone_1")).is_none());
    assert_eq!(
        refused(&world, &data, object("stone_1")),
        Rejection::VerbRefused {
            reason: Refusal::NotOffered
        }
    );
}

/// What the party neither sees nor knows of is not there: a hidden object not yet found, an
/// object on another level, a door it has never seen. What it knows of and cannot see is listed
/// dim, out of sight; what it knows of beyond the range is not listed and is refused as too far.
#[test]
fn what_the_party_cannot_see_is_dim_if_known_and_absent_if_not() {
    let data = objects();
    let world = at(&data, YARD, 0, 2, Facing::East);
    let no_such = |id: &str| Rejection::NoSuchObject { object: id.into() };
    let rows = listed(&world, &data);
    assert!(
        row(&rows, &object("lever_1")).is_none(),
        "hidden, not found"
    );
    assert_eq!(
        refused(&world, &data, object("lever_1")),
        no_such("lever_1")
    );
    assert_eq!(
        refused(&world, &data, object("crate_2")),
        no_such("crate_2"),
        "on the loft"
    );
    assert_eq!(
        refused(&world, &data, object("nothing")),
        no_such("nothing")
    );

    // The chest, seen from beside it and then looked for from the far corner over the hedge:
    // known, out of sight. The fountain behind the party's back was never seen: not there.
    let mut world = at(&data, YARD, 3, 3, Facing::East);
    go(&mut world, &data, YARD, 0, 0, Facing::North);
    let rows = listed(&world, &data);
    let chest = row(&rows, &object("chest_1")).expect("a chest the party has seen");
    assert_eq!(
        (chest.distance, chest.sight, chest.refusal.as_ref()),
        (5, Sight::Obscured, Some(&Rejection::OutOfSight))
    );
    assert!(row(&rows, &object("fountain_1")).is_none());
    assert_eq!(
        refused(&world, &data, object("fountain_1")),
        no_such("fountain_1")
    );

    let data = common::data();
    // Seen from inside its room, then looked for from the next room west, through the rock.
    let mut world = at(&data, DUNGEON, 9, 4, Facing::South);
    let south = door(9, 5, Facing::South);
    go(&mut world, &data, DUNGEON, 3, 0, Facing::North);
    let rows = listed(&world, &data);
    let known = row(&rows, &south).expect("a door the party has seen");
    assert_eq!(
        (known.distance, known.sight, known.refusal.as_ref()),
        (6, Sight::Obscured, Some(&Rejection::OutOfSight))
    );
    assert_eq!(refused(&world, &data, south.clone()), Rejection::OutOfSight);
    // A door two rooms east that it has never seen: within six tiles of nothing, and unknown.
    let unseen = door(15, 5, Facing::South);
    go(&mut world, &data, DUNGEON, 11, 4, Facing::West);
    assert!(row(&listed(&world, &data), &unseen).is_none());
    assert_eq!(refused(&world, &data, unseen), Rejection::NoDoor);
    // The door it knows, from too far: not listed, and too far.
    go(&mut world, &data, DUNGEON, 9, 4, Facing::South);
    go(&mut world, &data, DUNGEON, 0, 0, Facing::North);
    assert!(row(&listed(&world, &data), &south).is_none());
    assert_eq!(refused(&world, &data, south), Rejection::BeyondRange);
}

/// The kinds of range, on one door: a touch is the door the party faces; a range that asks no
/// clear line reaches a known door through the rock; sight reaches as far as the party sees.
#[test]
fn each_kind_of_range_reaches_what_it_says() {
    let south = door(9, 5, Facing::South);
    // Touch: the faced door and nothing else.
    let data = with_hand(common::data(), Targeting::TOUCH);
    let mut world = at(&data, DUNGEON, 9, 5, Facing::South);
    let rows = listed(&world, &data);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        (
            rows[0].target.clone(),
            rows[0].distance,
            rows[0].refusal.clone()
        ),
        (south.clone(), 0, None)
    );
    go(&mut world, &data, DUNGEON, 9, 5, Facing::East);
    assert!(listed(&world, &data).is_empty(), "beside it, facing away");
    assert_eq!(
        refused(&world, &data, south.clone()),
        Rejection::BeyondRange
    );
    go(&mut world, &data, DUNGEON, 9, 4, Facing::South);
    assert!(listed(&world, &data).is_empty(), "a tile short of it");

    // Touch on objects: what Use reaches, the gate the party stands in and nothing two tiles off.
    let data = with_hand(objects(), Targeting::TOUCH);
    let world = at(&data, YARD, 3, 3, Facing::East);
    let touched: Vec<Target> = listed(&world, &data)
        .into_iter()
        .map(|r| r.target)
        .collect();
    assert_eq!(touched, [object("gate_1")]);

    // Six tiles with no line asked: the known door through the rock, obscured and free.
    let data = with_hand(common::data(), Targeting::of(Range::Tiles(6)));
    let mut world = at(&data, DUNGEON, 9, 4, Facing::South);
    go(&mut world, &data, DUNGEON, 3, 0, Facing::North);
    let rows = listed(&world, &data);
    let known = row(&rows, &south).expect("the door");
    assert_eq!(
        (known.sight, known.refusal.as_ref()),
        (Sight::Obscured, None)
    );
    send(&mut world, &data, south.clone());
    let dungeon = data.registry.maps.get(DUNGEON).unwrap();
    assert!(world.maps[&dungeon].door_open(Level::GROUND, 9, 5, Facing::South));

    // Sight: as far as the party sees with a clear line (six tiles in the dungeon).
    let data = with_hand(common::data(), Targeting::SIGHT);
    let mut world = at(&data, DUNGEON, 9, 0, Facing::North);
    let rows = listed(&world, &data);
    let seen = row(&rows, &south).expect("down the room");
    assert_eq!(
        (seen.distance, seen.sight, seen.refusal.as_ref()),
        (5, Sight::Clear, None)
    );
    // From the next room it was never seen from here on, so it is not there at all...
    go(&mut world, &data, DUNGEON, 3, 0, Facing::North);
    assert!(row(&listed(&world, &data), &south).is_none());
    // ...until the party has stood before it: then it is known, and out of sight.
    go(&mut world, &data, DUNGEON, 9, 4, Facing::South);
    go(&mut world, &data, DUNGEON, 3, 0, Facing::North);
    assert_eq!(
        row(&listed(&world, &data), &south).and_then(|r| r.refusal.clone()),
        Some(Rejection::OutOfSight)
    );
}
