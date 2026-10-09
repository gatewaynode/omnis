//! The tactics commands (ARCHITECTURE.md §4.7, M7c): the reactions switch, accepted at any time
//! and costing nothing, and declaring or removing a reaction in the default runbook, accepted
//! outside a fight. Every input is player data and is checked whole before anything changes:
//! names and caps (`omnis_rules::tactics`), the ids a predicate names, an action the member has
//! that costs a reaction, and a trigger that action can answer.

use crate::command::Rejection;
use crate::event::Event;
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::{Cost, Data, SpellEffect};
use omnis_rules::tactics::{LIBRARY_SETS, RUNBOOK_ENTRIES};
use omnis_rules::{ActionRef, Character, Criteria, CriteriaSet, Predicate, TacticsFault, Trigger};
use serde::{Deserialize, Serialize};

/// A change to a member's tactics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TacticsCommand {
    /// Switch the member's reactions on or off; any time, in a fight too.
    SetReactions {
        /// The member's slot.
        member: CharacterId,
        /// On or off.
        on: bool,
    },
    /// Declare a reaction: the set goes in the library (once) and its entry in the default
    /// runbook, replacing the entry at `at` or after the last.
    PutReaction {
        /// The member's slot.
        member: CharacterId,
        /// The entry to replace; `None` appends.
        at: Option<u8>,
        /// The trigger, the criteria and the action.
        set: CriteriaSet,
    },
    /// Remove an entry from the default runbook; the library keeps the set.
    RemoveReaction {
        /// The member's slot.
        member: CharacterId,
        /// The entry.
        at: u8,
    },
}

impl TacticsCommand {
    /// Whether the command is accepted in a fight.
    #[must_use]
    pub const fn in_fight(&self) -> bool {
        matches!(self, TacticsCommand::SetReactions { .. })
    }
}

/// Apply a tactics command.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    command: &TacticsCommand,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let id = match command {
        TacticsCommand::SetReactions { member, .. }
        | TacticsCommand::PutReaction { member, .. }
        | TacticsCommand::RemoveReaction { member, .. } => *member,
    };
    let slot = world.party.slot_of(id)?;
    let member = &mut world.party.members[usize::from(slot)];
    match command {
        TacticsCommand::SetReactions { on, .. } => {
            member.tactics.reactions_on = *on;
            events.push(Event::ReactionsSwitched {
                member: member.id,
                on: *on,
            });
            return Ok(());
        }
        TacticsCommand::PutReaction { at, set, .. } => put(member, data, *at, set)?,
        TacticsCommand::RemoveReaction { at, .. } => {
            let book = default_book(member)?;
            if usize::from(*at) >= book.entries.len() {
                return Err(Rejection::NoSuchEntry { at: *at });
            }
            book.entries.remove(usize::from(*at));
        }
    }
    events.push(Event::TacticsChanged { member: member.id });
    Ok(())
}

fn default_book(member: &mut Character) -> Result<&mut omnis_rules::Runbook, Rejection> {
    let at = usize::from(member.tactics.default_runbook);
    member
        .tactics
        .runbooks
        .get_mut(at)
        .ok_or(Rejection::Tactics(TacticsFault::NoDefault))
}

fn put(
    member: &mut Character,
    data: &Data,
    at: Option<u8>,
    set: &CriteriaSet,
) -> Result<(), Rejection> {
    set.check().map_err(Rejection::Tactics)?;
    check_ids(&set.when, data)?;
    if !answers(member, data, &set.action, set.trigger) {
        return Err(Rejection::CannotReact);
    }
    let found = member.tactics.library.iter().position(|s| s == set);
    if found.is_none() && member.tactics.library.len() >= LIBRARY_SETS {
        return Err(Rejection::Tactics(TacticsFault::TooMany));
    }
    let entries = default_book(member)?.entries.len();
    match at {
        Some(at) if usize::from(at) >= entries => return Err(Rejection::NoSuchEntry { at }),
        None if entries >= RUNBOOK_ENTRIES => {
            return Err(Rejection::Tactics(TacticsFault::TooMany));
        }
        _ => {}
    }
    let index = match found {
        Some(index) => index,
        None => {
            member.tactics.library.push(set.clone());
            member.tactics.library.len() - 1
        }
    };
    let entry = (
        set.action.clone(),
        u16::try_from(index).map_err(|_| Rejection::Tactics(TacticsFault::TooMany))?,
    );
    let book = default_book(member)?;
    match at {
        Some(at) => book.entries[usize::from(at)] = entry,
        None => book.entries.push(entry),
    }
    Ok(())
}

/// Every monster and condition a tree names is loaded.
fn check_ids(criteria: &Criteria, data: &Data) -> Result<(), Rejection> {
    match criteria {
        Criteria::Always => Ok(()),
        Criteria::All(children) | Criteria::Any(children) => {
            children.iter().try_for_each(|c| check_ids(c, data))
        }
        Criteria::Is(
            Predicate::MonsterCount { monster, .. } | Predicate::MonsterShare { monster, .. },
        ) if !data.monsters.contains_key(monster) => Err(Rejection::UnknownId {
            id: alloc::format!("monster {}", monster.0),
        }),
        Criteria::Is(Predicate::HasCondition { condition, .. })
            if !data.conditions.contains_key(condition) =>
        {
            Err(Rejection::UnknownId {
                id: alloc::format!("condition {}", condition.0),
            })
        }
        Criteria::Is(_) => Ok(()),
    }
}

/// Whether the member has `action` as a reaction and it can answer `trigger`: a known spell that
/// costs a reaction (an armor bonus answers an attack on the caster; a heal answers an attack,
/// a wound or a fall in the row). The weapon answers nothing until a source raises
/// `EnemyFlees` (owner, 2026-10-04, B2: the panel offers only what the game can fire).
pub fn answers(member: &Character, data: &Data, action: &ActionRef, trigger: Trigger) -> bool {
    match action {
        ActionRef::Spell(id) => {
            let Some(spell) = data.spells.get(id) else {
                return false;
            };
            if !member.known_spells.contains(id) || spell.cost != Cost::Reaction {
                return false;
            }
            match spell.effect {
                Some(SpellEffect::Reaction { .. }) => trigger == Trigger::Attacked,
                Some(SpellEffect::Heal { .. }) => matches!(
                    trigger,
                    Trigger::Attacked
                        | Trigger::MemberAttacked
                        | Trigger::MemberWounded
                        | Trigger::MemberDying
                ),
                _ => false,
            }
        }
        ActionRef::Attack | ActionRef::Item(_) | ActionRef::Feature(_) => false,
    }
}
