//! A save spell that pushes (owner, 2026-11-16: "thunderwave does not seem to push back foes"):
//! those of a stack who fail the save are thrown back, as their own stack when some of the
//! stack held their ground (the owner's choice: "Split the stack"); a monster's push drives the
//! party's line back when half its front row fails ("Line driven back").

use crate::common::{self, data, party_of};
use crate::targets::fight;
use omnis_data::Data;
use omnis_sim::{
    ActorRef, CombatCommand, CombatState, Command, Event, Mode, Pay, Settings, Target, World, apply,
};

const WIZARD: usize = 2;

fn state(world: &World) -> &CombatState {
    match &world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

fn state_mut(world: &mut World) -> &mut CombatState {
    match &mut world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

/// The six against stacks of goblins with the hit points given, all engaged, on the wizard's
/// turn; each goblin has spent as many spell points as its place in its stack.
fn against(data: &Data, seed: u64, stacks: &[&[i32]]) -> World {
    let mut world = common::new_world(data, seed, Settings::default());
    party_of(&mut world, data, 6);
    for member in &mut world.party.members {
        member.hp = 500;
        member.hp_max = 500;
    }
    let goblins: Vec<(&str, u8)> = stacks
        .iter()
        .map(|hp| ("goblin", u8::try_from(hp.len()).unwrap()))
        .collect();
    fight(&mut world, data, &goblins, WIZARD);
    for (index, hp) in stacks.iter().enumerate() {
        common::stand_back(&mut world, index, 5);
        let stack = &mut state_mut(&mut world).encounter.stacks[index];
        stack.hp = hp.to_vec();
        stack.spent = (0..hp.len()).map(|i| u8::try_from(i).unwrap()).collect();
    }
    world
}

fn wave(world: &mut World, data: &Data, stack: u8) -> Vec<Event> {
    let cast = CombatCommand::Cast {
        spell: "base:spell:thunderwave".into(),
        target: Target::Stack(stack),
        pay: Pay::Action,
    };
    apply(world, data, Command::Combat(cast)).expect("the wave")
}

/// Who of `stack` failed the save, by index as the save was asked, ascending.
fn failed(events: &[Event], stack: u8) -> Vec<usize> {
    let mut failed: Vec<usize> = events
        .iter()
        .filter_map(|e| match e {
            Event::Check {
                actor: ActorRef::Monster { stack: s, index },
                success: false,
                ..
            } if *s == stack => Some(usize::from(*index)),
            _ => None,
        })
        .collect();
    failed.sort_unstable();
    failed
}

/// What the wave moved: the cast spends the wizard's action and the turn passes on, so only
/// the events before the next turn are the wave's.
fn moves(events: &[Event]) -> Vec<&Event> {
    let moved = |e: &&Event| matches!(e, Event::StackMoved { .. } | Event::StackSplit { .. });
    let own = events
        .iter()
        .take_while(|e| !matches!(e, Event::Turn { .. }));
    own.filter(moved).collect()
}

/// Whether the wave threw the whole of stack 0 back ten feet.
fn whole(events: &[Event]) -> bool {
    let moved = Event::StackMoved {
        stack: 0,
        from: 5,
        to: 15,
    };
    moves(events) == [&moved]
}

/// The hit points of the goblins of `stack`, each less what the wave did to it.
fn hurt(events: &[Event], before: &[i32], stack: u8) -> Vec<i32> {
    let mut after = before.to_vec();
    for event in events {
        if let Event::Damage {
            target: ActorRef::Monster { stack: s, index },
            amount,
            ..
        } = event
            && *s == stack
        {
            after[usize::from(*index)] -= i32::try_from(*amount).unwrap();
        }
    }
    after
}

#[test]
fn those_who_fail_are_thrown_back_and_leave_the_stack_when_others_held() {
    let data = data();
    let (mut none, mut all, mut some) = (0, 0, 0);
    for seed in 0..60 {
        let count = 1 + usize::try_from(seed % 4).unwrap();
        let before: Vec<i32> = (0..count)
            .map(|i| 100 + i32::try_from(i).unwrap())
            .collect();
        let mut world = against(&data, seed, &[&before]);
        let wizard = state(&world).current_actor().expect("the wizard");
        let events = wave(&mut world, &data, 0);
        // The turn passes on once, to someone else: nobody's place in the order was lost.
        let turns: Vec<&Event> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::Turn {
                        actor: ActorRef::Member(_)
                    }
                )
            })
            .collect();
        assert_eq!(turns.len(), 1, "seed {seed}");
        assert_ne!(turns[0], &Event::Turn { actor: wizard }, "seed {seed}");
        let failed = failed(&events, 0);
        let after = hurt(&events, &before, 0);
        let stacks = &state(&world).encounter.stacks;
        if failed.is_empty() {
            none += 1;
            assert!(moves(&events).is_empty(), "seed {seed}");
            assert_eq!(stacks.len(), 1);
        } else if failed.len() == count {
            all += 1;
            assert!(whole(&events), "seed {seed}");
            assert_eq!(stacks.len(), 1);
            assert_eq!(stacks[0].hp, after);
        } else {
            some += 1;
            let split = Event::StackSplit {
                stack: 0,
                into: 1,
                moved: failed.iter().map(|i| u8::try_from(*i).unwrap()).collect(),
                from: 5,
                to: 15,
            };
            assert_eq!(moves(&events), [&split]);
            assert_eq!(stacks.len(), 2);
            assert_eq!(stacks[1].monster, stacks[0].monster);
            // Each goes with its own hit points and spent points.
            let thrown = |i: &usize| failed.contains(i);
            let of = |keep: bool| (0..count).filter(move |i| thrown(i) == keep);
            let hp = |keep: bool| of(keep).map(|i| after[i]).collect::<Vec<_>>();
            let spent = |keep: bool| {
                of(keep)
                    .map(|i| u8::try_from(i).unwrap())
                    .collect::<Vec<_>>()
            };
            assert_eq!((&stacks[0].hp, &stacks[1].hp), (&hp(false), &hp(true)));
            assert_eq!(
                (&stacks[0].spent, &stacks[1].spent),
                (&spent(false), &spent(true))
            );
            // No kill is counted: each stack began as many as it holds.
            for stack in stacks {
                assert_eq!(usize::from(stack.initial), stack.hp.len());
            }
        }
    }
    assert!(none > 0 && all > 0 && some > 3, "{none} {all} {some}");
}

