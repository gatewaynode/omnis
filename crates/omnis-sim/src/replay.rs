//! A replay is a seed, the packs, and a command stream (ARCHITECTURE.md §4.6). Applying the
//! commands to a fresh world must reproduce the recorded fingerprint.

use crate::apply::apply;
use crate::command::Command;
use crate::world::{NewGameError, World};
use alloc::vec::Vec;
use core::fmt;
use omnis_data::{Data, DataError, PackFingerprint};
use serde::{Deserialize, Serialize};

/// A recorded session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    /// The world seed.
    pub seed: u64,
    /// The packs the session ran on.
    pub packs: Vec<PackFingerprint>,
    /// The commands, in order.
    pub commands: Vec<Command>,
    /// The fingerprint after the last command.
    pub fingerprint: u64,
}

/// Why a replay did not reproduce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// The loaded packs are not the recorded ones.
    PackMismatch,
    /// No entry map.
    NewGame(NewGameError),
    /// A command was refused.
    Rejected {
        /// Which command.
        index: usize,
    },
    /// Fingerprinting failed.
    Fingerprint(DataError),
    /// The final fingerprint differs.
    Diverged {
        /// What the replay recorded.
        expected: u64,
        /// What this run produced.
        actual: u64,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplayError::PackMismatch => f.write_str("loaded packs differ from the replay's"),
            ReplayError::NewGame(e) => write!(f, "{e}"),
            ReplayError::Rejected { index } => write!(f, "command {index} was rejected"),
            ReplayError::Fingerprint(e) => write!(f, "{e}"),
            ReplayError::Diverged { expected, actual } => write!(
                f,
                "fingerprint {actual:016x} differs from recorded {expected:016x}"
            ),
        }
    }
}

impl Replay {
    /// Run `commands` on a fresh world and record the result.
    pub fn record(data: &Data, seed: u64, commands: Vec<Command>) -> Result<Replay, ReplayError> {
        let fingerprint = run(data, seed, &commands)?;
        Ok(Replay {
            seed,
            packs: data.fingerprints.clone(),
            commands,
            fingerprint,
        })
    }

    /// Re-run and compare with the recorded fingerprint.
    pub fn check(&self, data: &Data) -> Result<(), ReplayError> {
        if self.packs != data.fingerprints {
            return Err(ReplayError::PackMismatch);
        }
        let actual = run(data, self.seed, &self.commands)?;
        if actual == self.fingerprint {
            Ok(())
        } else {
            Err(ReplayError::Diverged {
                expected: self.fingerprint,
                actual,
            })
        }
    }
}

/// Apply `commands` to a fresh world and return the final fingerprint.
pub fn run(data: &Data, seed: u64, commands: &[Command]) -> Result<u64, ReplayError> {
    let mut world = World::new(data, seed).map_err(ReplayError::NewGame)?;
    for (index, command) in commands.iter().enumerate() {
        apply(&mut world, data, *command).map_err(|_| ReplayError::Rejected { index })?;
    }
    world.fingerprint().map_err(ReplayError::Fingerprint)
}
