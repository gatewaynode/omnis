//! Monsters before the party: the trigger when a step lands on a placed group or a random
//! table fires, the stacks they stand in, surprise, and the four choices (attack, bribe, hide,
//! run) before a fight.

use crate::apply::retreat as retreat_to;
use crate::combat::state::can_fight;
use crate::combat::{self, Roller, run_dc};
use crate::command::Rejection;
use crate::event::{ActorRef, CheckKind, Event, Surprise};
use crate::world::{Mode, World};
use alloc::format;
use alloc::vec::Vec;
use core::cmp::Reverse;
use omnis_core::{Dice, Facing, MonsterId, Position, RollTrace, StreamName};
use omnis_data::omnis_expr::Value;
use omnis_data::{Ability, Data, Disposition, ResolvedRandom, Skill};
use omnis_rules::{
    Character, Roll, RollMode, RuleError, check, kept_d20, modifier_of, monster_hit_points,
    passive, passive_perception, skill_bonus,
};
use serde::{Deserialize, Serialize};

/// Where an encounter came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EncounterSource {
    /// A placement on the map, by its index in the map file.
    Fixed(u16),
    /// The map's random table.
    Random,
}

/// What the party does about the monsters ahead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EncounterChoice {
    /// Fight.
    Attack,
    /// Pay them to leave (`bribe.cost`); a friendly group asks nothing.
    Bribe,
    /// Slip past on Stealth against their passive Perception; they stay on the tile.
    Hide,
    /// Back off to the tile the party came from, on Dexterity against `run_dc`.
    Run,
}

/// A stack of one monster type. Individuals are their hit points; a dead one is removed, so
/// the length is the living count. An emptied stack keeps its slot so indices stay stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stack {
    /// The monster type.
    pub monster: MonsterId,
    /// How many there were.
    pub initial: u8,
    /// Hit points of each living individual, in order.
    pub hp: Vec<i32>,
}

impl Stack {
    /// Whether anyone in it still stands.
    #[must_use]
    pub fn alive(&self) -> bool {
        !self.hp.is_empty()
    }

    /// The living count.
    #[must_use]
    pub fn count(&self) -> u8 {
        u8::try_from(self.hp.len()).unwrap_or(u8::MAX)
    }
}

/// An encounter in progress: from the trigger through the fight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterState {
    /// Where it came from.
    pub source: EncounterSource,
    /// The stacks, in order; the first ones stand in front.
    pub stacks: Vec<Stack>,
    /// How the monsters feel about the party.
    pub disposition: Disposition,
    /// Where Run and Flee put the party: the tile it came from, facing away.
    pub retreat: Position,
}

/// The random encounter stream: separate from `combat`, so walking never moves the fight's dice.
fn encounter_stream() -> StreamName {
    StreamName::new("encounter")
}

/// After a step: a placed group on the tile that is not cleared, else the map's random table.
pub(crate) fn trigger(
    world: &mut World,
    data: &Data,
    from: Position,
    move_facing: Facing,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return Ok(());
    };
    let retreat = Position {
        map: from.map,
        x: from.x,
        y: from.y,
        facing: move_facing.opposite(),
    };
    let cleared = |i: &u16| {
        world
            .maps
            .get(&pos.map)
            .is_some_and(|s| s.cleared.contains(i))
    };
    let placed = map.encounter_at(pos.x, pos.y).filter(|(i, _)| !cleared(i));
    let (source, stacks, disposition, counts) = if let Some((i, e)) = placed {
        (
            EncounterSource::Fixed(i),
            e.stacks.clone(),
            e.disposition,
            Vec::new(),
        )
    } else if let Some(random) = &map.random {
        match random_roll(world, data, random, events)? {
            Some(picked) => picked,
            None => return Ok(()),
        }
    } else {
        return Ok(());
    };
    begin(
        world,
        data,
        source,
        stacks,
        disposition,
        counts,
        retreat,
        events,
    )
}

type Picked = (
    EncounterSource,
    Vec<(MonsterId, u8)>,
    Disposition,
    Vec<RollTrace>,
);

/// One d100 on the encounter stream against the map's chance; a hit picks an entry by weight
/// and rolls its counts on the same stream.
fn random_roll(
    world: &mut World,
    data: &Data,
    random: &ResolvedRandom,
    events: &mut Vec<Event>,
) -> Result<Option<Picked>, RuleError> {
    let stream = encounter_stream();
    let rng = world.stream(&stream);
    let roll = Dice::new(1, 100)
        .roll(rng, &stream)
        .map_err(|e| RuleError::new("encounter", format!("{e}")))?;
    let chance = random.chance_percent;
    // A pack without the slot (the test fixtures on their own) gets the plain comparison.
    let fired = if data.rules.slot("encounter.random").is_some() {
        let outcome = data.rules.eval(
            "encounter.random",
            &[
                ("roll", Value::Int(i64::from(roll.total))),
                ("chance_percent", Value::Int(i64::from(chance))),
            ],
            rng,
            &stream,
        )?;
        matches!(outcome.value, Value::Bool(true))
    } else {
        roll.total <= i32::from(chance)
    };
    events.push(Event::EncounterCheck {
        roll,
        chance,
        fired,
    });
    let total = random.total_weight();
    if !fired || total == 0 {
        return Ok(None);
    }
    let mut pick = rng.below(total);
    let Some(entry) = random.entries.iter().find(|e| {
        if pick < u32::from(e.weight) {
            true
        } else {
            pick -= u32::from(e.weight);
            false
        }
    }) else {
        return Ok(None);
    };
    let mut stacks = Vec::new();
    let mut counts = Vec::new();
    for (monster, dice) in &entry.stacks {
        let trace = dice
            .roll(rng, &stream)
            .map_err(|e| RuleError::new("encounter", format!("{e}")))?;
        stacks.push((
            *monster,
            u8::try_from(trace.total.clamp(1, 99)).unwrap_or(1),
        ));
        counts.push(trace);
    }
    Ok(Some((
        EncounterSource::Random,
        stacks,
        entry.disposition,
        counts,
    )))
}

