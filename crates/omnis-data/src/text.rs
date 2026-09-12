//! `text/<lang>/*.ron`: localized strings by key. The simulation only ever emits keys.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One text file: keys to strings with `{arg}` placeholders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextFile {
    /// Schema version of this file.
    pub schema: u32,
    /// Key (`pack:text:name`) to string.
    pub entries: BTreeMap<String, String>,
}
