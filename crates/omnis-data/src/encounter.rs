//! Encounters keyed to map tiles: fixed placements at a coordinate and a per-map random table,
//! as written in the map file and as resolved to interned monsters. The simulation triggers
//! them (M4); the editor places them (M5).

use crate::error::DataError;
use crate::limits::{MAX_COLLECTION, MAX_STACKS};
use crate::registry::Interner;
use omnis_core::{Dice, MonsterId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// How a group feels about the party before a word is spoken. The index is a rules input.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Disposition {
    /// Attacks on sight; bribes cost the most, running is hardest.
    #[default]
    Hostile,
    /// Fights when pressed.
    Wary,
    /// Can be bought or slipped past.
    Neutral,
    /// Lets the party pass; hiding and running always succeed, a bribe is free.
    Friendly,
}

impl Disposition {
    /// Every disposition, in rules-table order.
    pub const ALL: [Disposition; 4] = [
        Disposition::Hostile,
        Disposition::Wary,
        Disposition::Neutral,
        Disposition::Friendly,
    ];

    /// 0 for Hostile through 3 for Friendly: the value formulas and tables take.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// A group placed on a tile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedEncounter {
    /// Trigger column.
    pub x: u16,
    /// Trigger row.
    pub y: u16,
    /// Monster id and count per stack, in stack order (the first stacks stand in front).
    pub stacks: Vec<(String, u8)>,
    /// The group's disposition.
    #[serde(default)]
    pub disposition: Disposition,
    /// Whether clearing the group removes it from the tile for the rest of the game.
    #[serde(default)]
    pub once: bool,
}

/// One entry of a random table: its weight and the stacks it spawns, each with a count roll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomEntry {
    /// Relative weight among the table's entries.
    pub weight: u16,
    /// Monster id and count dice per stack, in stack order.
    pub stacks: Vec<(String, Dice)>,
    /// The group's disposition.
    #[serde(default)]
    pub disposition: Disposition,
}

/// The random encounters of a map: a chance per step onto a tile and the table it draws from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomEncounters {
    /// Chance per step, in percent.
    pub chance_percent: u8,
    /// The table.
    pub entries: Vec<RandomEntry>,
}

/// A fixed encounter with interned monsters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEncounter {
    /// Trigger column.
    pub x: u16,
    /// Trigger row.
    pub y: u16,
    /// Monster and count per stack.
    pub stacks: Vec<(MonsterId, u8)>,
    /// The group's disposition.
    pub disposition: Disposition,
    /// Whether clearing it is remembered.
    pub once: bool,
}

/// A random table entry with interned monsters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEntry {
    /// Relative weight.
    pub weight: u16,
    /// Monster and count dice per stack.
    pub stacks: Vec<(MonsterId, Dice)>,
    /// The group's disposition.
    pub disposition: Disposition,
}

/// A random table with interned monsters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRandom {
    /// Chance per step, in percent.
    pub chance_percent: u8,
    /// The table.
    pub entries: Vec<ResolvedEntry>,
}

impl ResolvedRandom {
    /// The sum of the entry weights.
    #[must_use]
    pub fn total_weight(&self) -> u32 {
        self.entries.iter().map(|e| u32::from(e.weight)).sum()
    }
}

/// Checks that need no other file: coordinates within `size`, stack shapes, chances, weights,
/// dice.
pub fn validate(
    size: (u16, u16),
    encounters: &[FixedEncounter],
    random: Option<&RandomEncounters>,
    file: &Path,
    errors: &mut Vec<DataError>,
) {
    let (width, height) = size;
    if encounters.len() > MAX_COLLECTION {
        errors.push(DataError::new(file, "too many encounters"));
    }
    for (i, e) in encounters.iter().enumerate() {
        if e.x >= width || e.y >= height {
            errors.push(DataError::new(
                file,
                format!("encounter {i} at ({}, {}) is outside the map", e.x, e.y),
            ));
        }
        if let Some(k) = encounters[..i]
            .iter()
            .position(|o| o.x == e.x && o.y == e.y)
        {
            errors.push(DataError::new(
                file,
                format!(
                    "encounter {i} shares tile ({}, {}) with encounter {k}",
                    e.x, e.y
                ),
            ));
        }
        check_stacks(&format!("encounter {i}"), e.stacks.len(), file, errors);
        if e.stacks.iter().any(|(_, count)| *count == 0) {
            errors.push(DataError::new(
                file,
                format!("encounter {i} stack count must be at least 1"),
            ));
        }
    }
    let Some(random) = random else {
        return;
    };
    if random.chance_percent > 100 {
        errors.push(DataError::new(
            file,
            "random chance_percent must be 0..=100",
        ));
    }
    if random.entries.is_empty() {
        errors.push(DataError::new(file, "random needs at least one entry"));
    }
    if random.entries.len() > MAX_COLLECTION {
        errors.push(DataError::new(file, "too many random entries"));
    }
    for (j, entry) in random.entries.iter().enumerate() {
        if entry.weight == 0 {
            errors.push(DataError::new(
                file,
                format!("random entry {j} weight must be at least 1"),
            ));
        }
        check_stacks(
            &format!("random entry {j}"),
            entry.stacks.len(),
            file,
            errors,
        );
        if entry
            .stacks
            .iter()
            .any(|(_, dice)| dice.count == 0 || dice.sides == 0)
        {
            errors.push(DataError::new(
                file,
                format!("random entry {j} count needs dice"),
            ));
        }
    }
}

