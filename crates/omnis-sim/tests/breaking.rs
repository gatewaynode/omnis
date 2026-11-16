//! Things that break (PRD §8.3; `tasks/plans/combat-in-feet.md` step 8): an object whose kind
//! has a durability is struck from the road by a spell that may strike objects or by a weapon,
//! loses hit points and breaks; broken, it blocks and offers nothing, and what it held is
//! spilled, less the fragile items a destructive kind of damage destroys.

use crate::objects::{objects, yard};
use crate::opening_strike::at;
use omnis_core::{Direction, Facing};
use omnis_data::{DamageType, Data, SpellEffect, Verb};
use omnis_sim::api::objects_view;
use omnis_sim::event::ItemPlace;
use omnis_sim::{
    Chosen, Command, Event, Mode, Rejection, Target, World, apply, objects as sim_objects, query,
    targets,
};

const FIGHTER: usize = 0;
const WIZARD: usize = 2;
const BOLT: &str = "base:spell:fire_bolt";

fn thing(id: &str) -> Target {
    Target::Object { object: id.into() }
}

fn bolt(world: &World, id: &str) -> Command {
    Command::Cast {
        caster: world.party.members[WIZARD].id,
        spell: BOLT.into(),
        target: thing(id),
    }
}

fn attack(world: &World, slot: usize, id: &str) -> Command {
    Command::Attack {
        member: world.party.members[slot].id,
        target: thing(id),
    }
}

fn cast(world: &mut World, data: &Data, id: &str) -> Result<Vec<Event>, Rejection> {
    let command = bolt(world, id);
    apply(world, data, command)
}

fn strike(world: &mut World, data: &Data, slot: usize, id: &str) -> Result<Vec<Event>, Rejection> {
    let command = attack(world, slot, id);
    apply(world, data, command)
}

fn seeded(world: &World, seed: u64) -> World {
    let mut world = world.clone();
    world.seed = seed;
    world.rngs.clear();
    world
}

/// The one blow among the events: `(hit, armor class, damage dice, amount, left)`.
fn blow(events: &[Event], id: &str) -> (Option<bool>, Option<i64>, usize, u16, u16) {
    let mut blows = events.iter().filter_map(|e| match e {
        Event::ObjectDamaged {
            object,
            attack,
            rolls,
            raw,
            amount,
            left,
            ..
        } if object == id => {
            if let Some(attack) = attack {
                assert_eq!(attack.hit, *amount > 0, "{e:?}");
            }
            if *amount > 0 {
                assert_eq!(i64::from(*amount), *raw, "{e:?}");
            }
            Some((
                attack.as_ref().map(|a| a.hit),
                attack.as_ref().map(|a| a.ac),
                rolls.len(),
                *amount,
                *left,
            ))
        }
        _ => None,
    });
    let one = blows.next().expect("a blow");
    assert!(blows.next().is_none(), "one blow a command");
    one
}

fn broke(events: &[Event], id: &str) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::ObjectBroken { object } if object == id))
}

/// Strike `id` with `command` until it breaks; the events of the breaking blow.
fn until_broken(world: &mut World, data: &Data, id: &str, command: &Command) -> Vec<Event> {
    for _ in 0..200 {
        let events = apply(world, data, command.clone()).expect("a blow");
        if broke(&events, id) {
            return events;
        }
    }
    panic!("{id} never broke");
}

fn damage_of(world: &World, data: &Data, id: &str) -> u16 {
    sim_objects::saved(world, yard(data), id).map_or(0, |s| s.damage)
}

