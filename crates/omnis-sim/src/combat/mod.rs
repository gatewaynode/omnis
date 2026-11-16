//! The fight (ARCHITECTURE.md §4.5, §4.7): a member's turn takes commands until their budget
//! has nothing left to pay for or `EndTurn`, then monster turns run until the next member who
//! can act, or the end. Every command is validated in full, its cost against the budget
//! included, before the first die, and the dice come from a copy of the `combat` stream
//! written back only when the command went through, so a rejection leaves the world exactly as
//! it was.

mod budget;
pub mod cast;
pub mod feature;
pub mod loot;
mod monster_cast;
mod opportunity;
mod push;
mod reaction;
mod resolve;
pub mod state;
mod turn;

pub use cast::Target;
pub use state::{Budget, CombatState, Initiative, frontage, pace};
pub use turn::run_dc;

pub(crate) use budget::affordable;
pub(crate) use monster_cast::points_of;

use crate::checks::{self, CheckSpec};
use crate::command::Rejection;
use crate::encounter::EncounterState;
use crate::event::{ActorRef, CheckKind, Event, Surprise};
use crate::items;
use crate::party;
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, Pcg32, StreamName};
use omnis_data::{Ability, Cost, Data, Skill};
use omnis_rules::{RollMode, RuleError, Weapon, best_weapon};
use serde::{Deserialize, Serialize};

