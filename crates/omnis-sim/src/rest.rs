//! Resting (PRD §8.2, SRD 5.1). A short rest is an hour and spends hit dice; a long rest is
//! eight hours, eats food, restores fully and comes at most once a day. Both may be ambushed
//! from the map's random table, at per-mille chances from rule slots (owner, 2026-09-27); an
//! ambush comes after a rolled part of the rest and restores nothing. A rest that runs its
//! course rolls the map's rest events for the party's terrain, which report and change nothing
//! yet. The long rest's restoration is shared with the inn's room (`service.rs`).

use crate::apply::advance;
use crate::combat::Roller;
use crate::combat::state::is_dead;
use crate::command::Rejection;
use crate::encounter;
use crate::event::Event;
use crate::party;
use crate::world::World;
use alloc::format;
use alloc::vec::Vec;
use omnis_core::{Dice, RollTrace};
use omnis_data::omnis_expr::Value;
use omnis_data::rest_event::PER_MILLE;
use omnis_data::{Ability, Data};
use omnis_rules::{RuleError, modifier};
use serde::{Deserialize, Serialize};

/// An hour's rest when the rules leave it out.
const DEFAULT_SHORT_REST_MINUTES: u32 = 60;
/// A night's rest when the rules leave it out.
pub(crate) const DEFAULT_LONG_REST_MINUTES: u32 = 480;
/// One long rest a day when the rules leave it out.
const DEFAULT_LONG_REST_EVERY: u32 = 1440;

/// A rest outside a service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestCommand {
    /// An hour. `dice[i]` is how many hit dice member `i` spends; members past the end spend
    /// none.
    Short {
        /// Hit dice per member, in party order.
        dice: Vec<u8>,
    },
    /// The night: food for every member not dead, at most once a day.
    Long,
}

/// Hit dice a long rest gives back: half the member's total (one per level), at least one.
#[must_use]
pub const fn hit_dice_back(level: u8) -> u8 {
    let half = level / 2;
    if half == 0 { 1 } else { half }
}

/// A rules value in minutes, or the default.
pub(crate) fn rule_minutes(data: &Data, key: &str, default: u32) -> u32 {
    data.rules
        .value(key)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(default)
}

/// The night's length in minutes.
pub(crate) fn long_rest_minutes(data: &Data) -> u32 {
    rule_minutes(data, "long_rest_minutes", DEFAULT_LONG_REST_MINUTES)
}

/// `RestTooSoon` when a long rest of `minutes` would end inside `long_rest_every_minutes` of
/// the last one's end (SRD: one in 24 hours, counted end to end).
pub(crate) fn too_soon(world: &World, data: &Data, minutes: u32) -> Result<(), Rejection> {
    let every = rule_minutes(data, "long_rest_every_minutes", DEFAULT_LONG_REST_EVERY);
    let Some(last) = world.party.last_long_rest else {
        return Ok(());
    };
    let ends = world.party_clock().elapsed + i64::from(minutes);
    let wait = last + i64::from(every) - ends;
    if wait > 0 {
        return Err(Rejection::RestTooSoon {
            minutes: u32::try_from(wait).unwrap_or(u32::MAX),
        });
    }
    Ok(())
}

/// The end of a long rest for every member. A member with at least one hit point regains every
/// hit point and spell point and half their hit dice; a member at zero who is not dead is
/// stable and wakes at one hit point within the eight hours, without the rest's benefits; the
/// dead stay dead. Each hit point gain is a `Healed` event.
pub(crate) fn long_rest_restore(world: &mut World, data: &Data, events: &mut Vec<Event>) {
    for index in 0..world.party.members.len() {
        let member = &mut world.party.members[index];
        if is_dead(member, data) {
            continue;
        }
        let gain = if member.is_down() {
            1 - member.hp
        } else {
            member.spell_points = member.spell_points_max;
            member.hit_dice_spent = member
                .hit_dice_spent
                .saturating_sub(hit_dice_back(member.level));
            member.hp_max - member.hp
        };
        if gain > 0 {
            party::heal(world, data, index, Vec::new(), i64::from(gain), events);
        }
    }
}