#[test]
fn a_fire_bolt_at_the_crate_rolls_against_its_armor_and_wears_it_down() {
    let data = objects();
    // Beside the crate at (1, 1): wood, Medium, so armor class 15 and 18 hit points.
    let start = at(&data, 2, 1, Facing::West);
    let (mut hits, mut misses) = (0, 0);
    for seed in 0..40 {
        let mut world = seeded(&start, seed);
        let minute = world.party_clock().elapsed;
        let events = cast(&mut world, &data, "crate_1").expect("a cast");
        let (hit, ac, dice, amount, left) = blow(&events, "crate_1");
        assert_eq!(ac, Some(15));
        assert_eq!(left, 18 - amount);
        assert_eq!(
            damage_of(&world, &data, "crate_1"),
            amount,
            "the save keeps it"
        );
        assert_eq!(world.party_clock().elapsed, minute + 1, "a minute");
        assert!(matches!(world.mode, Mode::Explore), "no fight starts");
        assert!(!broke(&events, "crate_1"), "one bolt is not enough");
        if hit == Some(true) {
            hits += 1;
            assert!(
                (1..=20).contains(&amount) && dice >= 1,
                "1d10, twice on a critical"
            );
        } else {
            misses += 1;
            assert_eq!((dice, amount), (0, 0));
            assert!(
                sim_objects::saved(&world, yard(&data), "crate_1").is_none(),
                "an unhurt thing saves nothing"
            );
        }
    }
    assert!(hits > 5 && misses > 2, "{hits} hits, {misses} misses");
}

#[test]
fn the_blows_add_up_and_at_no_hit_points_the_crate_breaks_for_good() {
    let data = objects();
    let mut world = at(&data, 2, 1, Facing::West);
    let step = Command::Step(Direction::Forward);
    let mut tried = world.clone();
    let _ = apply(&mut tried, &data, step.clone());
    assert_eq!(
        (tried.position.x, tried.position.y),
        (2, 1),
        "whole, it blocks"
    );
    let command = bolt(&world, "crate_1");
    let mut lost = 0;
    let last = loop {
        let events = apply(&mut world, &data, command.clone()).expect("a cast");
        let (.., amount, left) = blow(&events, "crate_1");
        lost += amount;
        assert_eq!(left, 18u16.saturating_sub(lost), "the blows add up");
        assert_eq!(
            broke(&events, "crate_1"),
            left == 0,
            "broken at none, not before"
        );
        if left == 0 {
            break events;
        }
    };
    assert!(lost >= 18);
    assert!(
        matches!(last.last(), Some(Event::ObjectBroken { .. })) || last.len() > 1,
        "{last:?}"
    );
    let saved = sim_objects::saved(&world, yard(&data), "crate_1").expect("saved");
    assert!(saved.broken);
    assert_eq!(
        (saved.damage, saved.state.as_deref()),
        (18, None),
        "a heap: no state"
    );
    // Broken, it is no target, offers nothing, and stands in nobody's way or sight.
    assert_eq!(apply(&mut world, &data, command), Err(Rejection::Broken));
    assert_eq!(
        strike(&mut world, &data, FIGHTER, "crate_1"),
        Err(Rejection::Broken)
    );
    let examine = Command::Act {
        object: "crate_1".into(),
        verb: Verb::Examine,
        option: 0,
    };
    assert_eq!(apply(&mut world, &data, examine), Err(Rejection::Broken));
    let view = objects_view(&world, &data);
    let seen = view.objects.iter().find(|o| o.object == "crate_1").unwrap();
    assert!(seen.broken && seen.verbs.is_empty());
    assert_eq!((seen.durability, seen.damage), (Some((15, 18)), 18));
    let drawn = query::viewport(&world, &data).unwrap().objects;
    let drawn = drawn.iter().find(|o| o.object == "crate_1").unwrap();
    assert!(drawn.broken && !drawn.blocks);
    let wizard = world.party.members[WIZARD].id;
    let chosen = Chosen::Spell {
        spell: BOLT.into(),
        pay: omnis_sim::Pay::default(),
    };
    let rows = targets(&world, &data, wizard, &chosen).unwrap();
    assert!(
        !rows.iter().any(|r| r.target == thing("crate_1")),
        "{rows:?}"
    );
    apply(&mut world, &data, step).expect("over the heap");
    assert_eq!((world.position.x, world.position.y), (1, 1));
}

