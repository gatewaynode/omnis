//! Monsters that cast (M7c, Bob the Rat King in the test pack): a dice roll among his staff and
//! the spells his points pay for; Fire Bolt as a +5 spell attack the members' declared Shield
//! answers; Magic Missile stopped by a shield (SRD); Thunderwave's Constitution saves, DC 13,
//! for the front row; Bob's own Shield as his stack's reaction; points and shields kept with the
//! right individual when one dies. Party: Brenna (fighter), Durin (cleric), Ilvara (wizard) in
//! front, Pip (rogue) behind.

use crate::common;

use common::{act, data, party_of};
use omnis_core::{MonsterId, SpellId};
use omnis_data::{Data, Disposition};
use omnis_sim::omnis_rules::{
    ActionRef, Criteria, CriteriaSet, EffectKind, Named, Predicate, Trigger,
};
use omnis_sim::tactics::TacticsCommand;
use omnis_sim::{
    ActorRef, CheckKind, CombatCommand, Command, EncounterSource, EncounterState, Event, Mode,
    PartyCommand, Pay, Settings, Stack, Surprise, Target, World, apply, combat,
};

const ILVARA: usize = 2;

fn bob(data: &Data) -> MonsterId {
    data.registry
        .monsters
        .get("test:monster:bob_the_rat_king")
        .unwrap()
}

/// A base spell's string id, as events name it.
fn named(name: &str) -> String {
    format!("base:spell:{name}")
}

fn spell(data: &Data, name: &str) -> SpellId {
    data.registry
        .spells
        .get(&format!("base:spell:{name}"))
        .unwrap()
}

/// Bob's data with only these spells and, unless `staff`, no weapon.
fn bob_casting(data: &mut Data, spells: &[&str], staff: bool) {
    let bob = bob(data);
    let m = data.monsters.get_mut(&bob).unwrap();
    m.casting.as_mut().unwrap().spells = spells.iter().map(|s| format!("base:spell:{s}")).collect();
    if !staff {
        m.attacks.clear();
    }
}

/// A fight against `count` Bobs at `hp` each, in front, with `members` members readied by
/// `ready` before it starts; the world and the events of the start (Bob may act in them).
fn against_bob(
    data: &Data,
    seed: u64,
    (members, count, hp): (usize, u8, i32),
    ready: impl FnOnce(&mut World),
) -> (World, Vec<Event>) {
    let mut world = common::new_world(data, seed, Settings::default());
    party_of(&mut world, data, members);
    ready(&mut world);
    let encounter = EncounterState {
        source: EncounterSource::Random,
        stacks: vec![Stack {
            monster: bob(data),
            initial: count,
            hp: vec![hp; usize::from(count)],
            spent: Vec::new(),
        }],
        disposition: Disposition::Hostile,
        retreat: world.position,
    };
    let mut events = Vec::new();
    combat::start(&mut world, data, encounter, Surprise::None, &mut events).unwrap();
    (world, events)
}

fn state(world: &mut World) -> &mut omnis_sim::CombatState {
    match &mut world.mode {
        Mode::Combat(state) => state,
        other => panic!("no fight: {other:?}"),
    }
}

fn declare_shield(world: &mut World, data: &Data, slot: usize) {
    let member = world.party.members[slot].id;
    let set = CriteriaSet::<Named> {
        name: "Shield".to_owned(),
        action: ActionRef::Spell("base:spell:shield".to_owned()),
        trigger: Trigger::Attacked,
        when: Criteria::Is(Predicate::WouldChangeOutcome),
    };
    let put = TacticsCommand::PutReaction {
        member,
        entry: None,
        set,
    };
    apply(world, data, Command::Party(PartyCommand::Tactics(put))).unwrap();
}

