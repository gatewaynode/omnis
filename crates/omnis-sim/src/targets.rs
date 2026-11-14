//! What an option may be aimed at (owner, 2026-11-14; PRD §7.3, §8.3;
//! `tasks/plans/targeting.md`). The flow is always: choose the option, then choose from its
//! possible targets, then the target is validated, the cost paid and the option carried out.
//!
//! Two things live here. [`in_range`] is the range rule, which the command paths call inside
//! their own validation. [`targets`] lists an option's possible targets and judges each row by
//! validating the very command a client would send, so the list and the command cannot
//! disagree: a row with no refusal is accepted, a row with one is rejected with it.

use crate::casting;
use crate::combat::cast::Target;
use crate::combat::{self, CombatCommand, CombatState, FeatureChoice, Pay, Roller};
use crate::command::Rejection;
use crate::items;
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::{Data, FeatureEffect, Range, SpellEffect, Targeting, UseEffect};
use serde::{Deserialize, Serialize};

/// The option a member has chosen, whose targets are asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Chosen {
    /// A spell the member knows.
    Spell {
        /// The spell's string id.
        spell: String,
        /// In a fight, what the cast is paid with.
        #[serde(default)]
        pay: Pay,
    },
    /// The use of an item the member carries.
    Item {
        /// The item's string id.
        item: String,
    },
    /// A class feature, in a fight.
    Feature {
        /// The feature's name key.
        feature: String,
        /// A feature that offers an exchange (Cunning Action): the exchange, not its other use.
        #[serde(default)]
        exchange: bool,
    },
    /// A weapon attack, in a fight.
    Attack,
    /// An exchange of places, in a fight.
    Exchange,
}

/// Whether the line to a target is clear.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Sight {
    /// Nothing stands between.
    #[default]
    Clear,
    /// Something does: the party knows of the target and cannot see it.
    Obscured,
}

/// One possible target of an option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetView {
    /// What would be aimed at, as the command takes it.
    pub target: Target,
    /// Tiles from the party, as a king moves; 0 for a member and for a combatant in a fight,
    /// which has no distances.
    pub distance: u16,
    /// Whether the line to it is clear.
    pub sight: Sight,
    /// Why the rules would refuse the option on this target now, if they would.
    pub refusal: Option<Rejection>,
}

/// The range rule: whether `target` is within what `targeting` reaches for the member in slot
/// `own`. `fight` is the fight when one is on. Members share the party's tile, so every member
/// is within touch, range and sight of every other; a fight has no distances, so there touch
/// is the front stacks and every other range every combatant.
pub(crate) fn in_range(
    world: &World,
    data: &Data,
    fight: Option<&CombatState>,
    own: usize,
    targeting: Targeting,
    target: Target,
) -> Result<(), Rejection> {
    let actor = world.party.members.get(own).map(|m| m.id);
    match (targeting.range, target) {
        (Range::Own, Target::Member(id)) if Some(id) == actor => Ok(()),
        (Range::Own | Range::Link, _) => Err(Rejection::BeyondRange),
        (Range::Touch | Range::Tiles(_) | Range::Sight, Target::Member(_)) => Ok(()),
        (Range::Touch, Target::Stack(stack)) => match fight {
            Some(state) if state.is_front(data, stack) => Ok(()),
            _ => Err(Rejection::OutOfReach { stack }),
        },
        (Range::Tiles(_) | Range::Sight, Target::Stack(_)) => Ok(()),
    }
}

/// Every member, in marching order.
fn members(world: &World) -> Vec<Target> {
    let ids = world.party.members.iter().map(|m| m.id);
    ids.map(Target::Member).collect()
}

/// Every stack still standing in the fight.
fn stacks(state: &CombatState) -> Vec<Target> {
    let stacks = state.encounter.stacks.iter().enumerate();
    stacks
        .filter(|(_, stack)| stack.alive())
        .filter_map(|(index, _)| u8::try_from(index).ok())
        .map(Target::Stack)
        .collect()
}

/// What a spell's rows are: the caster alone when it is the caster's own or lands on the whole
/// party, the members when it helps one, the standing stacks when it harms one.
fn spell_rows(world: &World, data: &Data, own: Target, spell: &str) -> Vec<Target> {
    let def = combat::cast::spell_id(data, spell)
        .ok()
        .and_then(|id| data.spells.get(&id));
    let Some((def, effect)) = def.and_then(|d| d.effect.as_ref().map(|e| (d, e))) else {
        return Vec::new();
    };
    let on_the_party = matches!(
        effect,
        SpellEffect::Light { .. } | SpellEffect::Reaction { .. } | SpellEffect::Utility(_)
    );
    if on_the_party || def.targeting().range == Range::Own {
        return alloc::vec![own];
    }
    match &world.mode {
        _ if effect.targets_members() => members(world),
        Mode::Combat(state) => stacks(state),
        _ => Vec::new(),
    }
}

