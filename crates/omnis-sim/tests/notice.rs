//! Hidden objects on the yard fixture (M8.5 step 2a, item 13): a sharp eye in sight and near, a
//! search of the party's cell and the one it faces, and a reveal sensing auras.

use crate::common::six;
use crate::objects::{in_yard, objects, yard};
use omnis_core::{Facing, Rotation};
use omnis_data::{Ability, Data};
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

#[test]
fn a_reveal_finds_the_rune_by_its_aura_through_the_hedge_but_not_the_lever() {
    let data = objects();
    // Facing the hedge, the lever behind: no eye finds it, so only the spell is in play.
    let mut world = eyed(&data, 2, 1, Facing::South, 1);
    let wizard = six().swap_remove(2);
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Create(wizard)),
    )
    .expect("a wizard");
    let ilvara = world.party.members[1].id;
    let spell = data
        .registry
        .spells
        .get("objects:spell:sense_magic")
        .expect("the fixture's reveal");
    world.party.members[1].known_spells.push(spell);
    let cast = Command::Cast {
        caster: ilvara,
        spell: "objects:spell:sense_magic".into(),
        target: Target::Member(ilvara),
    };
    let events = apply(&mut world, &data, cast).expect("cast");
    assert_eq!(
        noticed(&events),
        [("rune_1".to_owned(), Notice::Spell)],
        "three cells off, the spell's range exactly, unseen, with an aura; the lever has none"
    );
    assert!(!found(&world, &data, "lever_1"));
}

#[test]
fn search_is_a_script_word() {
    let data = objects();
    let world = in_yard(&data, 0, 0, 1, Facing::North);
    let word = omnis_sim::Word::parse("search").expect("a word");
    assert_eq!(word.command(&world, &data), Some(Command::Search));
    assert_eq!(Command::Search.word(), "search");
}
