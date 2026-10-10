//! Words for what the engine names (presentation-ARCHITECTURE §4.3). Views and events name
//! definitions by string id and text by key (protocol 2); this module is the one place that
//! turns them into words from the packs' text. No rule is read here: only labels.
//!
//! An id no pack defines reads as the id itself, so a stale or modded name shows what it is
//! rather than vanishing; a text key no pack defines reads as the key (`Data::label`), except a
//! `MessageKey`, which falls back to this client's own wording so a key never reaches the player.

use omnis_sim::omnis_core::money::{Coins, gp_floor};
use omnis_sim::omnis_data::Data;
use omnis_sim::{MessageKey, SaveRule};

/// The display text of a text key a view or event carries (`ItemView.name`, `SpellView.name`).
#[must_use]
pub fn text<'d>(data: &'d Data, key: &'d str) -> &'d str {
    data.label("en", key)
}

/// An item's name from its string id.
#[must_use]
pub fn item<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .items
        .get(id)
        .and_then(|n| data.items.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A spell's name from its string id.
#[must_use]
pub fn spell<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .spells
        .get(id)
        .and_then(|n| data.spells.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A monster's name from its string id.
#[must_use]
pub fn monster<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .monsters
        .get(id)
        .and_then(|n| data.monsters.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A condition's name from its string id.
#[must_use]
pub fn condition<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .conditions
        .get(id)
        .and_then(|n| data.conditions.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A town service's name from its string id.
#[must_use]
pub fn service<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .services
        .get(id)
        .and_then(|n| data.services.get(&n))
        .map_or(id, |d| text(data, &d.name))
}

/// A map's name from its string id.
#[must_use]
pub fn map<'d>(data: &'d Data, id: &'d str) -> &'d str {
    data.registry
        .maps
        .get(id)
        .and_then(|n| data.maps.get(&n))
        .map_or(id, |d| data.text("en", d.name))
}

/// What a `Message` event says. The packs' text wins; without it, this client's wording.
#[must_use]
pub fn message(data: &Data, key: MessageKey) -> &str {
    let found = data.label("en", key.text_key());
    if found != key.text_key() {
        return found;
    }
    match key {
        MessageKey::NothingHere => "There is nothing to do here",
    }
}

/// A purse or price in copper by coin, largest first: `15 gp 3 sp 7 cp` (as the other client
/// shows town prices and purses).
#[must_use]
pub fn coins(cp: u32) -> String {
    Coins::of(cp).to_string()
}

/// Copper as whole gold rounded down, where a line names gold: a bribe, the victory's loot.
#[must_use]
pub fn gold(cp: u32) -> String {
    format!("{} gold", gp_floor(cp))
}

/// The save rule as the new-game and pause screens name it.
#[must_use]
pub const fn save_rule(rule: SaveRule) -> &'static str {
    match rule {
        SaveRule::Anywhere => "Anywhere",
        SaveRule::Relief => "Inns, items, and spells",
        SaveRule::InnOnly => "Inns only",
    }
}
