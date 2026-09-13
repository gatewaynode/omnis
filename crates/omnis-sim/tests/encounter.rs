//! Encounters on the test dungeon: the placed rats wait on their tile and the walk reaches
//! them, the four choices resolve by the rules, random tables draw from their own stream, and
//! the protocol knows the choices.

mod common;

use common::{data, encounter, party_of, play, walk_to_the_rats, world};
use omnis_core::{Direction, Facing, Position, StreamName};
use omnis_data::{Data, Disposition};
use omnis_sim::command::parse_script;
use omnis_sim::{
    ActorRef, CheckKind, Command, EncounterChoice, EncounterSource, Event, Mode, ModeKind, Op,
    Rejection, Reply, Settings, Surprise, World, apply, dispatch,
};

fn dungeon(data: &Data) -> omnis_core::MapId {
    data.registry.maps.get("test:map:dungeon").unwrap()
}

/// The party standing on a dungeon tile with a hand-built encounter of the given placement
/// in front of it, as if the trigger had just noticed the party.
fn facing_encounter(
    data: &Data,
    seed: u64,
    index: u16,
    stacks: &[(&str, u8)],
    disposition: Disposition,
) -> World {
    let mut world = World::new(data, seed, Settings::default()).unwrap();
    party_of(&mut world, data, 6);
    let placement = &data.maps[&dungeon(data)].encounters[usize::from(index)];
    world.position = Position {
        map: dungeon(data),
        x: placement.x,
        y: placement.y,
        facing: Facing::South,
    };
    let retreat = Position {
        map: dungeon(data),
        x: placement.x,
        y: placement.y - 1,
        facing: Facing::North,
    };
    let mut state = encounter(data, stacks, disposition, retreat);
    state.source = EncounterSource::Fixed(index);
    world.mode = Mode::Encounter(state);
    world
}

fn choose(world: &mut World, data: &Data, choice: EncounterChoice) -> Vec<Event> {
    apply(world, data, Command::Encounter(choice)).unwrap_or_else(|r| panic!("{r}"))
}

#[test]
fn the_placed_rats_wait_on_their_tile_and_stay_cleared() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    let (commands, events) = play(&mut world, &data, &walk_to_the_rats());
    assert!(commands.len() >= walk_to_the_rats().len());
    let started: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::EncounterStarted { .. }))
        .collect();
    let Some(Event::EncounterStarted {
        source,
        stacks,
        disposition,
        counts,
        stealth,
        noticed,
        ..
    }) = started.last()
    else {
        panic!("no encounter: {events:?}")
    };
    assert_eq!(*source, EncounterSource::Fixed(0));
    assert_eq!(stacks.iter().map(|(_, n)| *n).collect::<Vec<_>>(), [2]);
    assert_eq!((*disposition, counts.len()), (Disposition::Hostile, 0));
    assert!(
        stealth.is_none() && *noticed,
        "surprise is off in the base rules: no Stealth roll, the party chooses"
    );
    assert!(
        !events.iter().any(|e| matches!(
            e,
            Event::CombatStarted {
                surprised: Surprise::Party
            }
        )),
        "never surprised with the check off"
    );
    assert!(events.iter().all(|e| match e {
        Event::EncounterCheck { roll, chance, .. } => {
            roll.trace_stream_is_encounter() && (*chance == 0 || *chance == 3)
        }
        _ => true,
    }));
    assert_eq!(world.mode, Mode::Explore);
    assert!(world.maps[&dungeon(&data)].cleared.contains(&0));
    let (_, again) = play(
        &mut world,
        &data,
        &[
            Command::Turn(Rotation::Around),
            Command::Step(Direction::Forward),
            Command::Turn(Rotation::Around),
            Command::Step(Direction::Forward),
        ],
    );
    assert!(
        !again.iter().any(|e| matches!(
            e,
            Event::EncounterStarted {
                source: EncounterSource::Fixed(0),
                ..
            }
        )),
        "a once encounter does not come back"
    );
    assert_eq!((world.position.x, world.position.y), (3, 8));
}

