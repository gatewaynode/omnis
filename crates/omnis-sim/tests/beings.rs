//! Visible beings (M8.5 step 2c): a fixed encounter that stands is seen, drawn as its front
//! monster's figure, met by stepping into its cell as before or by Use from the next cell, with
//! the retreat where the party stood; once beaten for good, it is gone.

use crate::common::{party_of, settle};
use crate::objects::{objects, yard};
use omnis_core::{Facing, Level, Position};
use omnis_data::{Data, Look, Shape};
use omnis_sim::api::{BeingView, Op, Reply, beings_on_level, dispatch, objects_view};
use omnis_sim::encounter::EncounterSource;
use omnis_sim::objects::Choice;
use omnis_sim::world::Mode;
use omnis_sim::{Command, Event, Rejection, Settings, World, apply, query};

/// Four of the six in the yard at `(x, y)`, facing `facing`.
fn four_at(data: &Data, x: u16, y: u16, facing: Facing) -> World {
    let mut world = World::new(data, 11, Settings::default()).expect("entry map");
    party_of(&mut world, data, 4);
    world.position = Position {
        map: yard(data),
        level: Level::GROUND,
        x,
        y,
        facing,
    };
    world
}

fn rats() -> BeingView {
    BeingView {
        being: "rat".into(),
        monster: "objects:monster:pack_rat".into(),
        count: 2,
        x: 5,
        y: 1,
        pos: (1, 0),
        facing: Facing::West,
        look: Look {
            shape: Shape::Figure,
            width: 3,
            depth: 3,
            height: 3,
        },
    }
}

fn seen(world: &World, data: &Data) -> Vec<BeingView> {
    query::viewport(world, data).expect("the yard").beings
}

#[test]
fn the_rats_stand_in_view_as_their_front_monster_s_figure() {
    let data = objects();
    let world = four_at(&data, 3, 1, Facing::East);
    assert_eq!(seen(&world, &data), [rats()]);
    let world = four_at(&data, 3, 1, Facing::West);
    assert!(seen(&world, &data).is_empty(), "behind the party");
    assert_eq!(
        beings_on_level(&world, &data, yard(&data), Level::GROUND),
        [rats()],
        "the level's, wherever the party looks"
    );
    assert!(beings_on_level(&world, &data, yard(&data), Level(1)).is_empty());
    // Every stack counts; the front one is drawn.
    let mut more = data.clone();
    let rat = more
        .registry
        .monsters
        .get("objects:monster:pack_rat")
        .expect("rat");
    let goblin = more
        .registry
        .monsters
        .get("base:monster:goblin")
        .expect("goblin");
    let stacks = &mut more.maps.get_mut(&yard(&data)).expect("yard").encounters[0].stacks;
    *stacks = vec![(rat, 2), (goblin, 3)];
    let all = beings_on_level(&world, &more, yard(&data), Level::GROUND);
    assert_eq!(
        (all[0].count, all[0].monster.as_str()),
        (5, "objects:monster:pack_rat")
    );
}

#[test]
fn use_from_the_next_cell_meets_them_and_the_retreat_is_where_the_party_stood() {
    let data = objects();
    let mut world = four_at(&data, 5, 2, Facing::North);
    let events = apply(&mut world, &data, Command::Interact).expect("use");
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::EncounterStarted {
                source: EncounterSource::Fixed(id),
                ..
            } if id == "rat"
        )),
        "{events:?}"
    );
    let Mode::Encounter(state) = &world.mode else {
        panic!("met: {:?}", world.mode);
    };
    let here = Position {
        map: yard(&data),
        level: Level::GROUND,
        x: 5,
        y: 2,
        facing: Facing::North,
    };
    assert_eq!(world.position, here, "the party does not move to meet them");
    assert_eq!(
        state.retreat,
        Position {
            facing: Facing::South,
            ..here
        }
    );
    assert!(seen(&world, &data).is_empty(), "not drawn while met");
}