/// Bob's first turn: in the start's events when he won the initiative, else after ending
/// members' turns until he acts.
fn bob_turn(world: &mut World, data: &Data, start: &[Event]) -> Vec<Event> {
    if let Some(turn) = turn_of_bob(start) {
        return turn;
    }
    for _ in 0..50 {
        let events = act(world, data, Command::Combat(CombatCommand::EndTurn)).unwrap();
        if let Some(turn) = turn_of_bob(&events) {
            return turn;
        }
    }
    panic!("Bob never acted");
}

/// The events of the Bob stack's turn among `events`, if it came.
fn turn_of_bob(events: &[Event]) -> Option<Vec<Event>> {
    let mine = |e: &Event| {
        matches!(
            e,
            Event::Turn {
                actor: ActorRef::Stack(0)
            }
        )
    };
    let at = events.iter().position(mine)?;
    let rest = &events[at + 1..];
    let end = rest
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::Turn { .. } | Event::RoundStarted { .. } | Event::CombatEnded { .. }
            )
        })
        .unwrap_or(rest.len());
    Some(rest[..end].to_vec())
}

/// End turns until the member in `slot` acts.
fn until_member(world: &mut World, data: &Data, slot: usize) {
    for _ in 0..50 {
        let id = world.party.members[slot].id;
        if state(world).current_actor() == Some(ActorRef::Member(id)) {
            return;
        }
        act(world, data, Command::Combat(CombatCommand::EndTurn)).unwrap();
    }
    panic!("the turn never came");
}

fn bob_actor(index: u8) -> ActorRef {
    ActorRef::Monster { stack: 0, index }
}

/// What Bob's turn was: the spell he cast, or `None` for his staff.
fn choice(events: &[Event]) -> Option<String> {
    match events.first() {
        Some(Event::MonsterCast { spell, .. }) => Some(spell.clone()),
        Some(Event::AttackResolved { attacker, .. }) if *attacker == bob_actor(0) => None,
        other => panic!("Bob's turn began with {other:?}"),
    }
}

#[test]
fn bob_rolls_among_his_staff_and_the_spells_his_points_pay_for() {
    let data = data();
    let (bolt, missile, wave, shield) = (
        named("fire_bolt"),
        named("magic_missile"),
        named("thunderwave"),
        named("shield"),
    );
    let mut seen = Vec::new();
    for seed in 0..60 {
        let (mut world, start) = against_bob(&data, seed, (4, 1, 200), |_| {});
        let events = bob_turn(&mut world, &data, &start);
        let picked = choice(&events);
        assert_ne!(
            picked,
            Some(shield.clone()),
            "Shield is a reaction, never his turn"
        );
        if let Some(levelled) = picked.clone().filter(|s| *s == missile || *s == wave) {
            assert_eq!(
                state(&mut world).encounter.stacks[0].spent,
                vec![1],
                "{levelled:?} costs its level"
            );
        }
        if !seen.contains(&picked) {
            seen.push(picked);
        }
    }
    for wanted in [None, Some(bolt.clone()), Some(missile), Some(wave)] {
        assert!(seen.contains(&wanted), "{wanted:?} never rolled: {seen:?}");
    }

    // Spent out: the staff and the free cantrip remain.
    let mut data = data;
    data.monsters
        .get_mut(&bob(&data))
        .unwrap()
        .casting
        .as_mut()
        .unwrap()
        .points = 0;
    let mut seen = Vec::new();
    for seed in 0..40 {
        let (mut world, start) = against_bob(&data, seed, (4, 1, 200), |_| {});
        let picked = choice(&bob_turn(&mut world, &data, &start));
        assert!(
            picked.is_none() || picked == Some(bolt.clone()),
            "{picked:?}"
        );
        if !seen.contains(&picked) {
            seen.push(picked);
        }
    }
    assert_eq!(seen.len(), 2, "both remain: {seen:?}");
}

