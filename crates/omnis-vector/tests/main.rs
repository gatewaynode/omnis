//! The integration tests of the vector client, one binary: macOS holds every newly linked
//! executable for a fixed wait on its first launch, so each test binary costs that wait once per
//! build (`scripts/check-test-modules.sh`).

mod actions;
mod agreement;
mod arena;
mod binding;
mod camp;
mod cast;
mod column;
mod combat;
mod combat_menu;
mod common;
mod create_party;
mod fight;
mod fight_screen;
mod inventory;
mod labels;
mod message;
mod minimap;
mod overlay;
mod rolllog;
mod roster;
mod screens;
mod service;
mod sheet;
mod shell;
mod socket;
mod tactics;