#[test]
fn a_whole_crate_is_listed_for_the_bolt_and_for_a_weapon_and_a_statue_is_not() {
    let data = objects();
    let world = at(&data, 2, 1, Facing::West);
    let wizard = world.party.members[WIZARD].id;
    let chosen = Chosen::Spell {
        spell: BOLT.into(),
        pay: omnis_sim::Pay::default(),
    };
    for chosen in [chosen, Chosen::Attack] {
        let rows = targets(&world, &data, wizard, &chosen).unwrap();
        let row = rows.iter().find(|r| r.target == thing("crate_1"));
        let row = row.unwrap_or_else(|| panic!("{chosen:?}: {rows:?}"));
        assert_eq!((row.distance, &row.refusal), (1, &None));
        assert!(
            !rows.iter().any(|r| r.target == thing("statue_1")),
            "what cannot be damaged is not offered"
        );
    }
    // The object's view says how well it stands, whole.
    let view = objects_view(&world, &data);
    let seen = view.objects.iter().find(|o| o.object == "crate_1").unwrap();
    assert_eq!(
        (seen.durability, seen.damage, seen.broken),
        (Some((15, 18)), 0, false)
    );
}

#[test]
fn what_cannot_be_damaged_and_what_does_not_strike_things_say_so() {
    let data = objects();
    // Facing the statue at (3, 0) from the south: stone, with no durability.
    let mut world = at(&data, 3, 1, Facing::North);
    assert_eq!(
        cast(&mut world, &data, "statue_1"),
        Err(Rejection::CannotBeDamaged)
    );
    assert_eq!(
        strike(&mut world, &data, FIGHTER, "statue_1"),
        Err(Rejection::CannotBeDamaged)
    );
    let none = Rejection::NoSuchObject {
        object: "barrel_9".into(),
    };
    assert_eq!(cast(&mut world, &data, "barrel_9"), Err(none.clone()));
    assert_eq!(strike(&mut world, &data, FIGHTER, "barrel_9"), Err(none));
    // A spell that strikes creatures only is refused at a thing that could be damaged.
    let mut creatures = data.clone();
    let id = creatures.registry.spells.get(BOLT).unwrap();
    creatures.spells.get_mut(&id).unwrap().objects = false;
    let mut world = at(&creatures, 2, 1, Facing::West);
    assert_eq!(
        cast(&mut world, &creatures, "crate_1"),
        Err(Rejection::CannotBeDamaged)
    );
    assert_eq!(damage_of(&world, &creatures, "crate_1"), 0);
    let chosen = Chosen::Spell {
        spell: BOLT.into(),
        pay: omnis_sim::Pay::default(),
    };
    let wizard = world.party.members[WIZARD].id;
    let rows = targets(&world, &creatures, wizard, &chosen).unwrap();
    assert!(
        !rows
            .iter()
            .any(|r| matches!(r.target, Target::Object { .. }))
    );
}

/// The bolt remade: another effect, on the same spell.
fn remade(effect: impl Fn(omnis_core::Dice) -> SpellEffect) -> Data {
    let mut data = objects();
    let id = data.registry.spells.get(BOLT).unwrap();
    let spell = data.spells.get_mut(&id).unwrap();
    let Some(SpellEffect::Attack { dice, .. }) = spell.effect else {
        panic!("the bolt is an attack");
    };
    spell.effect = Some(effect(dice));
    data
}

