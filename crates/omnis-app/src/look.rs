//! The LOOK shortcut: which item a press of LOOK or L uses. The first sense item carried by
//! a member who can act, in marching order; the simulation picks the eyes. Bevy-free.

use crate::defs;
use omnis_sim::api::PartyView;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, ItemCommand};

/// The use command a look sends, or `None` when nobody who can act carries a sense item.
#[must_use]
pub fn look_command(party: &PartyView, data: &Data) -> Option<Command> {
    party
        .members
        .iter()
        .filter(|m| !m.down && !m.dead)
        .find_map(|m| {
            let row = m
                .equipment
                .iter()
                .position(|i| defs::item(data, &i.id).is_some_and(|i| i.sense().is_some()))?;
            Some(Command::Item(ItemCommand::Use {
                member: m.id,
                item: u8::try_from(row).ok()?,
                target: None,
            }))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::tests::{data, facing};
    use omnis_sim::items::item_id;
    use omnis_sim::{World, party_view};

    fn look_command(world: &World, data: &Data) -> Option<Command> {
        super::look_command(&party_view(world, data), data)
    }

    #[test]
    fn the_first_glass_of_a_member_who_can_act_is_the_one_used() {
        let data = data();
        let mut world = facing(&data, &["fighter", "cleric"], &[]);
        assert_eq!(look_command(&world, &data), None);
        let glass = item_id(&data, "spyglass").unwrap();
        world.party.members[1].equipment.push((glass, 1));
        let row = u8::try_from(world.party.members[1].equipment.len() - 1).unwrap();
        let (first, second) = (world.party.members[0].id, world.party.members[1].id);
        assert_eq!(
            look_command(&world, &data),
            Some(Command::Item(ItemCommand::Use {
                member: second,
                item: row,
                target: None
            }))
        );
        world.party.members[0].equipment.insert(0, (glass, 1));
        assert!(matches!(
            look_command(&world, &data),
            Some(Command::Item(ItemCommand::Use {
                member,
                item: 0,
                ..
            })) if member == first
        ));
        world.party.members[0].hp = 0;
        assert!(matches!(
            look_command(&world, &data),
            Some(Command::Item(ItemCommand::Use { member, .. })) if member == second
        ));
    }
}