#[test]
fn with_the_surprise_value_on_an_unnoticed_party_starts_the_fight_surprised() {
    let mut data = data();
    data.rules.insert_value("surprise", 1);
    let mut seen = [false; 2];
    for seed in 0..40u64 {
        let mut world = World::new(&data, seed, Settings::default()).unwrap();
        party_of(&mut world, &data, 2);
        let placement = &data.maps[&dungeon(&data)].encounters[0];
        world.position = Position {
            map: dungeon(&data),
            x: placement.x,
            y: placement.y - 1,
            facing: Facing::South,
        };
        let events = apply(&mut world, &data, Command::Step(Direction::Forward)).unwrap();
        let Some(Event::EncounterStarted {
            stealth: Some(roll),
            perception,
            noticed,
            ..
        }) = events
            .iter()
            .find(|e| matches!(e, Event::EncounterStarted { .. }))
        else {
            panic!("{events:?}")
        };
        assert_eq!(roll.trace.stream, StreamName::new("combat"));
        assert_eq!(*noticed, roll.total < *perception);
        let surprised = events.iter().any(|e| {
            matches!(
                e,
                Event::CombatStarted {
                    surprised: Surprise::Party
                }
            )
        });
        assert_eq!(surprised, !*noticed, "unnoticed means surprised");
        assert_eq!(
            matches!(world.mode, Mode::Encounter(_)),
            *noticed,
            "noticed means the choice"
        );
        seen[usize::from(*noticed)] = true;
    }
    assert_eq!(seen, [true, true], "both outcomes over 40 seeds");
}

trait EncounterStream {
    fn trace_stream_is_encounter(&self) -> bool;
}

impl EncounterStream for omnis_core::RollTrace {
    fn trace_stream_is_encounter(&self) -> bool {
        self.stream == StreamName::new("encounter")
    }
}

use omnis_core::Rotation;

#[test]
fn a_bribe_costs_by_the_rules_and_clears_a_once_group() {
    let data = data();
    let mut world = facing_encounter(
        &data,
        1,
        0,
        &[("goblin", 3), ("giant_rat", 2)],
        Disposition::Hostile,
    );
    let before = world.clone();
    assert_eq!(
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Bribe)
        ),
        Err(Rejection::CannotAfford {
            cost: 200,
            gold: 90
        }),
        "3 goblins and 2 rats are worth 200 xp; a hostile bribe is all of it"
    );
    assert_eq!(world, before, "a refused bribe changes nothing");
    world.party.gold = 500;
    let events = choose(&mut world, &data, EncounterChoice::Bribe);
    assert_eq!(events[0], Event::Bribed { cost: 200 });
    assert_eq!((world.party.gold, &world.mode), (300, &Mode::Explore));
    assert!(world.maps[&dungeon(&data)].cleared.contains(&0));
    assert!(
        world.party.members.iter().all(|m| m.xp == 0),
        "no experience for paying"
    );

    let mut friendly = facing_encounter(&data, 2, 2, &[("giant_rat", 3)], Disposition::Friendly);
    let events = choose(&mut friendly, &data, EncounterChoice::Bribe);
    assert_eq!(events[0], Event::Bribed { cost: 0 });
    assert_eq!(friendly.party.gold, 90);
}

#[test]
fn hiding_and_running_roll_against_the_group_or_start_a_surprised_fight() {
    let data = data();
    let mut seen = [false; 4];
    for seed in 0..60u64 {
        let mut world = facing_encounter(&data, seed, 1, &[("skeleton", 2)], Disposition::Wary);
        let events = choose(&mut world, &data, EncounterChoice::Hide);
        let Event::Check {
            kind: CheckKind::Hide,
            roll: Some(roll),
            dc,
            success,
            actor: ActorRef::Member(_),
        } = &events[0]
        else {
            panic!("{events:?}")
        };
        assert_eq!(
            (*dc, *success),
            (9, roll.total >= 9),
            "a skeleton's passive Perception"
        );
        if *success {
            seen[0] = true;
            assert_eq!(world.mode, Mode::Explore);
            assert!(
                !world
                    .maps
                    .get(&dungeon(&data))
                    .is_some_and(|s| s.cleared.contains(&1))
            );
        } else {
            seen[1] = true;
            assert!(matches!(
                events[1],
                Event::CombatStarted {
                    surprised: Surprise::Party
                }
            ));
        }
        let mut world = facing_encounter(&data, seed, 1, &[("skeleton", 2)], Disposition::Wary);
        let from = world.position;
        let events = choose(&mut world, &data, EncounterChoice::Run);
        let Event::Check {
            kind: CheckKind::Run,
            roll: Some(roll),
            dc,
            success,
            ..
        } = &events[0]
        else {
            panic!("{events:?}")
        };
        assert_eq!((*dc, *success), (12, roll.total >= 12), "wary: 12");
        if *success {
            seen[2] = true;
            assert!(matches!(events[1], Event::Moved { to, .. } if to.y == from.y - 1));
            assert!(matches!(events[2], Event::TimeAdvanced { minutes: 1, .. }));
            assert_eq!(
                (world.position.y, world.position.facing, &world.mode),
                (from.y - 1, Facing::North, &Mode::Explore)
            );
        } else {
            seen[3] = true;
            assert!(matches!(world.mode, Mode::Combat(_)));
        }
    }
    assert_eq!(
        seen, [true; 4],
        "hide and run both succeeded and failed across seeds"
    );

    let mut friendly = facing_encounter(&data, 3, 2, &[("giant_rat", 3)], Disposition::Friendly);
    let events = choose(&mut friendly, &data, EncounterChoice::Hide);
    assert!(matches!(
        events[0],
        Event::Check {
            roll: None,
            success: true,
            ..
        }
    ));
    let mut friendly = facing_encounter(&data, 3, 2, &[("giant_rat", 3)], Disposition::Friendly);
    let events = choose(&mut friendly, &data, EncounterChoice::Run);
    assert!(matches!(
        events[0],
        Event::Check {
            roll: None,
            success: true,
            ..
        }
    ));
    assert_eq!(friendly.mode, Mode::Explore);
}

