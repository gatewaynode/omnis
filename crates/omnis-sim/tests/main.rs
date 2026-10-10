//! The integration tests of the simulation, one binary: macOS holds every newly linked executable
//! for a fixed wait on its first launch, so each test binary costs that wait once per build.
//! Kept apart in `Cargo.toml`: `measure`.

mod api_views;
mod casting;
mod combat;
mod common;
mod dev;
mod effects;
mod encounter;
mod items;
mod monster_cast;
mod movement;
mod ops;
mod party;
mod reactions;
mod rest;
mod rest_view;
mod save_and_replay;
mod sense;
mod service_view;
mod services;
mod time;
mod town;
mod training;
mod turn_budget;
mod views;
