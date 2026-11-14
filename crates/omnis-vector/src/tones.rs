//! What each line of the fight's arena is, and its ink in a palette (owner, 2026-11-13;
//! `tasks/plans/fight-on-the-palette.md`): the party in the party's ink and the monsters in the
//! hostile one, the marks and bars in the interface's. A figure that is down or dead, whoever
//! acts and a target are told by a shape as well (a slash, a cross, brackets, a frame), so none
//! is told by colour alone. Bevy-free.

use crate::combat_menu::Pick;
use crate::style::Palette;
use omnis_sim::omnis_data::ColourRole;

/// Whose a figure is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The party's member.
    Party,
    /// A monster.
    Monsters,
}

impl Side {
    /// The side of what a click picks.
    #[must_use]
    pub fn of(pick: Pick) -> Side {
        match pick {
            Pick::Member(_) => Side::Party,
            Pick::Stack(_) => Side::Monsters,
        }
    }

    /// The role its lines are drawn in.
    #[must_use]
    pub fn role(self) -> ColourRole {
        match self {
            Side::Party => ColourRole::Party,
            Side::Monsters => ColourRole::Hostile,
        }
    }
}

/// What a line is, for its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// A figure on its feet.
    Figure(Side),
    /// A figure that is down, and its mark.
    Down(Side),
    /// A dead figure, and its mark.
    Dead(Side),
    /// A health bar's frame.
    Bar,
    /// A health bar's fill.
    Health,
    /// The frame round whoever is acting.
    Acting(Side),
    /// The frame round a valid target.
    Target,
    /// The frame round the target under the pointer.
    Hover,
}

/// A fallen figure's share of its side's ink.
pub const DOWN: f32 = 0.25;
/// A dead figure's share of its side's ink.
pub const DEAD: f32 = 0.125;
/// A bar's frame, as a share of the interface's ink.
pub const BAR: f32 = 0.9;
/// A bar's fill, as a factor on the interface's ink: above 1.0, so it blooms.
pub const HEALTH: f32 = 2.2;
/// A target's frame, as a factor on the interface's ink.
pub const TARGET: f32 = 3.0;
/// The hovered target's frame: white, brighter than any ink, the same in every palette.
pub const HOVER: [f32; 3] = [3.0, 3.0, 3.0];

/// The ink of `tone` in `palette`, linear; a value above 1.0 blooms.
#[must_use]
pub fn ink(tone: Tone, palette: &Palette) -> [f32; 3] {
    let of = |role: ColourRole, k: f32| palette.ink(role).map(|c| c * k);
    match tone {
        Tone::Figure(side) | Tone::Acting(side) => of(side.role(), 1.0),
        Tone::Down(side) => of(side.role(), DOWN),
        Tone::Dead(side) => of(side.role(), DEAD),
        Tone::Bar => of(ColourRole::Interface, BAR),
        Tone::Health => of(ColourRole::Interface, HEALTH),
        Tone::Target => of(ColourRole::Interface, TARGET),
        Tone::Hover => HOVER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A palette with an ink of its own for each role the fight draws.
    fn palette() -> Palette {
        let mut palette = Palette::default();
        palette.inks[ColourRole::Party.index()] = [0.8, 0.4, 0.2];
        palette.inks[ColourRole::Hostile.index()] = [0.2, 0.4, 2.0];
        palette.inks[ColourRole::Interface.index()] = [1.0, 0.5, 0.1];
        palette
    }

    #[test]
    fn each_line_takes_its_role_s_ink_from_the_palette() {
        let p = palette();
        let (party, hostile) = (Side::Party, Side::Monsters);
        assert_eq!(ink(Tone::Figure(party), &p), [0.8, 0.4, 0.2]);
        assert_eq!(ink(Tone::Figure(hostile), &p), [0.2, 0.4, 2.0]);
        assert_eq!(ink(Tone::Acting(party), &p), [0.8, 0.4, 0.2]);
        assert_eq!(ink(Tone::Acting(hostile), &p), [0.2, 0.4, 2.0]);
        assert_eq!(ink(Tone::Down(party), &p), [0.2, 0.1, 0.05]);
        assert_eq!(ink(Tone::Down(hostile), &p), [0.05, 0.1, 0.5]);
        assert_eq!(ink(Tone::Dead(party), &p), [0.1, 0.05, 0.025]);
        assert_eq!(ink(Tone::Dead(hostile), &p), [0.025, 0.05, 0.25]);
        assert_eq!(ink(Tone::Bar, &p), [0.9, 0.45, 0.1 * 0.9]);
        assert_eq!(ink(Tone::Health, &p), [2.2, 1.1, 0.1 * 2.2]);
        assert_eq!(ink(Tone::Target, &p), [3.0, 1.5, 0.1 * 3.0]);
    }

    #[test]
    fn the_hovered_frame_is_white_in_every_palette() {
        assert_eq!(ink(Tone::Hover, &palette()), HOVER);
        assert_eq!(ink(Tone::Hover, &Palette::default()), HOVER);
        assert_eq!(HOVER, [3.0; 3]);
    }

    #[test]
    fn the_default_palette_draws_the_two_sides_in_their_own_inks() {
        let p = Palette::default();
        assert_eq!(ink(Tone::Figure(Side::Party), &p), p.ink(ColourRole::Party));
        assert_eq!(
            ink(Tone::Figure(Side::Monsters), &p),
            p.ink(ColourRole::Hostile)
        );
        assert_ne!(p.ink(ColourRole::Party), p.ink(ColourRole::Hostile));
    }
}
