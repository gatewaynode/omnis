//! `time.clocks` and the date in `game.status` (M8): every holder's clock and contact by name,
//! the party's age, shared time and date, and the date on the pack's calendar.

use crate::world::World;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Calendar, EraId, HolderId};
use omnis_data::Data;
use serde::{Deserialize, Serialize};

/// A date on the calendar, counted from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateView {
    /// Minutes on the calendar.
    pub minutes: i64,
    /// The year.
    pub year: i64,
    /// The day of the year.
    pub day: u32,
    /// The minute of the day.
    pub minute: u32,
    /// Whether it is night.
    pub night: bool,
    /// The era.
    pub era: EraId,
}

impl DateView {
    /// `minutes` in `era` on `calendar`.
    #[must_use]
    pub fn new(minutes: i64, era: EraId, calendar: Calendar) -> DateView {
        let date = calendar.date(minutes);
        DateView {
            minutes,
            year: date.year,
            day: date.day,
            minute: date.minute,
            night: calendar.night(minutes),
            era,
        }
    }
}

/// One holder's clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolderClock {
    /// `party:0`, or the region's id.
    pub holder: String,
    /// Minutes since its origin.
    pub elapsed: i64,
    /// Its era.
    pub era: EraId,
}

/// One side's memory of a meeting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactView {
    /// Who remembers.
    pub holder: String,
    /// Whom.
    pub other: String,
    /// The holder's clock then.
    pub self_elapsed: i64,
    /// The other's clock then.
    pub other_elapsed: i64,
}

/// `time.clocks`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeView {
    /// The party's age in minutes.
    pub age: i64,
    /// Its shared time in minutes (minutes lived × company, over 1000).
    pub shared: i64,
    /// The date it believes.
    pub date: DateView,
    /// Every clock, the party's first, then by holder.
    pub clocks: Vec<HolderClock>,
    /// Every contact, by holder then other.
    pub contacts: Vec<ContactView>,
}

/// A holder's name: `party:0` or the region's content id.
#[must_use]
pub fn holder_name(holder: HolderId, data: &Data) -> String {
    match holder {
        HolderId::Region(r) => data
            .registry
            .regions
            .name(r)
            .map_or_else(|| alloc::format!("{holder}"), String::from),
        other => alloc::format!("{other}"),
    }
}

/// The party's date on the pack's calendar.
#[must_use]
pub fn party_date(world: &World, data: &Data) -> DateView {
    let t = world.party_time;
    DateView::new(t.date, t.era, data.calendar())
}

/// `time.clocks`.
#[must_use]
pub fn time_view(world: &World, data: &Data) -> TimeView {
    TimeView {
        age: world.party_clock().elapsed,
        shared: world.party_time.shared_milli / 1000,
        date: party_date(world, data),
        clocks: world
            .clocks
            .iter()
            .map(|(holder, clock)| HolderClock {
                holder: holder_name(*holder, data),
                elapsed: clock.elapsed,
                era: clock.era,
            })
            .collect(),
        contacts: world
            .contacts
            .iter()
            .map(|((holder, other), contact)| ContactView {
                holder: holder_name(*holder, data),
                other: holder_name(*other, data),
                self_elapsed: contact.self_elapsed,
                other_elapsed: contact.other_elapsed,
            })
            .collect(),
    }
}
