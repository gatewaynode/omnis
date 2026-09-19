//! Item stock: counted lists of items on a member or in the party's stores, and the spell
//! components a cast consumes from the stores (PRD §8.2).

use crate::command::Rejection;
use crate::party::Party;
use alloc::vec::Vec;
use omnis_core::ItemId;
use omnis_data::Data;

/// Add `count` of an item to a counted list.
pub(crate) fn add_to(list: &mut Vec<(ItemId, u16)>, item: ItemId, count: u16) {
    if count == 0 {
        return;
    }
    match list.iter_mut().find(|(i, _)| *i == item) {
        Some((_, have)) => *have = have.saturating_add(count),
        None => list.push((item, count)),
    }
}

/// How many of an item a counted list holds.
#[must_use]
pub fn count_of(list: &[(ItemId, u16)], item: ItemId) -> u16 {
    list.iter()
        .filter(|(i, _)| *i == item)
        .map(|(_, n)| *n)
        .fold(0u16, u16::saturating_add)
}

/// Remove `count` of an item from a counted list, dropping the entry at zero, or refuse
/// without touching it.
pub(crate) fn take_from(
    list: &mut Vec<(ItemId, u16)>,
    item: ItemId,
    count: u16,
) -> Result<(), Rejection> {
    let have = count_of(list, item);
    if have < count {
        return Err(Rejection::NotEnough { item, have });
    }
    let mut left = count;
    for (i, n) in list.iter_mut() {
        if *i == item && left > 0 {
            let taken = (*n).min(left);
            *n -= taken;
            left -= taken;
        }
    }
    list.retain(|(_, n)| *n > 0);
    Ok(())
}

/// Whether the party's stores hold every `(item, count)`.
#[must_use]
pub fn has_all(party: &Party, needs: &[(ItemId, u16)]) -> bool {
    needs
        .iter()
        .all(|(item, count)| count_of(&party.inventory, *item) >= *count)
}

/// Remove every `(item, count)` from the party's stores, or refuse without touching them.
pub fn consume(party: &mut Party, needs: &[(ItemId, u16)]) -> Result<(), Rejection> {
    if let Some((item, _)) = needs
        .iter()
        .find(|(item, count)| count_of(&party.inventory, *item) < *count)
    {
        return Err(Rejection::NotEnough {
            item: *item,
            have: count_of(&party.inventory, *item),
        });
    }
    for (item, count) in needs {
        take_from(&mut party.inventory, *item, *count)?;
    }
    Ok(())
}

/// The item a pack defines under `<pack>:item:<name>`: the first interned id whose name ends
/// in `:<name>`, so the simulation finds `gem` in any pack.
#[must_use]
pub fn item_id(data: &Data, name: &str) -> Option<ItemId> {
    let registry = &data.registry.items;
    (0..u32::try_from(registry.len()).unwrap_or(u32::MAX))
        .map(ItemId)
        .find(|id| {
            registry
                .name(*id)
                .is_some_and(|n| n.rsplit(':').next() == Some(name) && n.contains(':'))
        })
}
