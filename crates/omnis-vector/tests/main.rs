//! The integration tests of the vector client, one binary: macOS holds every newly linked
//! executable for a fixed wait on its first launch, so each test binary costs that wait once per
//! build (`scripts/check-test-modules.sh`).

mod actions;
mod agreement;
mod arena;
mod binding;
mod combat;
mod combat_menu;
mod common;
mod fight;
mod minimap;
mod rolllog;
mod shell;
