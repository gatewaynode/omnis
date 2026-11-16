//! Doors that break (PRD §8.3; `tasks/plans/combat-in-feet.md` step 9): a door of a map whose
//! doors have a durability is struck as an object is, and broken it stands open for good. In
//! the yard: the torch's door on the south edge of (6, 3) and the hall's gate on the east edge
//! of (6, 2), wood and Medium by the default make: armor class 15, 18 hit points.

use crate::objects::{objects, yard};
use crate::opening_strike::at;
use omnis_core::{Direction, Facing, Level};
use omnis_data::Data;
use omnis_sim::query::EdgeView;
use omnis_sim::{Chosen, Command, Event, Mode, Rejection, Target, World, apply, query, targets};

const FIGHTER: usize = 0;
const WIZARD: usize = 2;
const BOLT: &str = "base:spell:fire_bolt";
const HAND: &str = "base:spell:mage_hand";
/// The torch's door, named from the tile north of it.
const TORCH: (u16, u16, Facing) = (6, 3, Facing::South);
/// The hall's gate, named from the yard's side.
const GATE: (u16, u16, Facing) = (6, 2, Facing::East);

fn door((x, y, facing): (u16, u16, Facing)) -> Target {
    Target::Door { x, y, facing }
}

fn spell(world: &World, spell: &str, at: (u16, u16, Facing)) -> Command {
    Command::Cast {
        caster: world.party.members[WIZARD].id,
        spell: spell.into(),
        target: door(at),
    }
}

fn attack(world: &World, slot: usize, at: (u16, u16, Facing)) -> Command {
    Command::Attack {
        member: world.party.members[slot].id,
        target: door(at),
    }
}

/// The one blow at a door among the events: `(armor class, amount, left)`.
fn blow(events: &[Event]) -> (Option<i64>, u16, u16) {
    let mut blows = events.iter().filter_map(|e| match e {
        Event::DoorDamaged {
            attack,
            amount,
            left,
            ..
        } => Some((attack.as_ref().map(|a| a.ac), *amount, *left)),
        _ => None,
    });
    let one = blows.next().expect("a blow at a door");
    assert!(blows.next().is_none(), "one blow a command");
    one
}

fn broke(events: &[Event]) -> bool {
    events.iter().any(|e| matches!(e, Event::DoorBroken { .. }))
}

/// Strike with `command` until the door breaks.
fn until_broken(world: &mut World, data: &Data, command: &Command) {
    for _ in 0..200 {
        if broke(&apply(world, data, command.clone()).expect("a blow")) {
            return;
        }
    }
    panic!("the door never broke");
}

fn wear(world: &World, data: &Data, (x, y, facing): (u16, u16, Facing)) -> (u16, bool) {
    let wear = world
        .maps
        .get(&yard(data))
        .map(|s| s.door_wear(Level::GROUND, x, y, facing))
        .unwrap_or_default();
    (wear.damage, wear.broken)
}

fn rows(world: &World, data: &Data, chosen: &Chosen) -> Vec<Target> {
    let wizard = world.party.members[WIZARD].id;
    let rows = targets(world, data, wizard, chosen).unwrap();
    rows.into_iter().map(|r| r.target).collect()
}

fn the_bolt() -> Chosen {
    Chosen::Spell {
        spell: BOLT.into(),
        pay: omnis_sim::Pay::default(),
    }
}

#[test]
fn a_bolt_and_a_blade_wear_the_faced_door_down_and_the_blows_add_up() {
    let data = objects();
    let mut world = at(&data, 6, 3, Facing::South);
    let (mut lost, mut landed) = (0, 0);
    for turn in 0..12 {
        let minute = world.party_clock().elapsed;
        let command = if turn % 2 == 0 {
            spell(&world, BOLT, TORCH)
        } else {
            attack(&world, FIGHTER, TORCH)
        };
        let events = apply(&mut world, &data, command).expect("a blow");
        let (ac, amount, left) = blow(&events);
        assert_eq!(ac, Some(15), "wood");
        lost += amount;
        landed += u32::from(amount > 0);
        assert_eq!(left, 18u16.saturating_sub(lost), "the blows add up");
        assert_eq!(wear(&world, &data, TORCH), (lost.min(18), left == 0));
        assert_eq!(broke(&events), left == 0, "broken at none, not before");
        assert_eq!(world.party_clock().elapsed, minute + 1, "a minute");
        assert!(matches!(world.mode, Mode::Explore), "no fight starts");
        if left == 0 {
            break;
        }
    }
    assert!(landed > 0, "something landed in twelve blows");
    // Named from the far tile, it is the same door.
    let before = wear(&world, &data, TORCH);
    assert_eq!(wear(&world, &data, (6, 4, Facing::North)), before);
}

