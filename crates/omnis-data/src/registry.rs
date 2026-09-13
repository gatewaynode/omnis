//! Interning of content ids. Files name things as `pack:type:name`; the simulation works with
//! `u32` newtypes. Interning order is load order, which is deterministic, so ids are stable
//! for one set of packs. Saves store the string form (later milestone) so a changed pack set
//! cannot silently renumber.

use omnis_core::{
    BackgroundId, ClassId, ConditionId, FlagId, ItemId, MapId, MonsterId, RaceId, SpellId, TextKey,
    TilesetId,
};
use std::collections::BTreeMap;

/// A string-to-id table for one id type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interner<I> {
    by_name: BTreeMap<String, I>,
    names: Vec<String>,
}

impl<I> Default for Interner<I> {
    fn default() -> Self {
        Interner {
            by_name: BTreeMap::new(),
            names: Vec::new(),
        }
    }
}

impl<I: Copy + From<u32> + Into<u32>> Interner<I> {
    /// The id for `name`, allocating one on first sight.
    pub fn intern(&mut self, name: &str) -> I {
        if let Some(id) = self.by_name.get(name) {
            return *id;
        }
        let index = u32::try_from(self.names.len()).expect("fewer than 2^32 ids");
        let id = I::from(index);
        self.names.push(name.to_owned());
        self.by_name.insert(name.to_owned(), id);
        id
    }

    /// The id for `name` if it has been interned.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<I> {
        self.by_name.get(name).copied()
    }

    /// The string form of `id`.
    #[must_use]
    pub fn name(&self, id: I) -> Option<&str> {
        self.names.get(id.into() as usize).map(String::as_str)
    }

    /// How many ids exist.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether nothing has been interned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Every name in id order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
}

/// Every id table the content set needs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    /// Maps.
    pub maps: Interner<MapId>,
    /// Tilesets.
    pub tilesets: Interner<TilesetId>,
    /// Text keys.
    pub text: Interner<TextKey>,
    /// World flags.
    pub flags: Interner<FlagId>,
    /// Races.
    pub races: Interner<RaceId>,
    /// Classes.
    pub classes: Interner<ClassId>,
    /// Backgrounds.
    pub backgrounds: Interner<BackgroundId>,
    /// Items.
    pub items: Interner<ItemId>,
    /// Conditions.
    pub conditions: Interner<ConditionId>,
    /// Spells.
    pub spells: Interner<SpellId>,
    /// Monsters.
    pub monsters: Interner<MonsterId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_is_stable_and_ordered() {
        let mut maps = Interner::<MapId>::default();
        let a = maps.intern("test:map:a");
        let b = maps.intern("test:map:b");
        assert_eq!(maps.intern("test:map:a"), a);
        assert_eq!((a, b), (MapId(0), MapId(1)));
        assert_eq!(maps.name(b), Some("test:map:b"));
        assert_eq!(maps.get("test:map:c"), None);
        assert_eq!(maps.name(MapId(9)), None);
        assert_eq!(maps.len(), 2);
    }
}
