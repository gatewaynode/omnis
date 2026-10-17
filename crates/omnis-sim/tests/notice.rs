//! Hidden objects on the yard fixture (M8.5 step 2a, item 13): a sharp eye in sight and near, a
//! search of the party's cell and the one it faces, and a reveal sensing auras.

use crate::common::six;
use crate::objects::{in_yard, objects, yard};
use omnis_core::{Facing, Rotation};
use omnis_data::{Ability, Data, School, SpellEffect, Utility};
use omnis_sim::event::Notice;
use omnis_sim::objects::Refusal;
use omnis_sim::{Command, Event, PartyCommand, Rejection, Target, World, apply};

/// One member at `(x, y)` facing `facing`, with Wisdom `wisdom`.
fn eyed(data: &Data, x: u16, y: u16, facing: Facing, wisdom: u8) -> World {
    let mut world = in_yard(data, 0, x, y, facing);
    world.party.members[0].scores[Ability::Wisdom.index()] = wisdom;
    world
}

fn noticed(events: &[Event]) -> Vec<(String, Notice)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Noticed { object, how } => Some((object.clone(), *how)),
            _ => None,
        })
        .collect()
}

fn found(world: &World, data: &Data, id: &str) -> bool {
    world
        .maps
        .get(&yard(data))
        .and_then(|s| s.objects.get(id))
        .is_some_and(|s| s.noticed)
}

fn sees(events: &[Event], cell: (u16, u16)) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::Visible { tiles } if tiles.iter().any(|t| (t.x, t.y) == cell)))
}

fn turn(world: &mut World, data: &Data, rotation: Rotation) -> Vec<Event> {
    apply(world, data, Command::Turn(rotation)).expect("a turn")
}

#[test]
fn a_sharp_eye_finds_the_lever_two_cells_off_in_sight() {
    let data = objects();
    let mut world = eyed(&data, 0, 2, Facing::South, 10);
    let eye = omnis_sim::omnis_rules::passive(
        &world.party.members[0],
        &data,
        omnis_data::Skill::Perception,
    );
    assert_eq!(eye, Ok(12), "the lever's difficulty exactly");
    let events = turn(&mut world, &data, Rotation::Around);
    assert!(sees(&events, (0, 0)));
    assert_eq!(noticed(&events), [("lever_1".to_owned(), Notice::Passive)]);
    assert!(found(&world, &data, "lever_1"));
    let again = turn(&mut world, &data, Rotation::Around);
    let back = turn(&mut world, &data, Rotation::Around);
    assert!(
        noticed(&again).is_empty() && noticed(&back).is_empty(),
        "found once"
    );
}

#[test]
fn a_dull_eye_a_far_one_and_a_turned_back_find_nothing() {
    let data = objects();
    let mut dull = eyed(&data, 0, 2, Facing::South, 1);
    assert!(noticed(&turn(&mut dull, &data, Rotation::Around)).is_empty());
    let mut away = eyed(&data, 0, 2, Facing::North, 18);
    let events = turn(&mut away, &data, Rotation::Around);
    assert!(
        !sees(&events, (0, 0)) && noticed(&events).is_empty(),
        "not in sight"
    );
    let mut far = eyed(&data, 3, 0, Facing::East, 18);
    far.position.x = 3;
    far.position.y = 1;
    let events = turn(&mut far, &data, Rotation::Around);
    assert!(
        sees(&events, (0, 0)),
        "in sight from three cells: {events:?}"
    );
    assert!(noticed(&events).is_empty(), "but three cells off");
}

#[test]
fn nothing_is_noticed_outside_exploring() {
    let data = objects();
    let mut world = eyed(&data, 0, 2, Facing::North, 18);
    world.settings.devtools = true;
    world.mode = omnis_sim::Mode::Encounter(omnis_sim::EncounterState {
        source: omnis_sim::EncounterSource::Random,
        stacks: Vec::new(),
        disposition: omnis_data::Disposition::Hostile,
        retreat: world.position,
    });
    let gold = Command::Dev(omnis_sim::DevCommand::SetGold { gold: 1 });
    let events = apply(&mut world, &data, gold).expect("a dev edit");
    assert!(noticed(&events).is_empty());
    assert!(!found(&world, &data, "lever_1"));
}

