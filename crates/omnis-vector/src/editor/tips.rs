//! The words that explain the editor (owner, 2026-11-13): the groups a form's fields stand
//! under, and what each field is for, shown as its tip. Bevy-free.

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

/// What the field named `label` of a `kind` form is for: what it means, its range, and what
/// leaving it empty means. `list` names the list when the field is in one of its rows.
#[must_use]
pub fn field(kind: &FormKind, list: Option<&str>, label: &str) -> Option<&'static str> {
    Some(match (kind, list, label) {
        (FormKind::Terrain(_), None, "glyph") => {
            "The one character that stands for this terrain in the map's layout. Not a space, \
             and not one another terrain of the map uses."
        }
        (FormKind::Terrain(_), None, "name") => {
            "The terrain's name in the editor and in rest events. One word."
        }
        (FormKind::Terrain(_), None, "floor") => "The tileset surface drawn as its ground. Needed.",
        (FormKind::Terrain(_), None, "ceiling") => {
            "The tileset surface overhead. Empty is open sky."
        }
        (FormKind::Terrain(_), None, "block") => {
            "The tileset surface of a solid block that fills the cell: rock, trees, a pillar. \
             Empty is no block."
        }
        (FormKind::Terrain(_), None, "passable") => "Whether the party may step onto it.",
        (FormKind::Terrain(_), None, "opaque") => {
            "Whether it blocks the line of sight: rock and dense trees do, water does not."
        }
        (FormKind::Terrain(_), None, "visibility depth") => {
            "How many cells the party sees while standing on it."
        }
        (FormKind::Terrain(_), None, "step minutes") => {
            "The minutes one step onto it costs. 0 is free."
        }
        (FormKind::Terrain(_), None, "climb") => {
            "The largest height difference a step onto it may cross, in eighths of a cell (a \
             wall is 8). A larger difference is a cliff."
        }
        (FormKind::Terrain(_), None, "join") => {
            "How a cell of it joins its level to the next: stairs up or down, a ladder, a hole. \
             None for plain ground."
        }
        (FormKind::Terrain(_), None, "role") => {
            "What its lines mean in the 3D view, and so their colour from the biosphere's \
             palette. Default is ground underfoot and structure where it is a block."
        }
        (FormKind::Terrain(_), None, "face") => {
            "How its block is drawn in the 3D view: plain is its box alone, hatched adds a \
             cliff's upright lines, which tells trees or rock from a wall."
        }
        (FormKind::Terrain(_), None, "red" | "green" | "blue") => {
            "Its colour on the automap and on this plan, 0 to 255. The 3D view takes its \
             colour from the role."
        }
        _ => return None,
    })
}
