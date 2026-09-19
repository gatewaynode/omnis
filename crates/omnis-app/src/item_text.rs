//! Item events as text: wearing and wielding, moves between kits and the stores, and a use.
//! Sits beside `spell_text.rs`; this file renders what items add. Bevy-free.

use crate::combat_text::{Line, Names};
use omnis_sim::omnis_data::EquipSlot;
use omnis_sim::{Event, ItemPlace};

/// One line for an item event, or `None` for events this file does not render.
#[must_use]
pub fn item_line(event: &Event, names: &Names) -> Option<Line> {
    Some(match event {
        Event::Equipped { member, slot, item } => {
            let verb = match slot {
                EquipSlot::Body => "wears",
                EquipSlot::MainHand | EquipSlot::OffHand | EquipSlot::Ranged => "wields",
            };
            Line::same(format!(
                "{} {verb} {}",
                names.member(*member),
                names.item(*item)
            ))
        }
        Event::Unequipped { member, item, .. } => Line::same(format!(
            "{} takes off {}",
            names.member(*member),
            names.item(*item)
        )),
        Event::ItemMoved {
            item,
            count,
            from,
            to,
        } => {
            let what = if *count == 1 {
                names.item(*item).to_owned()
            } else {
                format!("{count} {}", names.item(*item))
            };
            Line::same(match (from, to) {
                (ItemPlace::Member(from), ItemPlace::Member(to)) => format!(
                    "{} gives {what} to {}",
                    names.member(*from),
                    names.member(*to)
                ),
                (ItemPlace::Member(from), ItemPlace::Stores) => {
                    format!("{} stows {what}", names.member(*from))
                }
                (ItemPlace::Stores, ItemPlace::Member(to)) => {
                    format!("{} takes {what}", names.member(*to))
                }
                (ItemPlace::Stores, ItemPlace::Stores) => format!("{what} stays in the stores"),
            })
        }
        Event::ItemUsed {
            member,
            item,
            target,
            ..
        } => {
            let who = names.member(*member);
            let what = names.item(*item);
            Line::same(match target {
                Some(t) if t != member => format!("{who} uses {what} on {}", names.member(*t)),
                _ => format!("{who} uses {what}"),
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::tests::{data, facing};
    use omnis_sim::items::item_id;

    #[test]
    fn item_events_read_as_one_line_each() {
        let data = data();
        let world = facing(&data, &["fighter", "cleric"], &[]);
        let names = Names::new(&world, &data);
        let (brenna, gorm) = (world.party.members[0].id, world.party.members[1].id);
        let (sword, bolts, potion) = (
            item_id(&data, "longsword").unwrap(),
            item_id(&data, "crossbow_bolts").unwrap(),
            item_id(&data, "potion_of_healing").unwrap(),
        );
        let line = |event: &Event| item_line(event, &names).unwrap().long;
        assert_eq!(
            line(&Event::Equipped {
                member: brenna,
                slot: EquipSlot::MainHand,
                item: sword
            }),
            "Brenna wields Longsword"
        );
        assert_eq!(
            line(&Event::Equipped {
                member: brenna,
                slot: EquipSlot::Body,
                item: item_id(&data, "chain_mail").unwrap()
            }),
            "Brenna wears Chain mail"
        );
        assert_eq!(
            line(&Event::Unequipped {
                member: brenna,
                slot: EquipSlot::MainHand,
                item: sword
            }),
            "Brenna takes off Longsword"
        );
        assert_eq!(
            line(&Event::ItemMoved {
                item: bolts,
                count: 5,
                from: ItemPlace::Member(brenna),
                to: ItemPlace::Member(gorm)
            }),
            "Brenna gives 5 Crossbow bolt to Gorm"
        );
        assert_eq!(
            line(&Event::ItemMoved {
                item: sword,
                count: 1,
                from: ItemPlace::Member(brenna),
                to: ItemPlace::Stores
            }),
            "Brenna stows Longsword"
        );
        assert_eq!(
            line(&Event::ItemMoved {
                item: sword,
                count: 1,
                from: ItemPlace::Stores,
                to: ItemPlace::Member(gorm)
            }),
            "Gorm takes Longsword"
        );
        assert_eq!(
            line(&Event::ItemUsed {
                member: brenna,
                item: potion,
                target: Some(gorm),
                consumed: true
            }),
            "Brenna uses Potion of healing on Gorm"
        );
        assert_eq!(
            line(&Event::ItemUsed {
                member: brenna,
                item: potion,
                target: Some(brenna),
                consumed: true
            }),
            "Brenna uses Potion of healing"
        );
        assert!(item_line(&Event::PartyChanged, &names).is_none());
    }
}
