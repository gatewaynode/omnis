//! Rhai formula host for the Omnis rules (ARCHITECTURE.md §5).
//!
//! A [`Rules`] value owns one sandboxed engine and a set of named *slots*, each a single
//! expression over declared integer or boolean inputs. Formulas come from pack data, are
//! compiled at load, checked against their declared inputs, and can be swapped at runtime
//! (`rules.set`). Dice inside a formula draw from the caller's named RNG stream, so results are
//! deterministic and every roll is traced.
//!
//! This crate is std (Rhai needs it) but does no I/O and never touches a clock.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

mod check;
mod engine;
mod error;
mod rules;

pub use engine::Limits;
pub use error::{CompileError, RuleError};
pub use rules::{Outcome, Rules, Slot, Value};
