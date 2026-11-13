//! The words that explain the editor (owner, 2026-11-13): the groups a form's fields stand
//! under. Bevy-free.

use super::form::FormKind;

/// The heading of the group that starts at the field named `label` of a `kind` form, when one
/// starts there.
#[must_use]
pub fn section(kind: &FormKind, label: &str) -> Option<&'static str> {
    Some(match (kind, label) {
        (FormKind::Terrain(_) | FormKind::Encounter | FormKind::Object | FormKind::Lore, "id")
        | (FormKind::Terrain(_), "glyph") => "IDENTITY",
        (FormKind::Terrain(_), "floor") => "TILES",
        (FormKind::Terrain(_), "passable") => "MOVEMENT",
        (FormKind::Terrain(_), "role") => "LOOK",
        (FormKind::Terrain(_), "red") => "MAP COLOUR",
        (FormKind::Encounter, "disposition") => "BEHAVIOUR",
        (FormKind::Encounter, "stands apart") => "STAND",
        (FormKind::Object, "pos x") => "PLACE",
        (FormKind::Object, "state") => "STATE",
        (FormKind::Object | FormKind::Lore, "lore" | "text") => "LORE",
        _ => return None,
    })
}
