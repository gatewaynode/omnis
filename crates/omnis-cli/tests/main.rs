//! The integration tests of the command-line tool, one binary: macOS holds every newly linked executable
//! for a fixed wait on its first launch, so each test binary costs that wait once per build.
//! Kept apart in `Cargo.toml`: `headless`.

mod bake;
mod cli;
