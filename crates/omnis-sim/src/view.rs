//! The encounter or fight as a client sees it (`combat.get`).

use crate::combat::feature::refusal;
use crate::combat::{
    Budget, CombatState, FeatureChoice, Pay, Roller, SpellsCast, cast, monster_front_stacks,
    payable, points_of, weapon_for,
};
use crate::command::Rejection;
use crate::encounter::{EncounterSource, Stack};
use crate::event::{ActorRef, Surprise};
use crate::party;
use crate::world::{Mode, ModeKind, World};
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, Position};
use omnis_data::{Cost, Data, Disposition, FeatureEffect, Reach};
use omnis_rules::{Character, combat_features, uses_left};
use serde::{Deserialize, Serialize};

/// One stack as a client sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackView {
    /// Its index in the encounter, the number `attack` takes.
    pub index: u8,
    /// Monster id.
    pub monster: String,
    /// Text key of the monster's name.
    pub name: String,
    /// How many there were.
    pub initial: u8,
    /// Hit points of the living, in order.
    pub hp: Vec<i32>,
    /// Armor class.
    pub ac: u8,
    /// Whether it stands in front.
    pub front: bool,
    /// Whether anyone in it still stands.
    pub alive: bool,
    /// Whether the member whose turn it is can reach it.
    pub reachable: bool,
    /// Spell points each living individual has left, in order; empty for a monster that does
    /// not cast.
    #[serde(default)]
    pub points_left: Vec<u8>,
    /// The individuals whose Shield is up until the stack's next turn.
    #[serde(default)]
    pub shielded: Vec<u8>,
}

/// One spell the acting member knows, as a picker shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellView {
    /// Its index in the caster's list, the number `cast` takes.
    pub index: u8,
    /// Spell id.
    pub id: String,
    /// Text key of the spell's name.
    pub name: String,
    /// Spell level; 0 is a cantrip.
    pub level: u8,
    /// Points it costs.
    pub cost: u32,
    /// How far it reaches.
    pub reach: Reach,
    /// Aimed at members rather than monsters.
    pub targets_members: bool,
    /// Why it cannot be cast with the action now, if it cannot: the spell's own checks, the
    /// one-spell rule, then the budget.
    pub blocked: Option<Rejection>,
    /// Why it cannot be cast with the bonus action now, if it cannot.
    #[serde(default)]
    pub bonus: Option<Rejection>,
}

/// One class feature with an effect, as the Feature command numbers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureView {
    /// Its row, the number `Feature` takes.
    pub index: u8,
    /// The feature's name, the key a declared action names it by.
    pub name: String,
    /// What it costs from the turn's budget.
    pub cost: Cost,
    /// Uses left before a rest; `None` at will.
    pub uses_left: Option<u8>,
    /// Why it cannot be used now, if it cannot (`NotYourTurn` off the member's turn; Cunning
    /// Action is judged by its Hide).
    pub blocked: Option<Rejection>,
}

/// One member's side of the fight: the reactions switch and the features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FighterView {
    /// Position in marching order.
    pub index: u8,
    /// Whether the member's declared reactions fire.
    pub reactions_on: bool,
    /// The class features with an effect.
    pub features: Vec<FeatureView>,
}

/// The encounter or fight in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatView {
    /// `Encounter` before the choice, `Combat` during the fight.
    pub phase: ModeKind,
    /// Where the monsters came from.
    pub source: EncounterSource,
    /// How they feel about the party.
    pub disposition: Disposition,
    /// The stacks.
    pub stacks: Vec<StackView>,
    /// Where Run and Flee put the party.
    pub retreat: Position,
    /// The round, zero before the fight.
    pub round: u32,
    /// Whose turn it is.
    pub current: Option<ActorRef>,
    /// The initiative order with totals.
    pub order: Vec<(ActorRef, i64)>,
    /// Who lost their first round.
    pub surprised: Surprise,
    /// Members dodging this round.
    pub dodging: Vec<CharacterId>,
    /// Copper looted so far.
    pub gold: u32,
    /// Members in the front row.
    pub front_row: usize,
    /// Stacks in front.
    pub monster_front_stacks: usize,
    /// The acting member's spells; empty when no member acts.
    pub spells: Vec<SpellView>,
    /// What the acting member's turn has left to pay with.
    #[serde(default)]
    pub budget: Budget,
    /// Reactions left this round, by combatant.
    #[serde(default)]
    pub reactions: Vec<(ActorRef, u8)>,
    /// Members hidden (advantage on their next attack).
    #[serde(default)]
    pub hidden: Vec<CharacterId>,
    /// The spells cast this turn, for the one-spell rule.
    #[serde(default)]
    pub spells_cast: SpellsCast,
    /// Each member's reactions switch and features; empty before the fight.
    #[serde(default)]
    pub members: Vec<FighterView>,
}

