//! The full log's history (owner, 2026-10-17): every line the session said, exploration and
//! fight alike, each stamped with the party's clock when it was said, the newest [`HISTORY`]
//! kept. It lasts the session until the save model of PRD D28 keeps the party's interactions,
//! from which it will be read. Bevy-free.

use crate::labels;
use std::collections::VecDeque;

/// The most lines kept; the oldest go first.
pub const HISTORY: usize = 2000;

/// A line and when it was said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The party's clock, in minutes since the game began.
    pub minute: i64,
    /// The words.
    pub text: String,
}

impl Entry {
    /// The line with its stamp before it: "Day 3 14:20  Found: Lever".
    #[must_use]
    pub fn stamped(&self) -> String {
        format!("{}  {}", labels::clock(self.minute), self.text)
    }
}

/// The lines, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    entries: VecDeque<Entry>,
}

impl History {
    /// None yet.
    #[must_use]
    pub const fn new() -> History {
        History {
            entries: VecDeque::new(),
        }
    }

    /// Keep `text`, said at `minute`, dropping the oldest past [`HISTORY`].
    pub fn push(&mut self, minute: i64, text: String) {
        if self.entries.len() == HISTORY {
            self.entries.pop_front();
        }
        self.entries.push_back(Entry { minute, text });
    }

    /// How many lines are kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether none are.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The lines, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// The lines holding every word of `filter`, in any case, in the stamp or the words; every
    /// line when it has no words. Oldest first.
    #[must_use]
    pub fn matching(&self, filter: &str) -> Vec<&Entry> {
        let words: Vec<String> = filter.split_whitespace().map(str::to_lowercase).collect();
        self.entries
            .iter()
            .filter(|e| {
                let line = e.stamped().to_lowercase();
                words.iter().all(|w| line.contains(w.as_str()))
            })
            .collect()
    }
}