fn check_stacks(what: &str, stacks: usize, file: &Path, errors: &mut Vec<DataError>) {
    if stacks == 0 {
        errors.push(DataError::new(file, format!("{what} has no stacks")));
    }
    if stacks > MAX_STACKS {
        errors.push(DataError::new(
            file,
            format!("{what} has more than {MAX_STACKS} stacks"),
        ));
    }
}

/// Every monster an encounter names must be defined by some loaded pack.
pub(crate) fn resolve(
    encounters: &[FixedEncounter],
    random: Option<&RandomEncounters>,
    file: &Path,
    monsters: &Interner<MonsterId>,
    errors: &mut Vec<DataError>,
) -> (Vec<ResolvedEncounter>, Option<ResolvedRandom>) {
    let mut reported = BTreeSet::new();
    let mut lookup = |what: &str, id: &str| match monsters.get(id) {
        Some(m) => Some(m),
        None => {
            if reported.insert((what.to_owned(), id.to_owned())) {
                errors.push(DataError::new(
                    file,
                    format!("{what} monster '{id}' is not defined by any loaded pack"),
                ));
            }
            None
        }
    };
    let fixed = encounters
        .iter()
        .enumerate()
        .map(|(i, e)| ResolvedEncounter {
            x: e.x,
            y: e.y,
            stacks: e
                .stacks
                .iter()
                .filter_map(|(id, count)| {
                    lookup(&format!("encounter {i}"), id).map(|m| (m, *count))
                })
                .collect(),
            disposition: e.disposition,
            once: e.once,
        })
        .collect();
    let random = random.map(|random| ResolvedRandom {
        chance_percent: random.chance_percent,
        entries: random
            .entries
            .iter()
            .enumerate()
            .map(|(j, entry)| ResolvedEntry {
                weight: entry.weight,
                stacks: entry
                    .stacks
                    .iter()
                    .filter_map(|(id, dice)| {
                        lookup(&format!("random entry {j}"), id).map(|m| (m, *dice))
                    })
                    .collect(),
                disposition: entry.disposition,
            })
            .collect(),
    });
    (fixed, random)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Parts = (Vec<FixedEncounter>, Option<RandomEncounters>);

    fn parts(encounters: Vec<FixedEncounter>, random: Option<RandomEncounters>) -> Parts {
        (encounters, random)
    }

    fn messages(errors: &[DataError]) -> Vec<&str> {
        errors.iter().map(|e| e.message.as_str()).collect()
    }

    #[test]
    fn every_shape_problem_is_reported() {
        let def = parts(
            vec![
                FixedEncounter {
                    x: 2,
                    y: 0,
                    stacks: vec![("t:monster:a".into(), 0)],
                    disposition: Disposition::Hostile,
                    once: true,
                },
                FixedEncounter {
                    x: 1,
                    y: 1,
                    stacks: vec![],
                    disposition: Disposition::Wary,
                    once: false,
                },
                FixedEncounter {
                    x: 1,
                    y: 1,
                    stacks: vec![("t:monster:a".into(), 1); 5],
                    disposition: Disposition::Friendly,
                    once: false,
                },
            ],
            Some(RandomEncounters {
                chance_percent: 101,
                entries: vec![
                    RandomEntry {
                        weight: 0,
                        stacks: vec![],
                        disposition: Disposition::Neutral,
                    },
                    RandomEntry {
                        weight: 1,
                        stacks: vec![("t:monster:a".into(), Dice::new(0, 4))],
                        disposition: Disposition::Neutral,
                    },
                ],
            }),
        );
        let mut errors = vec![];
        validate((2, 2), &def.0, def.1.as_ref(), Path::new("m"), &mut errors);
        assert_eq!(
            messages(&errors),
            [
                "encounter 0 at (2, 0) is outside the map",
                "encounter 0 stack count must be at least 1",
                "encounter 1 has no stacks",
                "encounter 2 shares tile (1, 1) with encounter 1",
                "encounter 2 has more than 4 stacks",
                "random chance_percent must be 0..=100",
                "random entry 0 weight must be at least 1",
                "random entry 0 has no stacks",
                "random entry 1 count needs dice",
            ]
        );
        let mut errors = vec![];
        validate((2, 2), &[], None, Path::new("m"), &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn monsters_resolve_through_the_interner_or_are_reported() {
        let mut monsters = Interner::<MonsterId>::default();
        let rat = monsters.intern("t:monster:rat");
        let def = parts(
            vec![FixedEncounter {
                x: 0,
                y: 1,
                stacks: vec![("t:monster:rat".into(), 3), ("t:monster:none".into(), 1)],
                disposition: Disposition::Neutral,
                once: true,
            }],
            Some(RandomEncounters {
                chance_percent: 5,
                entries: vec![RandomEntry {
                    weight: 3,
                    stacks: vec![("t:monster:rat".into(), Dice::new(1, 4))],
                    disposition: Disposition::Hostile,
                }],
            }),
        );
        let mut errors = vec![];
        let (fixed, random) = resolve(
            &def.0,
            def.1.as_ref(),
            Path::new("m"),
            &monsters,
            &mut errors,
        );
        assert_eq!(
            messages(&errors),
            ["encounter 0 monster 't:monster:none' is not defined by any loaded pack"]
        );
        assert_eq!(fixed.len(), 1);
        assert_eq!(fixed[0].stacks, [(rat, 3)]);
        assert_eq!((fixed[0].x, fixed[0].y, fixed[0].once), (0, 1, true));
        let random = random.unwrap();
        assert_eq!(random.total_weight(), 3);
        assert_eq!(random.entries[0].stacks, [(rat, Dice::new(1, 4))]);
        assert_eq!(Disposition::Friendly.index(), 3);
        assert_eq!(Disposition::ALL[0], Disposition::default());
    }
}
