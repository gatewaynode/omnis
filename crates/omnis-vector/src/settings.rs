//! The player's own settings (presentation-ARCHITECTURE §10.5): reduced motion, the floor grid,
//! the quality preset, the font, the interface scale, and when a step levels the view. They are
//! the client's, never the world's: kept in a small RON file under `.omnis/`, never in a save,
//! never seen by the simulation. The file is read leniently, since a person may edit it: unknown
//! keys are ignored, an unknown word or a number out of range falls back or is clamped, and a
//! file that cannot be read at all gives the defaults with a note. Bevy-free.

use crate::look::LevelOnStep;
use omnis_sim::omnis_data::ron_io::{parse, read_text, write_ron};
use omnis_sim::ops::client_path;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The interface scale's range and step, in percent.
pub const SCALE_MIN: u16 = 50;
/// The largest interface scale, in percent.
pub const SCALE_MAX: u16 = 200;
/// The interface scale's step, in percent.
pub const SCALE_STEP: u16 = 10;

/// How much the view draws: the bloom and the line width today; the depth levels join with X17.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Quality {
    /// No bloom, thin lines.
    Low,
    /// Soft bloom.
    Medium,
    /// Today's look.
    #[default]
    High,
}

impl Quality {
    /// Every preset, in the choice's order.
    pub const ALL: [Quality; 3] = [Quality::Low, Quality::Medium, Quality::High];

    /// Its word, on the screen and in the file.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Quality::Low => "low",
            Quality::Medium => "medium",
            Quality::High => "high",
        }
    }

    /// The bloom's intensity; none at all when 0.
    #[must_use]
    pub const fn bloom(self) -> f32 {
        match self {
            Quality::Low => 0.0,
            Quality::Medium => 0.15,
            Quality::High => 0.25,
        }
    }

    /// The lines' width in pixels.
    #[must_use]
    pub const fn line_width(self) -> f32 {
        match self {
            Quality::Low => 1.0,
            Quality::Medium => 1.5,
            Quality::High => 2.0,
        }
    }
}

/// The interface's typeface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Face {
    /// Bevy's own, until the style step chooses one.
    #[default]
    BuiltIn,
    /// Inter (`assets/fonts/inter`, SIL OFL).
    Inter,
    /// Alegreya Sans (`assets/fonts/alegreya-sans`, SIL OFL).
    Alegreya,
}

impl Face {
    /// Every face, in the choice's order.
    pub const ALL: [Face; 3] = [Face::BuiltIn, Face::Inter, Face::Alegreya];

    /// Its name on the screen.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Face::BuiltIn => "built-in",
            Face::Inter => "Inter",
            Face::Alegreya => "Alegreya Sans",
        }
    }

    /// Its word in the file.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Face::BuiltIn => "built-in",
            Face::Inter => "inter",
            Face::Alegreya => "alegreya",
        }
    }
}

/// The settings as applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Prefs {
    /// The 90-degree turns, the drift back onto the cell and the return to level happen at once.
    pub reduced_motion: bool,
    /// The floor and ceiling grids are drawn.
    pub grid: bool,
    /// The quality preset.
    pub quality: Quality,
    /// The typeface.
    pub font: Face,
    /// The interface scale in percent, `SCALE_MIN..=SCALE_MAX` in steps of `SCALE_STEP`.
    pub scale: u16,
    /// When a step returns the view to level.
    pub level_on_step: LevelOnStep,
}

impl Default for Prefs {
    fn default() -> Prefs {
        Prefs {
            reduced_motion: false,
            grid: true,
            quality: Quality::High,
            font: Face::BuiltIn,
            scale: 100,
            level_on_step: LevelOnStep::Unlocked,
        }
    }
}

impl Prefs {
    /// The interface scale as a factor: 1.0 at 100%.
    #[must_use]
    pub fn factor(self) -> f32 {
        f32::from(self.scale) / 100.0
    }
}

/// The window the interface was laid out for, in logical pixels: at a larger scale the HUD, its
/// buttons and the party column need a window this many interface units across and down.
pub const DESIGN: (f32, f32) = (1280.0, 720.0);

/// The largest interface scale, in percent, a `width × height` window holds the design at (at
/// least 100, so the default is never reduced; at most `SCALE_MAX`), on the scale's steps.
#[must_use]
pub fn fits(width: f32, height: f32) -> u16 {
    let most = (width / DESIGN.0).min(height / DESIGN.1) * 100.0;
    #[allow(clippy::cast_possible_truncation)]
    let floored = (most.max(100.0) as i64) / i64::from(SCALE_STEP) * i64::from(SCALE_STEP);
    u16::try_from(floored.min(i64::from(SCALE_MAX))).unwrap_or(100)
}

