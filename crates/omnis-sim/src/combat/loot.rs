//! Loot (M8.5 step 2): the items a monster drops when it dies, drawn from the loot table it
//! names. The table is drawn `picks` times by weight; each drawn entry then rolls its chance in
//! a hundred and, when it drops, its count. The fight holds what fell and lays it on its cell on
//! victory; a flight or a defeat leaves it behind for good.

use crate::items::add_to;
use alloc::format;
use alloc::vec::Vec;
use omnis_core::{Dice, ItemId, Pcg32, StreamName};
use omnis_data::{Data, LootTable, Monster};
use omnis_rules::RuleError;

/// What one individual of `monster` drops, on `rng`; nothing when it names no table.
///
/// # Errors
/// A count's dice that cannot be rolled.
pub fn of(
    data: &Data,
    monster: &Monster,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<Vec<(ItemId, u16)>, RuleError> {
    let table = monster
        .loot
        .as_ref()
        .and_then(|name| data.registry.loot.get(name))
        .and_then(|id| data.loot.get(&id));
    match table {
        Some(table) => draw(data, table, rng, stream),
        None => Ok(Vec::new()),
    }
}

/// Draw a table: `picks` entries by weight, each then its chance and its count.
///
/// # Errors
/// A count's dice that cannot be rolled.
pub fn draw(
    data: &Data,
    table: &LootTable,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<Vec<(ItemId, u16)>, RuleError> {
    let roll = |dice: Dice, rng: &mut Pcg32| {
        dice.roll(rng, stream)
            .map(|t| t.total)
            .map_err(|e| RuleError::new("loot", format!("{e}")))
    };
    let total: u32 = table.entries.iter().map(|e| u32::from(e.weight)).sum();
    let mut dropped = Vec::new();
    for _ in 0..table.picks {
        let mut pick = rng.below(total);
        let Some(entry) = table.entries.iter().find(|e| {
            if pick < u32::from(e.weight) {
                return true;
            }
            pick -= u32::from(e.weight);
            false
        }) else {
            continue;
        };
        if roll(Dice::new(1, 100), rng)? > i32::from(entry.chance_percent) {
            continue;
        }
        let count = u16::try_from(roll(entry.count, rng)?.max(1)).unwrap_or(u16::MAX);
        if let Some(item) = data.registry.items.get(&entry.item) {
            add_to(&mut dropped, item, count);
        }
    }
    Ok(dropped)
}
