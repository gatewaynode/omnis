//! Omnis simulation: `World` + `Command` -> `Vec<Event>`. Pure Rust, no Bevy, no I/O, no wall
//! clock, no threads. Every client (renderer, editor, MCP, CLI, tests) drives this library.
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
