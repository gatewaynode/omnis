//! The fold (PRD D28, M8.5 7e): the world as it stands becomes the new bases, and the journal's
//! interactions move into the archive, the world's history, kept for later tools (the full log
//! reads the party's log instead, M8.5 7f). Older saves keep naming
//! the bases they started from, so loading one restores its moment. Pure: the new bases and the
//! archive segment reach disk with the next save (`save_files`), each named by its text's hash.

use super::{Manifest, split};
use crate::event::Event;
use crate::journal::Journal;
use crate::world::World;
use alloc::boxed::Box;
use alloc::string::String;
use omnis_data::Data;
use serde::{Deserialize, Serialize};

/// The rules value that sets how many interactions the journal holds before a safe point folds
/// it, when a pack sets it.
pub const FOLD_RULE: &str = "fold_interactions";

/// How many interactions the journal holds before a safe point folds it, when no pack says
/// (owner, 2026-10-18: about 1.5 MB of journal, some 1.5 million commands of play).
pub const FOLD_INTERACTIONS: usize = 5000;

/// An archive segment on disk: `archive/<name>.ron` in a world folder (M8.5 7h2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    /// The world folder it lies in. In a character save, left out for the save's own folder; in
    /// memory, always named, since the world's folder changes with its packs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world: Option<String>,
    /// The hash of its text.
    pub name: u64,
}

/// One archive segment: the interactions of a journal folded away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Archived {
    /// Folded in this session; the next save writes it, named by the hash of its text.
    Held {
        /// The interactions.
        journal: Journal,
        /// The world it was folded from when that is not the world saving it (the packs changed
        /// since): the next save writes it, and that world's manifest, under that world's folder.
        from: Option<Box<Manifest>>,
    },
    /// Known by name from a loaded save.
    Named(Segment),
}

/// Fold `world`: its bases become what it holds now, its journal goes to the archive and starts
/// again empty. It cannot fail, so a fold at the end of a command never half-happens.
pub fn fold(world: &mut World, data: &Data) -> Event {
    fold_from(world, data, None)
}

/// Fold `world`, whose journal was written in the world `from` when that is not the world the
/// loaded packs make (a load by name on other packs, M8.5 7h3): its segment goes to that
/// world's folder, in that world's numbering.
pub(crate) fn fold_from(world: &mut World, data: &Data, from: Option<Box<Manifest>>) -> Event {
    let journal = core::mem::take(&mut world.journal);
    let interactions = u32::try_from(journal.len()).unwrap_or(u32::MAX);
    if !journal.is_empty() {
        world.archive.push(Archived::Held { journal, from });
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
