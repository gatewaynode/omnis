//! Resolution of the character-facing content: races, classes, backgrounds, items, conditions,
//! spells, monsters, and rules. Cross-references are checked by id string while the file paths
//! are still known, then everything is interned and the rules are compiled.

use crate::character::{Background, Class, Race};
use crate::condition::Condition;
use crate::error::DataError;
use crate::item::Item;
use crate::loader::Data;
use crate::monster::Monster;
use crate::registry::Interner;
use crate::rules::RulesFile;
use crate::spell::Spell;
use crate::tileset::Tileset;
use omnis_expr::Rules;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A data type the loader gathers one file per id: its id and its own checks.
pub(crate) trait Content {
    fn id(&self) -> &str;
    fn validate(&self, file: &Path, errors: &mut Vec<DataError>);
}

macro_rules! content {
    ($($t:ty),* $(,)?) => {$(
        impl Content for $t {
            fn id(&self) -> &str {
                &self.id
            }
            fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
                <$t>::validate(self, file, errors);
            }
        }
    )*};
}
content!(
    Tileset, Race, Class, Background, Item, Spell, Monster, RulesFile
);

impl Content for Condition {
    fn id(&self) -> &str {
        &self.id
    }
    fn validate(&self, _file: &Path, _errors: &mut Vec<DataError>) {}
}

/// Files of one type keyed by content id, so later packs override earlier ones.
pub(crate) type Files<T> = BTreeMap<String, (PathBuf, T)>;

/// Everything gathered from `data/` besides tiles and maps.
#[derive(Default)]
pub(crate) struct RawContent {
    pub races: Files<Race>,
    pub classes: Files<Class>,
    pub backgrounds: Files<Background>,
    pub items: Files<Item>,
    pub conditions: Files<Condition>,
    pub spells: Files<Spell>,
    pub monsters: Files<Monster>,
    pub rules: Files<RulesFile>,
}

/// Check references and text keys, compile the rules, intern everything.
pub(crate) fn resolve_content(raw: RawContent, data: &mut Data, errors: &mut Vec<DataError>) {
    check_references(&raw, errors);
    check_text_keys(&raw, data, errors);
    data.rules = build_rules(&raw.rules, errors);
    data.races = intern(raw.races, &mut data.registry.races);
    data.classes = intern(raw.classes, &mut data.registry.classes);
    data.backgrounds = intern(raw.backgrounds, &mut data.registry.backgrounds);
    data.items = intern(raw.items, &mut data.registry.items);
    data.conditions = intern(raw.conditions, &mut data.registry.conditions);
    data.spells = intern(raw.spells, &mut data.registry.spells);
    data.monsters = intern(raw.monsters, &mut data.registry.monsters);
}

fn intern<I: Copy + Ord + From<u32> + Into<u32>, T>(
    files: Files<T>,
    interner: &mut Interner<I>,
) -> BTreeMap<I, T> {
    files
        .into_iter()
        .map(|(id, (_, value))| (interner.intern(&id), value))
        .collect()
}

/// Every item, spell, and class another file names must be defined by some loaded pack.
fn check_references(raw: &RawContent, errors: &mut Vec<DataError>) {
    let mut require = |file: &Path, what: &str, id: &str, exists: bool| {
        if !exists {
            errors.push(DataError::new(
                file,
                format!("{what} '{id}' is not defined by any loaded pack"),
            ));
        }
    };
    for (file, class) in raw.classes.values() {
        for id in &class.weapon_ids {
            require(file, "weapon", id, raw.items.contains_key(id));
        }
        for (id, _) in &class.starting_equipment {
            require(file, "equipment", id, raw.items.contains_key(id));
        }
        if let Some(casting) = &class.casting {
            for id in &casting.list {
                require(file, "spell", id, raw.spells.contains_key(id));
            }
        }
    }
    for (file, background) in raw.backgrounds.values() {
        for (id, _) in &background.equipment {
            require(file, "equipment", id, raw.items.contains_key(id));
        }
    }
    for (file, spell) in raw.spells.values() {
        for id in &spell.classes {
            require(file, "class", id, raw.classes.contains_key(id));
        }
        for (id, _) in &spell.components {
            require(file, "component", id, raw.items.contains_key(id));
        }
    }
}

/// Every display key must exist in at least one language.
fn check_text_keys(raw: &RawContent, data: &Data, errors: &mut Vec<DataError>) {
    let mut keys: Vec<(&Path, &str)> = Vec::new();
    for (file, race) in raw.races.values() {
        keys.push((file, &race.name));
        keys.extend(
            race.features
                .iter()
                .map(|f| (file.as_path(), f.name.as_str())),
        );
    }
    for (file, class) in raw.classes.values() {
        keys.push((file, &class.name));
        keys.extend(
            class
                .features
                .iter()
                .map(|f| (file.as_path(), f.name.as_str())),
        );
    }
    for (file, background) in raw.backgrounds.values() {
        keys.push((file, &background.name));
        keys.push((file, &background.feature.name));
    }
    for (file, item) in raw.items.values() {
        keys.push((file, &item.name));
    }
    for (file, condition) in raw.conditions.values() {
        keys.push((file, &condition.name));
        keys.push((file, &condition.description));
    }
    for (file, spell) in raw.spells.values() {
        keys.push((file, &spell.name));
        keys.push((file, &spell.description));
    }
    for (file, monster) in raw.monsters.values() {
        keys.push((file, &monster.name));
        keys.extend(
            monster
                .attacks
                .iter()
                .map(|a| (file.as_path(), a.name.as_str())),
        );
    }
    for (file, key) in keys {
        if data.registry.text.get(key).is_none() {
            errors.push(DataError::new(
                file,
                format!("text key '{key}' is not defined in any language"),
            ));
        }
    }
}

/// One rule set from every rules file, in id order; a compile error names the file and slot.
fn build_rules(files: &Files<RulesFile>, errors: &mut Vec<DataError>) -> Rules {
    let mut rules = Rules::new();
    for (file, def) in files.values() {
        for (name, value) in &def.values {
            rules.insert_value(name, *value);
        }
        for (name, table) in &def.tables {
            rules.insert_table(name, table.clone());
        }
        for (name, slot) in &def.slots {
            if let Err(error) = rules.add_slot(name, slot.inputs.clone(), &slot.expr) {
                errors.push(DataError::new(file, format!("slot '{name}': {error}")));
            }
        }
    }
    rules
}