#[test]
fn a_broken_door_stands_open_for_good_and_is_nothing_to_open_shut_or_strike() {
    let data = objects();
    let mut world = at(&data, 6, 3, Facing::South);
    let step = Command::Step(Direction::Forward);
    let mut tried = world.clone();
    let _ = apply(&mut tried, &data, step.clone());
    assert_eq!(
        (tried.position.x, tried.position.y),
        (6, 3),
        "shut, it blocks"
    );
    let view = query::viewport(&world, &data).unwrap();
    assert_eq!(view.door, Some((15, 18)), "the map's doors, whole");
    let whole = EdgeView::Door {
        open: false,
        damage: 0,
        broken: false,
    };
    assert_eq!(view.tiles[0].front, whole);
    let command = attack(&world, FIGHTER, TORCH);
    until_broken(&mut world, &data, &command);
    assert_eq!(wear(&world, &data, TORCH), (18, true));
    let broken = EdgeView::Door {
        open: true,
        damage: 18,
        broken: true,
    };
    let view = query::viewport(&world, &data).unwrap();
    assert_eq!(view.tiles[0].front, broken);
    assert!(query::door_open(
        &world,
        yard(&data),
        Level::GROUND,
        6,
        4,
        Facing::North
    ));
    // Nothing to open, shut or strike, by hand, by spell or by the square's own action.
    assert_eq!(
        apply(&mut world, &data, Command::Door),
        Err(Rejection::Broken)
    );
    assert_eq!(apply(&mut world, &data, command), Err(Rejection::Broken));
    let bolt = spell(&world, BOLT, TORCH);
    assert_eq!(apply(&mut world, &data, bolt), Err(Rejection::Broken));
    let hand = spell(&world, HAND, TORCH);
    assert_eq!(apply(&mut world, &data, hand), Err(Rejection::Broken));
    let was = world.clone();
    // The square offers nothing: the door is no choice, so there is nothing to refuse.
    let events = apply(&mut world, &data, Command::Interact).expect("nothing here");
    assert!(
        !events.iter().any(|e| matches!(e, Event::Door { .. })),
        "{events:?}"
    );
    assert_eq!(wear(&world, &data, TORCH), wear(&was, &data, TORCH));
    for chosen in [
        the_bolt(),
        Chosen::Attack,
        Chosen::Spell {
            spell: HAND.into(),
            pay: omnis_sim::Pay::default(),
        },
    ] {
        let listed = rows(&world, &data, &chosen);
        assert!(!listed.contains(&door(TORCH)), "{chosen:?}: {listed:?}");
    }
    // And through it, from either side.
    apply(&mut world, &data, step).expect("through the broken door");
    assert_eq!((world.position.x, world.position.y), (6, 4));
    apply(&mut world, &data, Command::Turn(omnis_core::Rotation::Left)).unwrap();
    apply(&mut world, &data, Command::Turn(omnis_core::Rotation::Left)).unwrap();
    apply(&mut world, &data, Command::Step(Direction::Forward)).expect("and back");
    assert_eq!((world.position.x, world.position.y), (6, 3));
}

#[test]
fn whole_doors_are_listed_for_the_bolt_and_for_a_weapon() {
    let data = objects();
    let world = at(&data, 6, 3, Facing::South);
    for chosen in [the_bolt(), Chosen::Attack] {
        let wizard = world.party.members[WIZARD].id;
        let listed = targets(&world, &data, wizard, &chosen).unwrap();
        let row = listed.iter().find(|r| r.target == door(TORCH));
        let row = row.unwrap_or_else(|| panic!("{chosen:?}: {listed:?}"));
        assert_eq!((row.distance, &row.refusal), (0, &None), "{chosen:?}");
    }
}

