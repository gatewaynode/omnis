//! The fight on the palette (owner, 2026-11-13; `tasks/plans/fight-on-the-palette.md`): a fight
//! is drawn in the palette of the map it is fought on, as the 3D view is.

use crate::common;
use common::app::{met_in_the_yard, met_with};
use omnis_sim::omnis_data::ColourRole;
use omnis_vector::shell::combat::palette;
use omnis_vector::shell::session::Session;
use omnis_vector::style::Palette;
use omnis_vector::tones::{Side, Tone, ink};

/// In the dungeon, which names no biosphere, the default; in the yard, its region's ember, so
/// the two sides and the bars are drawn in ember's inks.
#[test]
fn a_fight_is_drawn_in_the_palette_of_its_map() {
    let dungeon = met_with(1);
    assert_eq!(
        palette(dungeon.world().resource::<Session>()),
        Palette::default()
    );
    let yard = met_in_the_yard();
    let session = yard.world().resource::<Session>();
    let ember = Palette::of(session.data.biospheres.get("objects:biosphere:ember"));
    assert_ne!(ember, Palette::default());
    let drawn = palette(session);
    assert_eq!(drawn, ember);
    for (tone, role) in [
        (Tone::Figure(Side::Party), ColourRole::Party),
        (Tone::Figure(Side::Monsters), ColourRole::Hostile),
    ] {
        assert_eq!(ink(tone, &drawn), ember.ink(role));
        assert_ne!(ink(tone, &drawn), ink(tone, &Palette::default()));
    }
}
