//! Looking up and down (M8.5 step 4b, presentation-ARCHITECTURE §8.6, VA7).
//!
//! The pitch is the head's, not the body's: it lives beside the pose, never in it, so it never
//! reaches the binder or the simulation. It has no limits: past straight up or down the view
//! turns over and looks back over the head, while the body, its facing and its walking stay
//! where they were. A step can return it to level, as the player chooses. Bevy-free.

use core::f32::consts::{FRAC_PI_2, PI, TAU};

/// How long the return to level takes to close most of the way, in seconds.
pub const LEVEL_EASE: f32 = 0.15;

/// The pitch held to `(-π, π]`. Positive looks up.
#[must_use]
pub fn wrap_pitch(pitch: f32) -> f32 {
    let p = (pitch + PI).rem_euclid(TAU) - PI;
    if p <= -PI { p + TAU } else { p }
}

/// Whether the view has turned over: looking back over the head, the picture upside down.
#[must_use]
pub fn upside_down(pitch: f32) -> bool {
    wrap_pitch(pitch).abs() > FRAC_PI_2
}

/// The sign for a sideways turn of the mouse: reversed while upside down, so the picture still
/// turns the way the mouse moves.
#[must_use]
pub fn yaw_sign(pitch: f32) -> f32 {
    if upside_down(pitch) { -1.0 } else { 1.0 }
}

/// The pitch moved toward level over `dt` seconds by the shorter way round; at once under
/// reduced motion (`snap`), and the last small part at once so it reaches level.
#[must_use]
pub fn level(pitch: f32, dt: f32, snap: bool) -> f32 {
    let p = wrap_pitch(pitch);
    if snap {
        return 0.0;
    }
    let next = p * (-dt / LEVEL_EASE).exp();
    if next.abs() < 1e-3 { 0.0 } else { next }
}

/// When a step returns the view to level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum LevelOnStep {
    /// Unless the mouse is looking (the pointer locked).
    #[default]
    Unlocked,
    /// On every step.
    Always,
    /// Never: the view stays where it was put.
    Never,
}

impl LevelOnStep {
    /// Every choice, in the setting's order.
    pub const ALL: [LevelOnStep; 3] = [
        LevelOnStep::Unlocked,
        LevelOnStep::Always,
        LevelOnStep::Never,
    ];

    /// Its name on the screen.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            LevelOnStep::Unlocked => "when not mouse looking",
            LevelOnStep::Always => "always",
            LevelOnStep::Never => "never",
        }
    }

    /// Its word in the file.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            LevelOnStep::Unlocked => "unlocked",
            LevelOnStep::Always => "always",
            LevelOnStep::Never => "never",
        }
    }

    /// Whether a step levels the view, with the mouse `looking` or not.
    #[must_use]
    pub const fn levels(self, looking: bool) -> bool {
        match self {
            LevelOnStep::Unlocked => !looking,
            LevelOnStep::Always => true,
            LevelOnStep::Never => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_pitch_wraps_round_over_the_top_and_back() {
        assert!(close(wrap_pitch(0.3), 0.3));
        assert!(close(wrap_pitch(PI), PI));
        assert!(close(wrap_pitch(-PI), PI), "one end of the range");
        assert!(close(wrap_pitch(PI + 0.5), -PI + 0.5), "over the top");
        assert!(close(wrap_pitch(TAU + 0.2), 0.2), "a full loop");
        assert!(close(wrap_pitch(-TAU - 0.2), -0.2), "a full loop down");
    }

    #[test]
    fn past_straight_up_or_down_the_view_is_upside_down_and_the_mouse_turns_reversed() {
        assert!(!upside_down(FRAC_PI_2 - 0.01));
        assert!(upside_down(FRAC_PI_2 + 0.01));
        assert!(upside_down(-FRAC_PI_2 - 0.01));
        assert!(upside_down(PI));
        assert!(!upside_down(TAU));
        assert_eq!(yaw_sign(0.4), 1.0);
        assert_eq!(yaw_sign(2.0), -1.0);
        assert_eq!(yaw_sign(-2.0), -1.0);
    }

    #[test]
    fn levelling_takes_the_shorter_way_and_settles() {
        let up = level(0.5, 1.0 / 60.0, false);
        assert!(up > 0.0 && up < 0.5, "part of the way down: {up}");
        let down = level(-0.5, 1.0 / 60.0, false);
        assert!(down < 0.0 && down > -0.5, "part of the way up: {down}");
        // Turned on past the top (looking back and down): the short way is on round, under the
        // feet, not back over the top.
        let past = level(PI + 0.3, 1.0 / 60.0, false);
        assert!(past < 0.0 && past > -PI + 0.3, "on round: {past}");
        let mut p = 1.2;
        for _ in 0..120 {
            p = level(p, 1.0 / 60.0, false);
        }
        assert_eq!(p, 0.0, "settles exactly");
        assert_eq!(level(2.5, 1.0 / 60.0, true), 0.0, "reduced motion: at once");
    }

    #[test]
    fn each_choice_levels_when_it_says() {
        assert!(LevelOnStep::Unlocked.levels(false));
        assert!(!LevelOnStep::Unlocked.levels(true));
        assert!(LevelOnStep::Always.levels(false) && LevelOnStep::Always.levels(true));
        assert!(!LevelOnStep::Never.levels(false) && !LevelOnStep::Never.levels(true));
        assert_eq!(LevelOnStep::default(), LevelOnStep::Unlocked);
    }
}
