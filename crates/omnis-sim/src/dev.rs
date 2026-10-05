//! Debugging edits (ARCHITECTURE.md §4.2, §12): a `Dev` command changes the world through the
//! same stream as every other command, so a replay carries it, and it is accepted only when
//! `Settings.devtools` is on. Validation completes before anything changes.

use crate::apply::visit;
use crate::combat;
use crate::combat::Roller;
use crate::command::Rejection;
use crate::encounter::Stack;
use crate::event::{ActorRef, Event};
use crate::items::add_to;
use crate::party::{self, set_condition_id};
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Facing, Position};
use omnis_data::{Ability, Data};
use omnis_rules::{DeathSaves, level_up, modifier, ready, spell_point_pool};
use serde::{Deserialize, Serialize};

/// A debugging edit. Ids are strings, as a `Draft` names its race and class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevCommand {
    /// `count` of an item to a member's kit, or to the party's stores when `member` is `None`.
    GiveItem {
        /// The member's slot, or the stores.
        member: Option<u8>,
        /// `pack:item:name`.
        item: String,
        /// How many; at least one.
        count: u16,
    },
    /// Hit points, at least zero and free to pass `hp_max`. Zero downs the member (fresh death
    /// saves, `unconscious`); above zero clears `unconscious` and `dead` and resets the saves.
    SetHp {
        /// The member's slot.
        member: u8,
        /// The new hit points.
        hp: i32,
    },
    /// Spell points, free to pass the maximum.
    SetSpellPoints {
        /// The member's slot.
        member: u8,
        /// The new points.
        points: u32,
    },
    /// The party's purse.
    SetGold {
        /// Copper pieces.
        gold: u32,
    },
    /// The party's food.
    SetFood {
        /// Food units.
        food: u32,
    },
    /// A member's experience. Each level it reaches is granted at once by the trainer's rule,
    /// free (`Event::LevelUp` with no cost); a lower figure never takes a level away.
    SetXp {
        /// The member's slot.
        member: u8,
        /// Experience points.
        xp: u32,
    },
    /// One ability score, any `u8`. Constitution moves `hp_max` and hit points by the change in
    /// modifier times the level (the SRD's rule); a mental score recomputes the spell point pool,
    /// current points moving by the same amount. Everything else reads the scores live.
    SetScore {
        /// The member's slot.
        member: u8,
        /// Which score.
        ability: Ability,
        /// The new score.
        score: u8,
    },
    /// A pack condition on or off, raw: `dead` set this way does not zero hit points.
    SetCondition {
        /// The member's slot.
        member: u8,
        /// `pack:condition:name`.
        condition: String,
        /// On or off.
        applied: bool,
    },
    /// A world flag by registry id.
    SetFlag {
        /// `pack:flag:name`.
        flag: String,
        /// The value.
        value: i64,
    },
    /// Explore only: onto a cell of a loaded map, facing a way. No minutes pass and the
    /// tile's encounter does not trigger.
    Teleport {
        /// `pack:map:name`.
        map: String,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
        /// Facing.
        facing: Facing,
    },
    /// Fights only: one living individual's hit points; zero removes it without gold.
    SetMonsterHp {
        /// The stack.
        stack: u8,
        /// The individual among the living.
        index: u8,
        /// The new hit points.
        hp: i32,
    },
    /// Fights only: every individual of a stack dies, without gold; experience counts them.
    KillStack {
        /// The stack.
        stack: u8,
    },
}

/// Apply a dev command: the gate, the mode, the edit, with the command echoed as an event
/// before what it caused.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    command: &DevCommand,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    if !world.settings.devtools {
        return Err(Rejection::DevOnly);
    }
    let echo = Event::Dev {
        command: command.clone(),
    };
    let mut caused = Vec::new();
    match command {
        DevCommand::SetMonsterHp { stack, index, hp } => {
            set_monster_hp(world, *stack, *index, *hp, &mut caused)?;
        }
        DevCommand::KillStack { stack } => kill_stack(world, *stack, &mut caused)?,
        DevCommand::GiveItem {
            member,
            item,
            count,
        } => give_item(world, data, *member, item, *count)?,
        DevCommand::SetHp { member, hp } => set_hp(world, data, *member, *hp, &mut caused)?,
        DevCommand::SetSpellPoints { member, points } => {
            member_mut(world, *member)?.spell_points = *points;
        }
        DevCommand::SetGold { gold } => world.party.gold = *gold,
        DevCommand::SetFood { food } => world.party.food = *food,
        DevCommand::SetXp { member, xp } => set_xp(world, data, *member, *xp, &mut caused)?,
        DevCommand::SetScore {
            member,
            ability,
            score,
        } => set_score(world, data, *member, *ability, *score, &mut caused)?,
        DevCommand::SetCondition {
            member,
            condition,
            applied,
        } => {
            let id =
                data.registry
                    .conditions
                    .get(condition)
                    .ok_or_else(|| Rejection::UnknownId {
                        id: condition.clone(),
                    })?;
            set_condition_id(member_mut(world, *member)?, id, *applied, &mut caused);
        }
        DevCommand::SetFlag { flag, value } => {
            let id = data
                .registry
                .flags
                .get(flag)
                .ok_or_else(|| Rejection::UnknownId { id: flag.clone() })?;
            world.flags.insert(id, *value);
        }
        DevCommand::Teleport { map, x, y, facing } => {
            teleport(world, data, map, *x, *y, *facing, &mut caused)?;
        }
    }
    events.push(echo);
    events.append(&mut caused);
    if matches!(world.mode, Mode::Combat(_)) {
        combat::resume(world, data, events).map_err(Rejection::Rule)?;
    }
    Ok(())
}

