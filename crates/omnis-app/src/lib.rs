//! The Omnis Bevy application as a library, so tests can build the app without a window and
//! the binary in `main.rs` stays a few lines.
//!
//! Bevy-free modules (`layout`, `plan`) hold everything that can be unit-tested; the plugins
//! hold only ECS wiring and drawing. `SimPlugin` and `InputPlugin` run headless under
//! `MinimalPlugins`; `assets`, `pixel`, `viewport`, and `hud` need the render stack.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod assets;
#[cfg(feature = "devtools")]
pub mod dev;
pub mod hud;
pub mod input;
pub mod layout;
pub mod pixel;
pub mod plan;
pub mod sim;
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
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            packs: vec![PathBuf::from("packs/test")],
            seed: 1,
            save_path: PathBuf::from(".omnis/quick.ron"),
        }
    }
}