#[test]
fn a_thing_makes_no_save_and_shrugs_off_poison_psychic_and_what_is_under_its_threshold() {
    // A save or a hit that cannot miss: no roll to hit, the dice in full, every time.
    let save = remade(|dice| SpellEffect::Save {
        ability: omnis_data::Ability::Constitution,
        dice,
        damage_type: DamageType::Thunder,
        half_on_save: true,
    });
    let auto = remade(|dice| SpellEffect::AutoHit {
        dice,
        damage_type: DamageType::Force,
    });
    for data in [&save, &auto] {
        for seed in 0..6 {
            let mut world = seeded(&at(data, 2, 1, Facing::West), seed);
            let events = cast(&mut world, data, "crate_1").expect("a cast");
            let (hit, _, dice, amount, left) = blow(&events, "crate_1");
            assert_eq!((hit, dice), (None, 1));
            assert!((1..=10).contains(&amount) && left == 18 - amount);
        }
    }
    // Poison and psychic do nothing to a thing, hit or not.
    for kind in [DamageType::Poison, DamageType::Psychic] {
        let data = remade(|dice| SpellEffect::AutoHit {
            dice,
            damage_type: kind,
        });
        let mut world = at(&data, 2, 1, Facing::West);
        let events = cast(&mut world, &data, "crate_1").expect("a cast");
        let dealt = events.iter().find_map(|e| match e {
            Event::ObjectDamaged {
                raw, amount, left, ..
            } => Some((*raw > 0, *amount, *left)),
            _ => None,
        });
        assert_eq!(dealt, Some((true, 0, 18)), "{kind:?}");
    }
    // A threshold of 6: a blow of 5 or less does nothing, one of 6 or more lands whole.
    let mut data = auto.clone();
    let kind = data.registry.objects.get("objects:object:crate").unwrap();
    let make = data.objects.get_mut(&kind).unwrap();
    make.attributes.durability.as_mut().unwrap().threshold = 6;
    let (mut under, mut over) = (0, 0);
    for seed in 0..30 {
        let mut world = seeded(&at(&data, 2, 1, Facing::West), seed);
        let events = cast(&mut world, &data, "crate_1").expect("a cast");
        let Some(Event::ObjectDamaged { raw, amount, .. }) = events
            .iter()
            .find(|e| matches!(e, Event::ObjectDamaged { .. }))
        else {
            panic!("a blow");
        };
        if *raw < 6 {
            under += 1;
            assert_eq!(*amount, 0, "{raw}");
        } else {
            over += 1;
            assert_eq!(i64::from(*amount), *raw);
        }
    }
    assert!(under > 3 && over > 3, "{under} under, {over} over");
}

#[test]
fn a_weapon_strikes_a_thing_it_reaches_and_a_bow_one_it_sees() {
    let data = objects();
    // Beside the crate: the fighter's own weapon, at no disadvantage.
    let start = at(&data, 2, 1, Facing::West);
    let mut landed = 0;
    for seed in 0..12 {
        let mut world = seeded(&start, seed);
        let minute = world.party_clock().elapsed;
        let command = attack(&world, FIGHTER, "crate_1");
        let events = apply(&mut world, &data, command).expect("a blow");
        let Some(Event::ObjectDamaged {
            by, attack, amount, ..
        }) = events.first()
        else {
            panic!("{events:?}");
        };
        let roll = attack.as_ref().expect("a weapon rolls to hit");
        assert_eq!((*by, roll.ac), (world.party.members[FIGHTER].id, 15));
        assert_eq!(roll.roll.trace.rolls.len(), 1, "one d20: {roll:?}");
        assert_eq!(world.party_clock().elapsed, minute + 1);
        assert!(matches!(world.mode, Mode::Explore));
        landed += u32::from(*amount > 0);
    }
    assert!(landed > 2, "{landed}");
    // Three tiles off, at (4, 1) facing west: out of the party's reach, in its sight.
    let far = at(&data, 4, 1, Facing::West);
    let results: Vec<Result<(), Rejection>> = (0..far.party.members.len())
        .map(|slot| {
            apply(&mut far.clone(), &data, attack(&far, slot, "crate_1")).map(|events| {
                assert!(matches!(events[0], Event::ObjectDamaged { .. }));
            })
        })
        .collect();
    assert_eq!(
        results[WIZARD],
        Err(Rejection::NeedsRangedWeapon),
        "{results:?}"
    );
    assert!(
        results.iter().any(Result::is_ok),
        "someone has a bow: {results:?}"
    );
    // South of the hedge, at (0, 4): the crate is on the party's map but out of its sight.
    let mut hidden = at(&data, 2, 1, Facing::West);
    let teleport = omnis_sim::DevCommand::Teleport {
        map: "objects:map:yard".into(),
        level: omnis_core::Level::GROUND,
        x: 0,
        y: 4,
        facing: Facing::South,
    };
    apply(&mut hidden, &data, Command::Dev(teleport)).expect("south of the hedge");
    let bowman = results.iter().position(Result::is_ok).unwrap();
    assert_eq!(
        strike(&mut hidden, &data, bowman, "crate_1"),
        Err(Rejection::OutOfSight)
    );
}

