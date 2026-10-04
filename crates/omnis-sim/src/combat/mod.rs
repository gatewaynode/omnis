//! The fight (ARCHITECTURE.md §4.5, §4.7): a member's turn takes commands until their budget
//! has nothing left to pay for or `EndTurn`, then monster turns run until the next member who
//! can act, or the end. Every command is validated in full, its cost against the budget
//! included, before the first die, and the dice come from a copy of the `combat` stream
//! written back only when the command went through, so a rejection leaves the world exactly as
//! it was.

mod budget;
pub mod cast;
pub mod feature;
mod opportunity;
mod reaction;
mod resolve;
pub mod state;
mod turn;

pub use cast::Target;
pub use state::{Budget, CombatState, Initiative, SpellsCast, monster_front_stacks};
pub use turn::run_dc;

use crate::command::Rejection;
use crate::encounter::EncounterState;
use crate::event::{ActorRef, Event, Surprise};
use crate::items;
use crate::party;
use crate::world::{Mode, World};
use alloc::vec::Vec;
use omnis_core::{CharacterId, Pcg32, StreamName};
use omnis_data::{Cost, Data};
use omnis_rules::{RuleError, Weapon, best_weapon};
use serde::{Deserialize, Serialize};

/// One member's action on their turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CombatCommand {
    /// Attack a stack with the best weapon that reaches it.
    Attack {
        /// The stack, by its index in the encounter.
        stack: u8,
    },
    /// Cast a known spell (by its index in the caster's list) at a stack or a member.
    Cast {
        /// Index into the caster's known spells.
        spell: u8,
        /// Whom it goes to.
        target: Target,
        /// What it is paid with: the action, or the bonus action when the spell allows it.
        #[serde(default)]
        pay: Pay,
    },
    /// Use a carried item as the turn's action: a potion on a member, or the user when no
    /// target is named. A sense item is not used from a fight.
    Use {
        /// The row of the acting member's kit.
        item: u8,
        /// Whom a potion goes to; the user when `None`.
        target: Option<u8>,
    },
    /// Dodge until the round ends: attacks against the member have disadvantage.
    Dodge,
    /// Swap marching-order slots with another member.
    Exchange {
        /// The other member's slot.
        with: u8,
    },
    /// Try to get away; the whole party leaves on success.
    Run,
    /// Use a class feature, by its row among the member's features with effect.
    Feature {
        /// The row of `omnis_rules::combat_features`.
        feature: u8,
        /// What the feature is asked to do, for one that offers a choice.
        #[serde(default)]
        choice: FeatureChoice,
    },
    /// End the turn with budget left.
    EndTurn,
}

/// What a spell is paid with.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Pay {
    /// The action.
    #[default]
    Action,
    /// The bonus action (D24's `bonus_action_available`).
    BonusAction,
}

/// What a feature with a choice is asked to do.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum FeatureChoice {
    /// The feature does its one thing.
    #[default]
    None,
    /// Cunning Action: swap with the member in this slot, provoking nothing.
    Exchange {
        /// The other member's slot.
        with: u8,
    },
    /// Cunning Action: hide.
    Hide,
}

/// The `combat` stream, taken out of the world and written back once a command has gone
/// through, so a rejection or a rule error leaves `rngs` untouched.
pub(crate) struct Roller {
    /// The generator.
    pub rng: Pcg32,
    /// Its name, for the traces.
    pub stream: StreamName,
}

impl Roller {
    pub(crate) fn take(world: &World) -> Roller {
        Roller::take_stream(world, "combat")
    }

    /// A copy of any named stream; one stream per consumer (A14), created on first use.
    pub(crate) fn take_stream(world: &World, name: &str) -> Roller {
        let stream = StreamName::new(name);
        let rng = world
            .rngs
            .get(&stream)
            .copied()
            .unwrap_or_else(|| Pcg32::for_stream(world.seed, &stream));
        Roller { rng, stream }
    }

