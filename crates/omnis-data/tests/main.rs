//! The integration tests of the pack loader, one binary: macOS holds every newly linked executable
//! for a fixed wait on its first launch, so each test binary costs that wait once per build.

mod bad_packs;
mod beings;
mod common;
mod edit_authoring;
mod edit_document;
mod edit_grid;
mod edit_session;
mod files;
mod flags;
mod footprint;
mod load_base_pack;
mod load_test_pack;
mod migrations;
mod resolve;
mod round_trip;
mod seal;
