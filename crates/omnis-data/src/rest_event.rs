//! A map's rest events: what may happen while the party rests on a terrain of that map (owner,
//! 2026-09-27: stubs for towns, roads and meadows, filled in later). Each entry names a terrain,
//! the kind of rest, a per-mille chance and a line of text; the simulation rolls it after the
//! ambush check and reports a hit, which does nothing else yet.

use crate::error::DataError;
use crate::limits::MAX_COLLECTION;
use crate::map::MapDef;
use crate::registry::Interner;
use omnis_core::TextKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// The chance's scale: a d1000.
pub const PER_MILLE: u16 = 1000;

/// Which rest an entry applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RestKind {
    /// The hour's rest that spends hit dice.
    Short,
    /// The night's rest that costs food.
    Long,
    /// Either.
    Any,
}

impl RestKind {
    /// Whether an entry for `self` applies to a rest that is long or not.
    #[must_use]
    pub const fn covers(self, long: bool) -> bool {
        match self {
            RestKind::Short => !long,
            RestKind::Long => long,
            RestKind::Any => true,
        }
    }
}

/// One entry as written in the map file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestEventDef {
    /// A terrain name of this map.
    pub terrain: String,
    /// The rest it applies to.
    pub rest: RestKind,
    /// Per mille, 0 to 1000.
    pub chance: u16,
    /// Text key of the line the log shows.
    pub text: String,
}

/// An entry with its terrain as an index and its text interned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedRestEvent {
    /// Index into the map's terrains.
    pub terrain: u8,
    /// The rest it applies to.
    pub rest: RestKind,
    /// Per mille.
    pub chance: u16,
    /// The log line.
    pub text: TextKey,
}

/// Resolve a map's rest events: every terrain on the map, every chance at most 1000, one entry
/// per terrain and kind, every text key defined. Entries keep their file order (the index a
/// `RestEvent` names).
pub(crate) fn resolve(
    def: &MapDef,
    text: &Interner<TextKey>,
    file: &Path,
    errors: &mut Vec<DataError>,
) -> Vec<ResolvedRestEvent> {
    if def.rest_events.len() > MAX_COLLECTION {
        errors.push(DataError::new(
            file,
            format!("more than {MAX_COLLECTION} rest events"),
        ));
        return Vec::new();
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(def.rest_events.len());
    for (i, entry) in def.rest_events.iter().enumerate() {
        let before = errors.len();
        let terrain = def
            .terrains
            .iter()
            .position(|t| t.name == entry.terrain)
            .and_then(|t| u8::try_from(t).ok());
        if terrain.is_none() {
            errors.push(DataError::new(
                file,
                format!(
                    "rest event {i} names terrain '{}', which this map does not have",
                    entry.terrain
                ),
            ));
        }
        if entry.chance > PER_MILLE {
            errors.push(DataError::new(
                file,
                format!(
                    "rest event {i} has chance {} per mille; at most {PER_MILLE}",
                    entry.chance
                ),
            ));
        }
        if !seen.insert((entry.terrain.as_str(), entry.rest)) {
            errors.push(DataError::new(
                file,
                format!(
                    "rest event {i} repeats terrain '{}' for the same rest",
                    entry.terrain
                ),
            ));
        }
        let key = text.get(&entry.text);
        if key.is_none() {
            errors.push(DataError::new(
                file,
                format!(
                    "rest event {i} text key '{}' is not defined in any language",
                    entry.text
                ),
            ));
        }
        if let (Some(terrain), Some(text), true) = (terrain, key, errors.len() == before) {
            out.push(ResolvedRestEvent {
                terrain,
                rest: entry.rest,
                chance: entry.chance,
                text,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_covers_its_rests() {
        assert!(RestKind::Short.covers(false) && !RestKind::Short.covers(true));
        assert!(RestKind::Long.covers(true) && !RestKind::Long.covers(false));
        assert!(RestKind::Any.covers(true) && RestKind::Any.covers(false));
    }
}
