//! `party.get`: the party as a client sees it, each member's sheet with the derived numbers, the
//! kits and the stores as rows the item commands take.

use crate::command::Rejection;
use crate::party;
use crate::rest;
use crate::tactics::answers;
use crate::world::World;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, ItemId};
use omnis_data::{Ability, Alignment, Data, EquipSlot, Skill};
use omnis_rules::{
    ActionRef, ActiveEffect, Character, Criteria, DeathSaves, Equipped, Expiry, Trigger,
    casting_ability, condition_id, modifier, proficiency_bonus, skill_bonus,
};
use serde::{Deserialize, Serialize};

/// One party member as a client sees it: the sheet plus the derived numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberView {
    /// Position in marching order.
    pub index: u8,
    /// Stable identity.
    pub id: CharacterId,
    /// Display name.
    pub name: String,
    /// Race id.
    pub race: String,
    /// Class id.
    pub class: String,
    /// Level.
    pub level: u8,
    /// Experience.
    pub xp: u32,
    /// Hit points.
    pub hp: i32,
    /// Hit point maximum.
    pub hp_max: i32,
    /// Spell points.
    pub spell_points: u32,
    /// Spell point maximum.
    pub spell_points_max: u32,
    /// Armor class.
    pub ac: i64,
    /// The six scores in SRD order.
    pub scores: [u8; 6],
    /// In the front row.
    pub front: bool,
    /// Condition ids in effect.
    pub conditions: Vec<String>,
    /// At zero hit points.
    pub down: bool,
    /// Dead: carries the pack's `dead` condition.
    pub dead: bool,
    /// Death saving throws in progress.
    pub death_saves: DeathSaves,
    /// Spell ids known, in the order `cast` indexes them.
    #[serde(default)]
    pub spells: Vec<String>,
    /// The kit, in the order the item commands index it.
    #[serde(default)]
    pub equipment: Vec<ItemView>,
    /// What is worn and wielded, by slot.
    #[serde(default)]
    pub equipped: Vec<(EquipSlot, String)>,
    /// The effects on the member.
    #[serde(default)]
    pub effects: Vec<EffectView>,
    /// Hit dice in all: one per level.
    #[serde(default)]
    pub hit_dice: u8,
    /// Hit dice not yet spent on short rests.
    #[serde(default)]
    pub hit_dice_left: u8,
    /// Spells owed by levels and not yet chosen at a trainer.
    #[serde(default)]
    pub spell_picks: u8,
    /// The experience has reached a level a trainer has not granted.
    #[serde(default)]
    pub ready: bool,
    /// The declared reactions and what could be declared.
    #[serde(default)]
    pub tactics: TacticsView,
    /// Background id.
    #[serde(default)]
    pub background: String,
    /// Alignment.
    pub alignment: Alignment,
    /// Age in years: the age at creation plus the years the party has lived since.
    #[serde(default)]
    pub age_years: i64,
    /// Proficiency bonus.
    #[serde(default)]
    pub proficiency: i64,
    /// The experience the next level needs, when there is one.
    #[serde(default)]
    pub next_xp: Option<i64>,
    /// Faces of the class's hit die.
    #[serde(default)]
    pub hit_die: u8,
    /// The class's saving throws with their bonus.
    #[serde(default)]
    pub saves: Vec<(Ability, i64)>,
    /// The proficient skills with their bonus, in SRD order.
    #[serde(default)]
    pub skills: Vec<(Skill, i64)>,
    /// The ability the member casts with, when the class casts.
    #[serde(default)]
    pub casting: Option<Ability>,
}

/// One effect in force.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectView {
    /// The spell that made it.
    pub spell: String,
    /// Who cast it.
    pub caster: CharacterId,
    /// Minutes on the party clock before it ends; `None` when it ends at the bearer's next turn.
    pub minutes_left: Option<i64>,
}

/// A member's tactics as a client sees them (ARCHITECTURE.md §4.7).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticsView {
    /// The in-fight switch: off, no reaction fires.
    pub reactions_on: bool,
    /// The runbook takes the member's turns (stored; not built).
    pub auto: bool,
    /// The default runbook's entries, in the order they are tried.
    pub reactions: Vec<ReactionView>,
    /// The actions the member could declare, each with the triggers it answers.
    pub answers: Vec<AnswerView>,
}

