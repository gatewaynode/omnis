//! The measurement harness (LESSONS 2026-09-13: first content is measured before the owner
//! plays it). Run deliberately:
//! `cargo test -p omnis-sim --test measure -- --ignored --nocapture`. It prints, for parties of
//! two and six and for an attack-only policy against a cast-every-turn one, over 300 seeds of
//! two fights (the dungeon's rat pair and three goblins): the wipe rate, the mean rounds, the
//! mean spell points spent, and how often a pool emptied. `ambush_over_seeds` measures resting
//! in the dungeon (M7 step 5): how often a spent party is wiped when its rest is ambushed, what
//! that costs per 100 rests at a few chances, and the ambush rate the slots give.
//! `clear_over_seeds` measures one clear of the dungeon (M7b step 11) by parties of two, four
//! and six, going back to town to rest and train when spent: the wipe rate, the trips, the
//! experience and gold a clear brings, the level reached, and the trainer's price against that
//! income (levels the purse could not pay, gold left).

#[path = "../common/mod.rs"]
mod common;

mod boss;
mod budget;
mod clear;

use common::{data, encounter, party_of, reachable_stack};
use omnis_core::{Facing, Position};
use omnis_data::{Data, Disposition, SpellEffect};
use omnis_sim::{
    ActorRef, CombatCommand, CombatOutcome, Command, EncounterChoice, Event, Mode, Pay,
    RestCommand, Settings, Surprise, Target, World, apply, combat, combat_view,
};

pub(crate) const SEEDS: u64 = 300;

/// What a member does on their turn.
pub(crate) type Policy = fn(&World, &Data) -> Command;

#[derive(Default)]
struct Tally {
    fights: u32,
    wipes: u32,
    rounds: u64,
    points: u64,
    emptied: u32,
}

/// Cast something useful on the acting member's turn, else attack the nearest reachable
/// stack: heal a member under half, an area save at the biggest stack, a levelled bolt, a
/// cantrip, a buff once, in that order, whatever the pool allows.
pub(crate) fn cast_or_attack(world: &World, data: &Data) -> Command {
    choose(world, data, false)
}

/// [`cast_or_attack`], casting only cantrips when `cantrips_only` (the turn has cast a spell
/// with its bonus action).
pub(crate) fn choose(world: &World, data: &Data, cantrips_only: bool) -> Command {
    let view = combat_view(world, data).expect("a fight");
    let Some(ActorRef::Member(id)) = view.current else {
        return Command::Combat(CombatCommand::Dodge);
    };
    let own = world.party.members.iter().position(|m| m.id == id).unwrap();
    let hurt = world
        .party
        .members
        .iter()
        .enumerate()
        .filter(|(_, m)| m.hp > 0 && m.hp * 2 < m.hp_max)
        .min_by_key(|(_, m)| m.hp)
        .map(|(i, _)| u8::try_from(i).unwrap());
    let biggest = view
        .stacks
        .iter()
        .filter(|s| s.alive)
        .max_by_key(|s| s.hp.len())
        .map(|s| s.index);
    let front = view.stacks.iter().find(|s| s.alive).map(|s| s.index);
    let effect = |index: u8| {
        let id = world.party.members[own].known_spells[usize::from(index)];
        data.spells[&id].effect.clone()
    };
    let level = |index: u8| {
        let id = world.party.members[own].known_spells[usize::from(index)];
        data.spells[&id].level
    };
    let castable: Vec<&omnis_sim::SpellView> = view
        .spells
        .iter()
        .filter(|s| s.blocked.is_none() && (!cantrips_only || level(s.index) == 0))
        .collect();
    let pick = |wanted: &dyn Fn(&SpellEffect) -> bool, target: Option<Target>| {
        castable
            .iter()
            .find(|s| effect(s.index).is_some_and(|e| wanted(&e)))
            .and_then(|s| {
                target.map(|t| {
                    Command::Combat(CombatCommand::Cast {
                        spell: s.index,
                        target: t,
                        pay: Pay::Action,
                    })
                })
            })
    };
    let blessed = world.party.members.iter().any(|m| !m.effects.is_empty());
    let choice = pick(
        &|e| matches!(e, SpellEffect::Heal { .. }),
        hurt.map(Target::Member),
    )
    .or_else(|| {
        pick(
            &|e| matches!(e, SpellEffect::Save { .. }),
            biggest.map(Target::Stack),
        )
    })
    .or_else(|| {
        pick(
            &|e| matches!(e, SpellEffect::AutoHit { .. }),
            front.map(Target::Stack),
        )
    })
    .or_else(|| {
        pick(
            &|e| matches!(e, SpellEffect::Attack { .. }),
            front.map(Target::Stack),
        )
    })
    .or_else(|| {
        if blessed {
            None
        } else {
            pick(
                &|e| {
                    matches!(
                        e,
                        SpellEffect::Buff {
                            consumed: false,
                            ..
                        }
                    )
                },
                Some(Target::Member(0)),
            )
        }
    });
    choice.unwrap_or_else(|| attack_only(world, data))
}

