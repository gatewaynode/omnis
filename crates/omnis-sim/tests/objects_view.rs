//! What a client reads of objects (M8.5 step 2a, item 15): `objects.get` lists the objects in
//! reach with every verb they offer now and why each would be refused, and the viewport carries
//! the objects and piles in sight. Every verb the view lists is sent as `Act`: the command must
//! refuse it exactly as the view says, or take it when the view says nothing.

use crate::objects::{in_yard, objects, yard};
use omnis_core::{Facing, Level};
use omnis_data::{Ability, Data, Shape, Skill, Verb};
use omnis_sim::event::CheckKind;
use omnis_sim::objects::{ObjectState, Refusal};
use omnis_sim::objects_view::{
    CheckView, ItemCount, PileView, SeenObject, VerbView, objects_view, on_level,
};
use omnis_sim::{Command, ItemCommand, Rejection, World, apply, query};

fn set_state(world: &mut World, data: &Data, id: &str, state: &str) {
    world.map_state(yard(data)).objects.insert(
        id.into(),
        ObjectState {
            state: Some(state.into()),
            ..ObjectState::default()
        },
    );
}

fn lacks(item: &str) -> Option<Rejection> {
    Some(Rejection::VerbRefused {
        reason: Refusal::Lacks { item: item.into() },
    })
}

#[test]
fn the_shut_chest_offers_two_ways_open_and_says_why_one_is_refused() {
    let data = objects();
    let world = in_yard(&data, 0, 5, 4, Facing::North);
    let view = objects_view(&world, &data);
    assert_eq!(
        view.objects.len(),
        1,
        "the chest alone: the torch is beside"
    );
    let chest = &view.objects[0];
    assert_eq!(
        (
            chest.object.as_str(),
            chest.object_kind.as_str(),
            chest.name.as_str(),
            chest.state.as_str()
        ),
        (
            "chest_1",
            "objects:object:chest",
            "objects:text:chest.name",
            "closed"
        )
    );
    assert_eq!(chest.capacity, 8);
    assert_eq!(chest.contents, None, "shut: nothing seen inside");
    assert_eq!(
        chest.verbs,
        [
            VerbView {
                verb: Verb::Open,
                option: 0,
                label: Some("objects:text:chest.force".into()),
                checks: vec![CheckView {
                    kind: CheckKind::Skill(Skill::Athletics),
                    dc: 15
                }],
                minutes: 2,
                refusal: None,
            },
            VerbView {
                verb: Verb::Open,
                option: 1,
                label: Some("objects:text:chest.pick".into()),
                checks: vec![CheckView {
                    kind: CheckKind::Skill(Skill::SleightOfHand),
                    dc: 10
                }],
                minutes: 5,
                refusal: lacks("base:item:thieves_tools"),
            },
        ]
    );
    assert!(view.ground.is_empty());
}

#[test]
fn an_open_chest_shows_its_contents_and_a_spent_search_stays_listed_refused() {
    let data = objects();
    let mut world = in_yard(&data, 0, 5, 4, Facing::North);
    set_state(&mut world, &data, "chest_1", "open");
    let chest = objects_view(&world, &data).objects.remove(0);
    assert_eq!(
        chest.contents,
        Some(vec![ItemCount {
            item: "base:item:dagger".into(),
            count: 1
        }])
    );
    let verbs: Vec<(Verb, bool)> = chest
        .verbs
        .iter()
        .map(|v| (v.verb, v.refusal.is_none()))
        .collect();
    assert_eq!(verbs, [(Verb::Search, true), (Verb::Close, true)]);
    let search = Command::Act {
        object: "chest_1".into(),
        verb: Verb::Search,
        option: 0,
    };
    apply(&mut world, &data, search).expect("the gold");
    let chest = objects_view(&world, &data).objects.remove(0);
    assert_eq!(
        chest.verbs[0].refusal,
        Some(Rejection::VerbRefused {
            reason: Refusal::Spent
        })
    );
}

