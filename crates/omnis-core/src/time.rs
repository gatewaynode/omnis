//! Subjective time (ARCHITECTURE.md §4.4, A13). There is no global clock: every holder carries
//! its own `Clock`, and clocks reconcile only when holders interact.

use crate::{EraId, HolderId};
use serde::{Deserialize, Serialize};

/// A holder's own experience of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Clock {
    /// Subjective minutes since the holder's origin.
    pub elapsed: i64,
    /// Which age of the world the holder lives in. v1 content has a single era.
    pub era: EraId,
}

impl Clock {
    /// A clock at its origin in `era`.
    #[must_use]
    pub const fn new(era: EraId) -> Clock {
        Clock { elapsed: 0, era }
    }

    /// Advance by `minutes`. Returns whether a day boundary was crossed, given the calendar's
    /// `minutes_per_day` (data; 1440 for Toel's default calendar). Saturates at `i64::MAX`.
    pub fn advance(&mut self, minutes: u32, minutes_per_day: u32) -> bool {
        let before = self.day(minutes_per_day);
        self.elapsed = self.elapsed.saturating_add(i64::from(minutes));
        self.day(minutes_per_day) != before
    }

    /// The day index for a calendar of `minutes_per_day`. A zero-length day is treated as one
    /// minute so the query stays total.
    #[must_use]
    pub const fn day(&self, minutes_per_day: u32) -> i64 {
        let per_day = if minutes_per_day == 0 {
            1
        } else {
            minutes_per_day as i64
        };
        self.elapsed.div_euclid(per_day)
    }

    /// The minute within the current day.
    #[must_use]
    pub const fn minute_of_day(&self, minutes_per_day: u32) -> u32 {
        let per_day = if minutes_per_day == 0 {
            1
        } else {
            minutes_per_day as i64
        };
        self.elapsed.rem_euclid(per_day) as u32
    }
}

/// The calendar's shape, data in `rules/time.ron` (ARCHITECTURE.md §4.4): a rendering of a
/// date in minutes, and the hours of night.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Calendar {
    /// Minutes in a day; at least 1.
    pub minutes_per_day: u32,
    /// Days in a year; at least 1.
    pub days_per_year: u32,
    /// The minute of the day night begins.
    pub night_from: u32,
    /// The minute of the day night ends; night wraps past midnight when it is below `night_from`.
    pub night_to: u32,
}

impl Default for Calendar {
    fn default() -> Calendar {
        Calendar {
            minutes_per_day: 1440,
            days_per_year: 360,
            night_from: 1200,
            night_to: 360,
        }
    }
}

/// A date rendered on a calendar: all counted from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Date {
    /// The year.
    pub year: i64,
    /// The day of the year.
    pub day: u32,
    /// The minute of the day.
    pub minute: u32,
}

impl Calendar {
    fn per_day(self) -> i64 {
        i64::from(self.minutes_per_day.max(1))
    }

    /// `minutes` as a year, a day and a minute.
    #[must_use]
    pub fn date(self, minutes: i64) -> Date {
        let days = minutes.div_euclid(self.per_day());
        let per_year = i64::from(self.days_per_year.max(1));
        Date {
            year: days.div_euclid(per_year),
            day: u32::try_from(days.rem_euclid(per_year)).unwrap_or(0),
            minute: u32::try_from(minutes.rem_euclid(self.per_day())).unwrap_or(0),
        }
    }

    /// The day index of `minutes`, counted from 0.
    #[must_use]
    pub fn day(self, minutes: i64) -> i64 {
        minutes.div_euclid(self.per_day())
    }

    /// Whether `minutes` falls in the night.
    #[must_use]
    pub fn night(self, minutes: i64) -> bool {
        let m = self.date(minutes).minute;
        if self.night_from <= self.night_to {
            (self.night_from..self.night_to).contains(&m)
        } else {
            m >= self.night_from || m < self.night_to
        }
    }
}

/// The last meeting between two holders, kept on each side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Contact {
    /// The other holder.
    pub other: HolderId,
    /// This holder's clock when they last met.
    pub self_elapsed: i64,
    /// The other holder's clock when they last met.
    pub other_elapsed: i64,
    /// The party's shared time (minutes × company, per mille) when they last met; 0 on a
    /// side that is not the party.
    #[serde(default)]
    pub self_shared: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_rolls_on_the_boundary() {
        let mut clock = Clock::new(EraId(0));
        assert!(!clock.advance(1439, 1440));
        assert_eq!(clock.minute_of_day(1440), 1439);
        assert!(clock.advance(1, 1440));
        assert_eq!((clock.day(1440), clock.minute_of_day(1440)), (1, 0));
        assert!(clock.advance(2880, 1440));
        assert_eq!(clock.day(1440), 3);
    }

    #[test]
    fn advance_saturates_and_zero_day_is_total() {
        let mut clock = Clock {
            elapsed: i64::MAX - 1,
            era: EraId(0),
        };
        clock.advance(u32::MAX, 1440);
        assert_eq!(clock.elapsed, i64::MAX);
        assert_eq!(
            Clock {
                elapsed: 5,
                era: EraId(0)
            }
            .day(0),
            5
        );
    }

    #[test]
    fn a_calendar_renders_years_days_and_the_night() {
        let c = Calendar::default();
        let at = |y: i64, d: i64, m: i64| (y * 360 + d) * 1440 + m;
        assert_eq!(
            c.date(at(2, 39, 14 * 60 + 20)),
            Date {
                year: 2,
                day: 39,
                minute: 860
            }
        );
        assert!(c.night(at(0, 0, 0)), "midnight");
        assert!(c.night(at(0, 0, 359)));
        assert!(!c.night(at(0, 0, 360)), "dawn");
        assert!(!c.night(at(0, 0, 1199)));
        assert!(c.night(at(0, 0, 1200)), "dusk");
        let day_night = Calendar {
            night_from: 100,
            night_to: 200,
            ..c
        };
        assert!(day_night.night(150) && !day_night.night(99) && !day_night.night(200));
        let empty = Calendar {
            minutes_per_day: 0,
            days_per_year: 0,
            ..c
        };
        assert_eq!(
            empty.date(7),
            Date {
                year: 7,
                day: 0,
                minute: 0
            },
            "total on a zero calendar"
        );
    }
}
