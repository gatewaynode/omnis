//! Equipment slots: which carried item is worn or wielded where (PRD §8.1 items). The rules
//! read slots, never the whole kit; `auto_equip` picks what a new sheet wears, the same picks
//! the kit-counts-as-worn rule made before slots existed.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;
use omnis_core::ItemId;
use omnis_data::{ArmorKind, Data, EquipSlot, Item, ItemKind};
use serde::{Deserialize, Serialize};

/// What is worn and wielded, by slot.
pub type Equipped = BTreeMap<EquipSlot, ItemId>;

/// Why an item cannot be equipped. A rule refusal, not an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipRefusal {
    /// The member does not carry it.
    NotCarried,
    /// The item has no slot.
    NotEquippable,
    /// A two-handed weapon and a shield cannot both be held.
    HandsFull,
}

impl fmt::Display for EquipRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EquipRefusal::NotCarried => "the item is not carried",
            EquipRefusal::NotEquippable => "the item cannot be worn or wielded",
            EquipRefusal::HandsFull => "a two-handed weapon leaves no hand for a shield",
        })
    }
}

fn two_handed(data: &Data, equipped: &Equipped, slot: EquipSlot) -> bool {
    equipped
        .get(&slot)
        .and_then(|id| data.items.get(id))
        .is_some_and(Item::two_handed)
}

/// The slot the item would take, or why it cannot.
pub fn can_equip(
    data: &Data,
    carried: &[(ItemId, u16)],
    equipped: &Equipped,
    item: ItemId,
) -> Result<EquipSlot, EquipRefusal> {
    if !carried.iter().any(|(id, n)| *id == item && *n > 0) {
        return Err(EquipRefusal::NotCarried);
    }
    let def = data.items.get(&item).ok_or(EquipRefusal::NotEquippable)?;
    let slot = def.slot().ok_or(EquipRefusal::NotEquippable)?;
    let hands_full = match slot {
        EquipSlot::OffHand => two_handed(data, equipped, EquipSlot::MainHand),
        EquipSlot::MainHand => def.two_handed() && equipped.contains_key(&EquipSlot::OffHand),
        EquipSlot::Ranged | EquipSlot::Body => false,
    };
    if hands_full {
        return Err(EquipRefusal::HandsFull);
    }
    Ok(slot)
}

/// Wear or wield a carried item; the item the slot held before stays carried and is returned.
pub fn equip(
    data: &Data,
    carried: &[(ItemId, u16)],
    equipped: &mut Equipped,
    item: ItemId,
) -> Result<(EquipSlot, Option<ItemId>), EquipRefusal> {
    let slot = can_equip(data, carried, equipped, item)?;
    let displaced = equipped.insert(slot, item).filter(|old| *old != item);
    Ok((slot, displaced))
}

/// Empty a slot; the item stays carried.
pub fn unequip(equipped: &mut Equipped, slot: EquipSlot) -> Option<ItemId> {
    equipped.remove(&slot)
}

/// The item in a slot.
#[must_use]
pub fn equipped_item<'a>(data: &'a Data, equipped: &Equipped, slot: EquipSlot) -> Option<&'a Item> {
    equipped.get(&slot).and_then(|id| data.items.get(id))
}

/// What a sheet wears from its kit: the best armor for its Dexterity modifier, the best melee
/// and ranged weapons by average damage (the first carried on ties), and a shield unless the
/// melee weapon needs both hands.
#[must_use]
pub fn auto_equip(data: &Data, carried: &[(ItemId, u16)], dex: i64) -> Equipped {
    let items: Vec<(ItemId, &Item)> = carried
        .iter()
        .filter(|(_, n)| *n > 0)
        .filter_map(|(id, _)| data.items.get(id).map(|item| (*id, item)))
        .collect();
    let best = |score: &dyn Fn(&Item) -> Option<i64>| -> Option<ItemId> {
        items
            .iter()
            .filter_map(|(id, item)| score(item).map(|s| (s, *id)))
            .fold(None, |best: Option<(i64, ItemId)>, c| match best {
                Some(b) if b.0 >= c.0 => Some(b),
                _ => Some(c),
            })
            .map(|(_, id)| id)
    };
    let weapon = |ranged: bool| {
        move |item: &Item| match &item.kind {
            ItemKind::Weapon {
                damage, ranged: r, ..
            } if *r == ranged => {
                let dice = *damage;
                Some(i64::from(dice.min() + dice.max()))
            }
            _ => None,
        }
    };
    let mut out = Equipped::new();
    let picks = [
        (
            EquipSlot::Body,
            best(&|item| match &item.kind {
                ItemKind::Armor {
                    kind,
                    base_ac,
                    dex_cap,
                    ..
                } if *kind != ArmorKind::Shield => {
                    Some(i64::from(*base_ac) + dex_cap.map_or(dex, |cap| dex.min(i64::from(cap))))
                }
                _ => None,
            }),
        ),
        (EquipSlot::MainHand, best(&weapon(false))),
        (EquipSlot::Ranged, best(&weapon(true))),
    ];
    for (slot, pick) in picks {
        if let Some(id) = pick {
            out.insert(slot, id);
        }
    }
    if !two_handed(data, &out, EquipSlot::MainHand)
        && let Some(id) = best(&|item| match &item.kind {
            ItemKind::Armor {
                kind: ArmorKind::Shield,
                base_ac,
                ..
            } => Some(i64::from(*base_ac)),
            _ => None,
        })
    {
        out.insert(EquipSlot::OffHand, id);
    }
    out
}
