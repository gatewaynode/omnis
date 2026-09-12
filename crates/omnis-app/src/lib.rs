//! The Omnis Bevy application as a library, so tests can build the app without a window and
//! the binary in `main.rs` stays a few lines.
//!
//! Bevy-free modules (`layout`, `plan`) hold everything that can be unit-tested; the plugins
//! hold only ECS wiring and drawing.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod layout;
pub mod plan;