/// One member's spent hit dice, rolled.
struct Spend {
    index: usize,
    dice: u8,
    rolls: Vec<RollTrace>,
    amount: i64,
}

/// Rest: validate, roll everything on copies of the streams, then change the world.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    command: &RestCommand,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let long = matches!(command, RestCommand::Long);
    let (minutes, food, asked) = match command {
        RestCommand::Short { dice } => {
            let minutes = rule_minutes(data, "short_rest_minutes", DEFAULT_SHORT_REST_MINUTES);
            (minutes, 0, check_dice(world, data, dice)?)
        }
        RestCommand::Long => {
            let minutes = long_rest_minutes(data);
            too_soon(world, data, minutes)?;
            (minutes, food_needed(world, data)?, Vec::new())
        }
    };
    let mut risk = Roller::take_stream(world, "encounter");
    let interrupted = ambush_after(world, data, long, minutes, &mut risk)?;
    let mut hits = Vec::new();
    let mut spends = Vec::new();
    let mut dice = Roller::take_stream(world, "rest");
    if interrupted.is_none() {
        hits = rest_events(world, data, long, &mut risk)?;
        for (index, count) in asked {
            spends.push(roll_hit_dice(world, data, index, count, &mut dice)?);
        }
    }
    risk.store(world);
    dice.store(world);
    if let Some(after) = interrupted {
        advance(world, after, events);
        events.push(Event::RestInterrupted { minutes: after });
        return encounter::ambush(world, data, events).map_err(Rejection::Rule);
    }
    advance(world, minutes, events);
    let map = world.position.map;
    events.extend(
        hits.into_iter()
            .map(|index| Event::RestEvent { map, index }),
    );
    events.push(Event::Rested {
        long,
        minutes,
        food,
    });
    if long {
        world.party.food -= food;
        long_rest_restore(world, data, events);
        world.party.last_long_rest = Some(world.party_clock().elapsed);
    }
    for spend in spends {
        let member = &mut world.party.members[spend.index];
        member.hit_dice_spent += spend.dice;
        events.push(Event::HitDiceSpent {
            member: member.id,
            dice: spend.dice,
        });
        party::heal(world, data, spend.index, spend.rolls, spend.amount, events);
    }
    Ok(())
}

/// The members spending hit dice and how many, after every check: a slot in the party, not
/// dead, hit points to regain, and dice enough left.
fn check_dice(world: &World, data: &Data, dice: &[u8]) -> Result<Vec<(usize, u8)>, Rejection> {
    let members = &world.party.members;
    if dice.len() > members.len() {
        return Err(Rejection::NoSuchMember {
            index: u8::try_from(members.len()).unwrap_or(u8::MAX),
        });
    }
    let mut asked = Vec::new();
    for (index, (&count, member)) in dice.iter().zip(members).enumerate() {
        if count == 0 {
            continue;
        }
        let slot = u8::try_from(index).unwrap_or(u8::MAX);
        if is_dead(member, data) {
            return Err(Rejection::MemberDead { index: slot });
        }
        if member.hp >= member.hp_max {
            return Err(Rejection::NothingToTreat { index: slot });
        }
        let left = member.level.saturating_sub(member.hit_dice_spent);
        if count > left {
            return Err(Rejection::NoHitDice { index: slot, left });
        }
        asked.push((index, count));
    }
    Ok(asked)
}

/// Food the night eats: `rest_food_per_member` for every member not dead.
fn food_needed(world: &World, data: &Data) -> Result<u32, Rejection> {
    let each = data
        .rules
        .value("rest_food_per_member")
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(1);
    let eaters = world
        .party
        .members
        .iter()
        .filter(|m| !is_dead(m, data))
        .count();
    let need = each.saturating_mul(u32::try_from(eaters).unwrap_or(u32::MAX));
    if need > world.party.food {
        return Err(Rejection::NoFood {
            need,
            have: world.party.food,
        });
    }
    Ok(need)
}