/// The fight's stacks, or `WrongMode` while exploring.
fn stacks_mut(world: &mut World) -> Result<&mut Vec<Stack>, Rejection> {
    match &mut world.mode {
        Mode::Combat(state) => Ok(&mut state.encounter.stacks),
        _ => Err(Rejection::WrongMode),
    }
}

fn set_monster_hp(
    world: &mut World,
    stack: u8,
    index: u8,
    hp: i32,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let stacks = stacks_mut(world)?;
    let s = stacks
        .get_mut(usize::from(stack))
        .ok_or(Rejection::NoSuchStack { stack })?;
    let slot =
        s.hp.get_mut(usize::from(index))
            .ok_or(Rejection::OutOfRange)?;
    *slot = hp.max(0);
    if hp <= 0 {
        s.hp.remove(usize::from(index));
        events.push(Event::Death {
            target: ActorRef::Monster { stack, index },
            gold: None,
        });
    }
    Ok(())
}

fn kill_stack(world: &mut World, stack: u8, events: &mut Vec<Event>) -> Result<(), Rejection> {
    let stacks = stacks_mut(world)?;
    let s = stacks
        .get_mut(usize::from(stack))
        .ok_or(Rejection::NoSuchStack { stack })?;
    for index in (0..s.hp.len()).rev() {
        events.push(Event::Death {
            target: ActorRef::Monster {
                stack,
                index: u8::try_from(index).unwrap_or(u8::MAX),
            },
            gold: None,
        });
    }
    s.hp.clear();
    Ok(())
}

fn member_mut(world: &mut World, index: u8) -> Result<&mut omnis_rules::Character, Rejection> {
    world
        .party
        .members
        .get_mut(usize::from(index))
        .ok_or(Rejection::NoSuchMember { index })
}

fn give_item(
    world: &mut World,
    data: &Data,
    member: Option<u8>,
    item: &str,
    count: u16,
) -> Result<(), Rejection> {
    if count == 0 {
        return Err(Rejection::OutOfRange);
    }
    let id = data
        .registry
        .items
        .get(item)
        .filter(|id| data.items.contains_key(id))
        .ok_or_else(|| Rejection::UnknownId {
            id: String::from(item),
        })?;
    match member {
        Some(index) => add_to(&mut member_mut(world, index)?.equipment, id, count),
        None => add_to(&mut world.party.inventory, id, count),
    }
    Ok(())
}

fn set_hp(
    world: &mut World,
    data: &Data,
    index: u8,
    hp: i32,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let member = member_mut(world, index)?;
    let hp = hp.max(0);
    let was_down = member.is_down();
    member.hp = hp;
    if hp == 0 && !was_down {
        member.death_saves = DeathSaves::default();
        events.push(Event::Down { target: member.id });
        party::set_condition(member, data, "unconscious", true, events);
    } else if hp > 0 && was_down {
        member.death_saves = DeathSaves::default();
        party::set_condition(member, data, "unconscious", false, events);
        party::set_condition(member, data, "dead", false, events);
    }
    Ok(())
}

/// Experience, and every level it reaches, worked out on a copy on the `dev` stream.
fn set_xp(
    world: &mut World,
    data: &Data,
    index: u8,
    xp: u32,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let mut roller = Roller::take_stream(world, "dev");
    let mut after = member_mut(world, index)?.clone();
    after.xp = xp;
    let mut ups = Vec::new();
    while ready(&after, data).map_err(Rejection::Rule)? {
        let gains = level_up(&mut after, data, &mut roller.rng).map_err(Rejection::Rule)?;
        ups.push(Event::LevelUp {
            member: after.id,
            level: after.level,
            cost: 0,
            gains,
        });
    }
    *member_mut(world, index)? = after;
    roller.store(world);
    events.append(&mut ups);
    Ok(())
}

/// A score and the numbers kept from it: Constitution's hit points, a mental score's pool.
fn set_score(
    world: &mut World,
    data: &Data,
    index: u8,
    ability: Ability,
    score: u8,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let mut roller = Roller::take_stream(world, "dev");
    let mut after = member_mut(world, index)?.clone();
    let shift = modifier(score) - modifier(after.scores[ability.index()]);
    after.scores[ability.index()] = score;
    let mut hp = after.hp;
    if ability == Ability::Constitution {
        let gained = i32::try_from(shift * i64::from(after.level)).unwrap_or(0);
        after.hp_max = after.hp_max.saturating_add(gained).max(1);
        hp = hp.saturating_add(gained);
    }
    if matches!(
        ability,
        Ability::Intelligence | Ability::Wisdom | Ability::Charisma
    ) {
        let pool = spell_point_pool(&after, data, &mut roller.rng).map_err(Rejection::Rule)?;
        let moved =
            i64::from(after.spell_points) + i64::from(pool) - i64::from(after.spell_points_max);
        after.spell_points = u32::try_from(moved.max(0)).unwrap_or(u32::MAX);
        after.spell_points_max = pool;
    }
    *member_mut(world, index)? = after;
    roller.store(world);
    set_hp(world, data, index, hp, events)
}

fn teleport(
    world: &mut World,
    data: &Data,
    map: &str,
    x: u16,
    y: u16,
    facing: Facing,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    if world.mode != Mode::Explore {
        return Err(Rejection::WrongMode);
    }
    let id = data
        .registry
        .maps
        .get(map)
        .filter(|id| data.maps.contains_key(id))
        .ok_or_else(|| Rejection::UnknownId {
            id: String::from(map),
        })?;
    if data.maps[&id].cell(x, y).is_none() {
        return Err(Rejection::OffMap { x, y });
    }
    let from = world.position;
    let to = Position {
        map: id,
        x,
        y,
        facing,
    };
    world.position = to;
    events.push(Event::Moved { from, to });
    visit(world, data);
    Ok(())
}
