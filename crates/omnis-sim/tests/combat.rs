//! Fights on real pack data: a whole fight to victory with its payout, rejections that leave
//! the world untouched, surprise, dodging, exchanges, deaths and permadeath, saves mid-fight,
//! flight, and the protocol view.

mod common;

use common::{data, encounter, party_of, world};
use omnis_core::{Direction, Facing, Position};
use omnis_data::ron_io::{parse, to_string};
use omnis_data::{Data, Disposition};
use omnis_sim::command::parse_script;
use omnis_sim::omnis_rules::{RollMode, condition_id};
use omnis_sim::{
    ActorRef, CheckKind, CombatCommand, CombatOutcome, Command, Event, LoadError, Mode, ModeKind,
    Op, OpError, Rejection, Reply, Settings, Surprise, World, apply, combat, combat_view, dispatch,
};

fn here(world: &World) -> Position {
    world.position
}

fn start(world: &mut World, data: &Data, stacks: &[(&str, u8)], surprised: Surprise) -> Vec<Event> {
    let mut events = Vec::new();
    let encounter = encounter(data, stacks, Disposition::Hostile, here(world));
    combat::start(world, data, encounter, surprised, &mut events).unwrap();
    events
}

/// The first living stack the acting member can reach.
fn target(world: &World, data: &Data) -> u8 {
    let view = combat_view(world, data).expect("a fight");
    view.stacks
        .iter()
        .find(|s| s.alive && s.reachable)
        .map(|s| s.index)
        .expect("something to hit")
}

fn attack(world: &mut World, data: &Data, stack: u8) -> Vec<Event> {
    apply(
        world,
        data,
        Command::Combat(CombatCommand::Attack { stack }),
    )
    .unwrap_or_else(|r| panic!("{r}"))
}

/// Attack the nearest reachable stack on every member turn until the fight ends.
fn fight_to_the_end(world: &mut World, data: &Data) -> Vec<Event> {
    let mut all = Vec::new();
    for _ in 0..500 {
        if !matches!(world.mode, Mode::Combat(_)) {
            return all;
        }
        let stack = target(world, data);
        all.extend(attack(world, data, stack));
    }
    panic!("the fight did not end");
}

fn ended(events: &[Event]) -> Option<(CombatOutcome, u32, u32)> {
    events.iter().find_map(|e| match e {
        Event::CombatEnded {
            outcome, xp, gold, ..
        } => Some((*outcome, *xp, *gold)),
        _ => None,
    })
}

#[test]
fn a_fight_runs_to_victory_and_pays_out() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    let events = start(
        &mut world,
        &data,
        &[("goblin", 3), ("giant_rat", 2)],
        Surprise::None,
    );
    assert_eq!(
        events[0],
        Event::CombatStarted {
            surprised: Surprise::None
        }
    );
    let Event::Initiative { order, rolls } = &events[1] else {
        panic!("{events:?}")
    };
    assert_eq!((order.len(), rolls.len()), (8, 8));
    assert!(
        order.windows(2).all(|w| w[0].1 >= w[1].1),
        "sorted: {order:?}"
    );
    assert_eq!(events[2], Event::RoundStarted { round: 1 });
    assert!(matches!(
        events.last(),
        Some(Event::Turn {
            actor: ActorRef::Member(_)
        })
    ));
    let Mode::Combat(state) = &world.mode else {
        panic!("not fighting")
    };
    assert_eq!(state.front_stacks(&data), [0, 1]);
    let events = fight_to_the_end(&mut world, &data);
    let (outcome, xp, gold) = ended(&events).expect("the fight ends");
    assert_eq!(outcome, CombatOutcome::Victory);
    let survivors = u32::try_from(world.party.members.len()).unwrap();
    assert_eq!(xp, 200 / survivors, "3 goblins and 2 rats split evenly");
    assert!(world.party.members.iter().all(|m| m.xp == xp));
    let dropped: u32 = events
        .iter()
        .filter_map(|e| match e {
            Event::Death { gold: Some(g), .. } => Some(u32::try_from(g.total.max(0)).unwrap()),
            _ => None,
        })
        .sum();
    assert_eq!((gold, world.party.gold), (dropped, 15 * 6 + dropped));
    assert!(
        events.iter().all(|e| match e {
            Event::AttackResolved { roll, .. } => roll.trace.stream.0 == "combat",
            _ => true,
        }),
        "every attack draws from the combat stream"
    );
    assert!(
        world
            .rngs
            .contains_key(&omnis_core::StreamName::new("combat"))
    );
    assert_eq!(world.mode, Mode::Explore);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TimeAdvanced { .. }))
    );
}

