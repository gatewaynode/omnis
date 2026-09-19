//! Dev commands: the devtools gate, each explore-mode edit and its rejections, the echo event,
//! and that a dev world says so in its save and its replays.

mod common;

use common::{data, party_of, world};
use omnis_core::{Direction, Facing, Position};
use omnis_data::Ability;
use omnis_sim::items::count_of;
use omnis_sim::omnis_rules::{DeathSaves, armor_class};
use omnis_sim::world::layer;
use omnis_sim::{
    Command, DevCommand, EncounterChoice, Event, Mode, Rejection, Settings, World, apply, query,
    replay,
};

fn dev_world(data: &omnis_data::Data) -> World {
    let settings = Settings {
        devtools: true,
        ..Settings::default()
    };
    World::new(data, 0x0123_4567_89ab_cdef, settings).unwrap()
}

fn every_edit() -> Vec<DevCommand> {
    vec![
        DevCommand::GiveItem {
            member: Some(0),
            item: "base:item:spyglass".into(),
            count: 1,
        },
        DevCommand::SetHp { member: 0, hp: 1 },
        DevCommand::SetSpellPoints {
            member: 0,
            points: 1,
        },
        DevCommand::SetGold { gold: 5 },
        DevCommand::SetFood { food: 5 },
        DevCommand::SetXp { member: 0, xp: 5 },
        DevCommand::SetScore {
            member: 0,
            ability: Ability::Strength,
            score: 5,
        },
        DevCommand::SetCondition {
            member: 0,
            condition: "base:condition:poisoned".into(),
            applied: true,
        },
        DevCommand::SetFlag {
            flag: "test:flag:x".into(),
            value: 1,
        },
        DevCommand::Teleport {
            map: "test:map:dungeon".into(),
            x: 3,
            y: 8,
            facing: Facing::South,
        },
    ]
}

#[test]
fn dev_commands_need_the_devtools_setting() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 1);
    let before = world.clone();
    for edit in every_edit() {
        assert_eq!(
            apply(&mut world, &data, Command::Dev(edit.clone())),
            Err(Rejection::DevOnly),
            "{edit:?}"
        );
    }
    assert_eq!(
        world, before,
        "a refused edit changes nothing, not even the turn"
    );
    assert_eq!(Command::Dev(every_edit().remove(0)).word(), "dev");
    assert_eq!(
        Rejection::DevOnly.to_string(),
        "dev commands need a devtools world"
    );
}

#[test]
fn items_go_to_a_member_or_the_stores() {
    let data = data();
    let mut world = dev_world(&data);
    party_of(&mut world, &data, 2);
    let glass = data.registry.items.get("base:item:spyglass").unwrap();
    let give = |member, item: &str, count| {
        Command::Dev(DevCommand::GiveItem {
            member,
            item: item.into(),
            count,
        })
    };
    let turn = world.turn;
    let events = apply(&mut world, &data, give(Some(1), "base:item:spyglass", 2)).unwrap();
    assert!(matches!(
        &events[0],
        Event::Dev {
            command: DevCommand::GiveItem { count: 2, .. }
        }
    ));
    assert_eq!(count_of(&world.party.members[1].equipment, glass), 2);
    apply(&mut world, &data, give(Some(1), "base:item:spyglass", 1)).unwrap();
    assert_eq!(
        count_of(&world.party.members[1].equipment, glass),
        3,
        "counts merge"
    );
    apply(&mut world, &data, give(None, "base:item:gem", 4)).unwrap();
    let gem = data.registry.items.get("base:item:gem").unwrap();
    assert_eq!(world.party.inventory, [(gem, 4)]);
    assert_eq!(world.turn, turn + 3, "every accepted edit is a turn");
    let before = world.clone();
    assert_eq!(
        apply(&mut world, &data, give(Some(0), "base:item:spyglass", 0)),
        Err(Rejection::OutOfRange)
    );
    assert_eq!(
        apply(&mut world, &data, give(Some(0), "base:item:nope", 1)),
        Err(Rejection::UnknownId {
            id: "base:item:nope".into()
        })
    );
    assert_eq!(
        apply(&mut world, &data, give(Some(9), "base:item:gem", 1)),
        Err(Rejection::NoSuchMember { index: 9 })
    );
    assert_eq!(world, before);
}

