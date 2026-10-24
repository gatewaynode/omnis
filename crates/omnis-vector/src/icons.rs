//! The icons the client draws by name (owner, 2026-10-24: the Lucide set through `omnis-icon`),
//! read once and kept. Bevy-free.

use omnis_icon::Icon;
use std::sync::OnceLock;

/// The activities' placeholder: a grinning face, until each kind of activity has an icon of its
/// own (the set has no `smile`).
pub const ACTIVITY: &str = "face-grinning";

/// The icon `name` of the set, read once; an icon the set lacks is empty, drawing nothing.
#[must_use]
pub fn named(name: &str, cell: &'static OnceLock<Icon>) -> &'static Icon {
    cell.get_or_init(|| {
        omnis_icon::get(name).unwrap_or_else(|| Icon {
            name: name.to_owned(),
            strokes: Vec::new(),
        })
    })
}

/// The activities' placeholder icon.
#[must_use]
pub fn activity() -> &'static Icon {
    static CELL: OnceLock<Icon> = OnceLock::new();
    named(ACTIVITY, &CELL)
}