#[test]
fn a_door_out_of_reach_needs_range_and_sight() {
    let data = objects();
    // Two tiles west of the torch's door, south of the hedge: in sight, out of reach.
    let far = at(&data, 4, 3, Facing::East);
    let results: Vec<Result<(), Rejection>> = (0..far.party.members.len())
        .map(|slot| {
            apply(&mut far.clone(), &data, attack(&far, slot, TORCH)).map(|events| {
                assert!(matches!(events[0], Event::DoorDamaged { .. }));
            })
        })
        .collect();
    assert_eq!(
        results[WIZARD],
        Err(Rejection::NeedsRangedWeapon),
        "{results:?}"
    );
    assert!(results.iter().any(Result::is_ok), "a bow: {results:?}");
    let bolt = spell(&far, BOLT, TORCH);
    let events = apply(&mut far.clone(), &data, bolt).expect("the bolt's range");
    assert_eq!(blow(&events).0, Some(15));
    // North of the hedge the door is on no line of sight: known or not, it is not struck.
    let mut hidden = at(&data, 6, 3, Facing::South);
    let teleport = omnis_sim::DevCommand::Teleport {
        map: "objects:map:yard".into(),
        level: Level::GROUND,
        x: 0,
        y: 0,
        facing: Facing::North,
    };
    apply(&mut hidden, &data, Command::Dev(teleport)).expect("the far corner");
    let bowman = results.iter().position(Result::is_ok).unwrap();
    let shot = attack(&hidden, bowman, TORCH);
    assert_eq!(apply(&mut hidden, &data, shot), Err(Rejection::OutOfSight));
    // No door on that edge at all.
    let none = attack(&far, FIGHTER, (4, 3, Facing::East));
    assert_eq!(apply(&mut far.clone(), &data, none), Err(Rejection::NoDoor));
}

#[test]
fn doors_with_no_durability_and_spells_that_strike_no_things_say_so() {
    let mut data = objects();
    let world = at(&data, 6, 3, Facing::South);
    // A spell that strikes creatures only.
    let id = data.registry.spells.get(BOLT).unwrap();
    let mut plain = data.clone();
    plain.spells.get_mut(&id).unwrap().objects = false;
    let bolt = spell(&world, BOLT, TORCH);
    assert_eq!(
        apply(&mut world.clone(), &plain, bolt.clone()),
        Err(Rejection::CannotBeDamaged)
    );
    assert!(!rows(&world, &plain, &the_bolt()).contains(&door(TORCH)));
    // A map whose doors cannot be damaged.
    let map = yard(&data);
    data.maps.get_mut(&map).unwrap().def.door_make.durability = None;
    assert_eq!(
        apply(&mut world.clone(), &data, bolt),
        Err(Rejection::CannotBeDamaged)
    );
    let strike = attack(&world, FIGHTER, TORCH);
    assert_eq!(
        apply(&mut world.clone(), &data, strike),
        Err(Rejection::CannotBeDamaged)
    );
    for chosen in [the_bolt(), Chosen::Attack] {
        let listed = rows(&world, &data, &chosen);
        assert!(
            !listed.iter().any(|t| matches!(t, Target::Door { .. })),
            "{chosen:?}: {listed:?}"
        );
    }
    assert_eq!(query::viewport(&world, &data).unwrap().door, None);
}

#[test]
fn the_hall_s_gate_broken_from_the_yard_lets_the_hall_be_seen() {
    let data = objects();
    let mut world = at(&data, 6, 2, Facing::East);
    let sees = |world: &World| {
        let view = query::viewport(world, &data).unwrap();
        view.tiles.iter().any(|t| (t.x, t.y) == (7, 2))
    };
    assert!(!sees(&world), "shut, the gate hides the hall");
    let command = attack(&world, FIGHTER, GATE);
    until_broken(&mut world, &data, &command);
    assert!(sees(&world), "seen through the broken gate");
}

#[test]
fn a_worn_and_a_broken_door_come_back_from_a_save_and_the_journal_as_they_were() {
    let data = objects();
    let mut world = at(&data, 6, 3, Facing::South);
    let command = attack(&world, FIGHTER, TORCH);
    while wear(&world, &data, TORCH).0 == 0 {
        apply(&mut world, &data, command.clone()).expect("a blow");
    }
    if !wear(&world, &data, TORCH).1 {
        crate::disk::round_trip(&world, &data, "a worn door");
        until_broken(&mut world, &data, &command);
    }
    crate::disk::round_trip(&world, &data, "a broken door");
    let text = query::map_text(&world, &data, yard(&data), Level::GROUND).unwrap();
    let shut = at(&data, 6, 3, Facing::South);
    let was = query::map_text(&shut, &data, yard(&data), Level::GROUND).unwrap();
    assert_ne!(text, was, "the map's text shows it open");
}