#[test]
fn rejections_leave_the_world_untouched() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    let explore = world.clone();
    assert_eq!(
        apply(&mut world, &data, Command::Combat(CombatCommand::Dodge)),
        Err(Rejection::WrongMode)
    );
    assert_eq!(world, explore);
    for member in &mut world.party.members {
        member.equipment.retain(|(item, _)| {
            let name = data.registry.items.name(*item).unwrap();
            !name.ends_with("crossbow") && !name.ends_with("shortbow")
        });
    }
    start(
        &mut world,
        &data,
        &[("goblin", 1), ("giant_rat", 1), ("skeleton", 1)],
        Surprise::None,
    );
    let before = world.clone();
    let refuse = |world: &mut World, command: CombatCommand, expected: Rejection| {
        assert_eq!(
            apply(world, &data, Command::Combat(command)),
            Err(expected),
            "{command:?}"
        );
        assert_eq!(*world, before, "{command:?} changed the world");
    };
    refuse(
        &mut world,
        CombatCommand::Attack { stack: 9 },
        Rejection::NoSuchStack { stack: 9 },
    );
    refuse(
        &mut world,
        CombatCommand::Exchange { with: 9 },
        Rejection::NoSuchMember { index: 9 },
    );
    let view = combat_view(&world, &data).unwrap();
    let Some(ActorRef::Member(id)) = view.current else {
        panic!("{view:?}")
    };
    let own = world.party.members.iter().position(|m| m.id == id).unwrap();
    refuse(
        &mut world,
        CombatCommand::Exchange {
            with: u8::try_from(own).unwrap(),
        },
        Rejection::SameMember,
    );
    if own < 3 {
        refuse(
            &mut world,
            CombatCommand::Attack { stack: 2 },
            Rejection::OutOfReach { stack: 2 },
        );
        assert!(view.stacks[0].reachable && !view.stacks[2].reachable);
    } else {
        refuse(
            &mut world,
            CombatCommand::Attack { stack: 0 },
            Rejection::NeedsRangedWeapon,
        );
        assert!(view.stacks.iter().all(|s| !s.reachable));
    }
    assert_eq!(
        apply(&mut world, &data, Command::Step(Direction::Forward)),
        Err(Rejection::WrongMode)
    );
    assert_eq!(world, before);
}

#[test]
fn a_surprised_party_loses_the_first_round() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    let events = start(&mut world, &data, &[("goblin", 2)], Surprise::Party);
    let second = events
        .iter()
        .position(|e| *e == Event::RoundStarted { round: 2 })
        .expect("round two comes before any member acts");
    assert!(events[..second].iter().all(|e| !matches!(
        e,
        Event::Turn {
            actor: ActorRef::Member(_)
        } | Event::AttackResolved {
            attacker: ActorRef::Member(_),
            ..
        }
    )));
    assert!(events[..second].iter().any(|e| matches!(
        e,
        Event::AttackResolved {
            attacker: ActorRef::Monster { .. },
            ..
        }
    )));
    assert!(matches!(
        events.last(),
        Some(Event::Turn {
            actor: ActorRef::Member(_)
        })
    ));
}