/// One declared reaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionView {
    /// Its entry in the default runbook, the number `PutReaction` and `RemoveReaction` take.
    pub index: u8,
    /// The criteria set's name.
    pub name: String,
    /// The action as the commands carry it.
    pub action: ActionRef,
    /// The action's id (`attack` for the weapon).
    pub action_name: String,
    /// What raises it.
    pub trigger: Trigger,
    /// The criteria that must hold.
    pub when: Criteria,
}

/// An action a member could declare as a reaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerView {
    /// The action as the commands carry it.
    pub action: ActionRef,
    /// The action's id (`attack` for the weapon).
    pub name: String,
    /// The triggers it can answer.
    pub triggers: Vec<Trigger>,
}

/// One row of a kit or the stores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemView {
    /// Its row, the number the item commands take.
    pub index: u8,
    /// Item id.
    pub id: String,
    /// Text key of the item's name.
    pub name: String,
    /// How many.
    pub count: u16,
    /// The slot it equips into, if any.
    pub slot: Option<EquipSlot>,
    /// Whether Use does something with it.
    pub usable: bool,
    /// Whether a use spends one.
    pub consumable: bool,
    /// Worn or wielded by the kit's owner.
    pub equipped: bool,
}

/// The party as a client sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyView {
    /// Members in marching order.
    pub members: Vec<MemberView>,
    /// Slots the rules allow.
    pub slots: usize,
    /// Members in the front row.
    pub front_row: usize,
    /// The purse in copper pieces (100 to the gold piece).
    pub gold: u32,
    /// Gems.
    pub gems: u32,
    /// Food units.
    pub food: u32,
    /// The stores, in the order `Take` indexes them.
    #[serde(default)]
    pub inventory: Vec<ItemView>,
    /// The effects on the whole party.
    #[serde(default)]
    pub effects: Vec<EffectView>,
    /// Copper in the bank.
    #[serde(default)]
    pub bank: u32,
    /// The party clock's `elapsed` when the last long rest ended.
    #[serde(default)]
    pub last_long_rest: Option<i64>,
    /// Minutes before a long rest may begin; 0 when it may.
    #[serde(default)]
    pub long_rest_wait: u32,
}

/// `party.get`.
#[must_use]
pub fn party_view(world: &World, data: &Data) -> PartyView {
    let front_row = party::front_row(data);
    let now = world.party_clock().elapsed;
    let members = world
        .party
        .members
        .iter()
        .enumerate()
        .map(|(index, member)| member_view(data, now, index, member, index < front_row))
        .collect();
    PartyView {
        members,
        slots: party::slots(data),
        front_row,
        gold: world.party.gold,
        gems: world.party.gems,
        food: world.party.food,
        inventory: item_views(data, &world.party.inventory, None),
        effects: effect_views(data, &world.party.effects, now),
        bank: world.party.bank,
        last_long_rest: world.party.last_long_rest,
        long_rest_wait: match rest::too_soon(world, data, rest::long_rest_minutes(data)) {
            Err(Rejection::RestTooSoon { minutes }) => minutes,
            _ => 0,
        },
    }
}

/// A registry name, or `?` for an id no pack defines.
fn name_of(name: Option<&str>) -> String {
    name.unwrap_or("?").to_owned()
}