/// What an item's rows are: its user alone when the use is their own, else the members.
fn item_rows(world: &World, data: &Data, slot: usize, own: Target, item: &str) -> Vec<Target> {
    let carried = world.party.members[slot].equipment.iter();
    let def = carried
        .filter_map(|(id, _)| data.items.get(id))
        .find(|def| def.id == item);
    match def.and_then(|d| d.use_effect.as_ref().zip(d.targeting())) {
        Some((UseEffect::Heal { .. }, targeting)) if targeting.range != Range::Own => {
            members(world)
        }
        Some(_) => alloc::vec![own],
        None => Vec::new(),
    }
}

/// The command a client would send for `chosen` on `target` in a fight, when the two fit.
fn fight_command(
    world: &World,
    data: &Data,
    slot: usize,
    chosen: &Chosen,
    target: Target,
) -> Option<CombatCommand> {
    Some(match (chosen, target) {
        (Chosen::Spell { spell, pay }, target) => CombatCommand::Cast {
            spell: spell.clone(),
            target,
            pay: *pay,
        },
        (Chosen::Item { item }, Target::Member(receiver)) => CombatCommand::Use {
            item: item.clone(),
            receiver: Some(receiver),
        },
        (Chosen::Attack, Target::Stack(stack)) => CombatCommand::Attack { stack },
        (Chosen::Exchange, Target::Member(with)) => CombatCommand::Exchange { with },
        (Chosen::Feature { feature, exchange }, Target::Member(with)) => {
            let member = &world.party.members[slot];
            let cunning = omnis_rules::combat_features(member, data)
                .into_iter()
                .any(|f| f.name == *feature && f.effect == Some(FeatureEffect::Cunning));
            let choice = match (cunning, exchange) {
                (true, true) => FeatureChoice::Exchange { with },
                (true, false) => FeatureChoice::Hide,
                (false, _) => FeatureChoice::None,
            };
            CombatCommand::Feature {
                feature: feature.clone(),
                choice,
            }
        }
        _ => return None,
    })
}

/// The possible targets of `chosen` for `member`, each with why the rules would refuse it
/// now. An option with no one to choose (dodge, run, end turn) is not asked here. Outside a
/// fight and the road (in a service, before an encounter) nothing is listed.
///
/// # Errors
/// `NoSuchMember` for a member not in the party; in a fight, `NotYourTurn` for anyone but the
/// member whose turn it is.
pub fn targets(
    world: &World,
    data: &Data,
    member: CharacterId,
    chosen: &Chosen,
) -> Result<Vec<TargetView>, Rejection> {
    let slot = usize::from(world.party.slot_of(member)?);
    let own = Target::Member(member);
    let others = || {
        let all = members(world).into_iter();
        all.filter(|t| *t != own).collect::<Vec<_>>()
    };
    let (rows, judge): (Vec<Target>, &dyn Fn(Target) -> Option<Rejection>) = match &world.mode {
        Mode::Combat(state) => {
            if state.current_actor() != Some(crate::event::ActorRef::Member(member)) {
                return Err(Rejection::NotYourTurn);
            }
            let rows = match chosen {
                Chosen::Spell { spell, .. } => spell_rows(world, data, own, spell),
                Chosen::Item { item } => item_rows(world, data, slot, own, item),
                Chosen::Feature { exchange: true, .. } | Chosen::Exchange => others(),
                Chosen::Feature { .. } => alloc::vec![own],
                Chosen::Attack => stacks(state),
            };
            (rows, &|target| {
                let command = fight_command(world, data, slot, chosen, target);
                command.map_or(Some(Rejection::WrongTarget), |command| {
                    combat::refusal(world, data, command)
                })
            })
        }
        Mode::Explore => {
            let rows = match chosen {
                Chosen::Spell { spell, .. } => spell_rows(world, data, own, spell),
                Chosen::Item { item } => item_rows(world, data, slot, own, item),
                Chosen::Feature { .. } | Chosen::Attack | Chosen::Exchange => Vec::new(),
            };
            (rows, &|target| match (chosen, target) {
                (Chosen::Spell { spell, .. }, target) => {
                    let mut rng = Roller::take_stream(world, "cast").rng;
                    casting::validate(world, data, member, spell, target, &mut rng).err()
                }
                (Chosen::Item { item }, Target::Member(receiver)) => {
                    items::use_refusal(world, data, member, item, Some(receiver))
                }
                _ => Some(Rejection::WrongTarget),
            })
        }
        Mode::Encounter(_) | Mode::Town(_) => return Ok(Vec::new()),
    };
    Ok(rows
        .into_iter()
        .map(|target| TargetView {
            target,
            distance: 0,
            sight: Sight::Clear,
            refusal: judge(target),
        })
        .collect())
}
