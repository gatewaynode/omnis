//! Bob the Rat King measured (M7c): his group alone (three and two giant rats in front, Bob
//! behind), fought by new parties of two, four and six brought to levels 1, 2 and 3 with full
//! hit points and points, playing the whole turn budget with Shield declared. Per fight: Bob's
//! casts by spell, his shields, the party's shields, the missiles shields stopped on each side,
//! and the share of Thunderwave saves the members made.

use super::budget::budgeted;
use super::clear::{declare_shield, placed};
use super::common::{data, party_of};
use super::{hundredths, tenths};
use omnis_core::{Pcg32, SpellId, StreamName};
use omnis_data::Data;
use omnis_sim::omnis_rules::{ActionRef, eligible, level_up};
use omnis_sim::{
    ActorRef, CheckKind, CombatOutcome, Event, Mode, Settings, Surprise, World, apply, combat,
};

const SEEDS: u64 = 3000;

/// A new party of `members` raised to `level`, rested, its picks taken in list order.
fn party_at(data: &Data, seed: u64, members: usize, level: u8) -> World {
    let mut world = World::new(data, seed, Settings::default()).unwrap();
    party_of(&mut world, data, members);
    let mut rng = Pcg32::for_stream(seed, &StreamName::new("town"));
    for member in &mut world.party.members {
        while member.level < level {
            level_up(member, data, &mut rng).unwrap();
        }
        member.hp = member.hp_max;
        member.spell_points = member.spell_points_max;
        for spell in eligible(member, data).unwrap() {
            if member.spell_picks == 0 {
                break;
            }
            member.known_spells.push(spell);
            member.spell_picks -= 1;
        }
    }
    world
}

#[derive(Default)]
struct Tally {
    fights: u64,
    wipes: u64,
    rounds: u64,
    /// Fire Bolt, Magic Missile, Thunderwave, Shield.
    casts: [u64; 4],
    party_shields: u64,
    /// Missiles stopped at a member, at Bob.
    stopped: [u64; 2],
    /// Thunderwave saves made, rolled.
    saves: [u64; 2],
}

fn fight(data: &Data, world: &mut World, spells: &[SpellId; 4], tally: &mut Tally) {
    let depths = data.registry.maps.get("test:map:depths").unwrap();
    declare_shield(world, data);
    let encounter = placed(data, world, depths, 4);
    let mut events = Vec::new();
    combat::start(world, data, encounter, Surprise::None, &mut events).unwrap();
    let ids = spells.map(|s| data.registry.spells.name(s).unwrap().to_owned());
    let (mut rounds, mut outcome, mut last) = (1, None, None);
    for _ in 0..2000 {
        for event in &events {
            match event {
                Event::RoundStarted { round } => rounds = u64::from(*round),
                Event::CombatEnded { outcome: o, .. } => outcome = Some(*o),
                Event::MonsterCast { spell, .. } => {
                    last = Some(spell.clone());
                    if let Some(at) = ids.iter().position(|s| s == spell) {
                        tally.casts[at] += 1;
                    }
                }
                Event::Reaction {
                    action: ActionRef::Spell(spell),
                    ..
                } if *spell == ids[3] => tally.party_shields += 1,
                Event::ShieldStops { target } => {
                    let at = usize::from(!matches!(target, ActorRef::Member(_)));
                    tally.stopped[at] += 1;
                }
                Event::Check {
                    actor: ActorRef::Member(_),
                    kind: CheckKind::Save(_),
                    dc: 13,
                    success,
                    ..
                } if last.as_ref() == Some(&ids[2]) => {
                    tally.saves[0] += u64::from(*success);
                    tally.saves[1] += 1;
                }
                _ => {}
            }
        }
        if !matches!(world.mode, Mode::Combat(_)) {
            break;
        }
        events = apply(world, data, budgeted(world, data)).unwrap_or_else(|r| panic!("{r}"));
    }
    tally.fights += 1;
    tally.rounds += rounds;
    if outcome != Some(CombatOutcome::Victory) {
        tally.wipes += 1;
    }
}

#[test]
#[ignore = "the measurement harness; run with --nocapture to read the table"]
fn boss_over_seeds() {
    let data = data();
    let spell = |name: &str| {
        data.registry
            .spells
            .get(&format!("base:spell:{name}"))
            .unwrap()
    };
    let spells = [
        spell("fire_bolt"),
        spell("magic_missile"),
        spell("thunderwave"),
        spell("shield"),
    ];
    println!(
        "Bob the Rat King's group alone, {SEEDS} seeds, the whole budget with Shield declared. \
         Per fight: Bob's casts, shields on each side, missiles stopped at a member and at Bob; \
         the share of Thunderwave saves made."
    );
    println!(
        "{:>7} {:>5} {:>6} {:>6} {:>5} {:>7} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "members",
        "level",
        "wipe%",
        "rounds",
        "bolt",
        "missile",
        "wave",
        "bob sh",
        "pty sh",
        "stop@m",
        "stop@b",
        "saves%"
    );
    for members in [2usize, 4, 6] {
        for level in 1..=3u8 {
            let mut tally = Tally::default();
            for seed in 0..SEEDS {
                let mut world = party_at(&data, seed, members, level);
                fight(&data, &mut world, &spells, &mut tally);
            }
            let f = tally.fights.max(1);
            let per = |n: u64| hundredths(n * 100 / f);
            println!(
                "{:>7} {:>5} {:>6} {:>6} {:>5} {:>7} {:>5} {:>6} {:>6} {:>6} {:>6} {:>6}",
                members,
                level,
                tenths(tally.wipes * 1000 / f),
                per(tally.rounds),
                per(tally.casts[0]),
                per(tally.casts[1]),
                per(tally.casts[2]),
                per(tally.casts[3]),
                per(tally.party_shields),
                per(tally.stopped[0]),
                per(tally.stopped[1]),
                tenths(tally.saves[0] * 1000 / tally.saves[1].max(1))
            );
        }
    }
}
