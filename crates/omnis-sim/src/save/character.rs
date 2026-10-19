//! The character save (PRD D28, M8.5 7d): the party as it stands, the journal of interactions
//! since the world's last base with the changes each made, the names of the bases, and the
//! fingerprint the rebuilt world must have.

use super::{Checkpoint, Manifest, Segment};
use crate::journal::Journal;
use crate::log::Logged;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::fnv1a64;
use omnis_data::DataError;
use serde::{Deserialize, Serialize};

/// The bases a save starts from, by content hash; a base left out is base 0, the packs as
/// written, which has no file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseNames {
    /// The world base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world: Option<u64>,
    /// Each region's base, by the region's string id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub regions: BTreeMap<String, u64>,
}

/// A save: one file, small, naming the world it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterSave {
    /// `SAVE_SCHEMA`.
    pub schema: u32,
    /// The world folder's name: [`world_id`] of its manifest.
    pub world: String,
    /// The bases the journal starts from.
    #[serde(default)]
    pub bases: BaseNames,
    /// The party as it stood at the save.
    pub checkpoint: Checkpoint,
    /// The interactions since the bases, each with the changes it made to the world.
    #[serde(default)]
    pub journal: Journal,
    /// The world's fingerprint at the save, which the rebuilt world must have.
    pub fingerprint: u64,
    /// The archive segments before the bases, oldest first (M8.5 7e): the history the folds put
    /// away, each in a world folder's `archive/`, this save's own unless it names another
    /// (M8.5 7h2: a world whose packs changed folds into a new folder). A schema 8 save names
    /// each by its hash alone.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "segments"
    )]
    pub archive: Vec<Segment>,
    /// The party's log, oldest first (M8.5 7f): the newest `LOG_CAPACITY` events, stamped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<Logged>,
}

/// A world folder's name: sixteen hex digits of its manifest's hash, so a world is found by its
/// seed and packs and the name can never climb out of the folder.
pub fn world_id(manifest: &Manifest) -> Result<String, DataError> {
    let text = omnis_data::ron_io::to_string(manifest)?;
    Ok(format!("{:016x}", fnv1a64(text.as_bytes())))
}

/// Whether `id` has the form a world folder's name has.
#[must_use]
pub fn is_world_id(id: &str) -> bool {
    id.len() == 16
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A save's archive: each segment by its fields, or by its hash alone (schema 8), in this folder.
fn segments<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Segment>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Here(u64),
        Full(Segment),
    }
    let written = Vec::<Written>::deserialize(d)?;
    Ok(written
        .into_iter()
        .map(|w| match w {
            Written::Here(name) => Segment { world: None, name },
            Written::Full(segment) => segment,
        })
        .collect())
}
