//! Class features with effect in a fight (M7c): Second Wind heals, Action Surge adds an action,
//! Cunning Action exchanges without an opportunity attack or hides. A feature is named by its
//! name key among the member's `combat_features`; its cost comes from data and its uses are spent
//! here and given back by rests (`omnis_rules::recover_uses`).

use super::state::CombatState;
use super::{FeatureChoice, Roller, budget};
use crate::checks::{self, CheckSpec};
use crate::command::Rejection;
use crate::event::{ActorRef, CheckKind, Event};
use crate::party;
use crate::world::World;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::Dice;
use omnis_data::{Ability, ClassFeature, Cost, Data, FeatureEffect, Skill};
use omnis_rules::{RollMode, RuleError, combat_features, passive_perception, spend_use, uses_left};

/// What a validated feature does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Act {
    /// Heal the user.
    Heal {
        /// Dice.
        dice: Dice,
        /// Add the level.
        per_level: bool,
    },
    /// One more action.
    Surge,
    /// Swap with the member in `with`, provoking nothing.
    Exchange {
        /// The other slot.
        with: usize,
    },
    /// Stealth against the best passive Perception standing there.
    Hide,
}

/// A feature use that passed every check.
#[derive(Debug, Clone)]
pub(crate) struct FeaturePlan {
    /// The user's slot.
    pub own: usize,
    /// The feature as data.
    pub feature: ClassFeature,
    /// What it does.
    pub act: Act,
}

/// The feature named `key` for the member in slot `own`, its choice, a use left, and what it
/// costs.
pub(crate) fn validate(
    world: &World,
    data: &Data,
    own: usize,
    key: &str,
    choice: FeatureChoice,
) -> Result<(FeaturePlan, Cost), Rejection> {
    let member = world.party.members.get(own).ok_or(Rejection::NotYourTurn)?;
    let named = || String::from(key);
    let feature = combat_features(member, data)
        .into_iter()
        .find(|f| f.name == key)
        .ok_or_else(|| Rejection::NoSuchFeature { feature: named() })?;
    if uses_left(member, feature) == Some(0) {
        return Err(Rejection::NoUsesLeft { feature: named() });
    }
    let effect = feature
        .effect
        .ok_or_else(|| Rejection::NoSuchFeature { feature: named() })?;
    let act = match (effect, choice) {
        (FeatureEffect::Heal { dice, per_level }, FeatureChoice::None) => {
            Act::Heal { dice, per_level }
        }
        (FeatureEffect::ExtraAction, FeatureChoice::None) => Act::Surge,
        (FeatureEffect::Cunning, FeatureChoice::Exchange { with }) => {
            let index = usize::from(world.party.slot_of(with)?);
            if index == own {
                return Err(Rejection::SameMember);
            }
            Act::Exchange { with: index }
        }
        (FeatureEffect::Cunning, FeatureChoice::Hide) => Act::Hide,
        _ => return Err(Rejection::WrongChoice { feature: named() }),
    };
    Ok((
        FeaturePlan {
            own,
            feature: feature.clone(),
            act,
        },
        feature.cost,
    ))
}

/// Use the feature: spend a use, emit `FeatureUsed`, then the effect.
pub(crate) fn resolve(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    plan: &FeaturePlan,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let member = &mut world.party.members[plan.own];
    spend_use(member, &plan.feature);
    let id = member.id;
    events.push(Event::FeatureUsed {
        member: id,
        feature: String::from(plan.feature.name.as_str()),
    });
    match plan.act {
        Act::Heal { dice, per_level } => {
            let trace = dice
                .roll(&mut roller.rng, &roller.stream)
                .map_err(|e| RuleError::new("feature", alloc::format!("{e}")))?;
            let level = if per_level {
                i64::from(world.party.members[plan.own].level)
            } else {
                0
            };
            let amount = i64::from(trace.total) + level;
            party::heal(world, data, plan.own, alloc::vec![trace], amount, events);
        }
        Act::Surge => {
            state.budget.actions = state.budget.actions.saturating_add(1);
        }
        Act::Exchange { with } => super::turn::exchange(world, plan.own, with, events),
        Act::Hide => hide(world, data, state, plan.own, roller, events)?,
    }
    Ok(())
}

/// Stealth against the best passive Perception among the living stacks; on a success the
/// member's next attack this fight has advantage.
fn hide(
    world: &mut World,
    data: &Data,
    state: &mut CombatState,
    own: usize,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let dc = state
        .encounter
        .stacks
        .iter()
        .filter(|s| s.alive())
        .filter_map(|s| data.monsters.get(&s.monster))
        .map(passive_perception)
        .max()
        .unwrap_or(10);
    let spec = CheckSpec {
        skill: Some(Skill::Stealth),
        ability: Ability::Dexterity,
        mode: RollMode::Normal,
    };
    let roll = checks::roll(world, data, own, spec, roller, events)?;
    let success = roll.total >= dc;
    let id = world.party.members[own].id;
    events.push(Event::Check {
        actor: ActorRef::Member(id),
        kind: CheckKind::Hide,
        roll: Some(roll),
        dc,
        success,
    });
    if success && let Err(at) = state.hidden.binary_search(&id) {
        state.hidden.insert(at, id);
    }
    Ok(())
}

/// The cost of the feature named `key`, for a view that greys what the budget cannot pay.
pub fn refusal(
    world: &World,
    data: &Data,
    state: &CombatState,
    own: usize,
    key: &str,
    choice: FeatureChoice,
) -> Option<Rejection> {
    match validate(world, data, own, key, choice) {
        Ok((_, cost)) => budget::affordable(state.budget, cost).err(),
        Err(why) => Some(why),
    }
}
