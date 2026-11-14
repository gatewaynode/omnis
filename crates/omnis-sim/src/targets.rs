//! What an option may be aimed at (owner, 2026-11-14; PRD §7.3, §8.3;
//! `tasks/plans/targeting.md`). The flow is always: choose the option, then choose from its
//! possible targets, then the target is validated, the cost paid and the option carried out.
//!
//! Two things live here. [`in_range`] is the range rule, which the command paths call inside
//! their own validation. [`targets`] lists an option's possible targets and judges each row by
//! validating the very command a client would send, so the list and the command cannot
//! disagree: a row with no refusal is accepted, a row with one is rejected with it.

pub(crate) mod map;

use crate::casting;
use crate::combat::cast::Target;
use crate::combat::{self, CombatCommand, CombatState, FeatureChoice, Pay, Roller};
use crate::command::Rejection;
use crate::items;
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::{Data, FeatureEffect, Range, SpellEffect, Targeting, UseEffect, Utility};
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
    /// A weapon attack: at a stack in a fight, at a standing group in sight on the road.
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
    /// On the road, tiles from the party as a king moves; for a stack in a fight, its feet
    /// from the party's front line; 0 for a member.
    pub distance: u16,
    /// Whether the line to it is clear.
    pub sight: Sight,
    /// Why the rules would refuse the option on this target now, if they would.
    pub refusal: Option<Rejection>,
}

/// The range rule: whether `target` is within what `targeting` reaches for the member in slot
/// `own`. `fight` is the fight when one is on. Members share the party's tile, so every member
/// is within touch, range and sight of every other. A stack stands at its distance in feet: a
/// touch reaches five feet from the front row (the line advancing to it when it can), a range
/// in tiles is that many of the map's tiles in feet, and sight reaches every stack. A door or
/// an object is reached on the road only, by where it stands (`map`).
pub(crate) fn in_range(
    world: &World,
    data: &Data,
    fight: Option<&CombatState>,
    own: usize,
    targeting: Targeting,
    target: &Target,
) -> Result<(), Rejection> {
    let actor = world.party.members.get(own).map(|m| m.id);
    let of_the_map = matches!(
        target,
        Target::Door { .. } | Target::Object { .. } | Target::Being { .. }
    );
    if of_the_map && fight.is_some() {
        return Err(Rejection::WrongTarget);
    }
    if targeting.range == Range::Link {
        // A link reaches what its holder is linked to, wherever it is, and nothing else.
        let held = actor.and_then(|id| world.links.get(&id));
        if !held.is_some_and(|l| l.contains(&map::canonical(target.clone()))) {
            return Err(Rejection::BeyondRange);
        }
        return map::linked(world, data, target).map(|_| ());
    }
    match (targeting.range, target) {
        (_, Target::Door { x, y, facing }) => {
            let spot = map::door(world, data, *x, *y, *facing).ok_or(Rejection::NoDoor)?;
            map::within(world, data, targeting, spot)
        }
        (_, Target::Object { object }) => {
            let (_, spot) =
                map::object(world, data, object).ok_or_else(|| Rejection::NoSuchObject {
                    object: object.clone(),
                })?;
            map::within(world, data, targeting, spot)
        }
        (_, Target::Being { being }) => {
            let spot = map::being(world, data, being).ok_or_else(|| Rejection::NoSuchBeing {
                being: being.clone(),
            })?;
            map::within(world, data, targeting, spot)
        }
        (Range::Own, Target::Member(id)) if Some(*id) == actor => Ok(()),
        (Range::Own | Range::Link, _) => Err(Rejection::BeyondRange),
        (Range::Touch | Range::Tiles(_) | Range::Sight, Target::Member(_)) => Ok(()),
        (range, Target::Stack(stack)) => {
            let standing = fight.and_then(|s| s.encounter.stacks.get(usize::from(*stack)));
            let (Some(state), Some(distance)) = (fight, standing.map(|s| s.distance)) else {
                return Err(Rejection::OutOfReach { stack: *stack });
            };
            match range {
                // A touch is a reach of five feet from the front row, the line advancing to it
                // when the round's pace and the nearest stack leave room.
                Range::Touch => {
                    let gap = distance.saturating_sub(crate::encounter::ENGAGED_FEET);
                    let front = own < crate::party::front_row(data);
                    if front && gap <= state.room(&world.party, data) {
                        Ok(())
                    } else {
                        Err(Rejection::OutOfReach { stack: *stack })
                    }
                }
                Range::Tiles(n) if distance > crate::encounter::feet_off(world, data, n.into()) => {
                    Err(Rejection::BeyondRange)
                }
                _ => Ok(()),
            }
        }
    }
}

/// A row before it is judged: the target, how far, and whether seen.
type Row = (Target, u16, Sight);

/// Why the rules would refuse the option on a target now, if they would.
type Judge<'a> = dyn Fn(&Target) -> Option<Rejection> + 'a;

/// Rows with no distance: members, and the combatants of a fight.
fn near(targets: Vec<Target>) -> Vec<Row> {
    let rows = targets.into_iter();
    rows.map(|target| (target, 0, Sight::Clear)).collect()
}

/// Things of the map as rows.
fn seen(found: Vec<(Target, map::Spot)>) -> Vec<Row> {
    let found = found.into_iter();
    found
        .map(|(target, spot)| (target, spot.distance, spot.sight))
        .collect()
}

