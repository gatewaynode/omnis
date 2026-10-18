//! The character save (PRD D28, M8.5 7d): the party's checkpoint at the world's last base, the
//! journal of commands since, the names of the bases, and the fingerprint the replay must reach.

use super::{Checkpoint, Manifest};
use crate::command::Command;
use crate::journal::Journal;
use crate::world::LoadError;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::fnv1a64;
use omnis_data::DataError;
use serde::{Deserialize, Serialize};

/// The most commands a save's journal may expand to: about twenty seconds of replay. A run's
/// count is a number in the file, so without a cap a few bytes could ask for years of work.
pub const MAX_JOURNAL: usize = 1_000_000;

const fn one() -> u32 {
    1
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_one(n: &u32) -> bool {
    *n == 1
}

/// One command, repeated: most of play is steps and turns in a row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    /// The command.
    pub command: Command,
    /// How many times in a row; left out when once.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub times: u32,
}

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
    /// The party at the base.
    pub checkpoint: Checkpoint,
    /// The commands since, run-length.
    pub journal: Vec<Run>,
    /// The world's fingerprint at the save, which the replay must reach.
    pub fingerprint: u64,
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

/// The journal as runs of the same command.
#[must_use]
pub fn runs(journal: &Journal) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for entry in journal.entries() {
        match runs.last_mut() {
            Some(run) if run.command == entry.command && run.times < u32::MAX => run.times += 1,
            _ => runs.push(Run {
                command: entry.command.clone(),
                times: 1,
            }),
        }
    }
    runs
}

/// The commands the runs stand for, refusing an empty run and more than [`MAX_JOURNAL`].
pub fn expand(runs: &[Run]) -> Result<Vec<Command>, LoadError> {
    let mut total: usize = 0;
    for run in runs {
        if run.times == 0 {
            return Err(LoadError::BadSave("a journal run of no commands".into()));
        }
        total = total.saturating_add(run.times as usize);
        if total > MAX_JOURNAL {
            return Err(LoadError::BadSave(format!(
                "a journal of more than {MAX_JOURNAL} commands"
            )));
        }
    }
    let mut commands = Vec::with_capacity(total);
    for run in runs {
        for _ in 0..run.times {
            commands.push(run.command.clone());
        }
    }
    Ok(commands)
}
