//! The Omnis Bevy application as a library, so tests can build the app without a window and
//! the binary in `main.rs` stays a few lines.
//!
//! Bevy-free modules (`layout`, `plan`, `menu`) hold everything that can be unit-tested; the
//! plugins hold only ECS wiring and drawing. `SimPlugin`, `InputPlugin`, and `MenusPlugin` run
//! headless under `MinimalPlugins`; `assets`, `pixel`, `viewport`, `hud`, and `party_panel`
//! need the render stack.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod assets;
#[cfg(feature = "devtools")]
pub mod dev;
pub mod hud;
pub mod input;
pub mod layout;
pub mod menu;
pub mod menus;
pub mod party_panel;
pub mod pixel;
pub mod plan;
pub mod sim;
#[cfg(feature = "devtools")]
pub mod socket;
pub mod viewport;

use std::path::PathBuf;

/// What the binary was started with.
#[derive(Debug, Clone, bevy::prelude::Resource)]
pub struct AppConfig {
    /// Pack roots in load order; the last one provides the asset source.
    pub packs: Vec<PathBuf>,
    /// World seed for a new game.
    pub seed: u64,
    /// Where F5 writes and F9 reads.
    pub save_path: PathBuf,
    /// Skip the menus: create a world from `seed` with default settings and start exploring
    /// (scripts, captures, tests).
    pub autostart: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            packs: vec![PathBuf::from("packs/base"), PathBuf::from("packs/test")],
            seed: 1,
            save_path: PathBuf::from(".omnis/quick.ron"),
            autostart: false,
        }
    }
}

/// A seed from the clock. The simulation never touches entropy; it only receives the number.
#[must_use]
pub fn entropy_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    omnis_sim::omnis_core::splitmix64(nanos as u64 ^ (nanos >> 64) as u64)
}
