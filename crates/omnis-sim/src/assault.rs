//! A weapon attack from the road (owner, 2026-11-14; PRD §8.2; `tasks/plans/combat-in-feet.md`
//! step 6): a member strikes a group standing in sight with the weapon that reaches it, and the
//! fight starts as it does for a spell's opening strike, at the distance the blow crossed.

use crate::breaking::{self, Blow, Thing};
use crate::combat::Roller;
use crate::combat::cast::Target;
use crate::combat::{self, Opening, state};
use crate::command::Rejection;
use crate::encounter;
use crate::event::Event;
use crate::party;
use crate::world::World;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::CharacterId;
use omnis_data::{Data, SQUARE_FEET};
use omnis_rules::{Weapon, best_weapon};

/// Minutes a blow at a thing costs: coarse, as the road's other actions are.
const STRIKE_MINUTES: u32 = 1;

/// A road attack that passed every check.
pub(crate) struct Checked<'a> {
    own: usize,
    weapon: Weapon,
    /// Past the weapon's normal range: the attack has disadvantage.
    hindered: bool,
    at: At<'a>,
}

/// What a road attack lands on.
enum At<'a> {
    /// A standing group: the fight starts.
    Being(String),
    /// An object that can be damaged: a minute, and no fight.
    Thing(Thing<'a>),
}

/// Every check of a road attack: a member who can act, a group the party sees or an object
/// that can be damaged, and a weapon that reaches it. An object in the party's reach is in a
/// melee weapon's; one further off is so many tiles away. A melee weapon is used when its reach covers the distance (the back row
/// stands a square further off); else the ranged weapon within its long range, hindered past
/// its normal one.
pub(crate) fn validate<'a>(
    world: &World,
    data: &'a Data,
    member: CharacterId,
    target: &Target,
) -> Result<Checked<'a>, Rejection> {
    let own = usize::from(world.party.slot_of(member)?);
    let actor = &world.party.members[own];
    if state::is_dead(actor, data) {
        return Err(Rejection::MemberDead { member });
    }
    if actor.is_down() {
        return Err(Rejection::MemberDown { member });
    }
    let (at, spot) = match target {
        Target::Being { being } => {
            let spot = crate::targets::map::being(world, data, being).ok_or_else(|| {
                Rejection::NoSuchBeing {
                    being: being.clone(),
                }
            })?;
            (At::Being(being.clone()), spot)
        }
        Target::Object { object } => {
            let thing = breaking::thing(world, data, object)?;
            if thing.spot.sight != crate::targets::Sight::Clear {
                return Err(Rejection::OutOfSight);
            }
            let spot = thing.spot;
            (At::Thing(thing), spot)
        }
        _ => return Err(Rejection::WrongTarget),
    };
    // A thing within the party's reach is touched, wherever in the two cells it stands.
    let touched = matches!(at, At::Thing(_)) && spot.touch;
    let feet = if touched {
        0
    } else {
        encounter::feet_off(world, data, spot.distance)
    };
    let behind = if own < party::front_row(data) {
        0
    } else {
        SQUARE_FEET
    };
    let melee = best_weapon(actor, data, false).unwrap_or_else(Weapon::unarmed);
    let (weapon, hindered) = if feet.saturating_add(behind) <= u16::from(melee.reach) {
        (melee, false)
    } else {
        let ranged = best_weapon(actor, data, true).ok_or(Rejection::NeedsRangedWeapon)?;
        let (normal, long) = ranged.range.unwrap_or(omnis_data::DEFAULT_RANGE);
        if feet > long {
            return Err(Rejection::BeyondRange);
        }
        (ranged, feet > normal)
    };
    Ok(Checked {
        own,
        weapon,
        hindered,
        at,
    })
}

/// Apply a road attack: the group is met where the party stands, hostile whatever it was, and
/// the blow lands in the fight it starts, as the member's action for the first round.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    member: CharacterId,
    target: &Target,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    let Checked {
        own,
        weapon,
        hindered,
        at,
    } = validate(world, data, member, target)?;
    let being = match at {
        At::Being(being) => being,
        At::Thing(thing) => {
            let mut roller = Roller::take_stream(world, "strike");
            let blow = Blow::Weapon {
                weapon: &weapon,
                hindered,
            };
            breaking::strike(world, data, own, &thing, &blow, &mut roller, events)
                .map_err(Rejection::Rule)?;
            roller.store(world);
            crate::apply::advance(world, data, STRIKE_MINUTES, events);
            return Ok(());
        }
    };
    let retreat = omnis_core::Position {
        facing: world.position.facing.opposite(),
        ..world.position
    };
    let met = encounter::struck(world, data, &being, retreat, events);
    if let Some(met) = met.map_err(Rejection::Rule)? {
        let opening = Opening::Weapon {
            own,
            weapon: &weapon,
            hindered,
        };
        combat::start_struck(world, data, met, &opening, events).map_err(Rejection::Rule)?;
    }
    Ok(())
}