#[test]
fn fire_bolt_is_a_plus_five_spell_attack_of_2d10_that_a_declared_shield_answers() {
    let mut data = data();
    bob_casting(&mut data, &["fire_bolt"], false);
    let ilvara_shielded = (0..200).find_map(|seed| {
        let (mut world, start) = against_bob(&data, seed, (4, 1, 200), |w| {
            declare_shield(w, &data, ILVARA);
        });
        let ilvara = world.party.members[ILVARA].id;
        let events = bob_turn(&mut world, &data, &start);
        let Some(Event::AttackResolved { roll, hit, ac, .. }) = events
            .iter()
            .find(|e| matches!(e, Event::AttackResolved { .. }))
        else {
            panic!("no attack: {events:?}");
        };
        assert_eq!(roll.modifier + roll.proficiency, 5, "spell attack +5");
        if let Some(Event::Damage { rolls, .. }) =
            events.iter().find(|e| matches!(e, Event::Damage { .. }))
        {
            assert_eq!(
                (rolls[0].dice.count, rolls[0].dice.sides),
                (2, 10),
                "caster level 5"
            );
        }
        let reacted = events
            .iter()
            .any(|e| matches!(e, Event::Reaction { actor, .. } if *actor == ilvara));
        reacted.then(|| {
            assert!(!hit, "the shield turned the hit");
            let effects = &world.party.members[ILVARA].effects;
            assert!(
                effects
                    .iter()
                    .any(|e| matches!(e.kind, EffectKind::ArmorBonus(5)))
            );
            *ac
        })
    });
    let ac = ilvara_shielded.expect("a fire bolt the shield turned");
    assert!(ac >= 12, "judged at the shielded armor class: {ac}");
}

#[test]
fn magic_missile_stops_at_a_shield_raised_or_already_up() {
    let mut data = data();
    bob_casting(&mut data, &["magic_missile"], false);
    // Only Ilvara stands, so both missiles come at her.
    let (mut world, start) = against_bob(&data, 3, (3, 2, 200), |w| {
        declare_shield(w, &data, ILVARA);
        for slot in 0..2 {
            w.party.members[slot].hp = 0;
        }
    });
    let ilvara = world.party.members[ILVARA].id;
    let events = bob_turn(&mut world, &data, &start);
    let stops: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Event::ShieldStops { target } if *target == ActorRef::Member(ilvara)))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(stops.len(), 2, "both missiles stopped: {events:?}");
    let reactions = events
        .iter()
        .filter(|e| matches!(e, Event::Reaction { actor, .. } if *actor == ilvara))
        .count();
    assert_eq!(
        reactions, 1,
        "raised at the first, already up at the second"
    );
    assert!(
        !events.iter().any(
            |e| matches!(e, Event::Damage { target, .. } if *target == ActorRef::Member(ilvara))
        ),
        "no force damage through a shield"
    );

    // Without a shield declared the missile lands: 3d4 + 3.
    let (mut world, start) = against_bob(&data, 3, (3, 1, 200), |_| {});
    let events = bob_turn(&mut world, &data, &start);
    let Some(Event::Damage { amount, .. }) =
        events.iter().find(|e| matches!(e, Event::Damage { .. }))
    else {
        panic!("no damage: {events:?}");
    };
    assert!((6..=15).contains(amount), "{amount}");
}

