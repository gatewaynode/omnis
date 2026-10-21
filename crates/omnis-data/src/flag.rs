//! `data/flags`: whose a flag is (PRD D28 and §7.7, owner 2026-10-17). A party flag (a quest
//! taken, an oath, a debt) travels with the party in its character save; a world flag (a bridge
//! burned, a lord dead) belongs to the world and folds with it. A flag no file declares is the
//! world's, so declaring one is needed only to make it the party's.

use crate::error::DataError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Whose a flag is.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum FlagScope {
    /// The party's: it goes where the party goes.
    Party,
    /// The world's: it stays where it was set.
    #[default]
    World,
}

/// One flag, one file under `data/flags`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagDef {
    /// Schema version.
    pub schema: u32,
    /// `pack:flag:name`.
    pub id: String,
    /// Whose it is; the world's when left out.
    #[serde(default)]
    pub scope: FlagScope,
}

impl FlagDef {
    /// Nothing to check beyond the id and the schema, which every file gets.
    pub fn validate(&self, _file: &Path, _errors: &mut Vec<DataError>) {}
}