/// One member's action on their turn.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CombatCommand {
    /// Attack a stack with the best weapon that reaches it.
    Attack {
        /// The stack, by its index in the encounter.
        stack: u8,
    },
    /// Cast a known spell at a stack or a member.
    Cast {
        /// The spell's id (`base:spell:magic_missile`); the caster must know it.
        spell: String,
        /// Whom it goes to.
        target: Target,
        /// What it is paid with: the action, or the bonus action when the spell allows it.
        #[serde(default)]
        pay: Pay,
    },
    /// Use a carried item as the turn's action: a potion on a member, or the user when no
    /// target is named. A sense item is not used from a fight.
    Use {
        /// The item's id (`base:item:potion_of_healing`); the acting member's kit must hold it.
        item: String,
        /// Whom a potion goes to; the user when `None`.
        receiver: Option<CharacterId>,
    },
    /// Dodge until the round ends: attacks against the member have disadvantage.
    Dodge,
    /// Advance the party's line on the nearest stack (the fight is counted in feet, owner
    /// 2026-11-14): free while the round's pace lasts, then the SRD's Dash, the action for one
    /// more pace. The line moves as one and stops when a stack is engaged.
    Advance,
    /// Swap marching-order slots with another member.
    Exchange {
        /// The other member.
        with: CharacterId,
    },
    /// Try to get away; the whole party leaves on success.
    Run,
    /// Use a class feature with effect, by its name key.
    Feature {
        /// The feature's name key (`base:text:class.fighter.second_wind`), as `FeatureView`
        /// shows it.
        feature: String,
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
        /// The other member.
        with: CharacterId,
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
        Roller::take_named(world, StreamName::new(name))
    }

    /// A copy of the stream `stream`, for names built at run time (`time:<a>:<b>`).
    pub(crate) fn take_named(world: &World, stream: StreamName) -> Roller {
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
    /// An attack with the weapon the distance allows.
    Attack {
        /// The stack.
        stack: u8,
        /// The weapon, and what goes with it.
        strike: Strike,
    },
    /// The line advances.
    Advance {
        /// By how many feet.
        feet: u16,
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
    turn::start(world, data, encounter, surprised, None, &mut roller, events)?;
    roller.store(world);
    Ok(())
}

/// What opens a fight from the road (the opening strike, PRD §8.2).
pub(crate) enum Opening<'a> {
    /// A spell, paid already.
    Spell(&'a cast::CastPlan),
    /// A weapon attack by the member in slot `own`.
    Weapon {
        /// The attacker's slot.
        own: usize,
        /// The weapon that reaches.
        weapon: &'a Weapon,
        /// Past its normal range: disadvantage.
        hindered: bool,
    },
}

/// The party's group Stealth check against the group's best passive Perception (the SRD's
/// group check: everyone who can act rolls, and half or more passing carries all).
fn unseen(
    world: &mut World,
    data: &Data,
    encounter: &EncounterState,
    roller: &mut Roller,
    events: &mut Vec<Event>,
) -> Result<bool, RuleError> {
    let dc = crate::encounter::watch(data, &encounter.stacks);
    let spec = CheckSpec {
        skill: Some(Skill::Stealth),
        ability: Ability::Dexterity,
        mode: RollMode::Normal,
    };
    let (mut rolled, mut passed) = (0u32, 0u32);
    for index in 0..world.party.members.len() {
        if !state::can_fight(&world.party.members[index], data) {
            continue;
        }
        let roll = checks::roll(world, data, index, spec, roller, events)?;
        let success = roll.total >= dc;
        rolled += 1;
        passed += u32::from(success);
        events.push(Event::Check {
            actor: ActorRef::Member(world.party.members[index].id),
            kind: CheckKind::Hide,
            roll: Some(roll),
            dc,
            success,
        });
    }
    Ok(rolled > 0 && passed * 2 >= rolled)
}

/// Start a fight with a strike from the road (the opening strike, PRD §8.2): a spell paid
/// already or a weapon's blow lands on the group before the first round and was its maker's
/// action for that round. The party rolls Stealth as a group against the group's best passive
/// Perception: passing, the monsters are surprised and the strike's attack has advantage;
/// failing, the fight starts even. Written 0, the rules value `opening_strike_surprise` leaves
/// out the roll and the surprise.
pub(crate) fn start_struck(
    world: &mut World,
    data: &Data,
    encounter: EncounterState,
    opening: &Opening<'_>,
    events: &mut Vec<Event>,
) -> Result<(), RuleError> {
    let mut roller = Roller::take(world);
    let rolled = data.rules.value("opening_strike_surprise").unwrap_or(1) != 0;
    let surprised = if rolled && unseen(world, data, &encounter, &mut roller, events)? {
        Surprise::Monsters
    } else {
        Surprise::None
    };
    turn::start(
        world,
        data,
        encounter,
        surprised,
        Some(opening),
        &mut roller,
        events,
    )?;
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

/// Why a member command would be refused now, if it would: the command's own validation and
/// the turn's budget, on a copy of the dice, with nothing changed. The list of possible
/// targets ([`crate::targets`]) judges each of its rows here, so it cannot disagree with
/// [`apply`].
pub(crate) fn refusal(world: &World, data: &Data, command: CombatCommand) -> Option<Rejection> {
    let Mode::Combat(state) = &world.mode else {
        return Some(Rejection::WrongMode);
    };
    let mut roller = Roller::take(world);
    let (_, _, cost) = match validate(state, world, data, command, &mut roller.rng) {
        Ok(checked) => checked,
        Err(rejection) => return Some(rejection),
    };
    budget::affordable(state.budget, cost).err()
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

/// How a member attacks a stack: with what, after how long an advance, and how well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strike {
    /// The weapon that reaches.
    pub weapon: Weapon,
    /// Feet the party's line advances first.
    pub advance: u16,
    /// The attack has disadvantage: a shot past normal range, or with a foe upon the shooter.
    pub hindered: bool,
}

/// How the member in slot `own` attacks a stack, in feet (owner, 2026-11-14), or why nothing
/// reaches. In this order: the melee weapon when its reach covers the distance (the back row
/// stands five feet further off); else the ranged weapon within its long range, hindered past
/// its normal range or with a stack upon a front-row shooter; else, for the front row, the
/// melee weapon after the line advances, when the round's pace and the nearest stack leave
/// room for it.
pub fn strike_for(
    state: &CombatState,
    world: &World,
    data: &Data,
    own: usize,
    stack: u8,
) -> Result<Strike, Rejection> {
    let member = world.party.members.get(own).ok_or(Rejection::NotYourTurn)?;
    let stacks = &state.encounter.stacks;
    let distance = stacks
        .get(usize::from(stack))
        .ok_or(Rejection::NoSuchStack { stack })?
        .distance;
    let front = own < party::front_row(data);
    let behind = if front { 0 } else { omnis_data::SQUARE_FEET };
    let melee = best_weapon(member, data, false).unwrap_or_else(Weapon::unarmed);
    let reach = u16::from(melee.reach);
    if distance.saturating_add(behind) <= reach {
        return Ok(Strike {
            weapon: melee,
            advance: 0,
            hindered: false,
        });
    }
    let ranged = best_weapon(member, data, true);
    if let Some(weapon) = &ranged {
        let (normal, long) = weapon.range.unwrap_or(omnis_data::DEFAULT_RANGE);
        if distance <= long {
            return Ok(Strike {
                weapon: weapon.clone(),
                advance: 0,
                hindered: distance > normal || state.presses(own, data),
            });
        }
    }
    let advance = distance.saturating_sub(reach);
    if front && advance <= state.room(&world.party, data) {
        return Ok(Strike {
            weapon: melee,
            advance,
            hindered: false,
        });
    }
    Err(match (ranged, front) {
        (Some(_), _) => Rejection::BeyondRange,
        (None, true) => Rejection::OutOfReach { stack },
        (None, false) => Rejection::NeedsRangedWeapon,
    })
}

/// What an advance would be now: how far, and whether it takes a dash.
fn advance_for(state: &CombatState, world: &World, data: &Data) -> Result<(u16, bool), Rejection> {
    let gap = state
        .nearest()
        .map_or(0, |d| d.saturating_sub(crate::encounter::ENGAGED_FEET));
    if gap == 0 {
        return Err(Rejection::AlreadyEngaged);
    }
    match state.room(&world.party, data) {
        0 => Ok((state::pace(&world.party, data).min(gap), true)),
        room => Ok((room, false)),
    }
}

/// What paying for the spell `id` with `pay` costs, or why the spell cannot be paid that way.
fn spell_cost(spell: &omnis_data::Spell, id: &str, pay: Pay) -> Result<Cost, Rejection> {
    let spell_id = || alloc::string::String::from(id);
    match pay {
        Pay::Action => Ok(spell.cost),
        Pay::BonusAction if !spell.bonus_action_available => {
            Err(Rejection::NotABonusAction { spell: spell_id() })
        }
        Pay::BonusAction if spell.preparation_required_for_bonus_action => {
            Err(Rejection::NeedsPreparation { spell: spell_id() })
        }
        Pay::BonusAction => Ok(Cost::BonusAction),
    }
}

/// What paying for the spell `id` with `pay` costs this turn, or why it cannot be paid that
/// way: the spell's own terms, then the budget. The command and the view (`combat.get`'s spell
/// rows) both ask here.
pub(crate) fn payable(
    state: &CombatState,
    spell: &omnis_data::Spell,
    id: &str,
    pay: Pay,
) -> Result<Cost, Rejection> {
    let cost = spell_cost(spell, id, pay)?;
    budget::affordable(state.budget, cost)?;
    Ok(cost)
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
            let id = cast::spell_id(data, &spell)?;
            let plan = cast::validate(state, world, data, own, id, target, rng)?;
            let def = data
                .spells
                .get(&plan.spell)
                .ok_or(Rejection::UnknownSpell {
                    spell: spell.clone(),
                })?;
            cost = payable(state, def, &spell, pay)?;
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
                strike: strike_for(state, world, data, own, stack)?,
            }
        }
        CombatCommand::Advance => {
            let (feet, dash) = advance_for(state, world, data)?;
            if !dash {
                cost = Cost::Free;
            }
            Plan::Advance { feet }
        }
        CombatCommand::Use { item, receiver } => Plan::Use(items::validate_use(
            world, data, own, &item, receiver, true,
        )?),
        CombatCommand::Dodge => Plan::Dodge,
        CombatCommand::Exchange { with } => {
            let index = usize::from(world.party.slot_of(with)?);
            if index == own {
                return Err(Rejection::SameMember);
            }
            Plan::Exchange { own, with: index }
        }
        CombatCommand::Run => Plan::Run,
        CombatCommand::Feature { feature, choice } => {
            let (plan, feature_cost) = feature::validate(world, data, own, &feature, choice)?;
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