#[test]
fn thunderwave_makes_each_front_member_save_on_constitution_at_dc_13() {
    let mut data = data();
    bob_casting(&mut data, &["thunderwave"], false);
    let (mut saved, mut failed) = (0, 0);
    for seed in 0..20 {
        let (mut world, start) = against_bob(&data, seed, (4, 1, 200), |_| {});
        let front: Vec<ActorRef> = (0..3)
            .map(|slot| ActorRef::Member(world.party.members[slot].id))
            .collect();
        let events = bob_turn(&mut world, &data, &start);
        let checks: Vec<(ActorRef, bool)> = events
            .iter()
            .filter_map(|e| match e {
                Event::Check {
                    actor,
                    kind: CheckKind::Save(omnis_data::Ability::Constitution),
                    dc: 13,
                    success,
                    ..
                } => Some((*actor, *success)),
                _ => None,
            })
            .collect();
        assert_eq!(
            checks.iter().map(|(a, _)| *a).collect::<Vec<_>>(),
            front,
            "the front row saves; Pip behind does not"
        );
        let damage: Vec<(i64, i64)> = events
            .iter()
            .filter_map(|e| match e {
                Event::Damage { raw, amount, .. } => Some((*raw, *amount)),
                _ => None,
            })
            .collect();
        assert_eq!(damage.len(), 3);
        assert!(
            damage.iter().all(|(raw, _)| *raw == damage[0].0),
            "one roll for all"
        );
        for ((_, success), (raw, amount)) in checks.iter().zip(&damage) {
            if *success {
                saved += 1;
                assert_eq!(*amount, raw / 2, "half on a save");
            } else {
                failed += 1;
                assert_eq!(amount, raw);
            }
        }
    }
    assert!(saved > 0 && failed > 0, "{saved} saved, {failed} failed");
}

#[test]
fn bob_shields_himself_from_a_missile_and_a_hit_once_a_round() {
    // Bob does nothing on his turns here: his only spell is his reaction.
    let mut data = data();
    bob_casting(&mut data, &["shield"], false);
    // Ilvara's Magic Missile at Bob: his reaction raises the shield and it stops the missile.
    let (mut world, _) = against_bob(&data, 5, (4, 1, 200), |_| {});
    until_member(&mut world, &data, ILVARA);
    assert!(
        world.party.members[ILVARA]
            .known_spells
            .contains(&spell(&data, "magic_missile"))
    );
    let events = act(
        &mut world,
        &data,
        Command::Combat(CombatCommand::Cast {
            spell: "base:spell:magic_missile".to_owned(),
            target: Target::Stack(0),
            pay: Pay::Action,
        }),
    )
    .unwrap();
    let at = events
        .iter()
        .position(|e| {
            *e == Event::MonsterCast {
                caster: bob_actor(0),
                spell: named("shield"),
            }
        })
        .expect("Bob shields");
    assert_eq!(
        events[at + 1],
        Event::ShieldStops {
            target: bob_actor(0)
        }
    );

    // A hit the shield turns: from a fresh fight, the first member attack that hits below
    // 17 and at least 12 draws it, and the roll is judged at 17.
    let turned = (0..100).find_map(|seed| {
        let (mut world, _) = against_bob(&data, seed, (1, 1, 200), |_| {});
        until_member(&mut world, &data, 0);
        // A second action keeps Brenna's turn, so the state is read before Bob's.
        state(&mut world).budget.actions = 2;
        let events = apply(
            &mut world,
            &data,
            Command::Combat(CombatCommand::Attack { stack: 0 }),
        )
        .unwrap();
        let at = events
            .iter()
            .position(|e| matches!(e, Event::MonsterCast { .. }))?;
        let Event::AttackResolved { hit, ac, roll, .. } = &events[at + 1] else {
            panic!("the shield comes before the attack's judgement: {events:?}");
        };
        let state = state(&mut world);
        Some((
            *hit,
            *ac,
            roll.total,
            state.reactions_left(ActorRef::Stack(0)),
            state.encounter.stacks[0].spent.clone(),
        ))
    });
    let (hit, ac, total, reactions, spent) = turned.expect("a hit the shield turned");
    assert!(
        !hit && ac == 17 && (12..17).contains(&total),
        "{total} at {ac}"
    );
    assert_eq!(reactions, 0, "the stack's reaction is spent");
    assert_eq!(spent, vec![1], "and one of Bob's points");

    // A shield already up judges the roll at 17, not only reports it: a 12 to 16 misses.
    let judged = (0..200).find_map(|seed| {
        let (mut world, _) = against_bob(&data, seed, (1, 1, 200), |_| {});
        until_member(&mut world, &data, 0);
        let s = state(&mut world);
        s.monster_shields = vec![(0, 0)];
        s.set_reactions(ActorRef::Stack(0), 0);
        let events = apply(
            &mut world,
            &data,
            Command::Combat(CombatCommand::Attack { stack: 0 }),
        )
        .unwrap();
        events.iter().find_map(|e| match e {
            Event::AttackResolved {
                roll, hit, crit, ..
            } if (12..17).contains(&roll.total) && !crit => Some(*hit),
            _ => None,
        })
    });
    assert_eq!(judged, Some(false), "a 12 to 16 misses the shielded 17");

    // A hit the shield would not turn draws no shield: the reaction and the points are kept.
    let kept = (0..200).find_map(|seed| {
        let (mut world, _) = against_bob(&data, seed, (1, 1, 200), |_| {});
        until_member(&mut world, &data, 0);
        let events = apply(
            &mut world,
            &data,
            Command::Combat(CombatCommand::Attack { stack: 0 }),
        )
        .unwrap();
        let big = events.iter().any(|e| {
            matches!(e, Event::AttackResolved { roll, hit: true, crit: false, .. } if roll.total >= 17)
        });
        big.then(|| {
            let shielded = events.iter().any(|e| matches!(e, Event::MonsterCast { .. }));
            let s = state(&mut world);
            (shielded, s.reactions_left(ActorRef::Stack(0)))
        })
    });
    assert_eq!(kept, Some((false, 1)), "a 17 or more is not worth a shield");

    // A shield already up: every attack this round is judged at 17, and it drops at his turn.
    let (mut world, _) = against_bob(&data, 7, (1, 1, 200), |_| {});
    until_member(&mut world, &data, 0);
    let s = state(&mut world);
    s.monster_shields = vec![(0, 0)];
    s.set_reactions(ActorRef::Stack(0), 0);
    let events = apply(
        &mut world,
        &data,
        Command::Combat(CombatCommand::Attack { stack: 0 }),
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::AttackResolved { ac: 17, .. })),
        "{events:?}"
    );
    act(&mut world, &data, Command::Combat(CombatCommand::EndTurn)).unwrap();
    let s = state(&mut world);
    assert!(s.monster_shields.is_empty(), "dropped at his turn");
    assert_eq!(
        s.reactions_left(ActorRef::Stack(0)),
        1,
        "and his reaction back"
    );
}

