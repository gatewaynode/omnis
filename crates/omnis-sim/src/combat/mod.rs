//! The fight (ARCHITECTURE.md §4.5): one member command resolves, then monster turns run
//! until the next member who can act, or the end. Every command is validated in full before
//! the first die, and the dice come from a copy of the `combat` stream written back only when
//! the command went through, so a rejection leaves the world exactly as it was.

pub mod cast;
mod resolve;
pub mod state;
mod turn;

pub use cast::Target;
pub use state::{CombatState, Initiative, monster_front_stacks};
pub use turn::run_dc;

use crate::command::Rejection;
use crate::encounter::EncounterState;
use crate::event::{ActorRef, Event, Surprise};
use crate::party;
use crate::world::{Mode, World};
use alloc::vec::Vec;
use omnis_core::{CharacterId, Pcg32, StreamName};
use omnis_data::Data;
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
        let stream = StreamName::new("combat");
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
    let (actor, plan) = validate(state, world, data, command, &mut roller.rng)?;
    turn::act(world, data, actor, plan, &mut roller, events).map_err(Rejection::Rule)?;
    roller.store(world);
    Ok(())
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

fn validate(
    state: &CombatState,
    world: &World,
    data: &Data,
    command: CombatCommand,
    rng: &mut Pcg32,
) -> Result<(CharacterId, Plan), Rejection> {
    let (id, own) = acting_member(state, world)?;
    let plan = match command {
        CombatCommand::Cast { spell, target } => {
            Plan::Cast(cast::validate(state, world, data, own, spell, target, rng)?)
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
    };
    Ok((id, plan))
}
