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

/// The last meeting between two holders, kept on each side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Contact {
    /// The other holder.
    pub other: HolderId,
    /// This holder's clock when they last met.
    pub self_elapsed: i64,
    /// The other holder's clock when they last met.
    pub other_elapsed: i64,
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
}