/// One d1000 against the rest's ambush chance on a map with a random table; on a hit, the
/// minutes that pass first: 1d(hours) hours of a long rest, 1d(tens) ten-minute spans of a
/// short one.
fn ambush_after(
    world: &World,
    data: &Data,
    long: bool,
    minutes: u32,
    roller: &mut Roller,
) -> Result<Option<u32>, Rejection> {
    let Some(random) = data
        .maps
        .get(&world.position.map)
        .and_then(|m| m.random.as_ref())
    else {
        return Ok(None);
    };
    let slot = if long {
        "rest.ambush_chance"
    } else {
        "rest.short_ambush_chance"
    };
    let chance = int(
        data,
        slot,
        &[("map_chance", i64::from(random.chance_percent))],
        roller,
    )?;
    if i64::from(roll(PER_MILLE, roller)?.total) > chance {
        return Ok(None);
    }
    let step = if long { 60 } else { 10 };
    let spans = (minutes / step).max(1);
    let spans = u16::try_from(spans).unwrap_or(u16::MAX);
    let after = u32::try_from(roll(spans, roller)?.total).unwrap_or(1);
    Ok(Some(after * step))
}

/// The map's rest events for the party's terrain and this rest, each rolled on a d1000 in file
/// order (a chance of 0 still rolls, so the dice stay put when a chance changes); the indices
/// that hit.
fn rest_events(
    world: &World,
    data: &Data,
    long: bool,
    roller: &mut Roller,
) -> Result<Vec<u16>, Rejection> {
    let pos = world.position;
    let Some(map) = data.maps.get(&pos.map) else {
        return Ok(Vec::new());
    };
    let Some(cell) = map.cell(pos.x, pos.y) else {
        return Ok(Vec::new());
    };
    let mut hits = Vec::new();
    for (index, entry) in map.rest_events.iter().enumerate() {
        if entry.terrain != cell.terrain || !entry.rest.covers(long) {
            continue;
        }
        if roll(PER_MILLE, roller)?.total <= i32::from(entry.chance) {
            hits.push(u16::try_from(index).unwrap_or(u16::MAX));
        }
    }
    Ok(hits)
}

/// `count` of a member's hit dice, each through `rest.hit_die_heal` with the Constitution
/// modifier.
fn roll_hit_dice(
    world: &World,
    data: &Data,
    index: usize,
    count: u8,
    roller: &mut Roller,
) -> Result<Spend, Rejection> {
    let member = &world.party.members[index];
    let sides = data
        .classes
        .get(&member.class)
        .map_or(8, |c| u16::from(c.hit_die));
    let con = modifier(member.scores[Ability::Constitution.index()]);
    let mut rolls = Vec::with_capacity(usize::from(count));
    let mut amount = 0;
    for _ in 0..count {
        let trace = roll(sides, roller)?;
        amount += int(
            data,
            "rest.hit_die_heal",
            &[("die", trace.total.into()), ("con_mod", con)],
            roller,
        )?;
        rolls.push(trace);
    }
    Ok(Spend {
        index,
        dice: count,
        rolls,
        amount,
    })
}

/// One die of `sides` on the roller.
fn roll(sides: u16, roller: &mut Roller) -> Result<RollTrace, Rejection> {
    Dice::new(1, sides)
        .roll(&mut roller.rng, &roller.stream)
        .map_err(|e| Rejection::Rule(RuleError::new("rest", format!("{e}"))))
}

/// A rule slot's integer.
fn int(
    data: &Data,
    slot: &str,
    inputs: &[(&str, i64)],
    roller: &mut Roller,
) -> Result<i64, Rejection> {
    let inputs: Vec<(&str, Value)> = inputs.iter().map(|(n, v)| (*n, Value::Int(*v))).collect();
    match data
        .rules
        .eval(slot, &inputs, &mut roller.rng, &roller.stream)
        .map_err(Rejection::Rule)?
        .value
    {
        Value::Int(n) => Ok(n),
        _ => Err(Rejection::Rule(RuleError::new(slot, "not an integer"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_rest_gives_back_half_the_hit_dice_and_at_least_one() {
        assert_eq!(hit_dice_back(1), 1);
        assert_eq!(hit_dice_back(2), 1);
        assert_eq!(hit_dice_back(5), 2);
        assert_eq!(hit_dice_back(20), 10);
    }
}
