//! The full log retold from the party's log (M8.5 7f): after a load, each saved event is said
//! again in the current words, as `Session::take_in` said it live, and stamped with the clock
//! it was said at. What the session said of its own ("Saved …") and a search's summary of what
//! was in view are not in the party's log, so they are not retold. Bevy-free.

use super::text::describe;
use crate::history::History;
use crate::rolllog::{self, Names};
use omnis_sim::api::Logged;
use omnis_sim::omnis_data::Data;
use omnis_sim::party_view::PartyView;

/// The full log of `log`, and the names it leaves: the party's members, and the monsters of
/// the last encounter as its events numbered them.
#[must_use]
pub fn retold(log: &[Logged], party: &PartyView, data: &Data) -> (History, Names) {
    let mut names = Names::default();
    names.observe(party);
    let mut history = History::new();
    for logged in log {
        names.locate(logged.map);
        let line = describe(&logged.event, &names, data)
            .or_else(|| rolllog::describe(&logged.event, &names, data));
        if let Some(line) = line {
            history.push(logged.minute, line);
        }
        names.follow(&logged.event, data);
    }
    (history, names)
}
