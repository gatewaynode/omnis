//! The catalogue: every definition the loaded packs hold, by kind and by id, behind one table.
//! What the `data.*` ops read (ARCHITECTURE §4.9). A kind the packs gain later is one variant of
//! [`Definition`] and one row of [`KINDS`]; its references and its text keys are found by the same
//! walk as every other kind's ([`links`]), with no code of their own.
//!
//! Rules files are not here: [`Data`] keeps the compiled set, which `rules.list` serves.

use crate::character::{Background, Class, Race};
use crate::condition::Condition;
use crate::flag::FlagDef;
use crate::item::Item;
use crate::loader::Data;
use crate::loot::LootTable;
use crate::manifest::is_content_id;
use crate::map::MapDef;
use crate::monster::Monster;
use crate::object::ObjectKind;
use crate::region::RegionDef;
use crate::service::ServiceDef;
use crate::spell::Spell;
use crate::strings::strings;
use crate::tileset::Tileset;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One definition as its pack wrote it. On the wire it is two fields beside whatever holds it:
/// `kind`, the word its id carries, and `def`, the file's own shape (versioned by
/// [`crate::SCHEMA`], as the files are).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "def", rename_all = "snake_case")]
pub enum Definition {
    /// A tileset.
    Tileset(Tileset),
    /// A map, its terrains, levels and placements included.
    Map(Box<MapDef>),
    /// A race.
    Race(Race),
    /// A class.
    Class(Class),
    /// A background.
    Background(Background),
    /// An item.
    Item(Item),
    /// A condition.
    Condition(Condition),
    /// A spell.
    Spell(Spell),
    /// A monster.
    Monster(Monster),
    /// A town service.
    Service(ServiceDef),
    /// A region, written again from the loaded one: its ids as strings.
    Region(RegionDef),
    /// An object kind.
    Object(Box<ObjectKind>),
    /// A loot table.
    Loot(LootTable),
    /// A declared flag.
    Flag(FlagDef),
}

impl Definition {
    /// The text key of its display name; `None` for a kind that has none.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        Some(match self {
            Definition::Map(d) => &d.name,
            Definition::Race(d) => &d.name,
            Definition::Class(d) => &d.name,
            Definition::Background(d) => &d.name,
            Definition::Item(d) => &d.name,
            Definition::Condition(d) => &d.name,
            Definition::Spell(d) => &d.name,
            Definition::Monster(d) => &d.name,
            Definition::Service(d) => &d.name,
            Definition::Region(d) => &d.name,
            Definition::Object(d) => &d.name,
            Definition::Tileset(_) | Definition::Loot(_) | Definition::Flag(_) => return None,
        })
    }
}

/// One kind of definition: the word its ids carry and how the loaded data answers for it.
pub struct Kind {
    /// The kind as an id writes it (`pack:item:name`).
    pub word: &'static str,
    ids: for<'a> fn(&'a Data) -> Vec<&'a str>,
    get: fn(&Data, &str) -> Option<Definition>,
}

impl Kind {
    /// The id of every definition of this kind, sorted.
    #[must_use]
    pub fn ids<'a>(&self, data: &'a Data) -> Vec<&'a str> {
        let mut ids = (self.ids)(data);
        ids.sort_unstable();
        ids
    }

    /// The definition `id` names, when it is of this kind and loaded.
    #[must_use]
    pub fn get(&self, data: &Data, id: &str) -> Option<Definition> {
        (self.get)(data, id)
    }
}

/// A kind whose definitions [`Data`] holds as written, under the registry table of the same name.
macro_rules! kind {
    ($word:literal, $field:ident, $variant:ident) => {
        kind!($word, $field, |d| Definition::$variant(d.clone().into()))
    };
    ($word:literal, $field:ident, $made:expr) => {
        Kind {
            word: $word,
            ids: |data| {
                data.$field
                    .keys()
                    .filter_map(|k| data.registry.$field.name(*k))
                    .collect()
            },
            get: |data, id| {
                let key = data.registry.$field.get(id)?;
                data.$field.get(&key).map($made)
            },
        }
    };
}