/// A seed and the wave's events where some of six goblins failed and some held.
fn split(data: &Data, hp: &[i32]) -> (World, Vec<Event>) {
    for seed in 0..40 {
        let mut world = against(data, seed, &[hp]);
        let events = wave(&mut world, data, 0);
        if moves(&events)
            .iter()
            .any(|e| matches!(e, Event::StackSplit { .. }))
        {
            return (world, events);
        }
    }
    panic!("no seed split the stack");
}

#[test]
fn the_dead_are_not_thrown_and_the_living_who_failed_are() {
    let data = data();
    // Every other goblin dies of any wave; the rest live through it.
    let hp = [100, 1, 100, 1, 100, 1];
    let mut seen = 0;
    for seed in 0..40 {
        let mut world = against(&data, seed, &[&hp]);
        let events = wave(&mut world, &data, 0);
        let sturdy = failed(&events, 0).iter().filter(|i| *i % 2 == 0).count();
        let stacks = &state(&world).encounter.stacks;
        let living: usize = stacks.iter().map(|s| s.hp.len()).sum();
        assert_eq!(living, 3, "seed {seed}");
        assert!(stacks.iter().all(|s| s.hp.iter().all(|hp| *hp > 50)));
        let thrown = match moves(&events).first() {
            Some(Event::StackSplit { .. }) => stacks[1].hp.len(),
            Some(_) => 3,
            None => 0,
        };
        assert_eq!(thrown, sturdy, "seed {seed}");
        assert_eq!(whole(&events), sturdy == 3, "seed {seed}");
        // The three dead are counted once, in the stack they died in.
        let kills: usize = stacks
            .iter()
            .map(|s| usize::from(s.initial) - s.hp.len())
            .sum();
        assert_eq!(kills, 3);
        if let Some(Event::StackSplit { moved, .. }) = moves(&events).first() {
            seen += 1;
            assert_eq!(moved.len(), sturdy);
            assert!(moved.iter().all(|i| usize::from(*i) < 3), "{moved:?}");
        }
    }
    assert!(seen > 3, "{seen}");
}

