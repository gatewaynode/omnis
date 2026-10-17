//! The journal (PRD D28): every command the world accepted since its base, each stamped with the
//! party's age after it. Loading replays it from the base, so it is rebuilt by replay and is not
//! saved yet (M8.5 7d writes it into the character save).

use crate::command::Command;
use crate::world::World;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// One accepted command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The party's age in minutes once the command was done.
    pub minute: i64,
    /// The command.
    pub command: Command,
}

/// The commands since the world's base, in the order they were accepted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal(Vec<Entry>);

impl Journal {
    /// Append an accepted command.
    pub fn push(&mut self, entry: Entry) {
        self.0.push(entry);
    }

    /// The entries, oldest first.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.0
    }

    /// How many entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether nothing has been accepted since the base.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// How many commands a load would replay.
#[must_use]
pub fn journal_len(world: &World) -> usize {
    world.journal.len()
}