    pub(crate) fn store(self, world: &mut World) {
        world.rngs.insert(self.stream, self.rng);
    }
}

/// What a validated command resolves to.
pub(crate) enum Plan {
    /// An attack with the weapon the reach allows.
    Attack {
        /// The stack.
        stack: u8,
        /// The weapon.
        weapon: Weapon,
    },
    /// A cast that passed every check.
    Cast(cast::CastPlan),
    /// A use of an item that passed every check.
    Use(items::UsePlan),
    /// A dodge.
    Dodge,
    /// A swap of two slots.
    Exchange {
        /// The acting member's slot.
        own: usize,
        /// The other slot.
        with: usize,
    },
    /// A flight attempt.
    Run,
    /// A class feature that passed every check.
    Feature(feature::FeaturePlan),
    /// The turn ends.
    EndTurn,
}

/// Start a fight from an encounter: initiative, round one, and the monster turns up to the
/// first member who can act. Public so tools and tests can stage a fight without a map
/// trigger; the trigger uses it too.
pub fn start(
    world: &mut World,
    data: &Data,
    encounter: EncounterState,
    surprised: Surprise,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let mut roller = Roller::take(world);
    turn::start(world, data, encounter, surprised, &mut roller, events)?;
    roller.store(world);
    Ok(())
}

/// Apply one member command in a fight.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    command: CombatCommand,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let Mode::Combat(state) = &world.mode else {
        return Err(Rejection::WrongMode);
    };
    // Validation may draw (a pack's point-cost formula could roll): it draws from the copy,
    // which is stored only when the command went through.
    let mut roller = Roller::take(world);
    let (actor, plan, cost) = validate(state, world, data, command, &mut roller.rng)?;
    budget::affordable(state.budget, cost)?;
    turn::act(world, data, actor, plan, cost, &mut roller, events).map_err(Rejection::Rule)?;
    roller.store(world);
    Ok(())
}

/// After a dev edit in a fight: run the loop if the fight no longer waits on a member who can
/// act (monster turns, the round's end, or the finish), so the state a save checks holds.
pub(crate) fn resume(
    world: &mut World,
    data: &Data,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let Mode::Combat(mut state) = core::mem::replace(&mut world.mode, Mode::Explore) else {
        return Ok(());
    };
    let mut roller = Roller::take(world);
    let ended = match turn::settle(world, data, &mut state, &mut roller, events) {
        Ok(ended) => ended,
        Err(e) => {
            world.mode = Mode::Combat(state);
            return Err(e);
        }
    };
    roller.store(world);
    if !ended {
        world.mode = Mode::Combat(state);
    }
    Ok(())
}

/// A fight loaded from a save written before the turn budget (schema 5): every combatant gets
/// its reactions and the member the fight waits on a fresh budget, as if the turn had just
/// begun. The streams are read from copies and not moved; a rule that fails leaves the SRD's
/// one of each.
pub(crate) fn begin_after_load(world: &mut World, data: &Data) {
    let Mode::Combat(mut state) = core::mem::replace(&mut world.mode, Mode::Explore) else {
        return;
    };
    let mut roller = Roller::take(world);
    for entry in state.order.clone() {
        if budget::refresh_reactions(world, data, &mut state, entry.actor, &mut roller).is_err() {
            state.set_reactions(entry.actor, 1);
        }
    }
    if let Some(ActorRef::Member(id)) = state.current_actor()
        && budget::begin_member_turn(world, data, &mut state, id, &mut roller).is_err()
    {
        state.budget = Budget {
            actions: 1,
            bonus_actions: 1,
        };
    }
    world.mode = Mode::Combat(state);
}

