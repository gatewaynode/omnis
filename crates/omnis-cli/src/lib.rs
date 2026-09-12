//! Headless driver for the Omnis simulation (ARCHITECTURE.md §10). The binary in `main.rs` and
//! the MCP bridge both use this library so headless behaviour has one implementation.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod args;
pub mod bake;
pub mod headless;
pub mod schema;

pub use headless::{Headless, HeadlessError};

// The MCP bridge depends on this crate alone (ARCHITECTURE.md §3); the simulation's protocol
// types reach it through this re-export.
pub use omnis_sim;
