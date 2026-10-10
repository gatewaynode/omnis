//! The integration tests of the app, one binary: macOS holds every newly linked executable
//! for a fixed wait on its first launch, so each test binary costs that wait once per build.
//! Kept apart in `Cargo.toml`: `socket`.

mod camp;
mod cast;
mod combat;
mod common;
mod confirm;
mod debug;
mod feathers;
mod feathers_panel;
mod inventory;
mod look;
mod pointer;
mod service;
mod sheet;
mod smoke;
mod tactics;
mod tool_bar;