/// The member whose turn it is: the fight parks only on a member who can act.
fn acting_member(state: &CombatState, world: &World) -> Result<(CharacterId, usize), Rejection> {
    let id = match state.order.get(usize::from(state.current)).map(|e| e.actor) {
        Some(ActorRef::Member(id)) => id,
        _ => return Err(Rejection::NotYourTurn),
    };
    let own = world
        .party
        .members
        .iter()
        .position(|m| m.id == id)
        .ok_or(Rejection::NotYourTurn)?;
    if world.party.members[own].is_down() {
        return Err(Rejection::NotYourTurn);
    }
    Ok((id, own))
}

/// The weapon a member in slot `own` attacks a stack with: melee between the front lines,
/// a ranged weapon anywhere else, or why nothing reaches.
pub fn weapon_for(
    state: &CombatState,
    world: &World,
    data: &Data,
    own: usize,
    stack: u8,
) -> Result<Weapon, Rejection> {
    let member = world.party.members.get(own).ok_or(Rejection::NotYourTurn)?;
    let front_member = own < party::front_row(data);
    if front_member && state.is_front(data, stack) {
        return Ok(best_weapon(member, data, false).unwrap_or_else(Weapon::unarmed));
    }
    match best_weapon(member, data, true) {
        Some(weapon) => Ok(weapon),
        None if front_member => Err(Rejection::OutOfReach { stack }),
        None => Err(Rejection::NeedsRangedWeapon),
    }
}

/// What paying for a spell with `pay` costs, or why the spell cannot be paid that way.
fn spell_cost(spell: &omnis_data::Spell, index: u8, pay: Pay) -> Result<Cost, Rejection> {
    match pay {
        Pay::Action => Ok(spell.cost),
        Pay::BonusAction if !spell.bonus_action_available => {
            Err(Rejection::NotABonusAction { spell: index })
        }
        Pay::BonusAction if spell.preparation_required_for_bonus_action => {
            Err(Rejection::NeedsPreparation { spell: index })
        }
        Pay::BonusAction => Ok(Cost::BonusAction),
    }
}

fn validate(
    state: &CombatState,
    world: &World,
    data: &Data,
    command: CombatCommand,
    rng: &mut Pcg32,
) -> Result<(CharacterId, Plan, Cost), Rejection> {
    let (id, own) = acting_member(state, world)?;
    let mut cost = Cost::Action;
    let plan = match command {
        CombatCommand::Cast { spell, target, pay } => {
            let plan = cast::validate(state, world, data, own, spell, target, rng)?;
            let def = data
                .spells
                .get(&plan.spell)
                .ok_or(Rejection::UnknownSpell { spell })?;
            cost = spell_cost(def, spell, pay)?;
            if state.spells_cast.refuses(def.level, cost) {
                return Err(Rejection::OneSpellATurn { spell });
            }
            Plan::Cast(plan)
        }
        CombatCommand::Attack { stack } => {
            let target = state
                .encounter
                .stacks
                .get(usize::from(stack))
                .ok_or(Rejection::NoSuchStack { stack })?;
            if !target.alive() {
                return Err(Rejection::StackDead { stack });
            }
            Plan::Attack {
                stack,
                weapon: weapon_for(state, world, data, own, stack)?,
            }
        }
        CombatCommand::Use { item, target } => {
            Plan::Use(items::validate_use(world, data, own, item, target, true)?)
        }
        CombatCommand::Dodge => Plan::Dodge,
        CombatCommand::Exchange { with } => {
            let index = usize::from(with);
            if index >= world.party.members.len() {
                return Err(Rejection::NoSuchMember { index: with });
            }
            if index == own {
                return Err(Rejection::SameMember);
            }
            Plan::Exchange { own, with: index }
        }
        CombatCommand::Run => Plan::Run,
        CombatCommand::Feature { feature, choice } => {
            let (plan, feature_cost) = feature::validate(world, data, own, feature, choice)?;
            cost = feature_cost;
            Plan::Feature(plan)
        }
        CombatCommand::EndTurn => {
            cost = Cost::Free;
            Plan::EndTurn
        }
    };
    Ok((id, plan, cost))
}
