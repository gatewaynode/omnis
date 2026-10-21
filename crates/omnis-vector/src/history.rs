//! The full log's history (owner, 2026-10-17): every line the session said, exploration and
//! fight alike, each stamped with the party's clock when it was said, the newest [`HISTORY`]
//! kept. A load retells it from the party's saved log (M8.5 7f, `shell::retold`). Bevy-free.

use crate::labels;
use std::collections::VecDeque;

/// The most lines kept; the oldest go first. As many as the party's log keeps events (M8.5 7f).
pub const HISTORY: usize = omnis_sim::LOG_CAPACITY;

/// A line and when it was said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The party's clock, in minutes since the game began.
    pub minute: i64,
    /// The words.
    pub text: String,
    /// The stamped line in lower case, which a filter reads: made once, as the full log can
    /// hold [`HISTORY`] lines and its screen filters them every frame it is open.
    folded: String,
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
        let mut entry = Entry {
            minute,
            text,
            folded: String::new(),
        };
        entry.folded = entry.stamped().to_lowercase();
        self.entries.push_back(entry);
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
            .filter(|e| words.iter().all(|w| e.folded.contains(w.as_str())))
            .collect()
    }
}