/// The yard with a potion of healing beside the dagger in the chest.
fn with_a_potion() -> Data {
    let mut data = objects();
    let potion = data
        .registry
        .items
        .get("base:item:potion_of_healing")
        .unwrap();
    let yard = yard(&data);
    let map = data.maps.get_mut(&yard).unwrap();
    let chest = map.objects.iter_mut().find(|p| p.id == "chest_1").unwrap();
    chest.contents.push((potion, 2));
    data
}

fn items(events: &[Event]) -> Vec<(String, u16, bool)> {
    let from = ItemPlace::Object("chest_1".into());
    let found = events.iter().filter_map(|e| match e {
        Event::ItemMoved {
            item,
            count,
            from: f,
            to: ItemPlace::Ground,
        } if *f == from => Some((item.clone(), *count, false)),
        Event::ItemDestroyed {
            item,
            count,
            from: f,
        } if *f == from => Some((item.clone(), *count, true)),
        _ => None,
    });
    found.collect()
}

#[test]
fn a_smashed_chest_shows_its_broken_state_and_spills_what_it_held_less_the_fragile() {
    let data = with_a_potion();
    // South of the chest at (5, 3), facing it: Small and wood, armor class 15, 10 hit points.
    let mut world = at(&data, 5, 4, Facing::North);
    let view = objects_view(&world, &data);
    let chest = view.objects.iter().find(|o| o.object == "chest_1").unwrap();
    assert_eq!(chest.durability, Some((15, 10)));
    // Fire destroys the potions and leaves the dagger on the ground.
    let command = bolt(&world, "chest_1");
    let events = until_broken(&mut world, &data, "chest_1", &command);
    assert_eq!(
        items(&events),
        [
            ("base:item:dagger".to_owned(), 1, false),
            ("base:item:potion_of_healing".to_owned(), 2, true)
        ]
    );
    let saved = sim_objects::saved(&world, yard(&data), "chest_1").expect("saved");
    assert_eq!(
        saved.state.as_deref(),
        Some("smashed"),
        "its kind's broken state"
    );
    assert_eq!(saved.contents.as_deref(), Some(&[][..]), "emptied");
    let ground = objects_view(&world, &data).ground;
    assert_eq!(ground.len(), 1, "{ground:?}");
    assert_eq!(
        (ground[0].item.as_str(), ground[0].count),
        ("base:item:dagger", 1)
    );
    let take = Command::Item(omnis_sim::ItemCommand::TakeOut {
        member: world.party.members[FIGHTER].id,
        object: "chest_1".into(),
        item: "base:item:dagger".into(),
        count: 1,
    });
    assert_eq!(apply(&mut world, &data, take), Err(Rejection::Broken));
    // A blade breaks the chest and spills everything: its damage destroys nothing fragile.
    let mut world = at(&data, 5, 4, Facing::North);
    let command = attack(&world, FIGHTER, "chest_1");
    let events = until_broken(&mut world, &data, "chest_1", &command);
    let kind = events.iter().find_map(|e| match e {
        Event::ObjectDamaged { kind, .. } => Some(*kind),
        _ => None,
    });
    assert!(
        matches!(kind, Some(DamageType::Slashing | DamageType::Piercing)),
        "{kind:?}"
    );
    assert_eq!(
        items(&events),
        [
            ("base:item:dagger".to_owned(), 1, false),
            ("base:item:potion_of_healing".to_owned(), 2, false)
        ]
    );
}