#[test]
fn hit_points_clamp_and_move_a_member_down_and_back_up() {
    let data = data();
    let mut world = dev_world(&data);
    party_of(&mut world, &data, 1);
    let condition = |name: &str| {
        data.registry
            .conditions
            .get(&format!("base:condition:{name}"))
            .unwrap()
    };
    let set = |world: &mut World, hp| {
        apply(
            world,
            &data,
            Command::Dev(DevCommand::SetHp { member: 0, hp }),
        )
        .unwrap()
    };
    let max = world.party.members[0].hp_max;
    set(&mut world, 999);
    assert_eq!(world.party.members[0].hp, max, "clamped to the maximum");
    let events = set(&mut world, -4);
    let brenna = &world.party.members[0];
    assert_eq!(brenna.hp, 0);
    assert!(brenna.conditions.contains(&condition("unconscious")));
    assert_eq!(brenna.death_saves, DeathSaves::default());
    assert!(events.iter().any(|e| matches!(e, Event::Down { .. })));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Condition { applied: true, .. }))
    );
    world.party.members[0].death_saves.failures = 2;
    apply(
        &mut world,
        &data,
        Command::Dev(DevCommand::SetCondition {
            member: 0,
            condition: "base:condition:dead".into(),
            applied: true,
        }),
    )
    .unwrap();
    let events = set(&mut world, 5);
    let brenna = &world.party.members[0];
    assert_eq!(brenna.hp, 5);
    assert!(!brenna.conditions.contains(&condition("unconscious")));
    assert!(
        !brenna.conditions.contains(&condition("dead")),
        "revived for the temple test"
    );
    assert_eq!(brenna.death_saves, DeathSaves::default());
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Condition { applied: false, .. }))
            .count(),
        2
    );
    set(&mut world, 5);
    assert_eq!(
        world.party.members[0].conditions.len(),
        0,
        "nothing to clear twice"
    );
}

#[test]
fn points_gold_food_experience_and_scores_are_set() {
    let data = data();
    let mut world = dev_world(&data);
    party_of(&mut world, &data, 2);
    let dev = |world: &mut World, edit| apply(world, &data, Command::Dev(edit));
    dev(
        &mut world,
        DevCommand::SetSpellPoints {
            member: 1,
            points: 99,
        },
    )
    .unwrap();
    let durin = &world.party.members[1];
    assert_eq!(durin.spell_points, durin.spell_points_max);
    assert_eq!(durin.spell_points_max, 2, "the dwarf cleric's pool");
    dev(
        &mut world,
        DevCommand::SetSpellPoints {
            member: 1,
            points: 0,
        },
    )
    .unwrap();
    assert_eq!(world.party.members[1].spell_points, 0);
    dev(&mut world, DevCommand::SetGold { gold: 1234 }).unwrap();
    dev(&mut world, DevCommand::SetFood { food: 3 }).unwrap();
    dev(&mut world, DevCommand::SetXp { member: 0, xp: 350 }).unwrap();
    assert_eq!((world.party.gold, world.party.food), (1234, 3));
    assert_eq!(
        (world.party.members[0].xp, world.party.members[0].level),
        (350, 1)
    );
    let ac = armor_class(&world.party.members[0], &data);
    dev(
        &mut world,
        DevCommand::SetScore {
            member: 0,
            ability: Ability::Dexterity,
            score: 20,
        },
    )
    .unwrap();
    assert_eq!(
        world.party.members[0].scores[Ability::Dexterity.index()],
        20
    );
    assert_eq!(
        armor_class(&world.party.members[0], &data),
        ac,
        "chain mail caps Dexterity, so the armor class holds"
    );
    let before = world.clone();
    for score in [0, 31] {
        assert_eq!(
            dev(
                &mut world,
                DevCommand::SetScore {
                    member: 0,
                    ability: Ability::Strength,
                    score
                }
            ),
            Err(Rejection::OutOfRange)
        );
    }
    assert_eq!(world, before);
}

#[test]
fn conditions_and_flags_are_set_by_id() {
    let mut data = data();
    let flag = data.registry.flags.intern("test:flag:rats_cleared");
    let mut world = dev_world(&data);
    party_of(&mut world, &data, 1);
    let poisoned = data
        .registry
        .conditions
        .get("base:condition:poisoned")
        .unwrap();
    let set = |world: &mut World, applied| {
        apply(
            world,
            &data,
            Command::Dev(DevCommand::SetCondition {
                member: 0,
                condition: "base:condition:poisoned".into(),
                applied,
            }),
        )
    };
    let events = set(&mut world, true).unwrap();
    assert!(world.party.members[0].conditions.contains(&poisoned));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Condition { condition, applied: true, .. } if *condition == poisoned
    )));
    let events = set(&mut world, true).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Condition { .. }))
            .count(),
        0,
        "already on"
    );
    set(&mut world, false).unwrap();
    assert!(!world.party.members[0].conditions.contains(&poisoned));
    assert_eq!(
        apply(
            &mut world,
            &data,
            Command::Dev(DevCommand::SetCondition {
                member: 0,
                condition: "base:condition:sleepy".into(),
                applied: true
            })
        ),
        Err(Rejection::UnknownId {
            id: "base:condition:sleepy".into()
        })
    );
    apply(
        &mut world,
        &data,
        Command::Dev(DevCommand::SetFlag {
            flag: "test:flag:rats_cleared".into(),
            value: 7,
        }),
    )
    .unwrap();
    assert_eq!(world.flags.get(&flag), Some(&7));
    assert_eq!(
        apply(
            &mut world,
            &data,
            Command::Dev(DevCommand::SetFlag {
                flag: "test:flag:none".into(),
                value: 1
            })
        ),
        Err(Rejection::UnknownId {
            id: "test:flag:none".into()
        })
    );
}