#[test]
fn the_fountain_s_wish_lists_its_ability_check_and_the_gem_it_lacks() {
    let data = objects();
    let world = in_yard(&data, 0, 1, 4, Facing::North);
    let view = objects_view(&world, &data);
    let fountain = view
        .objects
        .iter()
        .find(|o| o.object == "fountain_1")
        .expect("in reach");
    assert_eq!(fountain.capacity, 0);
    let wish = &fountain.verbs[1];
    assert_eq!(wish.verb, Verb::Use);
    assert_eq!(
        wish.checks,
        [CheckView {
            kind: CheckKind::Ability(Ability::Wisdom),
            dc: 12
        }]
    );
    assert_eq!(wish.minutes, 1, "the default minute");
    assert_eq!(wish.refusal, lacks("base:item:gem"));
}

#[test]
fn the_pile_underfoot_is_what_pick_up_takes_from() {
    let data = objects();
    let world = in_yard(&data, 0, 2, 2, Facing::North);
    assert_eq!(
        objects_view(&world, &data).ground,
        [ItemCount {
            item: "base:item:arrows".into(),
            count: 10
        }]
    );
}

/// Worlds across the yard: every cell, facing and level; the chest open; the lever pulled (the
/// gate's flag set) and the lever found; and an encounter under way.
fn worlds(data: &Data) -> Vec<World> {
    let mut worlds = Vec::new();
    for level in [0, 1] {
        for y in 0..5 {
            for x in 0..7 {
                for facing in [Facing::North, Facing::East, Facing::South, Facing::West] {
                    let mut world = in_yard(data, level, x, y, facing);
                    if data
                        .maps
                        .get(&yard(data))
                        .and_then(|m| m.cell(Level(level), x, y))
                        .is_none()
                    {
                        continue;
                    }
                    worlds.push(world.clone());
                    set_state(&mut world, data, "chest_1", "open");
                    let lever = data.registry.flags.get("objects:flag:lever").expect("flag");
                    world.flags.insert(lever, 1);
                    world
                        .map_state(yard(data))
                        .objects
                        .entry("lever_1".into())
                        .or_default()
                        .noticed = true;
                    worlds.push(world);
                }
            }
        }
    }
    let mut fighting = in_yard(data, 0, 5, 4, Facing::North);
    fighting.mode = omnis_sim::Mode::Encounter(omnis_sim::EncounterState {
        source: omnis_sim::EncounterSource::Random,
        stacks: Vec::new(),
        disposition: omnis_data::Disposition::Hostile,
        retreat: fighting.position,
    });
    worlds.push(fighting);
    worlds
}

#[test]
fn every_verb_the_view_lists_is_refused_or_taken_as_it_says() {
    let data = objects();
    let mut verbs = 0;
    let mut refused = 0;
    for world in worlds(&data) {
        for object in objects_view(&world, &data).objects {
            for verb in object.verbs {
                verbs += 1;
                let act = Command::Act {
                    object: object.object.clone(),
                    verb: verb.verb,
                    option: verb.option,
                };
                let at = world.position;
                let outcome = apply(&mut world.clone(), &data, act).err();
                assert_eq!(
                    outcome, verb.refusal,
                    "{} {:?} at {at}",
                    object.object, verb
                );
                refused += usize::from(outcome.is_some());
            }
        }
    }
    assert!(
        verbs > 40 && refused > 5 && refused < verbs,
        "{verbs} verbs, {refused} refused"
    );
}

#[test]
fn the_view_and_use_reach_the_same_objects() {
    let data = objects();
    let mut asked = 0;
    for world in worlds(&data) {
        let view = objects_view(&world, &data).objects;
        let mut used = world.clone();
        let Err(Rejection::ChooseVerb { choices }) = apply(&mut used, &data, Command::Interact)
        else {
            continue;
        };
        asked += 1;
        let reached: Vec<String> = choices
            .into_iter()
            .filter_map(|c| match c {
                omnis_sim::objects::Choice::Object { object, .. } => Some(object),
                omnis_sim::objects::Choice::Door => None,
            })
            .collect();
        let at = world.position;
        for object in &reached {
            assert!(view.iter().any(|o| &o.object == object), "{object} at {at}");
        }
        for object in view
            .iter()
            .filter(|o| o.verbs.iter().any(|v| v.refusal.is_none()))
        {
            assert!(
                reached.contains(&object.object),
                "{} at {at}",
                object.object
            );
        }
    }
    assert!(asked > 4, "{asked} choices asked");
}

