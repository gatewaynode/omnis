//! Load errors. The loader collects every error it finds instead of stopping at the first
//! (ARCHITECTURE.md §6.2); a report names the file and, where known, the line.

use std::fmt;
use std::path::PathBuf;

/// One problem found while loading a pack.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}{}: {message}", file.display(), line.map(|l| format!(":{l}")).unwrap_or_default())]
pub struct DataError {
    /// The file the problem was found in, relative to the pack root when possible.
    pub file: PathBuf,
    /// The line, when the problem is a parse error.
    pub line: Option<usize>,
    /// What went wrong.
    pub message: String,
}

impl DataError {
    /// An error in `file` with no line.
    pub fn new(file: impl Into<PathBuf>, message: impl Into<String>) -> DataError {
        DataError {
            file: file.into(),
            line: None,
            message: message.into(),
        }
    }

    /// An error at a line of `file`.
    pub fn at(file: impl Into<PathBuf>, line: usize, message: impl Into<String>) -> DataError {
        DataError {
            file: file.into(),
            line: Some(line),
            message: message.into(),
        }
    }
}

/// Every error found while loading. A pack with a non-empty report is refused whole.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// The errors, in the order found.
    pub errors: Vec<DataError>,
}

impl LoadReport {
    /// Record an error.
    pub fn push(&mut self, error: DataError) {
        self.errors.push(error);
    }

    /// Whether anything went wrong.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// The messages only, for tests that assert an expected error list.
    #[must_use]
    pub fn messages(&self) -> Vec<String> {
        self.errors.iter().map(|e| e.message.clone()).collect()
    }
}

impl fmt::Display for LoadReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} error(s):", self.errors.len())?;
        for error in &self.errors {
            writeln!(f, "  {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for LoadReport {}