#[test]
fn teleport_moves_the_party_without_a_trigger_and_only_while_exploring() {
    let data = data();
    let mut world = dev_world(&data);
    party_of(&mut world, &data, 2);
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    let go = |world: &mut World, map: &str, x, y| {
        apply(
            world,
            &data,
            Command::Dev(DevCommand::Teleport {
                map: map.into(),
                x,
                y,
                facing: Facing::South,
            }),
        )
    };
    let clock = world.party_clock().elapsed;
    let events = go(&mut world, "test:map:dungeon", 3, 8).unwrap();
    assert_eq!(
        world.position,
        Position {
            map: dungeon,
            x: 3,
            y: 8,
            facing: Facing::South
        }
    );
    assert!(events.iter().any(|e| matches!(e, Event::Moved { .. })));
    assert!(
        !events.iter().any(|e| matches!(
            e,
            Event::EncounterStarted { .. } | Event::TimeAdvanced { .. }
        )),
        "no trigger, no minutes"
    );
    assert_eq!(world.mode, Mode::Explore);
    assert_eq!(world.party_clock().elapsed, clock);
    let known = &world.automap.maps[&dungeon][&(3, 8)];
    assert_ne!(known.layers & layer::VISITED, 0, "the tile is visited");
    let before = world.clone();
    assert_eq!(
        go(&mut world, "test:map:dungeon", 99, 99),
        Err(Rejection::OffMap { x: 99, y: 99 })
    );
    assert_eq!(
        go(&mut world, "test:map:cellar", 0, 0),
        Err(Rejection::UnknownId {
            id: "test:map:cellar".into()
        })
    );
    assert_eq!(world, before);

    let mut world = dev_world(&data);
    party_of(&mut world, &data, 2);
    go(&mut world, "test:map:dungeon", 3, 7).unwrap();
    apply(&mut world, &data, Command::Step(Direction::Forward)).unwrap();
    assert!(
        matches!(world.mode, Mode::Encounter(_)),
        "stepping onto the rats' tile triggers them: {:?}",
        world.mode
    );
    assert_eq!(
        go(&mut world, "test:map:dungeon", 1, 1),
        Err(Rejection::WrongMode)
    );
    apply(
        &mut world,
        &data,
        Command::Dev(DevCommand::SetGold { gold: 9 }),
    )
    .unwrap();
    assert_eq!(world.party.gold, 9, "party edits work before the fight");
    apply(
        &mut world,
        &data,
        Command::Encounter(EncounterChoice::Attack),
    )
    .unwrap();
    assert!(matches!(world.mode, Mode::Combat(_)));
    assert_eq!(
        apply(
            &mut world,
            &data,
            Command::Dev(DevCommand::SetGold { gold: 1 })
        ),
        Err(Rejection::WrongMode),
        "fight-mode edits arrive with the fight loop's settle"
    );
}

#[test]
fn a_dev_world_says_so_in_its_save_and_its_replays() {
    let data = data();
    let mut world = dev_world(&data);
    party_of(&mut world, &data, 1);
    let text = world.to_ron().unwrap();
    assert!(text.contains("devtools: true"));
    assert_eq!(
        query::path(&world, "settings.devtools").as_deref(),
        Some("true")
    );
    let loaded = World::from_ron(&text, &data, false).unwrap();
    assert!(loaded.settings.devtools);
    assert_eq!(loaded.to_ron().unwrap(), text);
    let commands = vec![
        Command::Party(omnis_sim::PartyCommand::Create(common::six().remove(0))),
        Command::Dev(DevCommand::SetGold { gold: 50 }),
    ];
    let dev = Settings {
        devtools: true,
        ..Settings::default()
    };
    let a = replay::run(&data, 1, dev, &commands).unwrap();
    let b = replay::run(&data, 1, dev, &commands).unwrap();
    assert_eq!(a, b, "dev edits replay");
    assert_eq!(
        replay::run(&data, 1, Settings::default(), &commands),
        Err(replay::ReplayError::Rejected { index: 1 })
    );
}
