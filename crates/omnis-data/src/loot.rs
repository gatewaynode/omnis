//! `data/loot`: tables of items a monster may drop (M8.5 step 2). A monster names a table; when
//! it dies, the table is drawn `picks` times by weight, and each drawn entry then rolls its own
//! chance, so a table can say both "one of these" and "maybe".

use crate::error::DataError;
use crate::limits::{MAX_LOOT_ENTRIES, MAX_LOOT_PICKS};
use omnis_core::Dice;
use serde::{Deserialize, Serialize};
use std::path::Path;

const fn one() -> u8 {
    1
}

/// One line of a loot table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootEntry {
    /// `pack:item:name`.
    pub item: String,
    /// How many, rolled when it drops.
    pub count: Dice,
    /// Its share of a draw, at least 1.
    pub weight: u16,
    /// Once drawn, the chance in a hundred that it drops, 1..=100.
    pub chance_percent: u8,
}

/// A table of items, one file under `data/loot`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootTable {
    /// Schema version.
    pub schema: u32,
    /// `pack:loot:name`.
    pub id: String,
    /// Draws per individual that dies, 1..=8.
    #[serde(default = "one")]
    pub picks: u8,
    /// The entries, 1..=64.
    pub entries: Vec<LootEntry>,
}

impl LootTable {
    /// Shape checks that need no other file.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        let mut error = |message: String| errors.push(DataError::new(file, message));
        if !(1..=MAX_LOOT_PICKS).contains(&self.picks) {
            error(format!("picks must be 1..={MAX_LOOT_PICKS}"));
        }
        if self.entries.is_empty() || self.entries.len() > MAX_LOOT_ENTRIES {
            error(format!("a loot table needs 1..={MAX_LOOT_ENTRIES} entries"));
        }
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.count.count == 0 || entry.count.sides == 0 {
                error(format!("loot entry {i} count needs dice"));
            }
            if entry.weight == 0 {
                error(format!("loot entry {i} weight must be at least 1"));
            }
            if !(1..=100).contains(&entry.chance_percent) {
                error(format!("loot entry {i} chance_percent must be 1..=100"));
            }
        }
    }
}
