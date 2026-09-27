//! Resting (PRD §8.2, SRD 5.1). The long rest's restoration is shared by a room at the inn
//! (M7 step 4) and a rest in the field (step 5); the time, the food and the ambush are the
//! caller's.

use crate::combat::state::is_dead;
use crate::event::Event;
use crate::party;
use crate::world::World;
use alloc::vec::Vec;
use omnis_data::Data;

/// Hit dice a long rest gives back: half the member's total (one per level), at least one.
#[must_use]
pub const fn hit_dice_back(level: u8) -> u8 {
    let half = level / 2;
    if half == 0 { 1 } else { half }
}

/// The end of a long rest for every member. A member with at least one hit point regains every
/// hit point and spell point and half their hit dice; a member at zero who is not dead is
/// stable and wakes at one hit point within the eight hours, without the rest's benefits; the
/// dead stay dead. Each hit point gain is a `Healed` event.
pub(crate) fn long_rest_restore(world: &mut World, data: &Data, events: &mut Vec<Event>) {
    for index in 0..world.party.members.len() {
        let member = &mut world.party.members[index];
        if is_dead(member, data) {
            continue;
        }
        let gain = if member.is_down() {
            1 - member.hp
        } else {
            member.spell_points = member.spell_points_max;
            member.hit_dice_spent = member
                .hit_dice_spent
                .saturating_sub(hit_dice_back(member.level));
            member.hp_max - member.hp
        };
        if gain > 0 {
            party::heal(world, data, index, Vec::new(), i64::from(gain), events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_rest_gives_back_half_the_hit_dice_and_at_least_one() {
        assert_eq!(hit_dice_back(1), 1);
        assert_eq!(hit_dice_back(2), 1);
        assert_eq!(hit_dice_back(5), 2);
        assert_eq!(hit_dice_back(20), 10);
    }
}