/// Build the stacks, settle surprise, and either offer the choice or start the fight with the
/// party surprised.
#[allow(clippy::too_many_arguments)]
fn begin(
    world: &mut World,
    data: &Data,
    source: EncounterSource,
    stacks: Vec<(MonsterId, u8)>,
    disposition: Disposition,
    counts: Vec<RollTrace>,
    retreat: Position,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let mut roller = Roller::take(world);
    let mut built = Vec::with_capacity(stacks.len());
    for (monster, count) in &stacks {
        let m = data
            .monsters
            .get(monster)
            .ok_or_else(|| RuleError::new("encounter", "an encounter names an unknown monster"))?;
        let mut hp = Vec::with_capacity(usize::from(*count));
        for _ in 0..*count {
            hp.push(monster_hit_points(
                m,
                data,
                &mut roller.rng,
                &roller.stream,
            )?);
        }
        built.push(Stack {
            monster: *monster,
            initial: *count,
            hp,
        });
    }
    let mut perception = 0;
    for member in world.party.members.iter().filter(|m| can_fight(m, data)) {
        perception = perception.max(passive(member, data, Skill::Perception)?);
    }
    // The surprise check is a rules value (`surprise`, off by default since 2026-09-13): with
    // it off every group is noticed and the party always gets its choice.
    let surprise = data.rules.value("surprise").unwrap_or(0) != 0;
    let (stealth, noticed) = if disposition == Disposition::Friendly || !surprise {
        (None, true)
    } else {
        let dex = stacks
            .iter()
            .filter_map(|(m, _)| data.monsters.get(m))
            .map(|m| modifier_of(m, Ability::Dexterity))
            .max()
            .unwrap_or(0);
        let (trace, face) = kept_d20(RollMode::Normal, &mut roller.rng, &roller.stream)?;
        let roll = Roll {
            trace,
            mode: RollMode::Normal,
            face,
            modifier: dex,
            proficiency: 0,
            total: i64::from(face) + dex,
        };
        let noticed = roll.total < perception;
        (Some(roll), noticed)
    };
    events.push(Event::EncounterStarted {
        source,
        stacks,
        disposition,
        counts,
        stealth,
        perception,
        noticed,
    });
    roller.store(world);
    let state = EncounterState {
        source,
        stacks: built,
        disposition,
        retreat,
    };
    if noticed {
        world.mode = Mode::Encounter(state);
        Ok(())
    } else {
        combat::start(world, data, state, Surprise::Party, events)
    }
}

/// The encounter, taken out of the world (the mode becomes Explore until put back).
fn take(world: &mut World) -> Option<EncounterState> {
    match core::mem::replace(&mut world.mode, Mode::Explore) {
        Mode::Encounter(e) => Some(e),
        other => {
            world.mode = other;
            None
        }
    }
}

/// A cleared placement stays cleared when the map file says `once`.
pub(crate) fn clear_once(world: &mut World, data: &Data, source: EncounterSource) {
    let EncounterSource::Fixed(i) = source else {
        return;
    };
    let map = world.position.map;
    let once = data
        .maps
        .get(&map)
        .and_then(|m| m.encounters.get(usize::from(i)))
        .is_some_and(|e| e.once);
    if once {
        world.map_state(map).cleared.insert(i);
    }
}

/// The party's choice before a fight.
pub(crate) fn apply_choice(
    world: &mut World,
    data: &Data,
    choice: EncounterChoice,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    if !matches!(world.mode, Mode::Encounter(_)) {
        return Err(Rejection::WrongMode);
    }
    match choice {
        EncounterChoice::Attack => {
            let Some(state) = take(world) else {
                return Err(Rejection::WrongMode);
            };
            combat::start(world, data, state, Surprise::None, events).map_err(Rejection::Rule)
        }
        EncounterChoice::Bribe => bribe(world, data, events),
        EncounterChoice::Hide => hide(world, data, events).map_err(Rejection::Rule),
        EncounterChoice::Run => run(world, data, events).map_err(Rejection::Rule),
    }
}

