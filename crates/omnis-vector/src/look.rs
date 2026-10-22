//! Looking up and down (M8.5 step 4b, presentation-ARCHITECTURE §8.6, VA7).
//!
//! The pitch is the head's, not the body's: it lives beside the pose, never in it, so it never
//! reaches the binder or the simulation. It stops just short of straight up and straight down
//! (`PITCH_LIMIT`, the first-person convention; owner, 2026-10-22, after the free head of
//! 2026-10-12 left the view turned over and walking the wrong way, `tasks/BUGS.md` B8), so the
//! picture is always upright and ahead of the body. A step can return it to level, as the player
//! chooses. Bevy-free.

use core::f32::consts::FRAC_PI_2;

/// How long the return to level takes to close most of the way, in seconds.
pub const LEVEL_EASE: f32 = 0.15;

/// The farthest the view looks up or down: one degree short of straight up, so the camera's up
/// never lies along its line of sight and the view never turns over.
pub const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.017_453_292;

/// The pitch held to `[-PITCH_LIMIT, PITCH_LIMIT]`. Positive looks up.
#[must_use]
pub fn clamp_pitch(pitch: f32) -> f32 {
    pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT)
}

/// The pitch moved toward level over `dt` seconds; at once under reduced motion (`snap`), and
/// the last small part at once so it reaches level.
#[must_use]
pub fn level(pitch: f32, dt: f32, snap: bool) -> f32 {
    let p = clamp_pitch(pitch);
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
    use core::f32::consts::{PI, TAU};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_pitch_stops_short_of_straight_up_and_down_and_never_turns_over() {
        let limit = clamp_pitch(10.0);
        assert!(limit < FRAC_PI_2 && limit > 1.55, "a degree short: {limit}");
        assert!(close(clamp_pitch(0.3), 0.3));
        assert!(close(clamp_pitch(-0.3), -0.3));
        assert!(
            close(clamp_pitch(FRAC_PI_2), PITCH_LIMIT),
            "straight up: held short"
        );
        assert!(
            close(clamp_pitch(-FRAC_PI_2), -PITCH_LIMIT),
            "straight down"
        );
        assert!(
            close(clamp_pitch(PI + 0.5), PITCH_LIMIT),
            "over the top: never"
        );
        assert!(close(clamp_pitch(-TAU), -PITCH_LIMIT), "a loop down: never");
    }

    #[test]
    fn levelling_settles_from_either_side() {
        let up = level(0.5, 1.0 / 60.0, false);
        assert!(up > 0.0 && up < 0.5, "part of the way down: {up}");
        let down = level(-0.5, 1.0 / 60.0, false);
        assert!(down < 0.0 && down > -0.5, "part of the way up: {down}");
        let from_limit = level(PI, 1.0 / 60.0, false);
        assert!(
            from_limit > 0.0 && from_limit < PITCH_LIMIT,
            "an out-of-range pitch levels from the limit: {from_limit}"
        );
        let mut p = 1.2;
        for _ in 0..120 {
            p = level(p, 1.0 / 60.0, false);
        }
        assert_eq!(p, 0.0, "settles exactly");
        assert_eq!(level(1.5, 1.0 / 60.0, true), 0.0, "reduced motion: at once");
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