/// Search from (0, 1) facing the lever at (0, 0), Wisdom too low for a sharp eye.
fn search_the_lever(seed: u64) -> (World, Vec<Event>, Data) {
    let data = objects();
    let mut world = eyed(&data, 0, 1, Facing::North, 1);
    world.seed = seed;
    let events = apply(&mut world, &data, Command::Search).expect("exploring");
    (world, events, data)
}

#[test]
fn a_search_finds_the_lever_on_a_roll_of_its_difficulty() {
    let (mut hits, mut misses) = (0, 0);
    for seed in 0..40 {
        let (world, events, data) = search_the_lever(seed);
        let roll = events
            .iter()
            .find_map(|e| match e {
                Event::Searched { member, roll } => {
                    assert_eq!(*member, world.party.members[0].id);
                    Some(roll.total)
                }
                _ => None,
            })
            .expect("a search rolls");
        let time: u32 = events
            .iter()
            .filter_map(|e| match e {
                Event::TimeAdvanced { minutes, .. } => Some(*minutes),
                _ => None,
            })
            .sum();
        assert_eq!(time, 10, "a search takes ten minutes");
        let expected = if roll >= 12 {
            hits += 1;
            vec![("lever_1".to_owned(), Notice::Search)]
        } else {
            misses += 1;
            Vec::new()
        };
        assert_eq!(noticed(&events), expected, "seed {seed}, roll {roll}");
        assert_eq!(found(&world, &data, "lever_1"), roll >= 12);
    }
    assert!(hits > 0 && misses > 0, "{hits} found, {misses} not");
}

#[test]
fn a_search_with_nothing_hidden_still_rolls_and_takes_its_time() {
    let data = objects();
    let mut world = eyed(&data, 4, 3, Facing::East, 10);
    let before = world.party_clock().elapsed;
    let events = apply(&mut world, &data, Command::Search).expect("exploring");
    assert!(events.iter().any(|e| matches!(e, Event::Searched { .. })));
    assert!(noticed(&events).is_empty());
    assert_eq!(world.party_clock().elapsed, before + 10);
    assert!(
        world
            .rngs
            .contains_key(&omnis_core::StreamName::new("object")),
        "the roll moved the object stream on"
    );
}

#[test]
fn nobody_who_can_act_cannot_search() {
    let data = objects();
    let mut world = eyed(&data, 0, 1, Facing::North, 10);
    world.party.members[0].hp = 0;
    let before = world.clone();
    assert_eq!(
        apply(&mut world, &data, Command::Search),
        Err(Rejection::VerbRefused {
            reason: Refusal::NoneCanAct
        })
    );
    assert_eq!(world, before);
}

/// The yard at `(x, y)` facing `facing`, Brenna with a dull eye and Ilvara the wizard as made,
/// knowing `spell` as well; the events of her casting it on herself.
fn reveal(x: u16, y: u16, facing: Facing, spell: &str) -> (World, Vec<Event>, Data) {
    let data = objects();
    let mut world = eyed(&data, x, y, facing, 1);
    let wizard = six().swap_remove(2);
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Create(wizard)),
    )
    .expect("a wizard");
    let ilvara = world.party.members[1].id;
    let known = data.registry.spells.get(spell).expect("a reveal");
    if !world.party.members[1].known_spells.contains(&known) {
        world.party.members[1].known_spells.push(known);
    }
    let cast = Command::Cast {
        caster: ilvara,
        spell: spell.into(),
        target: Target::Member(ilvara),
    };
    let events = apply(&mut world, &data, cast).expect("cast");
    (world, events, data)
}

