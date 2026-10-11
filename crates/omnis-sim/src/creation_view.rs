//! `creation.get` (parity P3): what point buy allows and what each race, class and background
//! brings, read through the same functions that check a draft (`omnis_rules::create`), so a
//! client builds drafts the rules accept without reading the rules itself.

use crate::party;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::Data;
use omnis_data::terms::{Ability, Alignment, Skill};
use omnis_rules::{NAME_MAX_BYTES, point_budget, race_grants, score_cost, score_range};
use serde::{Deserialize, Serialize};

/// A race a draft may take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaceOption {
    /// `pack:race:name`, the draft's `race`.
    pub id: String,
    /// Text key of the race's name.
    pub name: String,
    /// What the race adds to each bought score, SRD order.
    pub bonuses: [i8; 6],
    /// Skills the race's traits make a member proficient in.
    pub grants: Vec<Skill>,
}

/// A class a draft may take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassOption {
    /// `pack:class:name`, the draft's `class`.
    pub id: String,
    /// Text key of the class's name.
    pub name: String,
    /// How many skills a draft picks from `skill_list`, exactly.
    pub choose: u8,
    /// The class's skill list.
    pub skill_list: Vec<Skill>,
}

/// A background a draft may take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundOption {
    /// `pack:background:name`, the draft's `background`.
    pub id: String,
    /// Text key of the background's name.
    pub name: String,
    /// Skills the background makes a member proficient in; a draft may not pick them again.
    pub grants: Vec<Skill>,
}

/// What a draft may be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationView {
    /// How many members the party holds.
    pub slots: usize,
    /// The longest name, in bytes, after trimming.
    pub name_max: usize,
    /// The lowest bought score.
    pub score_min: u8,
    /// The highest bought score.
    pub score_max: u8,
    /// The points the six bought scores may cost together.
    pub points: i64,
    /// What each score costs, from `score_min` to `score_max`.
    pub costs: Vec<i64>,
    /// Races, in id order.
    pub races: Vec<RaceOption>,
    /// Classes, in id order.
    pub classes: Vec<ClassOption>,
    /// Backgrounds, in id order.
    pub backgrounds: Vec<BackgroundOption>,
    /// Every alignment, in SRD order.
    pub alignments: Vec<Alignment>,
}

/// What a draft may be under the loaded rules.
#[must_use]
pub fn creation_view(data: &Data) -> CreationView {
    let (score_min, score_max) = score_range(data);
    CreationView {
        slots: party::slots(data),
        name_max: NAME_MAX_BYTES,
        score_min,
        score_max,
        points: point_budget(data),
        costs: (score_min..=score_max)
            .map_while(|s| score_cost(s, data).ok())
            .collect(),
        races: data
            .races
            .values()
            .map(|race| {
                let mut bonuses = [0; 6];
                for ability in Ability::ALL {
                    bonuses[ability.index()] =
                        race.ability_bonuses.get(&ability).copied().unwrap_or(0);
                }
                RaceOption {
                    id: race.id.clone(),
                    name: race.name.clone(),
                    bonuses,
                    grants: race_grants(race),
                }
            })
            .collect(),
        classes: data
            .classes
            .values()
            .map(|class| ClassOption {
                id: class.id.clone(),
                name: class.name.clone(),
                choose: class.skills.choose,
                skill_list: class.skills.from.clone(),
            })
            .collect(),
        backgrounds: data
            .backgrounds
            .values()
            .map(|background| BackgroundOption {
                id: background.id.clone(),
                name: background.name.clone(),
                grants: background.skills.clone(),
            })
            .collect(),
        alignments: Alignment::ALL.to_vec(),
    }
}