#[test]
fn dodging_gives_the_monsters_disadvantage_for_the_round() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 1);
    start(&mut world, &data, &[("goblin", 3)], Surprise::None);
    let events = apply(&mut world, &data, Command::Combat(CombatCommand::Dodge)).unwrap();
    let id = world.party.members[0].id;
    assert_eq!(
        events[0],
        Event::Dodging {
            actor: ActorRef::Member(id)
        }
    );
    let next_round = events
        .iter()
        .position(|e| matches!(e, Event::RoundStarted { .. }))
        .unwrap_or(events.len());
    let modes: Vec<RollMode> = events[..next_round]
        .iter()
        .filter_map(|e| match e {
            Event::AttackResolved { roll, .. } => Some(roll.mode),
            _ => None,
        })
        .collect();
    assert!(
        !modes.is_empty() && modes.iter().all(|m| *m == RollMode::Disadvantage),
        "{modes:?}"
    );
    let later: Vec<RollMode> = events[next_round..]
        .iter()
        .filter_map(|e| match e {
            Event::AttackResolved { roll, .. } => Some(roll.mode),
            _ => None,
        })
        .collect();
    assert!(later.iter().all(|m| *m == RollMode::Normal), "{later:?}");
}

#[test]
fn exchange_swaps_two_slots() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    start(&mut world, &data, &[("giant_rat", 1)], Surprise::None);
    let view = combat_view(&world, &data).unwrap();
    let Some(ActorRef::Member(id)) = view.current else {
        panic!()
    };
    let own = world.party.members.iter().position(|m| m.id == id).unwrap();
    let with = (own + 1) % 6;
    let other = world.party.members[with].id;
    let events = apply(
        &mut world,
        &data,
        Command::Combat(CombatCommand::Exchange {
            with: u8::try_from(with).unwrap(),
        }),
    )
    .unwrap();
    assert_eq!(
        events[0],
        Event::Exchanged {
            a: u8::try_from(own).unwrap(),
            b: u8::try_from(with).unwrap()
        }
    );
    assert_eq!(events[1], Event::PartyChanged);
    assert_eq!(world.party.members[with].id, id);
    assert_eq!(world.party.members[own].id, other);
}

#[test]
fn the_dead_are_buried_under_permadeath_and_kept_otherwise() {
    let data = data();
    let dead = condition_id(&data, "dead").unwrap();
    for permadeath in [true, false] {
        let mut world = World::new(
            &data,
            5,
            Settings {
                permadeath,
                ..Settings::default()
            },
        )
        .unwrap();
        party_of(&mut world, &data, 2);
        let fallen = world.party.members[0].id;
        let kit = world.party.members[0].equipment.clone();
        world.party.members[0].hp = 0;
        world.party.members[0].conditions.push(dead);
        let mut encounter = encounter(
            &data,
            &[("giant_rat", 1)],
            Disposition::Hostile,
            here(&world),
        );
        encounter.stacks[0].hp = vec![1];
        let mut events = Vec::new();
        combat::start(&mut world, &data, encounter, Surprise::None, &mut events).unwrap();
        let order = events.iter().find_map(|e| match e {
            Event::Initiative { order, .. } => Some(order.clone()),
            _ => None,
        });
        assert_eq!(
            order.map(|o| o.len()),
            Some(2),
            "the dead roll no initiative"
        );
        let events = fight_to_the_end(&mut world, &data);
        let Some((outcome, xp, buried)) = events.iter().find_map(|e| match e {
            Event::CombatEnded {
                outcome,
                xp,
                fallen,
                ..
            } => Some((outcome, xp, fallen)),
            _ => None,
        }) else {
            panic!("{events:?}")
        };
        assert_eq!(
            (*outcome, *xp),
            (CombatOutcome::Victory, 25),
            "one survivor takes it all"
        );
        if permadeath {
            assert_eq!(buried, &[fallen]);
            assert_eq!(world.party.members.len(), 1);
            for (item, count) in &kit {
                assert!(world.party.inventory.contains(&(*item, *count)), "{item:?}");
            }
        } else {
            assert!(buried.is_empty());
            assert_eq!(world.party.members.len(), 2);
            assert!(world.party.members[0].conditions.contains(&dead));
            assert_eq!(world.party.members[0].xp, 0, "the dead earn nothing");
        }
        assert_eq!(world.party.members.last().unwrap().xp, 25);
    }
}