/// What the monsters ahead ask to leave (`bribe.cost` over their total XP and their
/// disposition), so a client can show the price before the choice. The formula draws no dice;
/// the world is read only.
pub fn bribe_cost(world: &World, data: &Data) -> Result<u32, Rejection> {
    let Mode::Encounter(state) = &world.mode else {
        return Err(Rejection::WrongMode);
    };
    cost_of(state, data, &mut Roller::take(world))
}

fn cost_of(state: &EncounterState, data: &Data, roller: &mut Roller) -> Result<u32, Rejection> {
    let xp: i64 = state
        .stacks
        .iter()
        .map(|s| {
            let each = data.monsters.get(&s.monster).map_or(0, |m| i64::from(m.xp));
            each * i64::from(s.initial)
        })
        .sum();
    let cost = data
        .rules
        .eval(
            "bribe.cost",
            &[
                ("xp", Value::Int(xp)),
                (
                    "disposition",
                    Value::Int(i64::try_from(state.disposition.index()).unwrap_or(0)),
                ),
            ],
            &mut roller.rng,
            &roller.stream,
        )
        .map_err(Rejection::Rule)?;
    cost.value
        .as_int()
        .map(|c| u32::try_from(c.max(0)).unwrap_or(u32::MAX))
        .ok_or_else(|| Rejection::Rule(RuleError::new("bribe.cost", "must be an integer")))
}

fn bribe(world: &mut World, data: &Data, events: &mut Vec<Event>) -> Result<(), Rejection> {
    let Mode::Encounter(state) = &world.mode else {
        return Err(Rejection::WrongMode);
    };
    let mut roller = Roller::take(world);
    let cost = cost_of(state, data, &mut roller)?;
    if world.party.gold < cost {
        return Err(Rejection::CannotAfford {
            cost,
            gold: world.party.gold,
        });
    }
    roller.store(world);
    world.party.gold -= cost;
    events.push(Event::Bribed { cost });
    if let Some(state) = take(world) {
        clear_once(world, data, state.source);
    }
    Ok(())
}

/// The member who rolls for the group: the best at `bonus`, the first in marching order on
/// ties.
fn best_member<'a>(
    world: &'a World,
    data: &Data,
    bonus: impl Fn(&Character) -> i64,
) -> Result<&'a Character, RuleError> {
    world
        .party
        .members
        .iter()
        .filter(|m| can_fight(m, data))
        .min_by_key(|m| Reverse(bonus(m)))
        .ok_or_else(|| RuleError::new("encounter", "no member can act"))
}

/// Hide: Stealth against the monsters' best passive Perception; they stay where they are.
fn hide(world: &mut World, data: &Data, events: &mut Vec<Event>) -> Result<(), RuleError> {
    let Mode::Encounter(state) = &world.mode else {
        return Ok(());
    };
    let dc = state
        .stacks
        .iter()
        .filter_map(|s| data.monsters.get(&s.monster))
        .map(passive_perception)
        .max()
        .unwrap_or(10);
    let friendly = state.disposition == Disposition::Friendly;
    let member = best_member(world, data, |m| {
        skill_bonus(m, data, Skill::Stealth).unwrap_or(0)
    })?;
    let mut roller = Roller::take(world);
    let (roll, success) = if friendly {
        (None, true)
    } else {
        let roll = check(
            member,
            data,
            Some(Skill::Stealth),
            Ability::Dexterity,
            RollMode::Normal,
            &mut roller.rng,
            &roller.stream,
        )?;
        let success = roll.total >= dc;
        (Some(roll), success)
    };
    events.push(Event::Check {
        actor: ActorRef::Member(member.id),
        kind: CheckKind::Hide,
        roll,
        dc,
        success,
    });
    roller.store(world);
    let Some(state) = take(world) else {
        return Ok(());
    };
    if success {
        Ok(())
    } else {
        combat::start(world, data, state, Surprise::Party, events)
    }
}

/// Run: Dexterity against the run difficulty; the party steps back on success.
fn run(world: &mut World, data: &Data, events: &mut Vec<Event>) -> Result<(), RuleError> {
    let Mode::Encounter(state) = &world.mode else {
        return Ok(());
    };
    let dc = run_dc(data, state.disposition);
    let friendly = state.disposition == Disposition::Friendly;
    let member = best_member(world, data, |m| {
        omnis_rules::modifier(m.scores[Ability::Dexterity.index()])
    })?;
    let mut roller = Roller::take(world);
    let (roll, success) = if friendly {
        (None, true)
    } else {
        let roll = check(
            member,
            data,
            None,
            Ability::Dexterity,
            RollMode::Normal,
            &mut roller.rng,
            &roller.stream,
        )?;
        let success = roll.total >= dc;
        (Some(roll), success)
    };
    events.push(Event::Check {
        actor: ActorRef::Member(member.id),
        kind: CheckKind::Run,
        roll,
        dc,
        success,
    });
    roller.store(world);
    let Some(state) = take(world) else {
        return Ok(());
    };
    if success {
        retreat_to(world, data, state.retreat, events);
        Ok(())
    } else {
        combat::start(world, data, state, Surprise::Party, events)
    }
}
