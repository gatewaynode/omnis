//! `data/conditions`: the SRD conditions by id, with localized names and descriptions.

use serde::{Deserialize, Serialize};

/// One condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    /// Schema version.
    pub schema: u32,
    /// `pack:condition:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// Text key of the description.
    pub description: String,
}
