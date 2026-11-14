//! What each line of the fight's arena is, and its ink in a palette (owner, 2026-11-13;
//! `tasks/plans/fight-on-the-palette.md`): the party in the party's ink and the monsters in the
//! hostile one, the marks and bars in the interface's. A figure that is down or dead, whoever
//! acts and a target are told by a shape as well (a slash, a cross, brackets, a frame), so none
//! is told by colour alone. Bevy-free.

use crate::combat_menu::Pick;
use crate::contrast::shown;
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

/// The brightest a side's ink glows in the fight: just over the screen's range, so a line has
/// a halo and does not flood (owner, 2026-11-14: the fight's colours were too saturated).
pub const GLOW_CAP: f32 = 1.25;
/// How far a side's ink is mixed toward white. No further than this: the default palette's
/// two sides are 0.107 apart to a green-blind eye here and under `contrast::APART` at 0.2
/// (`tests/contrast.rs`).
pub const PALE: f32 = 0.15;
/// A fallen figure's share of its side's ink.
pub const DOWN: f32 = 0.25;
/// A dead figure's share of its side's ink.
pub const DEAD: f32 = 0.125;
/// A bar's frame, as a share of the interface's ink.
pub const BAR: f32 = 0.9;
/// A bar's fill, as a factor on the interface's ink: above 1.0, so it glows.
pub const HEALTH: f32 = 1.4;
/// A target's frame, as a factor on the interface's ink.
pub const TARGET: f32 = 1.8;
/// The hovered target's frame: white, brighter than any ink, the same in every palette.
pub const HOVER: [f32; 3] = [2.2, 2.2, 2.2];

/// An ink mixed [`PALE`] of the way toward white at its own brightness: paler, its hue kept.
#[must_use]
pub fn pale(ink: [f32; 3]) -> [f32; 3] {
    let top = ink[0].max(ink[1]).max(ink[2]);
    ink.map(|c| c + (top - c) * PALE)
}

/// A side's ink as the fight draws it: its glow brought down to [`GLOW_CAP`], then paler. The
/// palette's own ink, which the 3D view draws, is not changed.
#[must_use]
pub fn calm(ink: [f32; 3]) -> [f32; 3] {
    let top = ink[0].max(ink[1]).max(ink[2]);
    let k = if top > GLOW_CAP { GLOW_CAP / top } else { 1.0 };
    pale(ink.map(|c| c * k))
}

