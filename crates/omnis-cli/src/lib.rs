//! Headless driver for the Omnis simulation. The binary in `main.rs` and the MCP bridge both use
//! this library so headless behaviour has one implementation.
//!
//! M1 ships `tileset::bake`; the headless game driver arrives with M2.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bake;
