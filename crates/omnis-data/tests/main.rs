//! The integration tests of the pack loader, one binary: macOS holds every newly linked executable
//! for a fixed wait on its first launch, so each test binary costs that wait once per build.

mod bad_packs;
mod common;
mod load_base_pack;
mod load_test_pack;
mod round_trip;
