//! Pack definitions by the string ids the engine's views carry (`ItemView::id`,
//! `EffectView::spell`, `MemberView::class`): the app resolves labels and details through
//! `Data`, which is part of the contract, never through the world. Bevy-free.

use omnis_sim::Place;
use omnis_sim::omnis_core::Position;
use omnis_sim::omnis_data::{
    Background, Class, Condition, Data, Item, MapData, Monster, Race, Spell, Tileset,
};

/// The item a view names.
#[must_use]
pub fn item<'d>(data: &'d Data, id: &str) -> Option<&'d Item> {
    data.items.get(&data.registry.items.get(id)?)
}

/// The spell a view names.
#[must_use]
pub fn spell<'d>(data: &'d Data, id: &str) -> Option<&'d Spell> {
    data.spells.get(&data.registry.spells.get(id)?)
}

/// The class a view names.
#[must_use]
pub fn class<'d>(data: &'d Data, id: &str) -> Option<&'d Class> {
    data.classes.get(&data.registry.classes.get(id)?)
}

/// The race a view names.
#[must_use]
pub fn race<'d>(data: &'d Data, id: &str) -> Option<&'d Race> {
    data.races.get(&data.registry.races.get(id)?)
}

/// The background a view names.
#[must_use]
pub fn background<'d>(data: &'d Data, id: &str) -> Option<&'d Background> {
    data.backgrounds.get(&data.registry.backgrounds.get(id)?)
}

/// The condition a view names.
#[must_use]
pub fn condition<'d>(data: &'d Data, id: &str) -> Option<&'d Condition> {
    data.conditions.get(&data.registry.conditions.get(id)?)
}

/// The monster a view names.
#[must_use]
pub fn monster<'d>(data: &'d Data, id: &str) -> Option<&'d Monster> {
    data.monsters.get(&data.registry.monsters.get(id)?)
}

/// The map a view names.
#[must_use]
pub fn map<'d>(data: &'d Data, id: &str) -> Option<&'d MapData> {
    data.maps.get(&data.registry.maps.get(id)?)
}

/// The tileset a view names.
#[must_use]
pub fn tileset<'d>(data: &'d Data, id: &str) -> Option<&'d Tileset> {
    data.tilesets.get(&data.registry.tilesets.get(id)?)
}

/// A place as the position the app's draw lists take, when a pack names its map.
#[must_use]
pub fn position(data: &Data, place: &Place) -> Option<Position> {
    Some(Position {
        map: data.registry.maps.get(&place.map)?,
        x: place.x,
        y: place.y,
        facing: place.facing,
    })
}

/// A localized label for a text key, owned; `?` stays `?`.
#[must_use]
pub fn label(data: &Data, key: &str) -> String {
    data.label("en", key).to_owned()
}
