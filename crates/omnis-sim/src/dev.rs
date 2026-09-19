//! Debugging edits (ARCHITECTURE.md §4.2, §12): a `Dev` command changes the world through the
//! same stream as every other command, so a replay carries it, and it is accepted only when
//! `Settings.devtools` is on. Validation completes before anything changes.

use crate::apply::visit;
use crate::command::Rejection;
use crate::event::Event;
use crate::items::add_to;
use crate::party::{self, set_condition_id};
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Facing, Position};
use omnis_data::{Ability, Data};
use omnis_rules::DeathSaves;
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
    /// Hit points, clamped to `0..=hp_max`. Zero downs the member (fresh death saves,
    /// `unconscious`); above zero clears `unconscious` and `dead` and resets the saves.
    SetHp {
        /// The member's slot.
        member: u8,
        /// The new hit points.
        hp: i32,
    },
    /// Spell points, clamped to the maximum.
    SetSpellPoints {
        /// The member's slot.
        member: u8,
        /// The new points.
        points: u32,
    },
    /// The party's gold.
    SetGold {
        /// Gold pieces.
        gold: u32,
    },
    /// The party's food.
    SetFood {
        /// Food units.
        food: u32,
    },
    /// A member's experience; the level does not move (levelling is bought in town).
    SetXp {
        /// The member's slot.
        member: u8,
        /// Experience points.
        xp: u32,
    },
    /// One ability score, `1..=30`. Derived numbers follow at read time; `hp_max` does not.
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
    if matches!(world.mode, Mode::Combat(_)) {
        return Err(Rejection::WrongMode);
    }
    let echo = Event::Dev {
        command: command.clone(),
    };
    let mut caused = Vec::new();
    match command {
        DevCommand::GiveItem {
            member,
            item,
            count,
        } => give_item(world, data, *member, item, *count)?,
        DevCommand::SetHp { member, hp } => set_hp(world, data, *member, *hp, &mut caused)?,
        DevCommand::SetSpellPoints { member, points } => {
            let m = member_mut(world, *member)?;
            m.spell_points = (*points).min(m.spell_points_max);
        }
        DevCommand::SetGold { gold } => world.party.gold = *gold,
        DevCommand::SetFood { food } => world.party.food = *food,
        DevCommand::SetXp { member, xp } => member_mut(world, *member)?.xp = *xp,
        DevCommand::SetScore {
            member,
            ability,
            score,
        } => {
            if *score == 0 || *score > 30 {
                return Err(Rejection::OutOfRange);
            }
            member_mut(world, *member)?.scores[ability.index()] = *score;
        }
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
    let hp = hp.clamp(0, member.hp_max);
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