#[test]
fn use_lists_the_rats_before_the_objects_in_reach() {
    let data = objects();
    // At (4, 1) facing East: the plaque on the edge to the south is in reach too.
    let mut world = four_at(&data, 4, 1, Facing::East);
    let Err(Rejection::ChooseVerb { choices }) = apply(&mut world, &data, Command::Interact) else {
        panic!("a choice");
    };
    assert_eq!(
        choices[0],
        Choice::Being {
            being: "rat".into()
        }
    );
    assert!(
        choices[1..]
            .iter()
            .all(|c| matches!(c, Choice::Object { .. }))
    );
    assert!(choices.len() > 1);
    // The choice's command.
    let events = apply(
        &mut world,
        &data,
        Command::Approach {
            being: "rat".into(),
        },
    )
    .expect("approach");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::EncounterStarted { .. }))
    );
}

#[test]
fn approach_names_a_standing_group_in_reach_only() {
    let data = objects();
    let approach = |world: &mut World, being: &str| {
        apply(
            world,
            &data,
            Command::Approach {
                being: being.into(),
            },
        )
    };
    let none = |being: &str| {
        Err(Rejection::NoSuchBeing {
            being: being.into(),
        })
    };
    let mut world = four_at(&data, 3, 1, Facing::East);
    assert_eq!(approach(&mut world, "rat"), none("rat"), "two cells off");
    let mut world = four_at(&data, 6, 1, Facing::West);
    assert_eq!(
        approach(&mut world, "crate_1"),
        none("crate_1"),
        "an object"
    );
    assert_eq!(approach(&mut world, "wolf"), none("wolf"));
    // On the ladder's top, a level up, facing over the rats' cell.
    let mut above = four_at(&data, 6, 1, Facing::West);
    above.position.level = Level(1);
    assert_eq!(approach(&mut above, "rat"), none("rat"), "another level");
    assert!(approach(&mut world, "rat").is_ok(), "from the east");
    assert_eq!(
        approach(&mut world, "rat"),
        Err(Rejection::WrongMode),
        "not while met"
    );
}

#[test]
fn beaten_once_they_are_gone_from_the_view_the_level_and_their_cell() {
    let data = objects();
    let mut world = four_at(&data, 5, 2, Facing::North);
    apply(&mut world, &data, Command::Interact).expect("use");
    let events = settle(&mut world, &data, &mut Vec::new());
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::CombatEnded {
                outcome: omnis_sim::CombatOutcome::Victory,
                ..
            }
        )),
        "{events:?}"
    );
    assert!(seen(&world, &data).is_empty());
    assert!(beings_on_level(&world, &data, yard(&data), Level::GROUND).is_empty());
    assert_eq!(
        apply(
            &mut world,
            &data,
            Command::Approach {
                being: "rat".into()
            }
        ),
        Err(Rejection::NoSuchBeing {
            being: "rat".into()
        })
    );
    let events = apply(
        &mut world,
        &data,
        Command::Step(omnis_core::Direction::Forward),
    )
    .expect("step");
    assert_eq!((world.position.x, world.position.y), (5, 1));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::EncounterStarted { .. })),
        "the cell is quiet"
    );
}

#[test]
fn stepping_into_their_cell_meets_them_as_an_unseen_group_is_met() {
    let data = objects();
    let mut world = four_at(&data, 4, 1, Facing::East);
    apply(
        &mut world,
        &data,
        Command::Step(omnis_core::Direction::Forward),
    )
    .expect("step");
    let Mode::Encounter(state) = &world.mode else {
        panic!("met: {:?}", world.mode);
    };
    assert_eq!(state.source, EncounterSource::Fixed("rat".into()));
    assert_eq!(
        (state.retreat.x, state.retreat.y),
        (4, 1),
        "the cell it came from"
    );
}

#[test]
fn objects_get_names_the_groups_standing_in_reach() {
    let data = objects();
    let world = four_at(&data, 4, 1, Facing::East);
    assert_eq!(objects_view(&world, &data).beings, [rats()]);
    let mut asked = world.clone();
    let Ok(Reply::Objects { beings, .. }) = dispatch(&mut asked, &data, &Op::ObjectsGet) else {
        panic!("objects");
    };
    assert_eq!(beings, [rats()], "over the op protocol too");
    let world = four_at(&data, 3, 1, Facing::East);
    assert!(
        objects_view(&world, &data).beings.is_empty(),
        "two cells off"
    );
}
