//! Omnis core: typed IDs, integer and fixed-point math, dice, deterministic RNG, and time
//! primitives shared by every simulation crate.
//!
//! Simulation crate rules (CLAUDE.md, ARCHITECTURE.md §11): `no_std`, integers only, ordered
//! collections only, no Bevy, no wall clock, no threads, no I/O.
#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
