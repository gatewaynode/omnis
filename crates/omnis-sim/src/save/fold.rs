//! The fold (PRD D28, M8.5 7e): the world as it stands becomes the new bases, and the journal's
//! interactions move into the archive, the world's history, kept for later tools (the full log
//! reads the party's log instead, M8.5 7f). Older saves keep naming
//! the bases they started from, so loading one restores its moment. Pure: the new bases and the
//! archive segment reach disk with the next save (`save_files`), each named by its text's hash.

use super::split;
use crate::event::Event;
use crate::journal::Journal;
use crate::world::World;
use omnis_data::Data;
use serde::{Deserialize, Serialize};

/// The rules value that sets how many interactions the journal holds before a safe point folds
/// it, when a pack sets it.
pub const FOLD_RULE: &str = "fold_interactions";

/// How many interactions the journal holds before a safe point folds it, when no pack says
/// (owner, 2026-10-18: about 1.5 MB of journal, some 1.5 million commands of play).
pub const FOLD_INTERACTIONS: usize = 5000;

/// One archive segment: the interactions of a journal folded away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Archived {
    /// Folded in this session; the next save writes it, named by the hash of its text.
    Held(Journal),
    /// Known by name from a loaded save: `archive/<name>.ron` in the world folder.
    Named(u64),
}

/// Fold `world`: its bases become what it holds now, its journal goes to the archive and starts
/// again empty. It cannot fail, so a fold at the end of a command never half-happens.
pub fn fold(world: &mut World, data: &Data) -> Event {
    let journal = core::mem::take(&mut world.journal);
    let interactions = u32::try_from(journal.len()).unwrap_or(u32::MAX);
    if !journal.is_empty() {
        world.archive.push(Archived::Held(journal));
    }
    world.origin = Some(split(world, data).into());
    Event::Folded { interactions }
}

/// How many interactions fold at a safe point: the pack's value, else [`FOLD_INTERACTIONS`].
#[must_use]
pub fn fold_at(data: &Data) -> usize {
    data.rules
        .value(FOLD_RULE)
        .map_or(FOLD_INTERACTIONS, |v| usize::try_from(v).unwrap_or(0))
}

/// Whether `journal` is due a fold at a safe point.
#[must_use]
pub fn due(journal: &Journal, data: &Data) -> bool {
    journal.len() >= fold_at(data)
}