#[test]
fn a_thrown_stack_has_its_turn_its_reactions_and_its_shields() {
    let data = data();
    let hp = [100; 6];
    let mut found = None;
    for seed in 0..40 {
        let mut world = against(&data, seed, &[&hp]);
        state_mut(&mut world).monster_shields = (0..6).map(|i| (0, i)).collect();
        let before = state(&world).clone();
        let events = wave(&mut world, &data, 0);
        if matches!(moves(&events).first(), Some(Event::StackSplit { .. })) {
            found = Some((world, before));
            break;
        }
    }
    let (mut world, before) = found.expect("a seed that splits");
    let now = state(&world);
    // Its place in the order: straight after the stack it left, at the same count.
    let place = |state: &CombatState, stack: u8| {
        let at = |e: &omnis_sim::Initiative| e.actor == ActorRef::Stack(stack);
        state.order.iter().position(at)
    };
    let parent = place(now, 0).expect("the parent");
    assert_eq!(place(now, 1), Some(parent + 1));
    assert_eq!(now.order.len(), before.order.len() + 1);
    assert_eq!(now.order[parent + 1].total, now.order[parent].total);
    // The others keep their order.
    let rest = |state: &CombatState| {
        let others = state.order.iter().filter(|e| e.actor != ActorRef::Stack(1));
        others.map(|e| e.actor).collect::<Vec<_>>()
    };
    assert_eq!(rest(now), rest(&before));
    // Reactions as its parent has them; every shield still on a living individual.
    let reactions = |stack: u8| {
        let mine = now
            .reactions
            .iter()
            .find(|(a, _)| *a == ActorRef::Stack(stack));
        mine.map(|(_, left)| *left)
    };
    assert_eq!(reactions(1), reactions(0));
    assert!(reactions(1).is_some());
    let mut shields = Vec::new();
    for (index, stack) in now.encounter.stacks.iter().enumerate() {
        for individual in 0..stack.hp.len() {
            shields.push((
                u8::try_from(index).unwrap(),
                u8::try_from(individual).unwrap(),
            ));
        }
    }
    assert_eq!(now.monster_shields, shields);
    // It closes again on its own turn, within a round.
    let round = now.round;
    let mut closed = false;
    for _ in 0..12 {
        if state(&world).round > round + 1 {
            break;
        }
        let events = apply(&mut world, &data, Command::Combat(CombatCommand::EndTurn)).unwrap();
        closed |= events.iter().any(|e| {
            matches!(
                e,
                Event::StackMoved {
                    stack: 1,
                    from: 15,
                    to: 5
                }
            )
        });
        if !matches!(world.mode, Mode::Combat(_)) {
            break;
        }
    }
    assert!(closed, "the thrown stack came back");
}

#[test]
fn a_fight_full_of_stacks_moves_the_stack_whole_or_not_at_all() {
    let data = data();
    let one: &[i32] = &[100; 4];
    let stacks = [one; 12];
    let (mut moved, mut held) = (0, 0);
    for seed in 0..40 {
        let mut world = against(&data, seed, &stacks);
        let events = wave(&mut world, &data, 0);
        let failed = failed(&events, 0).len();
        let now = state(&world);
        assert_eq!(now.encounter.stacks.len(), 12);
        assert!(!events.iter().any(|e| matches!(e, Event::StackSplit { .. })));
        let thrown = whole(&events);
        assert_eq!(thrown, failed * 2 >= 4, "seed {seed}: {failed} of 4 failed");
        moved += usize::from(thrown);
        held += usize::from(!thrown);
    }
    assert!(moved > 2 && held > 2, "{moved} {held}");
}

