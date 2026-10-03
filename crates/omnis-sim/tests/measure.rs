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

mod common;

use common::{data, encounter, party_of, reachable_stack};
use omnis_core::{Facing, Position};
use omnis_data::{Data, Disposition, SpellEffect};
use omnis_sim::{
    ActorRef, CombatCommand, CombatOutcome, Command, EncounterChoice, EncounterSource,
    EncounterState, Event, Mode, Pay, RestCommand, Settings, Stack, Surprise, Target, World, apply,
    combat, combat_view,
};

const SEEDS: u64 = 300;

/// What a member does on their turn.
type Policy = fn(&World, &Data) -> Command;

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
fn cast_or_attack(world: &World, data: &Data) -> Command {
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
    let castable: Vec<&omnis_sim::SpellView> =
        view.spells.iter().filter(|s| s.blocked.is_none()).collect();
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

fn attack_only(world: &World, data: &Data) -> Command {
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
const DUNGEON: [&str; 2] = ["test:map:dungeon", "test:map:depths"];

/// What one clear of the dungeon came to.
#[derive(Default)]
struct Clear {
    wiped: bool,
    fights: u32,
    trips: u32,
    gold: u64,
    /// Levels a member was ready for and the purse could not pay.
    unpaid: u32,
    /// Members raised at the temple on the way.
    raised: u32,
    /// The map (by its place in `DUNGEON`) and group a wipe came at.
    wiped_at: Option<(usize, usize)>,
}

/// Fight to the end under `policy`, adding the copper looted.
fn fight_out(world: &mut World, data: &Data, policy: Policy, clear: &mut Clear) {
    clear.fights += 1;
    for _ in 0..600 {
        if !matches!(world.mode, Mode::Combat(_)) {
            break;
        }
        let command = policy(world, data);
        for event in common::act(world, data, command).unwrap_or_else(|r| panic!("{r}")) {
            if let Event::CombatEnded { outcome, gold, .. } = event {
                clear.wiped |= outcome != CombatOutcome::Victory;
                clear.gold += u64::from(gold);
            }
        }
    }
    clear.wiped |= matches!(world.mode, Mode::Combat(_));
}

/// The placed group `index` of `map` as the game starts it on its tile: its own monsters and
/// counts, each stack's individuals at one roll of the rules' hit points, cleared by a victory
/// if it is `once`.
fn placed(data: &Data, world: &mut World, map: omnis_core::MapId, index: usize) -> EncounterState {
    let def = &data.maps[&map].encounters[index];
    world.position = Position {
        map,
        x: def.x,
        y: def.y,
        facing: Facing::North,
    };
    let stream = omnis_core::StreamName::new("combat");
    let mut rng = omnis_core::Pcg32::for_stream(world.seed ^ index as u64, &stream);
    let stacks = def
        .stacks
        .iter()
        .map(|(monster, count)| {
            let hp = omnis_sim::omnis_rules::monster_hit_points(
                &data.monsters[monster],
                data,
                &mut rng,
                &stream,
            )
            .unwrap();
            Stack {
                monster: *monster,
                initial: *count,
                hp: vec![hp; usize::from(*count)],
            }
        })
        .collect();
    EncounterState {
        source: EncounterSource::Fixed(u16::try_from(index).unwrap()),
        stacks,
        disposition: def.disposition,
        retreat: world.position,
    }
}

/// After a fight: an hour's rest spending every die of each member under half, and any
/// ambush it brings fought out.
fn short_rest(world: &mut World, data: &Data, policy: Policy, clear: &mut Clear) {
    let dice: Vec<u8> = world
        .party
        .members
        .iter()
        .map(|m| {
            if m.hp > 0 && m.hp * 2 < m.hp_max {
                m.level.saturating_sub(m.hit_dice_spent)
            } else {
                0
            }
        })
        .collect();
    if dice.iter().all(|d| *d == 0) {
        return;
    }
    if apply(world, data, Command::Rest(RestCommand::Short { dice })).is_err() {
        return;
    }
    if matches!(world.mode, Mode::Encounter(_)) {
        apply(world, data, Command::Encounter(EncounterChoice::Attack)).unwrap();
        fight_out(world, data, policy, clear);
    }
}

/// Spent: the party's hit points are under half of its maximum, or a member is down.
fn spent(world: &World) -> bool {
    let (hp, max) = world
        .party
        .members
        .iter()
        .fold((0, 0), |(h, m), c| (h + c.hp.max(0), m + c.hp_max));
    hp * 2 < max || world.party.members.iter().any(|m| m.hp <= 0)
}

/// Back to town: the temple raises the dead at `temple.raise_cost` (paid as far as the purse
/// goes) and cures the rest, the inn restores everyone (the walk and its random fights are not
/// modelled), the trainer grants every level the purse can pay for at `trainer.cost`, and each
/// caster takes the picks they are owed from their list in order.
fn to_town(world: &mut World, data: &Data, clear: &mut Clear) {
    clear.trips += 1;
    let mut rng = omnis_core::Pcg32::for_stream(world.seed, &omnis_core::StreamName::new("town"));
    let dead = omnis_sim::omnis_rules::condition_id(data, "dead");
    for index in 0..world.party.members.len() {
        let member = &mut world.party.members[index];
        if dead.is_some_and(|d| member.conditions.contains(&d)) {
            let cost = u32::try_from(slot(
                data,
                "temple.raise_cost",
                ("level", i64::from(member.level)),
            ))
            .unwrap();
            world.party.gold = world.party.gold.saturating_sub(cost);
            clear.raised += 1;
        }
        member.conditions.clear();
        member.death_saves = Default::default();
        member.hp = member.hp_max;
        member.spell_points = member.spell_points_max;
        member.hit_dice_spent = 0;
        while omnis_sim::omnis_rules::ready(member, data).unwrap() {
            let cost = training(data, member.level + 1);
            if cost > world.party.gold {
                clear.unpaid += 1;
                break;
            }
            world.party.gold -= cost;
            omnis_sim::omnis_rules::level_up(member, data, &mut rng).unwrap();
            member.hp = member.hp_max;
            member.spell_points = member.spell_points_max;
        }
        for spell in omnis_sim::omnis_rules::eligible(member, data).unwrap() {
            if member.spell_picks == 0 {
                break;
            }
            member.known_spells.push(spell);
            member.spell_picks -= 1;
        }
    }
}

/// `trainer.cost` for a level, in copper.
fn training(data: &Data, level: u8) -> u32 {
    u32::try_from(slot(data, "trainer.cost", ("level", i64::from(level)))).unwrap()
}

/// A one-input price slot, in copper.
fn slot(data: &Data, name: &str, input: (&str, i64)) -> u64 {
    let stream = omnis_core::StreamName::new("town");
    let mut rng = omnis_core::Pcg32::for_stream(0, &stream);
    let value = data
        .rules
        .eval(
            name,
            &[(input.0, omnis_data::omnis_expr::Value::Int(input.1))],
            &mut rng,
            &stream,
        )
        .unwrap()
        .value;
    u64::try_from(value.as_int().unwrap()).unwrap()
}

/// One clear by a new level-1 party: every `once` group of each dungeon map in file order, a
/// short rest after each fight, back to town whenever the party is spent and before each
/// deeper map. Returns the clear and the world at its end.
fn clear_dungeon(data: &Data, seed: u64, members: usize, policy: Policy) -> (Clear, World) {
    let mut world = World::new(data, seed, Settings::default()).unwrap();
    party_of(&mut world, data, members);
    let mut clear = Clear::default();
    for (n, name) in DUNGEON.iter().enumerate() {
        let Some(map) = data.registry.maps.get(name) else {
            continue;
        };
        if n > 0 {
            to_town(&mut world, data, &mut clear);
        }
        for index in 0..data.maps[&map].encounters.len() {
            if !data.maps[&map].encounters[index].once {
                continue;
            }
            if spent(&world) {
                to_town(&mut world, data, &mut clear);
            }
            let encounter = placed(data, &mut world, map, index);
            let mut events = Vec::new();
            combat::start(&mut world, data, encounter, Surprise::None, &mut events).unwrap();
            fight_out(&mut world, data, policy, &mut clear);
            if !clear.wiped {
                short_rest(&mut world, data, policy, &mut clear);
            }
            if clear.wiped {
                clear.wiped_at = Some((n, index));
                return (clear, world);
            }
        }
    }
    to_town(&mut world, data, &mut clear);
    (clear, world)
}

#[test]
#[ignore = "the measurement harness; run with --nocapture to read the table"]
fn clear_over_seeds() {
    let data = data();
    let maps: Vec<&str> = DUNGEON
        .iter()
        .copied()
        .filter(|m| data.registry.maps.get(m).is_some())
        .collect();
    println!(
        "One clear of {maps:?} by a new level-1 party, cast-or-attack, over {SEEDS} seeds: back to \
         town to rest and train when spent and before each deeper map."
    );
    println!(
        "Prices: train to 2 {} gp, to 3 {} gp a member; raise at level 1 {} gp.",
        hundredths(slot(&data, "trainer.cost", ("level", 2))),
        hundredths(slot(&data, "trainer.cost", ("level", 3))),
        hundredths(slot(&data, "temple.raise_cost", ("level", 1)))
    );
    println!(
        "{:>7} {:>6} {:>6} {:>7} {:>7} {:>9} {:>6} {:>7} {:>7} {:>8}",
        "members",
        "wipe%",
        "trips",
        "fights",
        "xp/mem",
        "gp/clear",
        "level",
        "raised",
        "unpaid",
        "gp left"
    );
    for members in [2usize, 4, 6] {
        let (mut wipes, mut done) = (0u64, 0u64);
        let (mut trips, mut fights, mut xp, mut gold) = (0u64, 0u64, 0u64, 0u64);
        let (mut levels, mut unpaid, mut left, mut raised) = (0u64, 0u64, 0u64, 0u64);
        let mut at = std::collections::BTreeMap::new();
        for seed in 0..SEEDS {
            let (clear, world) = clear_dungeon(&data, seed, members, cast_or_attack);
            if clear.wiped {
                wipes += 1;
                *at.entry(clear.wiped_at).or_insert(0u32) += 1;
                continue;
            }
            done += 1;
            trips += u64::from(clear.trips);
            fights += u64::from(clear.fights);
            xp += world
                .party
                .members
                .iter()
                .map(|m| u64::from(m.xp))
                .sum::<u64>();
            gold += clear.gold;
            unpaid += u64::from(clear.unpaid);
            raised += u64::from(clear.raised);
            left += u64::from(world.party.gold);
            levels += world
                .party
                .members
                .iter()
                .map(|m| u64::from(m.level))
                .sum::<u64>();
        }
        let d = done.max(1);
        let m = u64::try_from(members).unwrap();
        println!(
            "{:>7} {:>6} {:>6} {:>7} {:>7} {:>9} {:>6} {:>7} {:>7} {:>8}",
            members,
            tenths(wipes * 1000 / SEEDS),
            hundredths(trips * 100 / d),
            hundredths(fights * 100 / d),
            xp / (d * m),
            hundredths(gold / d),
            hundredths(levels * 100 / (d * m)),
            hundredths(raised * 100 / d),
            hundredths(unpaid * 100 / d),
            hundredths(left / d)
        );
        if !at.is_empty() {
            println!("        wipes by (map, group): {at:?}");
        }
    }
}

/// `n` tenths as `x.y`.
fn tenths(n: u64) -> String {
    format!("{}.{}", n / 10, n % 10)
}

/// `n` hundredths as `x.yz`.
fn hundredths(n: u64) -> String {
    format!("{}.{:02}", n / 100, n % 100)
}
