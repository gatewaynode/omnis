//! The core error type. Errors are bugs or bad input; rule refusals are `Rejection`s in `omnis-sim`.

use core::fmt;

/// Failures that core primitives can report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An integer or fixed-point operation left the representable range.
    Overflow,
    /// A dice expression had zero sides or zero dice.
    InvalidDice,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Overflow => f.write_str("arithmetic overflow"),
            Error::InvalidDice => f.write_str("dice must have at least one die and one side"),
        }
    }
}

impl core::error::Error for Error {}
