//! Errors: a formula that does not compile is bad data; a formula that fails at runtime is a
//! bug or bad data too. Neither is a rule refusal.

use core::fmt;

/// A formula was rejected at compile time. Positions are 1-based; `None` when the problem has no
/// single position (a missing slot, an unknown input name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    /// Line within the formula text.
    pub line: Option<usize>,
    /// Column within that line.
    pub column: Option<usize>,
    /// What was wrong.
    pub message: String,
}

impl CompileError {
    pub(crate) fn at(
        line: Option<usize>,
        column: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        CompileError {
            line,
            column,
            message: message.into(),
        }
    }

    pub(crate) fn plain(message: impl Into<String>) -> Self {
        CompileError::at(None, None, message)
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => write!(f, "{line}:{column}: {}", self.message),
            (Some(line), None) => write!(f, "{line}: {}", self.message),
            _ => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for CompileError {}

/// A formula failed at evaluation: unknown slot, a missing input, a runtime error such as
/// division by zero or the operation limit, or a result of the wrong type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleError {
    /// The slot being evaluated.
    pub slot: String,
    /// The Rhai error or our own description.
    pub message: String,
}

impl RuleError {
    pub(crate) fn new(slot: &str, message: impl Into<String>) -> Self {
        RuleError {
            slot: slot.to_owned(),
            message: message.into(),
        }
    }
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rule '{}': {}", self.slot, self.message)
    }
}

impl std::error::Error for RuleError {}
