//! The turn budget (PRD D21, ARCHITECTURE.md §4.7): the `turn.*` slots evaluated at the start
//! of a combatant's turn, what a command spends, and whether a member's turn goes on. A turn
//! ends by itself when no action is left and nothing the bonus action could pay for, nor a
//! free feature, is available; `EndTurn` ends it sooner.

use super::state::{Budget, CombatState};
use super::{Roller, cast};
use crate::command::Rejection;
use crate::event::ActorRef;
use crate::world::World;
use omnis_core::CharacterId;
use omnis_data::omnis_expr::Value;
use omnis_data::{Cost, Data};
use omnis_rules::{RuleError, combat_features, uses_left};

/// One `turn.*` slot for a member of `level`, or a stack (`level` 0, not a member).
fn slot(
    data: &Data,
    name: &str,
    level: u8,
    is_member: bool,
    roller: &mut Roller,
) -> Result<u8, RuleError> {
    let outcome = data.rules.eval(
        name,
        &[
            ("level", Value::Int(i64::from(level))),
            ("is_member", Value::Bool(is_member)),
        ],
        &mut roller.rng,
        &roller.stream,
    )?;
    let value = outcome
        .value
        .as_int()
        .ok_or_else(|| RuleError::new(name, "formula must produce an integer"))?;
    u8::try_from(value).map_err(|_| RuleError::new(name, "a turn budget is 0..=255"))
}

/// The actor's level and whether it is a member, as the slots take them.
fn inputs(world: &World, actor: ActorRef) -> (u8, bool) {
    match actor {
        ActorRef::Member(id) => (
            world
                .party
                .members
                .iter()
                .find(|m| m.id == id)
                .map_or(1, |m| m.level),
            true,
        ),
        _ => (0, false),
    }
}

/// Refresh `actor`'s reactions from `turn.reactions`: at the start of the fight and of its own
/// turn.
pub(crate) fn refresh_reactions(
    world: &World,
    data: &Data,
    state: &mut CombatState,
    actor: ActorRef,
    roller: &mut Roller,
) -> Result<(), RuleError> {
    let (level, member) = inputs(world, actor);
    let count = slot(data, "turn.reactions", level, member, roller)?;
    state.set_reactions(actor, count);
    Ok(())
}

/// A member's turn begins: their actions and bonus actions, and their reactions back.
pub(crate) fn begin_member_turn(
    world: &World,
    data: &Data,
    state: &mut CombatState,
    id: CharacterId,
    roller: &mut Roller,
) -> Result<(), RuleError> {
    let actor = ActorRef::Member(id);
    let (level, member) = inputs(world, actor);
    state.budget = Budget {
        actions: slot(data, "turn.actions", level, member, roller)?,
        bonus_actions: slot(data, "turn.bonus_actions", level, member, roller)?,
    };
    refresh_reactions(world, data, state, actor, roller)
}

/// Whether the budget can pay `cost`; a reaction is never paid from a turn.
pub(crate) fn affordable(budget: Budget, cost: Cost) -> Result<(), Rejection> {
    match cost {
        Cost::Action if budget.actions == 0 => Err(Rejection::NoActionLeft),
        Cost::BonusAction if budget.bonus_actions == 0 => Err(Rejection::NoBonusActionLeft),
        Cost::Reaction => Err(Rejection::ReactionOnly),
        _ => Ok(()),
    }
}

/// Take `cost` from the budget; validation made sure it can be paid.
pub(crate) const fn spend(budget: &mut Budget, cost: Cost) {
    match cost {
        Cost::Action => budget.actions = budget.actions.saturating_sub(1),
        Cost::BonusAction => budget.bonus_actions = budget.bonus_actions.saturating_sub(1),
        Cost::Reaction | Cost::Free => {}
    }
}

/// Whether the member in slot `own` has anything left to do this turn: an action, a free
/// feature with a use left, or something the bonus action pays for (a feature, or a spell that
/// may take the bonus action and can be cast now).
pub(crate) fn goes_on(
    world: &World,
    data: &Data,
    state: &CombatState,
    own: usize,
    roller: &Roller,
) -> bool {
    let Some(member) = world.party.members.get(own) else {
        return false;
    };
    if member.is_down() {
        return false;
    }
    if state.budget.actions > 0 {
        return true;
    }
    let features = combat_features(member, data);
    let usable = |cost: Cost| {
        features
            .iter()
            .any(|f| f.cost == cost && uses_left(member, f) != Some(0))
    };
    if usable(Cost::Free) {
        return true;
    }
    if state.budget.bonus_actions == 0 {
        return false;
    }
    if usable(Cost::BonusAction) {
        return true;
    }
    // A copy of the stream: a cost formula may roll, and looking must not move the dice.
    let mut rng = roller.rng;
    (0..member.known_spells.len()).any(|index| {
        let Ok(index) = u8::try_from(index) else {
            return false;
        };
        cast::check(world, data, own, index, true, &mut rng).is_ok_and(|(_, spell, _)| {
            spell.bonus_action_available && !spell.preparation_required_for_bonus_action
        })
    })
}
