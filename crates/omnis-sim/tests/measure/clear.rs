//! One clear of the test dungeon (M7b step 11), played by a [`Play`]: a new level-1 party
//! fights every placed group in file order, rests short after each fight and goes back to town
//! to rest and train when spent and before each deeper map.

use super::common::{self, data, party_of};
use super::{Policy, SEEDS, cast_or_attack, hundredths, tenths};
use omnis_core::{Facing, Position};
use omnis_data::Data;
use omnis_sim::omnis_rules::{ActionRef, Criteria, CriteriaSet, Predicate, Trigger};
use omnis_sim::tactics::TacticsCommand;
use omnis_sim::{
    CombatCommand, CombatOutcome, Command, EncounterChoice, EncounterSource, EncounterState, Event,
    Mode, PartyCommand, Pay, RestCommand, Settings, Stack, Surprise, World, apply, combat,
};

/// How the party plays its fights.
#[derive(Clone, Copy)]
pub(crate) struct Play {
    /// The next command for the member whose turn it is.
    pub policy: Policy,
    /// The policy plays the whole turn and ends it; otherwise one command is the turn.
    pub whole_turns: bool,
    /// Members who know Shield declare it on being attacked when it would turn the hit.
    pub shield: bool,
}

/// M7b's play: one command a turn, cast or attack, nothing declared.
pub(crate) const ONE_COMMAND: Play = Play {
    policy: cast_or_attack,
    whole_turns: false,
    shield: false,
};

const DUNGEON: [&str; 2] = ["test:map:dungeon", "test:map:depths"];

/// What one clear of the dungeon came to.
#[derive(Default)]
pub(crate) struct Clear {
    pub wiped: bool,
    pub fights: u32,
    trips: u32,
    gold: u64,
    /// Levels a member was ready for and the purse could not pay.
    unpaid: u32,
    /// Members raised at the temple on the way.
    raised: u32,
    /// The map (by its place in `DUNGEON`) and group a wipe came at.
    wiped_at: Option<(usize, usize)>,
    /// Rounds fought, over every fight.
    pub rounds: u64,
    /// Declared reactions fired.
    pub reactions: u32,
    /// Opportunity attacks the monsters took.
    pub opportunity: u32,
    /// Second Wind, Action Surge, Cunning Action and spells paid with the bonus action.
    pub uses: [u32; 4],
}

/// Fight to the end under `play`, adding the copper looted and what was used.
fn fight_out(world: &mut World, data: &Data, play: Play, clear: &mut Clear) {
    clear.fights += 1;
    let mut rounds = 1;
    for _ in 0..2000 {
        if !matches!(world.mode, Mode::Combat(_)) {
            break;
        }
        let command = (play.policy)(world, data);
        if matches!(
            command,
            Command::Combat(CombatCommand::Cast {
                pay: Pay::BonusAction,
                ..
            })
        ) {
            clear.uses[3] += 1;
        }
        let events = if play.whole_turns {
            apply(world, data, command.clone())
        } else {
            common::act(world, data, command.clone())
        };
        for event in events.unwrap_or_else(|r| panic!("{command:?}: {r}")) {
            match event {
                Event::CombatEnded { outcome, gold, .. } => {
                    clear.wiped |= outcome != CombatOutcome::Victory;
                    clear.gold += u64::from(gold);
                }
                Event::RoundStarted { round } => rounds = u64::from(round),
                Event::Reaction { .. } => clear.reactions += 1,
                Event::OpportunityAttack { .. } => clear.opportunity += 1,
                Event::FeatureUsed { feature, .. } => {
                    let at = ["second_wind", "action_surge", "cunning"]
                        .iter()
                        .position(|name| feature.contains(name))
                        .unwrap_or_else(|| panic!("{feature}"));
                    clear.uses[at] += 1;
                }
                _ => {}
            }
        }
    }
    clear.rounds += rounds;
    clear.wiped |= matches!(world.mode, Mode::Combat(_));
}

/// Each member who knows Shield and has declared nothing declares it as the save migration
/// does: on being attacked, when the +5 would turn the hit.
fn declare_shield(world: &mut World, data: &Data) {
    let shield = data.registry.spells.get("base:spell:shield").unwrap();
    for member in 0..world.party.members.len() {
        let m = &world.party.members[member];
        if !m.known_spells.contains(&shield) || !m.tactics.reactions().is_empty() {
            continue;
        }
        let set = CriteriaSet {
            name: "Shield".to_owned(),
            action: ActionRef::Spell(shield),
            trigger: Trigger::Attacked,
            when: Criteria::Is(Predicate::WouldChangeOutcome),
        };
        let command = TacticsCommand::PutReaction {
            member: u8::try_from(member).unwrap(),
            at: None,
            set,
        };
        apply(world, data, Command::Party(PartyCommand::Tactics(command))).unwrap();
    }
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
                spent: Vec::new(),
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
fn short_rest(world: &mut World, data: &Data, play: Play, clear: &mut Clear) {
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
        fight_out(world, data, play, clear);
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
pub(crate) fn clear_dungeon(data: &Data, seed: u64, members: usize, play: Play) -> (Clear, World) {
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
            if play.shield {
                declare_shield(&mut world, data);
            }
            let encounter = placed(data, &mut world, map, index);
            let mut events = Vec::new();
            combat::start(&mut world, data, encounter, Surprise::None, &mut events).unwrap();
            fight_out(&mut world, data, play, &mut clear);
            if !clear.wiped {
                short_rest(&mut world, data, play, &mut clear);
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
            let (clear, world) = clear_dungeon(&data, seed, members, ONE_COMMAND);
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