/// A scale in percent held to the range and rounded to the nearest step (halves up).
#[must_use]
pub fn clamp_scale(percent: i64) -> u16 {
    let held = percent.clamp(i64::from(SCALE_MIN), i64::from(SCALE_MAX));
    let step = i64::from(SCALE_STEP);
    let rounded = (held + step / 2) / step * step;
    u16::try_from(rounded.min(i64::from(SCALE_MAX))).unwrap_or(100)
}

/// The file's shape: plain words and numbers, every key optional, so an edited file still reads.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct File {
    reduced_motion: bool,
    floor_grid: bool,
    quality: String,
    font: String,
    scale: i64,
    level_on_step: String,
}

impl Default for File {
    fn default() -> File {
        File::from(Prefs::default())
    }
}

impl From<Prefs> for File {
    fn from(p: Prefs) -> File {
        File {
            reduced_motion: p.reduced_motion,
            floor_grid: p.grid,
            quality: p.quality.word().to_owned(),
            font: p.font.word().to_owned(),
            scale: i64::from(p.scale),
            level_on_step: p.level_on_step.word().to_owned(),
        }
    }
}

impl From<File> for Prefs {
    fn from(f: File) -> Prefs {
        let d = Prefs::default();
        Prefs {
            reduced_motion: f.reduced_motion,
            grid: f.floor_grid,
            quality: Quality::ALL
                .into_iter()
                .find(|q| q.word() == f.quality)
                .unwrap_or(d.quality),
            font: Face::ALL
                .into_iter()
                .find(|x| x.word() == f.font)
                .unwrap_or(d.font),
            scale: clamp_scale(f.scale),
            level_on_step: LevelOnStep::ALL
                .into_iter()
                .find(|l| l.word() == f.level_on_step)
                .unwrap_or(d.level_on_step),
        }
    }
}

/// The file `path` names under `root`, once `path` passes the client path rule (relative, no
/// `..`, a `.ron` file).
///
/// # Errors
/// The rule's refusal, worded.
pub fn file(root: &Path, path: &str) -> Result<PathBuf, String> {
    client_path(path, &["ron"])
        .map(|p| root.join(p))
        .map_err(|e| e.to_string())
}

/// The settings kept at `path` under `root`, and a note when the file was there but could not
/// be used (the defaults are used then, and the file is left as it is until the player
/// applies new settings). A missing file is the first start: the defaults, no note.
#[must_use]
pub fn read(root: &Path, path: &str) -> (Prefs, Option<String>) {
    let file = match file(root, path) {
        Ok(f) => f,
        Err(e) => return (Prefs::default(), Some(format!("settings {path}: {e}"))),
    };
    if std::fs::symlink_metadata(&file).is_err() {
        return (Prefs::default(), None);
    }
    match read_text(&file, Path::new(path)).and_then(|text| parse::<File>(&text)) {
        Ok(f) => (Prefs::from(f), None),
        Err(e) => (
            Prefs::default(),
            Some(format!("settings {path} not used: {e}")),
        ),
    }
}

/// Keep `prefs` at `path` under `root`, creating its directory.
///
/// # Errors
/// The path is refused or the file cannot be written, worded.
pub fn write(root: &Path, path: &str, prefs: Prefs) -> Result<(), String> {
    let file = file(root, path)?;
    write_ron(&file, &File::from(prefs)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_holds_the_design_up_to_a_step() {
        assert_eq!(fits(1280.0, 720.0), 100);
        assert_eq!(fits(1600.0, 900.0), 120);
        assert_eq!(fits(2560.0, 1440.0), 200);
        assert_eq!(fits(5120.0, 1440.0), 200);
        assert_eq!(fits(800.0, 600.0), 100, "never below the default");
    }

    #[test]
    fn the_scale_is_held_to_its_range_and_rounded_to_its_step() {
        assert_eq!(clamp_scale(7), 50);
        assert_eq!(clamp_scale(333), 200);
        assert_eq!(clamp_scale(105), 110);
        assert_eq!(clamp_scale(104), 100);
        assert_eq!(clamp_scale(-40), 50);
        assert_eq!(clamp_scale(i64::MAX), 200);
    }

    #[test]
    fn an_unknown_word_falls_back_and_the_rest_is_kept() {
        let p = Prefs::from(File {
            reduced_motion: true,
            floor_grid: false,
            quality: "ultra".to_owned(),
            font: "inter".to_owned(),
            scale: 150,
            level_on_step: "never".to_owned(),
        });
        assert_eq!(
            p,
            Prefs {
                reduced_motion: true,
                grid: false,
                quality: Quality::High,
                font: Face::Inter,
                scale: 150,
                level_on_step: LevelOnStep::Never,
            }
        );
    }
}
