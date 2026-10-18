//! Use (`Command::Interact`) on the yard fixture (M8.5 step 2a, item 10): one thing in reach is
//! done at once, nothing is "nothing here", and several are refused with the list to choose from.

use crate::objects::{in_yard, objects, yard};
use omnis_core::Facing;
use omnis_data::{Data, Verb};
use omnis_sim::objects::{Choice, ObjectState, Refusal};
use omnis_sim::{Command, Event, MessageKey, Rejection, World, apply};

fn object(object: &str, verb: Verb, option: u8) -> Choice {
    Choice::Object {
        object: object.into(),
        verb,
        option,
    }
}

fn set(world: &mut World, data: &Data, id: &str, state: ObjectState) {
    world.map_state(yard(data)).objects.insert(id.into(), state);
}

fn door_toggled(events: &[Event]) -> Option<bool> {
    events.iter().find_map(|e| match e {
        Event::Door { open, .. } => Some(*open),
        _ => None,
    })
}

#[test]
fn one_interaction_in_reach_is_done_at_once() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 1, Facing::East);
    let mut acted = world.clone();
    let used = apply(&mut world, &data, Command::Interact).expect("explore");
    let direct = apply(
        &mut acted,
        &data,
        Command::Act {
            object: "crate_1".into(),
            verb: Verb::Examine,
            option: 0,
        },
    )
    .expect("in reach");
    assert_eq!(used, direct, "Use on the lone crate is Examine on it");
    acted.journal.clone_from(&world.journal);
    assert_eq!(world, acted, "the same world, reached by another command");
}

#[test]
fn several_interactions_are_refused_with_the_choices_and_change_nothing() {
    let data = objects();
    let mut world = in_yard(&data, 0, 1, 3, Facing::North);
    let before = world.clone();
    assert_eq!(
        apply(&mut world, &data, Command::Interact),
        Err(Rejection::ChooseVerb {
            choices: vec![
                object("fountain_1", Verb::Drink, 0),
                object("fountain_1", Verb::Use, 0),
            ]
        })
    );
    assert_eq!(world, before);
}

#[test]
fn nothing_in_reach_is_nothing_here_and_a_hidden_object_is_not_offered() {
    let data = objects();
    let mut world = in_yard(&data, 0, 2, 3, Facing::South);
    let events = apply(&mut world, &data, Command::Interact).expect("explore");
    assert!(events.contains(&Event::Message {
        key: MessageKey::NothingHere
    }));
    assert!(!events.iter().any(|e| matches!(e, Event::ObjectUsed { .. })));
    world.known.notice(yard(&data), "rune_1");
    let found = apply(&mut world, &data, Command::Interact);
    assert!(
        matches!(found, Err(Rejection::VerbRefused { .. })),
        "the found rune is the one choice, and its refusal says why: {found:?}"
    );
}

#[test]
fn a_spent_once_is_not_a_choice() {
    let data = objects();
    let mut world = in_yard(&data, 0, 5, 3, Facing::West);
    assert_eq!(
        apply(&mut world, &data, Command::Interact),
        Err(Rejection::ChooseVerb {
            choices: vec![
                object("chest_1", Verb::Open, 0),
                object("chest_1", Verb::Open, 1),
            ]
        }),
        "two ways to open the shut chest, counted per verb"
    );
    let open = ObjectState {
        state: Some("open".into()),
        ..ObjectState::default()
    };
    set(&mut world, &data, "chest_1", open.clone());
    assert_eq!(
        apply(&mut world, &data, Command::Interact),
        Err(Rejection::ChooseVerb {
            choices: vec![
                object("chest_1", Verb::Search, 0),
                object("chest_1", Verb::Close, 0),
            ]
        })
    );
    let searched = ObjectState {
        used: [2].into(),
        ..open
    };
    set(&mut world, &data, "chest_1", searched);
    let events = apply(&mut world, &data, Command::Interact).expect("only Close is left");
    assert!(events.contains(&Event::ObjectState {
        object: "chest_1".into(),
        state: "closed".into(),
    }));
}

#[test]
fn a_door_with_an_object_in_reach_is_a_choice_and_door_opens_it() {
    let data = objects();
    let mut world = in_yard(&data, 0, 6, 4, Facing::North);
    let before = world.clone();
    assert_eq!(
        apply(&mut world, &data, Command::Interact),
        Err(Rejection::ChooseVerb {
            choices: vec![Choice::Door, object("torch_1", Verb::Extinguish, 0)]
        })
    );
    assert_eq!(world, before);
    let events = apply(&mut world, &data, Command::Door).expect("a door north");
    assert_eq!(door_toggled(&events), Some(true));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TimeAdvanced { minutes: 1, .. })),
        "a door costs what Use on it costs"
    );
}

#[test]
fn a_door_alone_is_used_at_once_and_a_shut_one_hides_what_is_behind() {
    let data = objects();
    let mut world = in_yard(&data, 0, 6, 3, Facing::South);
    let events = apply(&mut world, &data, Command::Interact).expect("explore");
    assert_eq!(door_toggled(&events), Some(true), "the torch is behind it");
    assert_eq!(
        apply(&mut world, &data, Command::Interact),
        Err(Rejection::ChooseVerb {
            choices: vec![Choice::Door, object("torch_1", Verb::Extinguish, 0)]
        }),
        "the open door lets the torch in reach"
    );
    let events = apply(&mut world, &data, Command::Door).expect("the door");
    assert_eq!(door_toggled(&events), Some(false));
}

#[test]
fn door_without_a_door_is_refused() {
    let data = objects();
    let mut world = in_yard(&data, 0, 1, 1, Facing::West);
    let before = world.clone();
    assert_eq!(
        apply(&mut world, &data, Command::Door),
        Err(Rejection::NoDoor)
    );
    assert_eq!(world, before);
}

#[test]
fn door_is_a_script_word() {
    let data = objects();
    let world = in_yard(&data, 0, 6, 4, Facing::North);
    let word = omnis_sim::Word::parse("door").expect("a word");
    assert_eq!(word.command(&world, &data), Some(Command::Door));
    assert_eq!(Command::Door.word(), "door");
}

#[test]
fn an_object_in_both_cells_in_reach_is_one_choice() {
    let data = objects();
    let mut world = in_yard(&data, 0, 3, 2, Facing::South);
    assert_eq!(
        apply(&mut world, &data, Command::Interact),
        Err(Rejection::VerbRefused {
            reason: Refusal::Flag {
                flag: "objects:flag:lever".into()
            }
        }),
        "the gate stands in both cells and is pushed once; the lever is not yet pulled"
    );
}
