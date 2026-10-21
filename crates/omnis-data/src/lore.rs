//! Lore (owner, 2026-10-16): authored text a search turns up. A map writes it on cells; an object
//! kind or one placement of it carries its own, the placement's winning. Each may ask a
//! difficulty of the search's Investigation roll, and may be told once only, which the save keeps
//! (a cell's by its id, an object's by its placement). Shapes are checked with the map; levels,
//! ground and text keys when the map is resolved.

use crate::error::DataError;
use crate::limits::{MAX_CHECK_DC, MAX_COLLECTION, MAX_LORE_PER_CELL};
use crate::map::MapDef;
use crate::placement::is_placement_id;
use crate::registry::Interner;
use omnis_core::{Level, TextKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Text an object carries for a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lore {
    /// Text key of what the party learns.
    pub text: String,
    /// The Investigation roll it needs, 1..=30; none, and every search tells it.
    #[serde(default)]
    pub dc: Option<u8>,
    /// Told on the first search that meets it, never again.
    #[serde(default)]
    pub once: bool,
}

impl Lore {
    /// Whether its difficulty, if any, is in range.
    #[must_use]
    pub fn dc_in_range(&self) -> bool {
        dc_in_range(self.dc)
    }
}

/// Text written on a cell of a map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellLore {
    /// Unique among the map's lore, `[a-z0-9_]{1,32}`; the save's key for a told `once`.
    pub id: String,
    /// The level of its cell.
    #[serde(default)]
    pub level: Level,
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Text key of what the party learns.
    pub text: String,
    /// The Investigation roll it needs, 1..=30; none, and every search tells it.
    #[serde(default)]
    pub dc: Option<u8>,
    /// Told on the first search that meets it, never again.
    #[serde(default)]
    pub once: bool,
}

fn dc_in_range(dc: Option<u8>) -> bool {
    dc.is_none_or(|dc| (1..=MAX_CHECK_DC).contains(&dc))
}

/// Shape checks with the map: ids, bounds, difficulties, the count.
pub(crate) fn validate(def: &MapDef, file: &Path, errors: &mut Vec<DataError>) {
    let mut error = |message: String| errors.push(DataError::new(file, message));
    if def.lore.len() > MAX_COLLECTION {
        error(format!("more than {MAX_COLLECTION} lore"));
        return;
    }
    let mut ids = BTreeSet::new();
    for l in &def.lore {
        if !is_placement_id(&l.id) {
            error(format!("lore id '{}' is not [a-z0-9_]{{1,32}}", l.id));
        } else if !ids.insert(l.id.as_str()) {
            error(format!("lore '{}' is placed twice", l.id));
        }
        if l.x >= def.width || l.y >= def.height {
            error(format!(
                "lore '{}' at ({}, {}) is outside the map",
                l.id, l.x, l.y
            ));
        }
        if !dc_in_range(l.dc) {
            error(format!("lore '{}' dc must be 1..={MAX_CHECK_DC}", l.id));
        }
    }
}

/// A map's lore by cell, in file order: each on a level the map has, on ground, with its text
/// defined, at most [`MAX_LORE_PER_CELL`] a cell.
pub(crate) fn resolve(
    def: &MapDef,
    has_level: impl Fn(Level) -> bool,
    has_ground: impl Fn(Level, u16, u16) -> bool,
    text: &Interner<TextKey>,
    error: &mut impl FnMut(String),
) -> BTreeMap<(Level, u16, u16), Vec<CellLore>> {
    let mut out: BTreeMap<(Level, u16, u16), Vec<CellLore>> = BTreeMap::new();
    for l in &def.lore {
        if !has_level(l.level) {
            error(format!(
                "lore '{}' is on level {}, which the map does not have",
                l.id, l.level
            ));
            continue;
        }
        if !has_ground(l.level, l.x, l.y) {
            error(format!(
                "lore '{}' at ({}, {}) has no ground",
                l.id, l.x, l.y
            ));
        }
        if text.get(&l.text).is_none() {
            error(format!(
                "text key '{}' is not defined in any language",
                l.text
            ));
        }
        out.entry((l.level, l.x, l.y)).or_default().push(l.clone());
    }
    for ((level, x, y), list) in &out {
        if list.len() > MAX_LORE_PER_CELL {
            error(format!(
                "cell ({x}, {y}) on level {level} holds more than {MAX_LORE_PER_CELL} lore"
            ));
        }
    }
    out
}