#[test]
fn a_damaged_and_a_broken_thing_come_back_from_a_save_as_they_were() {
    let data = with_a_potion();
    let mut world = at(&data, 5, 4, Facing::North);
    // One blow that lands, then the save; then broken, spilled, and the save again.
    while damage_of(&world, &data, "chest_1") == 0 {
        cast(&mut world, &data, "chest_1").expect("a cast");
    }
    if !sim_objects::broken(&world, yard(&data), world_chest(&data)) {
        crate::disk::round_trip(&world, &data, "a damaged chest");
    }
    let command = attack(&world, FIGHTER, "chest_1");
    if !sim_objects::broken(&world, yard(&data), world_chest(&data)) {
        until_broken(&mut world, &data, "chest_1", &command);
    }
    crate::disk::round_trip(&world, &data, "a broken chest");
}

fn world_chest(data: &Data) -> &omnis_data::placement::ResolvedPlacement {
    let map = &data.maps[&yard(data)];
    map.objects.iter().find(|p| p.id == "chest_1").unwrap()
}

/// The attack roll of the one blow among the events.
fn attack_roll(events: &[Event]) -> omnis_rules::AttackRoll {
    let found = events.iter().find_map(|e| match e {
        Event::ObjectDamaged { attack, .. } => attack.clone(),
        _ => None,
    });
    found.expect("a roll to hit")
}

#[test]
fn reach_range_and_the_dice_hold_for_a_thing_as_for_a_group() {
    // Tiles of 30 ft: the crate three tiles off is 90 ft away, past a bow's normal range.
    let mut wide = objects();
    let id = yard(&wide);
    wide.maps.get_mut(&id).unwrap().def.tile_feet = Some(30);
    let far = at(&wide, 4, 1, Facing::West);
    let bowman = (0..far.party.members.len())
        .find(|slot| strike(&mut far.clone(), &wide, *slot, "crate_1").is_ok())
        .expect("someone has a bow");
    let events = strike(&mut far.clone(), &wide, bowman, "crate_1").unwrap();
    assert_eq!(
        attack_roll(&events).roll.mode,
        omnis_rules::RollMode::Disadvantage,
        "past normal range"
    );
    let near = at(&objects(), 4, 1, Facing::West);
    let events = strike(&mut near.clone(), &objects(), bowman, "crate_1").unwrap();
    assert_eq!(
        attack_roll(&events).roll.mode,
        omnis_rules::RollMode::Normal
    );
    // A thing in the party's reach is in a melee weapon's, however wide the tiles: the wizard,
    // who has no bow, strikes the crate beside the party.
    let mut beside = at(&wide, 2, 1, Facing::West);
    let events = strike(&mut beside, &wide, WIZARD, "crate_1").expect("in reach");
    let first = attack_roll(&events);
    assert_eq!(first.roll.mode, omnis_rules::RollMode::Normal);
    // The dice move on: the next blow is another roll of the same stream.
    let events = strike(&mut beside, &wide, WIZARD, "crate_1").expect("again");
    let second = attack_roll(&events);
    assert_eq!(first.roll.trace.stream, second.roll.trace.stream);
    assert!(
        second.roll.trace.rolls[0].draw > first.roll.trace.rolls[0].draw,
        "{first:?} then {second:?}"
    );
    // A spell reaches no further at a thing than at a group: the bolt cut to one tile.
    let mut short = objects();
    let bolt = short.registry.spells.get(BOLT).unwrap();
    short.spells.get_mut(&bolt).unwrap().targeting = Some(omnis_data::Targeting {
        range: omnis_data::Range::Tiles(1),
        sees: true,
    });
    let mut world = at(&short, 4, 1, Facing::West);
    assert_eq!(
        cast(&mut world, &short, "crate_1"),
        Err(Rejection::BeyondRange)
    );
    let mut world = at(&short, 2, 1, Facing::West);
    assert!(cast(&mut world, &short, "crate_1").is_ok());
}