pub(crate) fn attack_only(world: &World, data: &Data) -> Command {
    match reachable_stack(world, data) {
        Some(stack) => Command::Combat(CombatCommand::Attack { stack }),
        None => Command::Combat(CombatCommand::Dodge),
    }
}

fn run_fight(
    data: &Data,
    seed: u64,
    members: usize,
    stacks: &[(&str, u8)],
    policy: Policy,
    tally: &mut Tally,
) {
    let mut world = common::new_world(data, seed, Settings::default());
    party_of(&mut world, data, members);
    let here = world.position;
    let encounter = encounter(data, stacks, Disposition::Hostile, here);
    let mut events = Vec::new();
    combat::start(&mut world, data, encounter, Surprise::None, &mut events).unwrap();
    let mut rounds = 1u64;
    let mut points = 0u64;
    let mut outcome = None;
    for _ in 0..600 {
        if !matches!(world.mode, Mode::Combat(_)) {
            break;
        }
        let command = policy(&world, data);
        let events = common::act(&mut world, data, command).unwrap_or_else(|r| panic!("{r}"));
        for event in &events {
            match event {
                Event::RoundStarted { round } => rounds = rounds.max(u64::from(*round)),
                Event::SpellCast { points: p, .. } => points += u64::from(*p),
                Event::CombatEnded { outcome: o, .. } => outcome = Some(*o),
                _ => {}
            }
        }
    }
    tally.fights += 1;
    if outcome == Some(CombatOutcome::Defeat) || outcome.is_none() {
        tally.wipes += 1;
    }
    tally.rounds += rounds;
    tally.points += points;
    if world
        .party
        .members
        .iter()
        .any(|m| m.spell_points_max > 0 && m.spell_points == 0)
    {
        tally.emptied += 1;
    }
}

#[test]
#[ignore = "the measurement harness; run with --nocapture to read the table"]
fn casting_over_seeds() {
    let data = data();
    let fights: [(&str, &[(&str, u8)]); 2] = [
        ("rats x2 (the placement)", &[("giant_rat", 2)]),
        ("goblins x3", &[("goblin", 3)]),
    ];
    let policies: [(&str, Policy); 2] = [
        ("attack only", attack_only),
        ("cast every turn", cast_or_attack),
    ];
    println!(
        "{:<26} {:>7} {:<16} {:>6} {:>7} {:>7} {:>8}",
        "fight", "members", "policy", "wipe%", "rounds", "points", "emptied%"
    );
    for (name, stacks) in fights {
        for members in [2usize, 6] {
            for (policy_name, policy) in policies {
                let mut tally = Tally::default();
                for seed in 0..SEEDS {
                    run_fight(&data, seed, members, stacks, policy, &mut tally);
                }
                let f = u64::from(tally.fights.max(1));
                // Integers only in a simulation crate: percentages and means in tenths and
                // hundredths, printed with their decimal point.
                let per_mille = |n: u32| u64::from(n) * 1000 / f;
                println!(
                    "{:<26} {:>7} {:<16} {:>6} {:>7} {:>7} {:>8}",
                    name,
                    members,
                    policy_name,
                    tenths(per_mille(tally.wipes)),
                    hundredths(tally.rounds * 100 / f),
                    hundredths(tally.points * 100 / f),
                    tenths(per_mille(tally.emptied))
                );
            }
        }
    }
}

