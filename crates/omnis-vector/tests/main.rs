//! The integration tests of the vector client, one binary: macOS holds every newly linked
//! executable for a fixed wait on its first launch, so each test binary costs that wait once per
//! build (`scripts/check-test-modules.sh`).

mod actions;
mod agreement;
mod arena;
mod beings;
mod binding;
mod camp;
mod cast;
mod column;
mod combat;
mod combat_menu;
mod common;
mod create_party;
mod debug;
mod fight;
mod fight_screen;
mod fill;
mod full_log;
mod game_io;
mod geometry;
mod ground;
mod inventory;
mod labels;
mod levels;
mod load_report;
mod log;
mod look;
mod minimap;
mod notice;
mod object_lines;
mod overlay;
mod piles;
mod rolllog;
mod roster;
mod scene;
mod screens;
mod service;
mod settings;
mod shapes;
mod sheet;
mod shell;
mod socket;
mod tactics;
mod verbs;
