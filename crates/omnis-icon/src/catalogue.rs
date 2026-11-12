//! The whole set by name, compiled in by `build.rs` from `assets/svg-icons/icons/*.svg`, and the
//! set's licence.

use crate::Icon;

include!(concat!(env!("OUT_DIR"), "/catalogue.rs"));

/// Every icon's name, sorted.
pub fn names() -> impl Iterator<Item = &'static str> {
    ICONS.iter().map(|(name, _)| *name)
}

/// The SVG text of the icon `name`, if the set has it.
#[must_use]
pub fn text(name: &str) -> Option<&'static str> {
    ICONS
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|i| ICONS[i].1)
}

/// The icon `name` as strokes, if the set has it (its text is read on each call: a few
/// microseconds; a client keeps what it draws). None for a name the set lacks, or an icon the
/// reader cannot take, which the crate's tests rule out for the set as compiled.
#[must_use]
pub fn get(name: &str) -> Option<Icon> {
    Icon::parse(name, text(name)?).ok()
}

/// How many icons the set has.
#[must_use]
pub fn count() -> usize {
    ICONS.len()
}