/// The view, or `None` while exploring.
#[must_use]
pub fn combat_view(world: &World, data: &Data) -> Option<CombatView> {
    let (encounter, fight) = match &world.mode {
        Mode::Explore | Mode::Town(_) => return None,
        Mode::Encounter(e) => (e, None),
        Mode::Combat(c) => (&c.encounter, Some(c)),
    };
    let front_count = monster_front_stacks(data);
    let acting = fight.and_then(|c| match c.current_actor() {
        Some(ActorRef::Member(id)) => world.party.members.iter().position(|m| m.id == id),
        _ => None,
    });
    let mut in_front = 0;
    let stacks = encounter
        .stacks
        .iter()
        .enumerate()
        .map(|(i, stack)| {
            let index = u8::try_from(i).unwrap_or(u8::MAX);
            let front = stack.alive() && in_front < front_count;
            if front {
                in_front += 1;
            }
            let reachable = match (fight, acting) {
                (Some(c), Some(own)) => weapon_for(c, world, data, own, index).is_ok(),
                _ => false,
            };
            let shielded = fight.map_or_else(Vec::new, |c| {
                c.monster_shields
                    .iter()
                    .filter(|(s, _)| *s == index)
                    .map(|(_, i)| *i)
                    .collect()
            });
            StackView {
                shielded,
                ..stack_view(data, stack, index, front, reachable)
            }
        })
        .collect();
    Some(CombatView {
        phase: world.mode.kind(),
        source: encounter.source,
        disposition: encounter.disposition,
        stacks,
        retreat: encounter.retreat,
        round: fight.map_or(0, |c| c.round),
        current: fight.and_then(CombatState::current_actor),
        order: fight.map_or_else(Vec::new, |c| {
            c.order.iter().map(|i| (i.actor, i.total)).collect()
        }),
        surprised: fight.map_or(Surprise::None, |c| c.surprised),
        dodging: fight.map_or_else(Vec::new, |c| c.dodging.clone()),
        gold: fight.map_or(0, |c| c.gold),
        front_row: party::front_row(data),
        monster_front_stacks: front_count,
        spells: match (fight, acting) {
            (Some(c), Some(own)) => spell_views(c, world, data, own),
            _ => Vec::new(),
        },
        budget: fight.map_or_else(Budget::default, |c| c.budget),
        reactions: fight.map_or_else(Vec::new, |c| c.reactions.clone()),
        hidden: fight.map_or_else(Vec::new, |c| c.hidden.clone()),
        spells_cast: fight.map_or_else(SpellsCast::default, |c| c.spells_cast),
        members: fight.map_or_else(Vec::new, |c| {
            world
                .party
                .members
                .iter()
                .enumerate()
                .map(|(own, member)| fighter_view(c, world, data, own, member, acting == Some(own)))
                .collect()
        }),
    })
}

/// A member's reactions switch and features; only the acting member's features can be free.
fn fighter_view(
    state: &CombatState,
    world: &World,
    data: &Data,
    own: usize,
    member: &Character,
    acting: bool,
) -> FighterView {
    let features = combat_features(member, data)
        .into_iter()
        .enumerate()
        .filter_map(|(row, feature)| {
            let index = u8::try_from(row).ok()?;
            let choice = match feature.effect {
                Some(FeatureEffect::Cunning) => FeatureChoice::Hide,
                _ => FeatureChoice::None,
            };
            let blocked = if acting {
                refusal(world, data, state, own, index, choice)
            } else {
                Some(Rejection::NotYourTurn)
            };
            Some(FeatureView {
                index,
                name: feature.name.clone(),
                cost: feature.cost,
                uses_left: uses_left(member, feature),
                blocked,
            })
        })
        .collect();
    FighterView {
        index: u8::try_from(own).unwrap_or(u8::MAX),
        reactions_on: member.tactics.reactions_on,
        features,
    }
}

/// The spells a member knows with what each costs and why it is blocked, for the action and for
/// the bonus action, read off a copy of the combat stream so the view draws nothing.
fn spell_views(state: &CombatState, world: &World, data: &Data, own: usize) -> Vec<SpellView> {
    let Some(member) = world.party.members.get(own) else {
        return Vec::new();
    };
    let mut rng = Roller::take(world).rng;
    member
        .known_spells
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let index = u8::try_from(i).ok()?;
            let spell = data.spells.get(id)?;
            let checked = cast::check(world, data, own, index, true, &mut rng).err();
            let pays = |pay| {
                checked
                    .clone()
                    .or_else(|| payable(state, spell, index, pay).err())
            };
            Some(SpellView {
                index,
                id: data.registry.spells.name(*id).unwrap_or("?").to_owned(),
                name: spell.name.clone(),
                level: spell.level,
                cost: spell.point_cost(),
                reach: spell.reach,
                targets_members: spell.effect.as_ref().is_some_and(|e| e.targets_members()),
                blocked: pays(Pay::Action),
                bonus: pays(Pay::BonusAction),
            })
        })
        .collect()
}

fn stack_view(data: &Data, stack: &Stack, index: u8, front: bool, reachable: bool) -> StackView {
    let monster = data.monsters.get(&stack.monster);
    StackView {
        index,
        monster: data
            .registry
            .monsters
            .name(stack.monster)
            .unwrap_or("?")
            .to_owned(),
        name: monster.map_or_else(|| "?".to_owned(), |m| m.name.clone()),
        initial: stack.initial,
        hp: stack.hp.clone(),
        ac: monster.map_or(0, |m| m.ac),
        front,
        alive: stack.alive(),
        reachable,
        points_left: points_of(data, stack),
        shielded: Vec::new(),
    }
}