/// A new game of `members` at level 1 lying at the dungeon's entrance, spent: each at a
/// quarter of their hit points (at least 1) with no spell points, food for a night.
fn spent_in_the_dungeon(data: &Data, seed: u64, members: usize) -> World {
    let mut world = World::new(data, seed, Settings::default()).unwrap();
    party_of(&mut world, data, members);
    world.position = Position {
        map: data.registry.maps.get("test:map:dungeon").unwrap(),
        x: 1,
        y: 0,
        facing: Facing::South,
    };
    world.party.food = 10;
    for member in &mut world.party.members {
        member.hp = (member.hp_max / 4).max(1);
        member.spell_points = 0;
    }
    world
}

/// Rest, and when the rest is ambushed, fight whatever came under `policy`; whether the
/// party was ambushed and whether it was wiped.
fn rest_and_fight(
    data: &Data,
    world: &mut World,
    command: RestCommand,
    policy: Policy,
) -> (bool, bool) {
    apply(world, data, Command::Rest(command)).unwrap_or_else(|r| panic!("{r}"));
    if !matches!(world.mode, Mode::Encounter(_)) {
        return (false, false);
    }
    apply(world, data, Command::Encounter(EncounterChoice::Attack)).unwrap();
    let mut outcome = None;
    for _ in 0..600 {
        if !matches!(world.mode, Mode::Combat(_)) {
            break;
        }
        let command = policy(world, data);
        for event in common::act(world, data, command).unwrap_or_else(|r| panic!("{r}")) {
            if let Event::CombatEnded { outcome: o, .. } = event {
                outcome = Some(o);
            }
        }
    }
    (true, outcome != Some(CombatOutcome::Victory))
}

#[test]
#[ignore = "the measurement harness; run with --nocapture to read the table"]
fn ambush_over_seeds() {
    let data = data();
    // Every rest ambushed: the dungeon's table at 100% makes both slots return 1000 per mille
    // or more.
    let mut sure = data.clone();
    let dungeon = sure.registry.maps.get("test:map:dungeon").unwrap();
    if let Some(random) = sure.maps.get_mut(&dungeon).unwrap().random.as_mut() {
        random.chance_percent = 100;
    }
    let policies: [(&str, Policy); 2] = [
        ("attack only", attack_only),
        ("cast every turn", cast_or_attack),
    ];
    println!("A spent level-1 party ambushed at rest in the dungeon, fighting every ambush.");
    println!(
        "{:>7} {:<16} {:>6} {:>13} {:>13} {:>13}",
        "members", "policy", "wipe%", "wipes/100@10", "wipes/100@30", "wipes/100@50"
    );
    for members in [2usize, 6] {
        for (name, policy) in policies {
            let mut wipes = 0u64;
            for seed in 0..SEEDS {
                let mut world = spent_in_the_dungeon(&sure, seed, members);
                let (ambushed, wiped) =
                    rest_and_fight(&sure, &mut world, RestCommand::Long, policy);
                assert!(ambushed, "a sure ambush");
                wipes += u64::from(wiped);
            }
            // Per mille of ambushes that wipe; wipes per 100 rests at c per mille, in
            // hundredths: c * wipe‰ / 100.
            let wipe = wipes * 1000 / SEEDS;
            let at = |chance: u64| hundredths(chance * wipe / 100);
            println!(
                "{:>7} {:<16} {:>6} {:>13} {:>13} {:>13}",
                members,
                name,
                tenths(wipe),
                at(10),
                at(30),
                at(50)
            );
        }
    }
    let rests = 3000u64;
    for (label, command) in [
        ("long", RestCommand::Long),
        ("short", RestCommand::Short { dice: Vec::new() }),
    ] {
        let mut ambushes = 0u64;
        for seed in 0..rests {
            let mut world = spent_in_the_dungeon(&data, seed, 2);
            apply(&mut world, &data, Command::Rest(command.clone())).unwrap();
            ambushes += u64::from(matches!(world.mode, Mode::Encounter(_)));
        }
        println!(
            "{label} rests ambushed at the slots' chance: {ambushes} of {rests} ({} per mille)",
            tenths(ambushes * 10_000 / rests)
        );
    }
}

/// The dungeon's maps, in the order a clear goes down them; a map the packs lack is skipped.
/// `n` tenths as `x.y`.
pub(crate) fn tenths(n: u64) -> String {
    format!("{}.{}", n / 10, n % 10)
}

/// `n` hundredths as `x.yz`.
pub(crate) fn hundredths(n: u64) -> String {
    format!("{}.{:02}", n / 100, n % 100)
}
