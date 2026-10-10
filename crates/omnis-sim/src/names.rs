//! How the protocol names what the `World` numbers (protocol 2, ARCHITECTURE.md §4.9): a
//! definition by its string id, a place by its map's id. The `World` keeps registry numbers;
//! events and views convert here, so the save and the fingerprint never see a name.

use alloc::string::String;
use omnis_core::{Facing, Position};
use omnis_data::Data;
use omnis_data::registry::Interner;
use omnis_rules::{ActionRef, Named};
use serde::{Deserialize, Serialize};

/// A tile on a map and the way the party faces there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Place {
    /// `pack:map:name`, or `#n` for a map no pack names.
    pub map: String,
    /// Column, growing east.
    pub x: u16,
    /// Row, growing south.
    pub y: u16,
    /// The direction the party looks.
    pub facing: Facing,
}

impl Place {
    /// A position with its map named.
    #[must_use]
    pub fn of(position: Position, data: &Data) -> Self {
        Place {
            map: id_of(&data.registry.maps, position.map),
            x: position.x,
            y: position.y,
            facing: position.facing,
        }
    }
}

/// The string id of a registry number, or `#n` for a number no pack names (a world built by
/// hand, or a pack since removed).
pub(crate) fn id_of<I: Copy + From<u32> + Into<u32>>(registry: &Interner<I>, id: I) -> String {
    registry
        .name(id)
        .map_or_else(|| alloc::format!("#{}", id.into()), String::from)
}

/// A runbook action with its spell or item named.
pub(crate) fn action_named(action: &ActionRef, data: &Data) -> ActionRef<Named> {
    match action {
        ActionRef::Attack => ActionRef::Attack,
        ActionRef::Spell(spell) => ActionRef::Spell(id_of(&data.registry.spells, *spell)),
        ActionRef::Item(item) => ActionRef::Item(id_of(&data.registry.items, *item)),
        ActionRef::Feature(key) => ActionRef::Feature(key.clone()),
    }
}
