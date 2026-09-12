//! `data/rules`: formula slots, plain values, and tables (ARCHITECTURE.md §5). Every file
//! adds to one rule set; a later file's slot or value with the same name replaces the earlier.

use crate::error::DataError;
use crate::limits::{MAX_COLLECTION, string_fits};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One formula: its declared inputs and its Rhai expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotDef {
    /// The only names the expression may use.
    pub inputs: Vec<String>,
    /// The expression.
    pub expr: String,
}

/// One rules file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesFile {
    /// Schema version.
    pub schema: u32,
    /// `pack:rules:name`.
    pub id: String,
    /// Formula slots by name.
    #[serde(default)]
    pub slots: BTreeMap<String, SlotDef>,
    /// Plain integers by name.
    #[serde(default)]
    pub values: BTreeMap<String, i64>,
    /// Integer tables by name.
    #[serde(default)]
    pub tables: BTreeMap<String, Vec<i64>>,
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl RulesFile {
    /// Self-contained checks; every problem is pushed. Compilation happens at resolution,
    /// once every file is in.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.slots.len() > MAX_COLLECTION
            || self.values.len() > MAX_COLLECTION
            || self.tables.len() > MAX_COLLECTION
            || self.tables.values().any(|t| t.len() > MAX_COLLECTION)
        {
            errors.push(DataError::new(file, "too many entries"));
        }
        for (name, slot) in &self.slots {
            if name.is_empty() || !string_fits(name) {
                errors.push(DataError::new(file, "a slot name is empty or too long"));
            }
            for input in &slot.inputs {
                if !is_identifier(input) {
                    errors.push(DataError::new(
                        file,
                        format!("slot '{name}': input '{input}' is not an identifier"),
                    ));
                }
            }
            if slot.expr.trim().is_empty() || !string_fits(&slot.expr) {
                errors.push(DataError::new(
                    file,
                    format!("slot '{name}': expression is empty or too long"),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_lowercase_snake_case() {
        assert!(is_identifier("level"));
        assert!(is_identifier("cast_mod2"));
        assert!(is_identifier("_x"));
        assert!(!is_identifier(""));
        assert!(!is_identifier("1x"));
        assert!(!is_identifier("Level"));
        assert!(!is_identifier("a-b"));
    }
}