/// The ink of `tone` in `palette`, linear; a value above 1.0 glows. The sides are calmed; the
/// marks are the interface's ink without its glow, times their own factor.
#[must_use]
pub fn ink(tone: Tone, palette: &Palette) -> [f32; 3] {
    let side = |side: Side, k: f32| calm(palette.ink(side.role())).map(|c| c * k);
    let mark = |k: f32| shown(palette.ink(ColourRole::Interface)).map(|c| c * k);
    match tone {
        Tone::Figure(s) | Tone::Acting(s) => side(s, 1.0),
        Tone::Down(s) => side(s, DOWN),
        Tone::Dead(s) => side(s, DEAD),
        Tone::Bar => mark(BAR),
        Tone::Health => mark(HEALTH),
        Tone::Target => mark(TARGET),
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

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
    }

    /// How far an ink is from grey: 0.0 for a grey, 1.0 for a pure hue.
    fn saturation([r, g, b]: [f32; 3]) -> f32 {
        let (top, low) = (r.max(g).max(b), r.min(g).min(b));
        (top - low) / top
    }

    /// The owner's try (2026-11-14): the fight's colours were too saturated. A side's ink keeps
    /// its hue, glows no further than the cap and is paler than the palette's.
    #[test]
    fn a_side_s_ink_is_calmed_its_glow_capped_and_its_hue_paler() {
        // Under the cap: the brightness is kept, each channel moves part-way to the brightest.
        assert!(close(calm([0.8, 0.4, 0.2]), [0.8, 0.46, 0.29]));
        // Over the cap: scaled to it first.
        assert!(close(calm([0.2, 0.4, 2.0]), [0.29375, 0.4, GLOW_CAP]));
        for ink in [
            Palette::default().ink(ColourRole::Party),
            Palette::default().ink(ColourRole::Hostile),
            [0.2, 0.4, 2.0],
        ] {
            let calmed = calm(ink);
            let peak = calmed[0].max(calmed[1]).max(calmed[2]);
            assert!(peak <= GLOW_CAP + 1e-6, "{ink:?} glows to {peak}");
            let paler = saturation(ink) * (1.0 - PALE);
            assert!(
                (saturation(calmed) - paler).abs() < 1e-4,
                "{ink:?} is paler"
            );
            // The order of its channels, which is its hue, is kept.
            for (a, b) in [(0, 1), (1, 2), (0, 2)] {
                assert_eq!(ink[a] > ink[b], calmed[a] > calmed[b], "{ink:?}");
            }
        }
        // A grey stays a grey, and a dark ink is not lifted.
        assert!(close(calm([0.5, 0.5, 0.5]), [0.5, 0.5, 0.5]));
        assert!(close(calm([3.0, 3.0, 3.0]), [GLOW_CAP; 3]));
    }

    #[test]
    fn each_line_takes_its_role_s_ink_from_the_palette() {
        let p = palette();
        let (party, hostile) = (Side::Party, Side::Monsters);
        let (ours, theirs) = (calm([0.8, 0.4, 0.2]), calm([0.2, 0.4, 2.0]));
        assert_ne!(ours, theirs);
        assert_eq!(ink(Tone::Figure(party), &p), ours);
        assert_eq!(ink(Tone::Figure(hostile), &p), theirs);
        assert_eq!(ink(Tone::Acting(party), &p), ours);
        assert_eq!(ink(Tone::Acting(hostile), &p), theirs);
        assert_eq!(ink(Tone::Down(party), &p), ours.map(|c| c * DOWN));
        assert_eq!(ink(Tone::Down(hostile), &p), theirs.map(|c| c * DOWN));
        assert_eq!(ink(Tone::Dead(party), &p), ours.map(|c| c * DEAD));
        assert_eq!(ink(Tone::Dead(hostile), &p), theirs.map(|c| c * DEAD));
        assert_eq!(ink(Tone::Bar, &p), [1.0, 0.5, 0.1].map(|c| c * BAR));
        assert_eq!(ink(Tone::Health, &p), [1.0, 0.5, 0.1].map(|c| c * HEALTH));
        assert_eq!(ink(Tone::Target, &p), [1.0, 0.5, 0.1].map(|c| c * TARGET));
    }

    /// The marks are the interface's ink without its glow, so a biosphere's glowing interface
    /// ink floods no more than the default's; and they stay in order of brightness.
    #[test]
    fn the_marks_take_the_interface_s_ink_without_its_glow() {
        let mut p = palette();
        p.inks[ColourRole::Interface.index()] = [4.0, 2.0, 0.4];
        assert_eq!(ink(Tone::Target, &p), [1.0, 0.5, 0.1].map(|c| c * TARGET));
        assert_eq!(ink(Tone::Health, &p), [1.0, 0.5, 0.1].map(|c| c * HEALTH));
        const { assert!(BAR < HEALTH && HEALTH < TARGET && TARGET < HOVER[0]) };
        const { assert!(TARGET <= 2.0, "no mark floods") };
    }

    #[test]
    fn the_hovered_frame_is_white_in_every_palette() {
        assert_eq!(ink(Tone::Hover, &palette()), HOVER);
        assert_eq!(ink(Tone::Hover, &Palette::default()), HOVER);
        assert_eq!(HOVER[0], HOVER[1]);
        assert_eq!(HOVER[1], HOVER[2]);
    }

    #[test]
    fn the_default_palette_draws_the_two_sides_in_their_own_inks() {
        let p = Palette::default();
        assert_eq!(
            ink(Tone::Figure(Side::Party), &p),
            calm(p.ink(ColourRole::Party))
        );
        assert_eq!(
            ink(Tone::Figure(Side::Monsters), &p),
            calm(p.ink(ColourRole::Hostile))
        );
        assert_ne!(
            ink(Tone::Figure(Side::Party), &p),
            ink(Tone::Figure(Side::Monsters), &p)
        );
    }
}
