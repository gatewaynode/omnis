//! What a spell, a usable item or a class feature may be aimed at (owner, 2026-11-14; PRD §7.3,
//! §8.3; `tasks/plans/targeting.md`). The data says how far the option reaches; the simulation
//! lists what stands within that and judges a chosen target with the same rule.

use crate::error::DataError;
use crate::limits::MAX_VISIBILITY_DEPTH;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// How far an option reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Range {
    /// The actor only.
    Own,
    /// Those nearby, within one action's movement: every member; in a fight the front stacks;
    /// on the road what stands on the party's cell or the faced one.
    Touch,
    /// Everything within this many tiles of the party, counted as a king moves. A fight has
    /// no distances, so there it is every combatant.
    Tiles(u8),
    /// What the party sees: a clear line within its visibility depth.
    Sight,
    /// What the actor is linked to, wherever it is.
    Link,
}

/// An option's targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Targeting {
    /// How far it reaches.
    pub range: Range,
    /// A clear line to the target is needed as well ("a creature you can see within range").
    /// Only [`Range::Tiles`] takes it: sight already needs one, and the others never do.
    #[serde(default)]
    pub sees: bool,
}

impl Targeting {
    /// The actor only.
    pub const OWN: Targeting = Targeting::of(Range::Own);
    /// Those nearby.
    pub const TOUCH: Targeting = Targeting::of(Range::Touch);
    /// What the party sees.
    pub const SIGHT: Targeting = Targeting::of(Range::Sight);

    /// `range`, with no line needed beyond what the range itself asks.
    #[must_use]
    pub const fn of(range: Range) -> Targeting {
        Targeting { range, sees: false }
    }

    /// Whether a target must be in a clear line.
    #[must_use]
    pub const fn needs_sight(&self) -> bool {
        self.sees || matches!(self.range, Range::Sight)
    }

    /// Self-contained checks; every problem is pushed.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        match self.range {
            Range::Tiles(n) if n == 0 || n > MAX_VISIBILITY_DEPTH => {
                errors.push(DataError::new(
                    file,
                    format!("targeting range must be 1..={MAX_VISIBILITY_DEPTH} tiles"),
                ));
            }
            Range::Tiles(_) => {}
            Range::Own | Range::Touch | Range::Sight | Range::Link if self.sees => {
                errors.push(DataError::new(
                    file,
                    "targeting `sees` is for a range in tiles: sight needs a line already, and self, touch and link never do",
                ));
            }
            Range::Own | Range::Touch | Range::Sight | Range::Link => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ron_io::{from_str, to_string};

    fn faults(targeting: Targeting) -> Vec<String> {
        let mut errors = Vec::new();
        targeting.validate(Path::new("x.ron"), &mut errors);
        errors.into_iter().map(|e| e.to_string()).collect()
    }

    #[test]
    fn the_five_kinds_are_written_as_the_owner_named_them_and_read_back() {
        for (text, targeting) in [
            ("(range:Own)", Targeting::OWN),
            ("(range:Touch)", Targeting::TOUCH),
            ("(range:Tiles(12))", Targeting::of(Range::Tiles(12))),
            ("(range:Sight)", Targeting::SIGHT),
            ("(range:Link)", Targeting::of(Range::Link)),
            (
                "(range:Tiles(6),sees:true)",
                Targeting {
                    range: Range::Tiles(6),
                    sees: true,
                },
            ),
        ] {
            let read: Targeting = from_str(text, Path::new("x.ron")).expect(text);
            assert_eq!(read, targeting, "{text}");
            let written = to_string(&targeting).expect("written");
            let back: Targeting = from_str(&written, Path::new("x.ron")).expect("read");
            assert_eq!(back, targeting);
            assert!(faults(targeting).is_empty(), "{text}");
        }
    }

    #[test]
    fn a_range_in_tiles_is_within_what_can_be_seen_and_sees_is_only_for_it() {
        assert_eq!(faults(Targeting::of(Range::Tiles(0))).len(), 1);
        assert_eq!(
            faults(Targeting::of(Range::Tiles(MAX_VISIBILITY_DEPTH))).len(),
            0
        );
        assert_eq!(
            faults(Targeting::of(Range::Tiles(MAX_VISIBILITY_DEPTH + 1))).len(),
            1
        );
        for range in [Range::Own, Range::Touch, Range::Sight, Range::Link] {
            assert_eq!(
                faults(Targeting { range, sees: true }).len(),
                1,
                "{range:?}"
            );
        }
    }

    #[test]
    fn sight_is_needed_by_the_sight_range_and_by_sees() {
        assert!(Targeting::SIGHT.needs_sight());
        assert!(
            Targeting {
                range: Range::Tiles(3),
                sees: true
            }
            .needs_sight()
        );
        for range in [Range::Own, Range::Touch, Range::Tiles(3), Range::Link] {
            assert!(!Targeting::of(range).needs_sight(), "{range:?}");
        }
    }
}