/// Fight with a fragile member on every seed until the death rules have all been seen.
#[test]
fn members_go_down_save_and_die_by_the_srd() {
    let data = data();
    let unconscious = condition_id(&data, "unconscious").unwrap();
    let dead = condition_id(&data, "dead").unwrap();
    let mut seen = (false, false, false, false);
    for seed in 0..120u64 {
        let mut world = World::new(&data, seed, Settings::default()).unwrap();
        party_of(&mut world, &data, 2);
        world.party.members[0].hp = 2;
        world.party.members[0].hp_max = 5;
        let fragile = world.party.members[0].id;
        let mut events = start(&mut world, &data, &[("goblin", 3)], Surprise::None);
        events.extend(fight_to_the_end(&mut world, &data));
        for (i, e) in events.iter().enumerate() {
            match e {
                Event::Down { target } if *target == fragile => {
                    seen.0 = true;
                    assert!(matches!(
                        events[i + 1],
                        Event::Condition {
                            condition,
                            applied: true,
                            ..
                        } if condition == unconscious
                    ));
                }
                Event::DeathSave {
                    member,
                    successes,
                    failures,
                    result,
                    ..
                } if *member == fragile => {
                    seen.1 = true;
                    assert!(*successes <= 3 && *failures <= 3, "{e:?}");
                    if *result == omnis_sim::omnis_rules::DeathSaveResult::Revived {
                        seen.2 = true;
                    }
                }
                Event::Death {
                    target: ActorRef::Member(m),
                    ..
                } if *m == fragile => seen.3 = true,
                _ => {}
            }
        }
        let member = world
            .party
            .members
            .iter()
            .find(|m| m.id == fragile)
            .unwrap();
        if member.conditions.contains(&dead) {
            assert!(member.is_down() && !member.conditions.contains(&unconscious));
        }
        if seen.0 && seen.1 && seen.2 && seen.3 {
            return;
        }
    }
    panic!("not every death rule was exercised: {seen:?}");
}

#[test]
fn a_fight_survives_a_save_and_a_bad_one_is_refused() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    start(
        &mut world,
        &data,
        &[("goblin", 2), ("skeleton", 1)],
        Surprise::None,
    );
    let text = world.to_ron().unwrap();
    let mut loaded = World::from_ron(&text, &data, false).unwrap_or_else(|e| panic!("{e}"));
    world.log.clear();
    assert_eq!(loaded, world, "equal but for the unsaved log");
    let stack = target(&world, &data);
    assert_eq!(
        attack(&mut loaded, &data, stack),
        attack(&mut world, &data, stack)
    );
    assert_eq!(loaded.fingerprint().unwrap(), world.fingerprint().unwrap());

    let mut broken = world.clone();
    if let Mode::Combat(state) = &mut broken.mode {
        let stack = state
            .order
            .iter()
            .position(|e| matches!(e.actor, ActorRef::Stack(_)))
            .unwrap();
        state.current = u8::try_from(stack).unwrap();
    }
    assert_eq!(
        World::from_ron(&broken.to_ron().unwrap(), &data, false).unwrap_err(),
        LoadError::BadCombat("the fight is not waiting on a living member")
    );
    let mut broken = world.clone();
    if let Mode::Combat(state) = &mut broken.mode {
        state.encounter.stacks[0].monster = omnis_core::MonsterId(99);
    }
    assert_eq!(
        World::from_ron(&broken.to_ron().unwrap(), &data, false).unwrap_err(),
        LoadError::BadCombat("a stack names an unknown monster")
    );
}