#[test]
fn the_viewport_carries_the_objects_and_piles_in_sight() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 2, Facing::North);
    let viewport = query::viewport(&world, &data).expect("the yard");
    let seen: Vec<&str> = viewport.objects.iter().map(|o| o.object.as_str()).collect();
    assert!(seen.contains(&"stone_1"), "underfoot: {seen:?}");
    assert!(!seen.contains(&"lever_1"), "hidden");
    let stone = viewport
        .objects
        .iter()
        .find(|o| o.object == "stone_1")
        .expect("seen");
    assert_eq!(
        (stone.x, stone.y, stone.pos, stone.object_kind.as_str()),
        (0, 2, (2, -2), "objects:object:stone")
    );
    world
        .map_state(yard(&data))
        .objects
        .entry("lever_1".into())
        .or_default()
        .noticed = true;
    let viewport = query::viewport(&world, &data).expect("the yard");
    let lever = viewport
        .objects
        .iter()
        .find(|o| o.object == "lever_1")
        .expect("found, in sight");
    assert_eq!(lever.look.shape, Shape::Post);

    // From the west, not through the door on its north edge.
    let mut torch = in_yard(&data, 0, 5, 4, Facing::East);
    let open = |w: &World| {
        query::viewport(w, &data)
            .expect("vp")
            .objects
            .into_iter()
            .find(|o| o.object == "torch_1")
            .map(|o| o.state)
    };
    assert_eq!(open(&torch), Some("lit".to_owned()));
    set_state(&mut torch, &data, "torch_1", "out");
    assert_eq!(open(&torch), Some("out".to_owned()));

    let by_the_arrows = in_yard(&data, 0, 2, 1, Facing::South);
    let viewport = query::viewport(&by_the_arrows, &data).expect("the yard");
    assert_eq!(viewport.piles.len(), 1);
    let pile = &viewport.piles[0];
    assert_eq!((pile.x, pile.y, pile.pos), (2, 2, (1, 1)));
    assert_eq!(
        pile.items,
        [ItemCount {
            item: "base:item:arrows".into(),
            count: 10
        }]
    );
}

#[test]
fn a_viewport_with_no_objects_writes_no_new_keys() {
    let data = objects();
    let mut world = in_yard(&data, 0, 4, 4, Facing::South);
    world.position.level = Level(0);
    let viewport = query::viewport(&world, &data).expect("the yard");
    assert!(viewport.objects.is_empty() && viewport.piles.is_empty());
    let text = omnis_data::ron_io::to_string(&viewport).expect("ron");
    assert!(
        !text.contains("piles") && !text.contains("objects: [") && !text.contains("objects:["),
        "{text}"
    );
}

#[test]
fn a_placement_s_own_name_wins_over_its_kind_s() {
    let data = objects();
    let world = in_yard(&data, 0, 3, 1, Facing::North);
    let view = objects_view(&world, &data);
    let statue = view
        .objects
        .iter()
        .find(|o| o.object == "statue_1")
        .expect("in reach");
    assert_eq!(statue.name, "objects:text:statue.knight");
}

#[test]
fn the_viewport_lists_a_spanning_object_once_and_a_taken_one_never() {
    let data = objects();
    let gate = in_yard(&data, 0, 3, 1, Facing::South);
    let viewport = query::viewport(&gate, &data).expect("the yard");
    assert!(
        viewport.tiles.iter().any(|t| (t.x, t.y) == (3, 3)),
        "both of the gate's cells in sight"
    );
    let gates = viewport
        .objects
        .iter()
        .filter(|o| o.object == "gate_1")
        .count();
    assert_eq!(gates, 1);

    let mut stone = in_yard(&data, 0, 0, 2, Facing::North);
    let take = Command::Act {
        object: "stone_1".into(),
        verb: Verb::Take,
        option: 0,
    };
    apply(&mut stone, &data, take).expect("taken");
    let viewport = query::viewport(&stone, &data).expect("the yard");
    assert!(viewport.objects.iter().all(|o| o.object != "stone_1"));
}

#[test]
fn the_look_is_the_state_s_own() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 2, Facing::North);
    set_state(&mut world, &data, "lever_1", "down");
    world
        .map_state(yard(&data))
        .objects
        .get_mut("lever_1")
        .expect("set")
        .noticed = true;
    let viewport = query::viewport(&world, &data).expect("the yard");
    let lever = viewport
        .objects
        .iter()
        .find(|o| o.object == "lever_1")
        .expect("found, in sight");
    assert_eq!(
        (lever.state.as_str(), lever.look.depth, lever.look.height),
        ("down", 2, 2)
    );
}