/// What a hand may be sent at on the road: the doors and objects its range reaches or would
/// with a clear line, less the objects that offer a hand nothing.
fn hand_rows(world: &World, data: &Data, targeting: Targeting) -> Vec<Row> {
    let nothing = Rejection::VerbRefused {
        reason: crate::objects::Refusal::NotOffered,
    };
    let things = map::things(world, data, targeting).into_iter();
    things
        .filter(|(target, _)| match target {
            Target::Object { object } => map::object(world, data, object).is_some_and(|(p, _)| {
                crate::objects::pick(world, data, p, &crate::utility::HAND)
                    .err()
                    .as_ref()
                    != Some(&nothing)
            }),
            _ => true,
        })
        .map(|(target, spot)| (target, spot.distance, spot.sight))
        .collect()
}

/// Every member, in marching order.
fn members(world: &World) -> Vec<Target> {
    let ids = world.party.members.iter().map(|m| m.id);
    ids.map(Target::Member).collect()
}

/// Every stack still standing in the fight, with its feet from the party's front line.
fn stacks(state: &CombatState) -> Vec<Row> {
    let stacks = state.encounter.stacks.iter().enumerate();
    stacks
        .filter(|(_, stack)| stack.alive())
        .filter_map(|(index, stack)| Some((u8::try_from(index).ok()?, stack.distance)))
        .map(|(index, feet)| (Target::Stack(index), feet, Sight::Clear))
        .collect()
}

/// The actor's links that an option of this kind may be sent at and that are still there.
fn link_rows(world: &World, data: &Data, own: &Target, fits: impl Fn(&Target) -> bool) -> Vec<Row> {
    let Target::Member(actor) = own else {
        return Vec::new();
    };
    let held = world.links.get(actor).into_iter().flatten();
    held.filter(|target| fits(target))
        .filter_map(|target| {
            let (distance, sight) = map::linked(world, data, target).ok()?;
            Some((target.clone(), distance, sight))
        })
        .collect()
}

/// What a spell's rows are: the caster alone when it is the caster's own or lands on the whole
/// party, the members when it helps one, the standing stacks when it harms one, and for a hand
/// on the road the things of the map.
fn spell_rows(world: &World, data: &Data, own: Target, spell: &str) -> Vec<Row> {
    let def = combat::cast::spell_id(data, spell)
        .ok()
        .and_then(|id| data.spells.get(&id));
    let Some((def, effect)) = def.and_then(|d| d.effect.as_ref().map(|e| (d, e))) else {
        return Vec::new();
    };
    let hand = matches!(effect, SpellEffect::Utility(Utility::OpenDoor { .. }));
    if def.targeting().range == Range::Link {
        let road = matches!(world.mode, Mode::Explore);
        return link_rows(world, data, &own, |target| match target {
            Target::Member(_) => effect.targets_members(),
            Target::Stack(_) => effect.strikes() && !road,
            Target::Door { .. } | Target::Object { .. } => hand && road,
            Target::Being { .. } => effect.strikes() && road,
        });
    }
    if hand && matches!(world.mode, Mode::Explore) {
        return hand_rows(world, data, def.targeting());
    }
    if effect.strikes() && matches!(world.mode, Mode::Explore) {
        return seen(map::beings(world, data, def.targeting()));
    }
    let on_the_party = matches!(
        effect,
        SpellEffect::Light { .. } | SpellEffect::Reaction { .. } | SpellEffect::Utility(_)
    );
    if on_the_party || def.targeting().range == Range::Own {
        return near(alloc::vec![own]);
    }
    match &world.mode {
        _ if effect.targets_members() => near(members(world)),
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
    target: &Target,
) -> Option<CombatCommand> {
    Some(match (chosen, target.clone()) {
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
        near(all.filter(|t| *t != own).collect())
    };
    let item_rows = |item: &str| near(item_rows(world, data, slot, own.clone(), item));
    let (rows, judge): (Vec<Row>, &Judge<'_>) = match &world.mode {
        Mode::Combat(state) => {
            if state.current_actor() != Some(crate::event::ActorRef::Member(member)) {
                return Err(Rejection::NotYourTurn);
            }
            let rows = match chosen {
                Chosen::Spell { spell, .. } => spell_rows(world, data, own.clone(), spell),
                Chosen::Item { item } => item_rows(item),
                Chosen::Feature { exchange: true, .. } | Chosen::Exchange => others(),
                Chosen::Feature { .. } => near(alloc::vec![own.clone()]),
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
                Chosen::Spell { spell, .. } => spell_rows(world, data, own.clone(), spell),
                Chosen::Item { item } => item_rows(item),
                Chosen::Attack => seen(map::seen(world, data)),
                Chosen::Feature { .. } | Chosen::Exchange => Vec::new(),
            };
            (rows, &|target| match (chosen, target) {
                (Chosen::Spell { spell, .. }, target) => {
                    let mut rng = Roller::take_stream(world, "cast").rng;
                    casting::validate(world, data, member, spell, target.clone(), &mut rng).err()
                }
                (Chosen::Item { item }, Target::Member(receiver)) => {
                    items::use_refusal(world, data, member, item, Some(*receiver))
                }
                (Chosen::Attack, target) => {
                    crate::assault::validate(world, data, member, target).err()
                }
                _ => Some(Rejection::WrongTarget),
            })
        }
        Mode::Encounter(_) | Mode::Town(_) => return Ok(Vec::new()),
    };
    Ok(rows
        .into_iter()
        .map(|(target, distance, sight)| TargetView {
            refusal: judge(&target),
            target,
            distance,
            sight,
        })
        .collect())
}
