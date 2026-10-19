//! The name table (M8.5 7h2): what each number in a world folder's files stood for when they were
//! written. Content ids are numbered in sorted string-id order per kind across the packs, so a
//! definition added or removed renumbers the rest; a save keeps numbers, and its world folder's
//! manifest keeps this table to read them by name on other packs (7h3). Rows the files keep by
//! position are named here too: each map's terrains, and each object kind's `once` interactions.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::registry::Interner;
use omnis_data::{Data, FlagScope, Verb};
use serde::{Deserialize, Serialize};

/// What an interaction is, for an object's `used` set to find it again: its verb, its label and
/// the state it needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// The verb.
    pub verb: Verb,
    /// Its label, when the kind gives it one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The state it needs, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// The string ids behind a world folder's numbers, each kind's by number.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameTable {
    /// Maps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub maps: Vec<String>,
    /// Flags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
    /// Regions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<String>,
    /// Items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
    /// Spells.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spells: Vec<String>,
    /// Monsters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monsters: Vec<String>,
    /// Conditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
    /// Races.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub races: Vec<String>,
    /// Classes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    /// Backgrounds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backgrounds: Vec<String>,
    /// Services.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<String>,
    /// The flags a pack declares the party's, which the checkpoint holds (M8.5 7c).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub party_flags: Vec<String>,
    /// Each map's terrains by row, by name: what the automap's `terrain` numbers stand for.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub terrains: BTreeMap<String, Vec<String>>,
    /// Each object kind's `once` interactions, by their row in the kind: what an object's
    /// `used` numbers stand for.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub interactions: BTreeMap<String, BTreeMap<u8, Signature>>,
}

fn names<I: Copy + From<u32> + Into<u32>>(interner: &Interner<I>) -> Vec<String> {
    interner.names().map(String::from).collect()
}

impl NameTable {
    /// The table of the loaded packs.
    #[must_use]
    pub fn of(data: &Data) -> NameTable {
        let registry = &data.registry;
        let name_of = |id| registry.maps.name(id).map(String::from);
        NameTable {
            maps: names(&registry.maps),
            flags: names(&registry.flags),
            regions: names(&registry.regions),
            items: names(&registry.items),
            spells: names(&registry.spells),
            monsters: names(&registry.monsters),
            conditions: names(&registry.conditions),
            races: names(&registry.races),
            classes: names(&registry.classes),
            backgrounds: names(&registry.backgrounds),
            services: names(&registry.services),
            party_flags: data
                .flags
                .iter()
                .filter(|(_, def)| def.scope == FlagScope::Party)
                .filter_map(|(id, _)| registry.flags.name(*id).map(String::from))
                .collect(),
            terrains: data
                .maps
                .iter()
                .filter_map(|(id, map)| {
                    let terrains = map.def.terrains.iter().map(|t| t.name.clone()).collect();
                    Some((name_of(*id)?, terrains))
                })
                .collect(),
            interactions: data
                .objects
                .iter()
                .filter_map(|(id, kind)| {
                    let once: BTreeMap<u8, Signature> = kind
                        .interactions
                        .iter()
                        .enumerate()
                        .filter(|(_, i)| i.once)
                        .filter_map(|(row, i)| {
                            let signature = Signature {
                                verb: i.verb,
                                label: i.label.clone(),
                                state: i.when_state.clone(),
                            };
                            Some((u8::try_from(row).ok()?, signature))
                        })
                        .collect();
                    let name = registry.objects.name(*id)?;
                    (!once.is_empty()).then(|| (String::from(name), once))
                })
                .collect(),
        }
    }

    /// How many names it holds: every kind's, the party's flags, the terrains and the
    /// interactions.
    #[must_use]
    pub fn len(&self) -> usize {
        let kinds = [
            &self.maps,
            &self.flags,
            &self.regions,
            &self.items,
            &self.spells,
            &self.monsters,
            &self.conditions,
            &self.races,
            &self.classes,
            &self.backgrounds,
            &self.services,
            &self.party_flags,
        ];
        kinds.iter().map(|k| k.len()).sum::<usize>()
            + self.terrains.values().map(Vec::len).sum::<usize>()
            + self.interactions.values().map(BTreeMap::len).sum::<usize>()
    }

    /// Whether it holds nothing: a manifest written before 7h2.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
