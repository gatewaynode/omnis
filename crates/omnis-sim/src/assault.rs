//! A weapon attack from the road (owner, 2026-11-14; PRD §8.2; `tasks/plans/combat-in-feet.md`
//! step 6): a member strikes a group standing in sight with the weapon that reaches it, and the
//! fight starts as it does for a spell's opening strike, at the distance the blow crossed.

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

/// A road attack that passed every check.
pub(crate) struct Checked {
    own: usize,
    weapon: Weapon,
    /// Past the weapon's normal range: the attack has disadvantage.
    hindered: bool,
    being: String,
}

/// Every check of a road attack: a member who can act, a group the party sees, and a weapon
/// that reaches it. A melee weapon is used when its reach covers the distance (the back row
/// stands a square further off); else the ranged weapon within its long range, hindered past
/// its normal one.
pub(crate) fn validate(
    world: &World,
    data: &Data,
    member: CharacterId,
    target: &Target,
) -> Result<Checked, Rejection> {
    let own = usize::from(world.party.slot_of(member)?);
    let actor = &world.party.members[own];
    if state::is_dead(actor, data) {
        return Err(Rejection::MemberDead { member });
    }
    if actor.is_down() {
        return Err(Rejection::MemberDown { member });
    }
    let Target::Being { being } = target else {
        return Err(Rejection::WrongTarget);
    };
    let spot =
        crate::targets::map::being(world, data, being).ok_or_else(|| Rejection::NoSuchBeing {
            being: being.clone(),
        })?;
    let feet = encounter::feet_off(world, data, spot.distance);
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
        being: being.clone(),
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
        being,
    } = validate(world, data, member, target)?;
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