/// Every kind of definition, in the order a listing gives them.
pub const KINDS: [Kind; 14] = [
    kind!("background", backgrounds, Background),
    kind!("class", classes, Class),
    kind!("condition", conditions, Condition),
    kind!("flag", flags, Flag),
    kind!("item", items, Item),
    kind!("loot", loot, Loot),
    kind!("map", maps, |m| Definition::Map(Box::new(m.def.clone()))),
    kind!("monster", monsters, Monster),
    kind!("object", objects, Object),
    kind!("race", races, Race),
    Kind {
        word: "region",
        ids: |data| {
            data.regions
                .keys()
                .filter_map(|k| data.registry.regions.name(*k))
                .collect()
        },
        get: region,
    },
    kind!("service", services, Service),
    kind!("spell", spells, Spell),
    kind!("tileset", tilesets, Tileset),
];

/// A region as its file wrote it: the loaded one holds registry numbers, which never leave the
/// engine.
fn region(data: &Data, id: &str) -> Option<Definition> {
    let registry = &data.registry;
    let loaded = data.regions.get(&registry.regions.get(id)?)?;
    let named = |name: Option<&str>| name.unwrap_or_default().to_owned();
    Some(Definition::Region(RegionDef {
        schema: crate::SCHEMA,
        id: id.to_owned(),
        name: named(registry.text.name(loaded.name)),
        kind: loaded.kind,
        company: loaded.company,
        stability: loaded.stability,
        rule: loaded.rule.clone(),
        couplings: loaded
            .couplings
            .iter()
            .map(|r| named(registry.regions.name(*r)))
            .collect(),
        maps: loaded
            .maps
            .iter()
            .map(|m| named(registry.maps.name(*m)))
            .collect(),
    }))
}

/// The kind an id names (its second segment), whether or not this engine knows the kind.
#[must_use]
pub fn kind_word(id: &str) -> Option<&str> {
    is_content_id(id).then(|| id.split(':').nth(1)).flatten()
}

/// The row of [`KINDS`] for a kind's word.
#[must_use]
pub fn kind(word: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.word == word)
}

/// The definition `id` names, of whatever kind.
#[must_use]
pub fn get(data: &Data, id: &str) -> Option<Definition> {
    kind(kind_word(id)?)?.get(data, id)
}

/// What a definition names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Links {
    /// Every loaded definition it names, and every flag, by id. Never itself.
    pub refers: BTreeSet<String>,
    /// Every text key it carries that some language file has.
    pub keys: BTreeSet<String>,
}

/// What the definition under `id` names: each string in it that is a loaded id is a reference,
/// each that is a loaded text key is a key. Read from the definition as a whole, so a field added
/// to a data type is covered the day it is added.
#[must_use]
pub fn links(data: &Data, id: &str, definition: &Definition) -> Links {
    let mut links = Links::default();
    for found in strings(definition) {
        if found == id || !is_content_id(&found) {
            continue;
        }
        if data.registry.text.get(&found).is_some() {
            links.keys.insert(found);
        } else if get(data, &found).is_some() || data.registry.flags.get(&found).is_some() {
            links.refers.insert(found);
        }
    }
    links
}

/// The id of every definition that names `id`, in [`KINDS`] order then by id. It reads every
/// definition once: a question for an author's tool, not for a frame.
#[must_use]
pub fn referrers<'a>(data: &'a Data, id: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    for kind in &KINDS {
        for other in kind.ids(data) {
            let names_it = kind
                .get(data, other)
                .is_some_and(|d| other != id && strings(&d).iter().any(|s| s == id));
            if names_it {
                out.push(other);
            }
        }
    }
    out
}

/// The pack that supplied the definition loaded under `id`: the last that wrote it.
#[must_use]
pub fn supplier<'a>(data: &'a Data, id: &str) -> Option<&'a str> {
    data.sources.get(id)?.last().map(String::as_str)
}

/// The packs whose definition under `id` a later pack replaced, in load order.
#[must_use]
pub fn replaced<'a>(data: &'a Data, id: &str) -> &'a [String] {
    data.sources
        .get(id)
        .and_then(|packs| packs.split_last())
        .map_or(&[], |(_, earlier)| earlier)
}
