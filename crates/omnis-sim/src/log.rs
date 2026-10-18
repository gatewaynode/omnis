//! The party's log (M8.5 7f): the newest [`LOG_CAPACITY`] events, each stamped with the party's
//! clock and the map it stood on when it was said. It is the party's, so the character save
//! carries it (the events a full log tells, [`told`]) and a fold leaves it be; a client reads
//! its full log back from it after a load. Not part of the world's RON or the fingerprint.

use crate::LOG_CAPACITY;
use crate::event::Event;
use crate::world::World;
use alloc::vec::Vec;
use omnis_core::MapId;
use serde::{Deserialize, Serialize};

/// An event and when and where it was said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Logged {
    /// The party's age in minutes when it was said.
    pub minute: i64,
    /// The map the party stood on, whose placements an object event names.
    pub map: MapId,
    /// The event.
    pub event: Event,
}

/// Whether the character save keeps `event`: all but the view and the dice of each step
/// (`Visible`, `EncounterCheck`, `TimeAdvanced`), which no full log tells and which would make
/// the saved log six times the size (owner, 2026-10-18). The log in memory keeps them all.
#[must_use]
pub fn told(event: &Event) -> bool {
    !matches!(
        event,
        Event::Visible { .. } | Event::EncounterCheck { .. } | Event::TimeAdvanced { .. }
    )
}

/// The party's log, oldest first.
#[must_use]
pub fn history(world: &World) -> &[Logged] {
    &world.log
}

/// Keep the newest [`LOG_CAPACITY`] of `log`.
pub(crate) fn bound(log: &mut Vec<Logged>) {
    let excess = log.len().saturating_sub(LOG_CAPACITY);
    log.drain(..excess);
}