#[test]
fn points_and_shields_stay_with_their_individual_when_the_lead_dies() {
    let mut data = data();
    bob_casting(&mut data, &["shield"], false);
    let killed = (0..100).find_map(|seed| {
        let (mut world, _) = against_bob(&data, seed, (1, 2, 27), |_| {});
        until_member(&mut world, &data, 0);
        let s = state(&mut world);
        s.encounter.stacks[0].hp = vec![1, 27];
        s.encounter.stacks[0].spent = vec![3, 1];
        s.monster_shields = vec![(0, 0), (0, 1)];
        s.set_reactions(ActorRef::Stack(0), 0);
        // A second action keeps Brenna's turn, so the state is read before Bob's.
        state(&mut world).budget.actions = 2;
        let events = apply(
            &mut world,
            &data,
            Command::Combat(CombatCommand::Attack { stack: 0 }),
        )
        .unwrap();
        events
            .iter()
            .any(|e| matches!(e, Event::Death { target, .. } if *target == bob_actor(0)))
            .then(|| {
                let s = state(&mut world);
                (
                    s.encounter.stacks[0].hp.len(),
                    s.encounter.stacks[0].spent.clone(),
                    s.monster_shields.clone(),
                )
            })
    });
    let (living, spent, shields) = killed.expect("the lead Bob fell");
    assert_eq!(living, 1);
    assert_eq!(spent, vec![1], "the survivor keeps his own points");
    assert_eq!(
        shields,
        vec![(0, 0)],
        "the fallen one's shield goes, the survivor's moves up one"
    );
}