fn member_view(data: &Data, now: i64, index: usize, member: &Character, front: bool) -> MemberView {
    let dead = condition_id(data, "dead");
    let class = data.classes.get(&member.class);
    let proficiency = proficiency_bonus(member.level, data).unwrap_or(2);
    let calendar = data.calendar();
    let year =
        i64::from(calendar.minutes_per_day.max(1)) * i64::from(calendar.days_per_year.max(1));
    MemberView {
        index: u8::try_from(index).unwrap_or(u8::MAX),
        id: member.id,
        name: member.name.clone(),
        race: name_of(data.registry.races.name(member.race)),
        class: name_of(data.registry.classes.name(member.class)),
        level: member.level,
        xp: member.xp,
        hp: member.hp,
        hp_max: member.hp_max,
        spell_points: member.spell_points,
        spell_points_max: member.spell_points_max,
        ac: omnis_rules::armor_class(member, data),
        scores: member.scores,
        front,
        conditions: member
            .conditions
            .iter()
            .map(|c| name_of(data.registry.conditions.name(*c)))
            .collect(),
        down: member.is_down(),
        dead: dead.is_some_and(|d| member.conditions.contains(&d)),
        death_saves: member.death_saves,
        spells: member
            .known_spells
            .iter()
            .map(|s| name_of(data.registry.spells.name(*s)))
            .collect(),
        equipment: item_views(data, &member.equipment, Some(&member.equipped)),
        equipped: member
            .equipped
            .iter()
            .map(|(slot, id)| (*slot, name_of(data.registry.items.name(*id))))
            .collect(),
        effects: effect_views(data, &member.effects, now),
        hit_dice: member.level,
        hit_dice_left: member.level.saturating_sub(member.hit_dice_spent),
        spell_picks: member.spell_picks,
        ready: omnis_rules::ready(member, data).unwrap_or(false),
        tactics: tactics_view(data, member),
        background: name_of(data.registry.backgrounds.name(member.background)),
        alignment: member.alignment,
        age_years: i64::from(member.age_years) + (now - member.created_at).max(0) / year,
        proficiency,
        next_xp: data
            .rules
            .table("xp_thresholds")
            .and_then(|t| t.get(usize::from(member.level)).copied()),
        hit_die: class.map_or(8, |c| c.hit_die),
        saves: class.map_or_else(Vec::new, |c| {
            c.saving_throws
                .iter()
                .map(|a| (*a, modifier(member.scores[a.index()]) + proficiency))
                .collect()
        }),
        skills: Skill::ALL
            .iter()
            .filter(|s| member.skills.contains(s))
            .map(|s| (*s, skill_bonus(member, data, *s).unwrap_or(0)))
            .collect(),
        casting: casting_ability(member, data),
    }
}

/// The default runbook's entries and the actions that could answer a trigger: the weapon and
/// each known spell (items and features answer nothing yet).
fn tactics_view(data: &Data, member: &Character) -> TacticsView {
    let tactics = &member.tactics;
    let reactions = tactics.active().map_or_else(Vec::new, |book| {
        book.entries
            .iter()
            .enumerate()
            .filter_map(|(row, (_, set))| {
                let set = tactics.library.get(usize::from(*set))?;
                Some(ReactionView {
                    index: u8::try_from(row).ok()?,
                    name: set.name.clone(),
                    action: set.action.clone(),
                    action_name: action_name(data, &set.action),
                    trigger: set.trigger,
                    when: set.when.clone(),
                })
            })
            .collect()
    });
    let candidates = core::iter::once(ActionRef::Attack)
        .chain(member.known_spells.iter().map(|id| ActionRef::Spell(*id)));
    let answers = candidates
        .filter_map(|action| {
            let triggers: Vec<Trigger> = Trigger::ALL
                .into_iter()
                .filter(|t| answers(member, data, &action, *t))
                .collect();
            (!triggers.is_empty()).then(|| AnswerView {
                name: action_name(data, &action),
                action,
                triggers,
            })
        })
        .collect();
    TacticsView {
        reactions_on: tactics.reactions_on,
        auto: tactics.auto,
        reactions,
        answers,
    }
}

/// An action's id as the packs name it.
fn action_name(data: &Data, action: &ActionRef) -> String {
    match action {
        ActionRef::Attack => "attack".to_owned(),
        ActionRef::Spell(id) => name_of(data.registry.spells.name(*id)),
        ActionRef::Item(id) => name_of(data.registry.items.name(*id)),
        ActionRef::Feature(name) => name.clone(),
    }
}

/// A counted list as rows; `equipped` marks the rows a kit's owner wears.
fn item_views(data: &Data, list: &[(ItemId, u16)], equipped: Option<&Equipped>) -> Vec<ItemView> {
    list.iter()
        .enumerate()
        .map(|(index, (id, count))| {
            let def = data.items.get(id);
            ItemView {
                index: u8::try_from(index).unwrap_or(u8::MAX),
                id: name_of(data.registry.items.name(*id)),
                name: def.map_or_else(|| "?".to_owned(), |d| d.name.clone()),
                count: *count,
                slot: def.and_then(|d| d.slot()),
                usable: def.is_some_and(|d| d.use_effect.is_some()),
                consumable: def.is_some_and(|d| d.consumable),
                equipped: equipped.is_some_and(|e| e.values().any(|worn| worn == id)),
            }
        })
        .collect()
}

fn effect_views(data: &Data, effects: &[ActiveEffect], now: i64) -> Vec<EffectView> {
    effects
        .iter()
        .map(|e| EffectView {
            spell: name_of(data.registry.spells.name(e.source)),
            caster: e.caster,
            minutes_left: match e.until {
                Expiry::Minute(m) => Some((m - now).max(0)),
                Expiry::NextTurn => None,
            },
        })
        .collect()
}