#[test]
fn attacking_starts_the_fight_and_the_protocol_knows_the_choices() {
    let data = data();
    let mut world = facing_encounter(
        &data,
        4,
        0,
        &[("goblin", 3), ("giant_rat", 2)],
        Disposition::Hostile,
    );
    let Reply::Combat { combat } = dispatch(&mut world, &data, &Op::CombatGet).unwrap() else {
        panic!()
    };
    assert_eq!(
        (combat.phase, combat.round, combat.current),
        (ModeKind::Encounter, 0, None)
    );
    assert!(combat.order.is_empty() && combat.stacks.iter().all(|s| !s.reachable));
    assert!(combat.stacks[0].front && combat.stacks[1].front);
    assert_eq!(
        apply(&mut world, &data, Command::Step(Direction::Forward)),
        Err(Rejection::WrongMode),
        "no walking away without choosing"
    );
    let events = choose(&mut world, &data, EncounterChoice::Attack);
    assert_eq!(
        events[0],
        Event::CombatStarted {
            surprised: Surprise::None
        }
    );
    assert!(matches!(world.mode, Mode::Combat(_)));
    assert_eq!(
        apply(&mut world, &data, Command::Encounter(EncounterChoice::Run)),
        Err(Rejection::WrongMode),
        "the choice is made"
    );
    assert_eq!(
        parse_script("fight, bribe, hide, run").unwrap(),
        [
            Command::Encounter(EncounterChoice::Attack),
            Command::Encounter(EncounterChoice::Bribe),
            Command::Encounter(EncounterChoice::Hide),
            Command::Encounter(EncounterChoice::Run),
        ]
    );
    assert_eq!(Command::Encounter(EncounterChoice::Attack).word(), "fight");
}

#[test]
fn random_encounters_draw_from_their_own_stream() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    let mut checks = 0;
    for _ in 0..6 {
        let events = apply(&mut world, &data, Command::Step(Direction::Forward)).unwrap();
        checks += events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::EncounterCheck {
                        fired: false,
                        chance: 0,
                        ..
                    }
                )
            })
            .count();
    }
    assert_eq!(
        checks, 6,
        "the meadow's table rolls every step and never fires"
    );
    let encounter = StreamName::new("encounter");
    assert_eq!(world.rngs[&encounter].draws(), 6);
    assert!(
        !world.rngs.contains_key(&StreamName::new("combat")),
        "walking never touches the fight's dice"
    );
    let mut hot = data.clone();
    hot.rules.set_slot("encounter.random", "true").unwrap();
    let events = apply(&mut world, &hot, Command::Step(Direction::Forward)).unwrap();
    let Some(Event::EncounterStarted {
        source,
        counts,
        stacks,
        ..
    }) = events
        .iter()
        .find(|e| matches!(e, Event::EncounterStarted { .. }))
    else {
        panic!("{events:?}")
    };
    assert_eq!(*source, EncounterSource::Random);
    assert_eq!(counts.len(), 1);
    assert!(counts[0].trace_stream_is_encounter());
    assert!((1..=2).contains(&stacks[0].1));
    assert!(
        world.rngs[&encounter].draws() >= 8,
        "the check, the entry, the count"
    );
    assert!(
        world.rngs.contains_key(&StreamName::new("combat")),
        "the stealth roll"
    );
    assert!(matches!(world.mode, Mode::Encounter(_) | Mode::Combat(_)));
    if let Mode::Encounter(state) = &world.mode {
        assert_eq!(state.retreat.y, world.position.y + 1);
        assert_eq!(state.retreat.facing, Facing::South);
    }
}