#[test]
fn a_fight_saved_after_a_split_comes_back_as_it_was() {
    let data = data();
    let (world, _) = split(&data, &[100; 6]);
    let text = world.to_ron().expect("written");
    let back = World::from_ron(&text, &data, false).expect("read");
    assert_eq!(back.to_ron().unwrap(), text);
    assert_eq!(back.fingerprint().unwrap(), world.fingerprint().unwrap());
    // And plays on: the thrown stack is still there to be fought.
    let mut back = back;
    apply(&mut back, &data, Command::Combat(CombatCommand::EndTurn)).expect("a turn ended");
    assert_eq!(state(&back).encounter.stacks.len(), 2);
}

/// The stacks within their reach fill the front's squares nearest first, whatever their order.
#[test]
fn the_nearer_stack_takes_the_front_s_squares_first() {
    let mut data = data();
    let goblin = data.registry.monsters.get("base:monster:goblin").unwrap();
    for attack in &mut data.monsters.get_mut(&goblin).unwrap().attacks {
        if !attack.ranged {
            attack.reach = 10;
        }
    }
    let one: &[i32] = &[100; 6];
    let mut world = against(&data, 1, &[one, one]);
    common::stand_back(&mut world, 0, 10);
    let slots = |stack: u8| state(&world).melee_slots(&world.party, &data, stack);
    assert!(slots(1) > 0, "the engaged stack strikes");
    assert_eq!(slots(0), 0, "the one behind it has no square left");
}

/// What a client draws and offers: the thrown stack is in the fight's view at its distance,
/// and a row among the acting member's targets.
#[test]
fn the_thrown_stack_is_in_the_view_and_among_the_targets() {
    let data = data();
    let (world, events) = split(&data, &[100; 6]);
    let Some(Event::StackSplit { moved, .. }) = moves(&events).first().copied() else {
        panic!("a split");
    };
    let view = omnis_sim::combat_view(&world, &data).expect("a fight");
    assert_eq!(view.stacks.len(), 2);
    let thrown = &view.stacks[1];
    assert_eq!((thrown.stack, thrown.hps.len()), (1, moved.len()));
    assert_eq!(thrown.monster, view.stacks[0].monster);
    assert_eq!(usize::from(thrown.initial), moved.len());
    let Some(ActorRef::Member(acting)) = view.current else {
        panic!("a member's turn: {:?}", view.current);
    };
    let rows = omnis_sim::targets(&world, &data, acting, &omnis_sim::Chosen::Attack).unwrap();
    assert!(
        rows.iter().any(|row| row.target == Target::Stack(1)),
        "{rows:?}"
    );
}

/// The wave cast from the road at a group in touch opens the fight and throws back as in one.
#[test]
fn the_wave_from_the_road_throws_back_too() {
    let data = crate::objects::objects();
    let mut thrown = 0;
    for seed in 0..30 {
        let mut world = crate::opening_strike::at(&data, 4, 1, omnis_core::Facing::East);
        world.seed = seed;
        world.rngs.clear();
        let wizard = world.party.members[WIZARD].id;
        let cast = Command::Cast {
            caster: wizard,
            spell: "base:spell:thunderwave".into(),
            target: Target::Being {
                being: "rat".into(),
            },
        };
        let events = apply(&mut world, &data, cast).expect("the wave from the road");
        // Who failed and lived through it: a save failed, and no death of that one follows.
        let dead = |stack: u8, index: u8| {
            let target = ActorRef::Monster { stack, index };
            events
                .iter()
                .any(|e| matches!(e, Event::Death { target: t, .. } if *t == target))
        };
        let lived = failed(&events, 0)
            .iter()
            .filter(|i| !dead(0, u8::try_from(**i).unwrap()))
            .count();
        let any_death = events.iter().any(|e| matches!(e, Event::Death { .. }));
        if lived > 0 && !any_death {
            thrown += 1;
            assert_eq!(moves(&events).len(), 1, "seed {seed}: {events:?}");
        }
        if lived == 0 {
            assert!(moves(&events).is_empty(), "seed {seed}");
        }
    }
    assert!(thrown > 0, "no seed threw a rat back from the road");
}