#[test]
fn flight_takes_the_party_back_or_costs_the_turn() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 3);
    let retreat = Position {
        map: world.position.map,
        x: 16,
        y: 17,
        facing: Facing::South,
    };
    let mut events = Vec::new();
    let friendly = encounter(&data, &[("giant_rat", 2)], Disposition::Friendly, retreat);
    combat::start(&mut world, &data, friendly, Surprise::None, &mut events).unwrap();
    let events = apply(&mut world, &data, Command::Combat(CombatCommand::Run)).unwrap();
    assert!(matches!(
        events[0],
        Event::Check {
            kind: CheckKind::Flee,
            roll: None,
            success: true,
            ..
        }
    ));
    assert!(matches!(events[1], Event::Moved { to, .. } if to == retreat));
    assert_eq!(ended(&events).map(|e| e.0), Some(CombatOutcome::Fled));
    assert_eq!((world.position, &world.mode), (retreat, &Mode::Explore));
    assert!(world.party.members.iter().all(|m| m.xp == 0));

    let mut outcomes = (false, false);
    for seed in 0..40u64 {
        let mut world = World::new(&data, seed, Settings::default()).unwrap();
        party_of(&mut world, &data, 3);
        start(&mut world, &data, &[("giant_rat", 1)], Surprise::None);
        let events = apply(&mut world, &data, Command::Combat(CombatCommand::Run)).unwrap();
        let Event::Check {
            kind: CheckKind::Flee,
            roll: Some(roll),
            dc,
            success,
            ..
        } = &events[0]
        else {
            panic!("{events:?}")
        };
        assert_eq!((*dc, *success), (15, roll.total >= 15));
        if *success {
            outcomes.0 = true;
            assert_eq!(world.mode, Mode::Explore);
        } else {
            outcomes.1 = true;
            assert!(matches!(world.mode, Mode::Combat(_)));
        }
    }
    assert_eq!(outcomes, (true, true));
}

#[test]
fn the_protocol_reports_the_fight() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    assert_eq!(
        dispatch(&mut world, &data, &Op::CombatGet).unwrap_err(),
        OpError::NoEncounter
    );
    start(
        &mut world,
        &data,
        &[("goblin", 2), ("giant_rat", 1), ("skeleton", 1)],
        Surprise::None,
    );
    let Reply::Combat { combat } = dispatch(&mut world, &data, &Op::CombatGet).unwrap() else {
        panic!("not a combat reply")
    };
    assert_eq!((combat.phase, combat.round), (ModeKind::Combat, 1));
    assert_eq!(combat.stacks.len(), 3);
    assert_eq!(combat.stacks[2].monster, "base:monster:skeleton");
    assert!(combat.stacks[0].front && combat.stacks[1].front && !combat.stacks[2].front);
    assert!(matches!(combat.current, Some(ActorRef::Member(_))));
    assert_eq!(combat.order.len(), 9);
    let text = to_string(&Reply::Combat {
        combat: combat.clone(),
    })
    .unwrap();
    assert_eq!(parse::<Reply>(&text).unwrap(), Reply::Combat { combat });
    let Reply::Status(status) = dispatch(&mut world, &data, &Op::GameStatus).unwrap() else {
        panic!()
    };
    assert_eq!(status.mode, ModeKind::Combat);
    assert_eq!(
        parse_script("attack, attack-1, dodge, swap-3, flee").unwrap(),
        [
            Command::Combat(CombatCommand::Attack { stack: 0 }),
            Command::Combat(CombatCommand::Attack { stack: 1 }),
            Command::Combat(CombatCommand::Dodge),
            Command::Combat(CombatCommand::Exchange { with: 3 }),
            Command::Combat(CombatCommand::Run),
        ]
    );
    assert_eq!(Command::from_word("attack-256"), None);
    assert_eq!(
        Command::Combat(CombatCommand::Attack { stack: 2 }).word(),
        "attack"
    );
    assert_eq!(
        Command::Combat(CombatCommand::Exchange { with: 1 }).word(),
        "swap"
    );
}