#[test]
fn a_reveal_finds_the_rune_by_its_aura_through_the_hedge_but_not_the_lever() {
    // Facing the hedge, the lever behind: no eye finds it, so only the spell is in play.
    let (world, events, data) = reveal(2, 1, Facing::South, "objects:spell:sense_magic");
    assert_eq!(
        noticed(&events),
        [("rune_1".to_owned(), Notice::Spell)],
        "three cells off, the spell's range exactly, unseen, with an aura; the lever has none"
    );
    assert!(!found(&world, &data, "lever_1"));
}

#[test]
fn detect_magic_is_the_base_pack_s_reveal_six_cells_deep() {
    let far = (6, 0, Facing::North);
    let (_, short, _) = reveal(far.0, far.1, far.2, "objects:spell:sense_magic");
    assert!(
        noticed(&short).is_empty(),
        "four cells: past the fixture's three"
    );
    let (world, events, data) = reveal(far.0, far.1, far.2, "base:spell:detect_magic");
    assert_eq!(noticed(&events), [("rune_1".to_owned(), Notice::Spell)]);
    let detect = data.registry.spells.get("base:spell:detect_magic");
    let spell = detect
        .and_then(|id| data.spells.get(&id))
        .expect("in the base pack");
    assert_eq!(
        spell.effect,
        Some(SpellEffect::Utility(Utility::Reveal {
            range_tiles: 6,
            minutes: 10
        })),
        "30 feet, 10 minutes"
    );
    assert!(spell.concentration && spell.ritual && spell.level == 1);
    assert_eq!(spell.school, School::Divination);
    assert!(
        world.party.members[1]
            .known_spells
            .iter()
            .any(|s| Some(*s) == detect),
        "a new wizard knows it from the class list"
    );
    assert_eq!(data.rules.value("notice_tiles"), Some(2));
    assert_eq!(data.rules.value("search_minutes"), Some(10));
}

#[test]
fn search_is_a_script_word() {
    let data = objects();
    let world = in_yard(&data, 0, 0, 1, Facing::North);
    let word = omnis_sim::Word::parse("search").expect("a word");
    assert_eq!(word.command(&world, &data), Some(Command::Search));
    assert_eq!(Command::Search.word(), "search");
}

// Lore (owner, 2026-10-16): text a search turns up in reach, from a cell or an object.

/// The lore a command's events told, as (text key, object).
fn told(events: &[Event]) -> Vec<(String, Option<String>)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Lore { key, object } => Some((key.clone(), object.clone())),
            _ => None,
        })
        .collect()
}

/// The search's roll among `events`.
fn rolled(events: &[Event]) -> i64 {
    events
        .iter()
        .find_map(|e| match e {
            Event::Searched { roll, .. } => Some(roll.total),
            _ => None,
        })
        .expect("a search rolls")
}

fn search(world: &mut World, data: &Data) -> Vec<Event> {
    apply(world, data, Command::Search).expect("exploring")
}

const BOOTPRINTS: &str = "objects:text:lore.bootprints";

#[test]
fn a_cell_s_lore_is_told_by_every_search_after_the_finds() {
    let data = objects();
    let mut world = eyed(&data, 2, 2, Facing::North, 1);
    for _ in 0..2 {
        let events = search(&mut world, &data);
        assert_eq!(told(&events), [(BOOTPRINTS.to_owned(), None)]);
        let lore = events.iter().position(|e| matches!(e, Event::Lore { .. }));
        let searched = events
            .iter()
            .position(|e| matches!(e, Event::Searched { .. }));
        let time = events
            .iter()
            .position(|e| matches!(e, Event::TimeAdvanced { .. }));
        assert!(
            searched < lore && lore < time,
            "after the roll, before the time"
        );
    }
    assert!(
        world
            .maps
            .get(&yard(&data))
            .is_none_or(|s| s.lore_told.is_empty()),
        "lore told every time is not remembered"
    );
}