fn ids(objects: &[SeenObject]) -> Vec<&str> {
    objects.iter().map(|o| o.object.as_str()).collect()
}

#[test]
fn a_level_lists_every_object_and_pile_wherever_the_party_stands() {
    let data = objects();
    let here = in_yard(&data, 0, 0, 0, Facing::North);
    let there = in_yard(&data, 1, 6, 0, Facing::South);
    for world in [&here, &there] {
        let (objects, piles) = on_level(world, &data, yard(&data), Level(0));
        assert_eq!(
            ids(&objects),
            [
                "crate_1",
                "statue_1",
                "stone_1",
                "plaque_1",
                "gate_1",
                "fountain_1",
                "chest_1",
                "torch_1"
            ],
            "file order; the lever and the rune hidden, the loft crate on level 1"
        );
        assert_eq!(
            piles,
            [PileView {
                x: 2,
                y: 2,
                pos: (1, 1),
                items: vec![ItemCount {
                    item: "base:item:arrows".into(),
                    count: 10
                }],
            }]
        );
        let (loft, piles) = on_level(world, &data, yard(&data), Level(1));
        assert_eq!(ids(&loft), ["crate_2"]);
        assert!(piles.is_empty());
    }
    let elsewhere = data.registry.maps.get("test:map:meadow").expect("meadow");
    assert_eq!(
        on_level(&here, &data, elsewhere, Level(0)),
        (vec![], vec![])
    );
}

#[test]
fn finding_taking_and_moving_items_change_what_a_level_lists() {
    let data = objects();
    let level = |w: &World| on_level(w, &data, yard(&data), Level(0));
    let mut world = in_yard(&data, 0, 0, 2, Facing::North);
    world
        .map_state(yard(&data))
        .objects
        .entry("lever_1".into())
        .or_default()
        .noticed = true;
    assert!(ids(&level(&world).0).contains(&"lever_1"), "found");
    let take = Command::Act {
        object: "stone_1".into(),
        verb: Verb::Take,
        option: 0,
    };
    apply(&mut world, &data, take).expect("taken");
    assert!(!ids(&level(&world).0).contains(&"stone_1"), "taken away");

    world.position.x = 2;
    let member = world.party.members[0].id;
    let item = |command| Command::Item(command);
    apply(
        &mut world,
        &data,
        item(ItemCommand::PickUp {
            member,
            item: "base:item:arrows".into(),
            count: 10,
        }),
    )
    .expect("picked up");
    assert!(level(&world).1.is_empty(), "the pile is gone");
    world.position.y = 3;
    world.position.facing = Facing::West;
    apply(
        &mut world,
        &data,
        item(ItemCommand::Drop {
            member,
            item: "base:item:arrows".into(),
            count: 4,
        }),
    )
    .expect("dropped");
    let piles = level(&world).1;
    assert_eq!(
        piles.iter().map(|p| (p.x, p.y, p.pos)).collect::<Vec<_>>(),
        [(2, 3, (-2, 0))],
        "two eighths ahead, to the west"
    );
}

#[test]
fn a_shut_gate_blocks_and_an_open_one_does_not() {
    let data = objects();
    let mut world = in_yard(&data, 0, 0, 0, Facing::North);
    let blocks = |w: &World, id: &str| {
        on_level(w, &data, yard(&data), Level(0))
            .0
            .into_iter()
            .find(|o| o.object == id)
            .map(|o| o.blocks)
    };
    assert_eq!(blocks(&world, "gate_1"), Some(true));
    assert_eq!(blocks(&world, "crate_1"), Some(true));
    assert_eq!(blocks(&world, "plaque_1"), Some(false));
    set_state(&mut world, &data, "gate_1", "open");
    assert_eq!(blocks(&world, "gate_1"), Some(false));
}

#[test]
fn what_the_viewport_shows_is_what_the_level_lists() {
    let data = objects();
    let mut compared = 0;
    for world in worlds(&data) {
        let Some(viewport) = query::viewport(&world, &data) else {
            continue;
        };
        let (all, _) = on_level(&world, &data, yard(&data), viewport.level);
        for seen in &viewport.objects {
            assert_eq!(all.iter().find(|o| o.object == seen.object), Some(seen));
            compared += 1;
        }
    }
    assert!(compared > 100, "{compared} objects compared");
}