#[test]
fn lore_in_the_faced_cell_is_told_and_none_through_a_wall() {
    let data = objects();
    let mut world = eyed(&data, 2, 1, Facing::South, 1);
    assert_eq!(
        told(&search(&mut world, &data)),
        [(BOOTPRINTS.to_owned(), None)]
    );
    // South of the hedge, facing the bootprints across it.
    let mut world = eyed(&data, 2, 3, Facing::North, 1);
    assert!(told(&search(&mut world, &data)).is_empty());
}

#[test]
fn lore_with_a_difficulty_is_told_on_a_roll_that_meets_it_and_once_only() {
    let data = objects();
    let carving = ("objects:text:lore.carving".to_owned(), None);
    let (mut hits, mut misses) = (0, 0);
    for seed in 0..40 {
        let mut world = eyed(&data, 4, 3, Facing::West, 1);
        world.seed = seed;
        let events = search(&mut world, &data);
        let met = rolled(&events) >= 10;
        assert_eq!(
            told(&events),
            if met {
                vec![carving.clone()]
            } else {
                Vec::new()
            },
            "seed {seed}"
        );
        let kept = |w: &World| {
            w.maps
                .get(&yard(&data))
                .is_some_and(|s| s.lore_told.contains("carving"))
        };
        assert_eq!(kept(&world), met);
        if !met {
            misses += 1;
            continue;
        }
        hits += 1;
        // Told once: never again, through a save and a load.
        let text = omnis_sim::ops::save_text(&world).expect("saves");
        let mut back = omnis_sim::ops::load_text(&text, &data, false).expect("loads");
        assert!(kept(&back));
        for _ in 0..3 {
            assert!(told(&search(&mut back, &data)).is_empty());
        }
    }
    assert!(hits > 0 && misses > 0, "{hits} told, {misses} not");
}

#[test]
fn an_object_s_lore_is_its_placement_s_over_its_kind_s() {
    let data = objects();
    // The crate's is its kind's, every time.
    let mut world = eyed(&data, 1, 2, Facing::North, 1);
    assert_eq!(
        told(&search(&mut world, &data)),
        [(
            "objects:text:crate.lore".to_owned(),
            Some("crate_1".to_owned())
        )]
    );
    // The statue's own, DC 12 and once, stands in for its kind's moss: a low roll tells nothing.
    let name = (
        "objects:text:statue.lore".to_owned(),
        Some("statue_1".to_owned()),
    );
    let (mut hits, mut misses) = (0, 0);
    for seed in 0..40 {
        let mut world = eyed(&data, 3, 1, Facing::North, 1);
        world.seed = seed;
        let events = search(&mut world, &data);
        if rolled(&events) >= 12 {
            hits += 1;
            assert_eq!(told(&events), std::slice::from_ref(&name));
            let state = &world.maps[&yard(&data)].objects["statue_1"];
            assert!(state.told);
            assert!(told(&search(&mut world, &data)).is_empty(), "once");
        } else {
            misses += 1;
            assert!(told(&events).is_empty(), "seed {seed}: not the kind's moss");
        }
    }
    assert!(hits > 0 && misses > 0, "{hits} told, {misses} not");
}

#[test]
fn a_hidden_object_s_lore_is_told_only_once_it_is_found() {
    let lore = (
        "objects:text:lever.lore".to_owned(),
        Some("lever_1".to_owned()),
    );
    let (mut hits, mut misses) = (0, 0);
    for seed in 0..40 {
        let (_, events, _) = search_the_lever(seed);
        if noticed(&events).is_empty() {
            misses += 1;
            assert!(told(&events).is_empty(), "seed {seed}");
        } else {
            hits += 1;
            assert_eq!(told(&events), std::slice::from_ref(&lore), "seed {seed}");
        }
    }
    assert!(hits > 0 && misses > 0);
}
