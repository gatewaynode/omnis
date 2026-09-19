//! A character sheet, and its creation from a draft by point buy (PRD §7.1, owner decision
//! 2026-09-12: bought scores only).

use crate::effect::ActiveEffect;
use crate::equip::{Equipped, auto_equip};
use crate::stats::{int_result, modifier, point_cost, spell_point_pool};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::fmt;
use omnis_core::{
    BackgroundId, CharacterId, ClassId, ConditionId, ItemId, Pcg32, RaceId, SpellId, StreamName,
};
use omnis_data::limits::string_fits;
use omnis_data::{Ability, Alignment, Background, Class, Data, Effect, Race, Skill};
use omnis_expr::{RuleError, Value};
use serde::{Deserialize, Serialize};

/// Longest name a character may have, in bytes.
pub const NAME_MAX_BYTES: usize = 32;

/// Death saving throws in progress: counters while a member lies at zero hit points.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeathSaves {
    /// Successes so far, 0..=3.
    pub successes: u8,
    /// Failures so far, 0..=3.
    pub failures: u8,
    /// Stable: no more saves until healed or hurt again.
    pub stable: bool,
}

/// A party member's sheet. Scores include racial bonuses. `equipment` is everything carried;
/// `equipped` says what is worn and wielded, and only that counts for the rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Character {
    /// Stable identity within the world.
    pub id: CharacterId,
    /// Display name, as typed.
    pub name: String,
    /// Race.
    pub race: RaceId,
    /// Class.
    pub class: ClassId,
    /// Background.
    pub background: BackgroundId,
    /// Alignment.
    pub alignment: Alignment,
    /// Level, 1..=20.
    pub level: u8,
    /// Experience points.
    pub xp: u32,
    /// The six scores in SRD order.
    pub scores: [u8; 6],
    /// Proficient skills, sorted.
    pub skills: Vec<Skill>,
    /// Current hit points.
    pub hp: i32,
    /// Hit point maximum.
    pub hp_max: i32,
    /// Current spell points.
    pub spell_points: u32,
    /// Spell point maximum.
    pub spell_points_max: u32,
    /// Spells known.
    pub known_spells: Vec<SpellId>,
    /// Items carried as `(item, count)`.
    pub equipment: Vec<(ItemId, u16)>,
    /// Conditions in effect.
    pub conditions: Vec<ConditionId>,
    /// Age in years at creation; the rest is subjective elapsed time (PRD §7.5).
    pub age_years: u16,
    /// The party clock when the character was created.
    pub created_at: i64,
    /// Death saving throws in progress, meaningful while `hp` is zero.
    #[serde(default)]
    pub death_saves: DeathSaves,
    /// What is worn and wielded, by slot; every entry is also carried.
    #[serde(default)]
    pub equipped: Equipped,
    /// Spell effects in force on this member; only live ones are kept.
    #[serde(default)]
    pub effects: Vec<ActiveEffect>,
    /// Reaction spells the member casts on their own when the moment comes, sorted.
    #[serde(default)]
    pub auto_cast: Vec<SpellId>,
}

impl Character {
    /// At zero hit points or below: unconscious or dead, out of the fight.
    #[must_use]
    pub const fn is_down(&self) -> bool {
        self.hp <= 0
    }
}

/// What the player chooses. Ids are strings so a command carries no pack-specific numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    /// Display name.
    pub name: String,
    /// `pack:race:name`.
    pub race: String,
    /// `pack:class:name`.
    pub class: String,
    /// `pack:background:name`.
    pub background: String,
    /// Alignment.
    pub alignment: Alignment,
    /// Bought scores in SRD order, each within the point-buy range.
    pub scores: [u8; 6],
    /// Skills picked from the class list, exactly as many as the class allows.
    #[serde(default)]
    pub skills: Vec<Skill>,
}

/// Why a draft was refused. A rule refusal, not an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreationError {
    /// Empty, blank, or longer than [`NAME_MAX_BYTES`].
    Name,
    /// No such race is loaded.
    UnknownRace(String),
    /// No such class is loaded.
    UnknownClass(String),
    /// No such background is loaded.
    UnknownBackground(String),
    /// A score outside the point-buy range.
    ScoreRange {
        /// The offending score.
        score: u8,
        /// Lowest allowed.
        min: u8,
        /// Highest allowed.
        max: u8,
    },
    /// More points spent than the budget allows.
    Points {
        /// Points the scores cost.
        spent: i64,
        /// Points available.
        budget: i64,
    },
    /// Wrong number of skills, a skill off the class list, or one already held.
    Skills(String),
    /// A rule formula failed; bad data rather than a bad draft.
    Rule(RuleError),
}

impl fmt::Display for CreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreationError::Name => write!(f, "name must be 1..={NAME_MAX_BYTES} bytes"),
            CreationError::UnknownRace(id) => write!(f, "unknown race '{id}'"),
            CreationError::UnknownClass(id) => write!(f, "unknown class '{id}'"),
            CreationError::UnknownBackground(id) => write!(f, "unknown background '{id}'"),
            CreationError::ScoreRange { score, min, max } => {
                write!(f, "score {score} is outside {min}..={max}")
            }
            CreationError::Points { spent, budget } => {
                write!(f, "{spent} points spent of {budget}")
            }
            CreationError::Skills(why) => write!(f, "skills: {why}"),
            CreationError::Rule(e) => write!(f, "{e}"),
        }
    }
}

impl From<RuleError> for CreationError {
    fn from(e: RuleError) -> Self {
        CreationError::Rule(e)
    }
}

/// Build a level-1 character from a draft against the loaded data. `created_at` is the party
/// clock; `rng` serves any formula that rolls.
pub fn create(
    draft: &Draft,
    data: &Data,
    id: CharacterId,
    created_at: i64,
    rng: &mut Pcg32,
) -> Result<Character, CreationError> {
    let name = draft.name.trim();
    if name.is_empty() || name.len() > NAME_MAX_BYTES || !string_fits(name) {
        return Err(CreationError::Name);
    }
    let (race_id, race) = lookup(&data.registry.races, &data.races, &draft.race)
        .ok_or_else(|| CreationError::UnknownRace(draft.race.clone()))?;
    let (class_id, class) = lookup(&data.registry.classes, &data.classes, &draft.class)
        .ok_or_else(|| CreationError::UnknownClass(draft.class.clone()))?;
    let (background_id, background) = lookup(
        &data.registry.backgrounds,
        &data.backgrounds,
        &draft.background,
    )
    .ok_or_else(|| CreationError::UnknownBackground(draft.background.clone()))?;
    let spent = point_cost(draft.scores, data)?;
    let budget = data.rules.value("point_budget").unwrap_or(27);
    if spent > budget {
        return Err(CreationError::Points { spent, budget });
    }
    let skills = chosen_skills(draft, class, background, race)?;
    let scores = with_bonuses(draft.scores, race);
    let per_level: i64 = race
        .features
        .iter()
        .map(|f| match f.effect {
            Effect::HitPointsPerLevel(n) => i64::from(n),
            _ => 0,
        })
        .sum();
    let hp = data.rules.eval(
        "hit_points.first_level",
        &[
            ("hit_die", Value::Int(i64::from(class.hit_die))),
            (
                "con_mod",
                Value::Int(modifier(scores[Ability::Constitution.index()])),
            ),
            ("per_level", Value::Int(per_level)),
        ],
        rng,
        &StreamName::new("party"),
    )?;
    let hp = i32::try_from(int_result("hit_points.first_level", hp.value)?).unwrap_or(i32::MAX);
    let equipment = starting_kit(class, background, data);
    let equipped = auto_equip(
        data,
        &equipment,
        modifier(scores[Ability::Dexterity.index()]),
    );
    let mut character = Character {
        id,
        name: name.to_string(),
        race: race_id,
        class: class_id,
        background: background_id,
        alignment: draft.alignment,
        level: 1,
        xp: 0,
        scores,
        skills,
        hp,
        hp_max: hp,
        spell_points: 0,
        spell_points_max: 0,
        known_spells: known_spells(class, data),
        equipment,
        conditions: vec![],
        age_years: race.starting_age,
        created_at,
        death_saves: DeathSaves::default(),
        equipped,
        effects: Vec::new(),
        auto_cast: Vec::new(),
    };
    let pool = spell_point_pool(&character, data, rng)?;
    character.spell_points = pool;
    character.spell_points_max = pool;
    Ok(character)
}

/// The class's starting equipment and the background's, as interned ids.
fn starting_kit(class: &Class, background: &Background, data: &Data) -> Vec<(ItemId, u16)> {
    class
        .starting_equipment
        .iter()
        .chain(&background.equipment)
        .filter_map(|(item, count)| data.registry.items.get(item).map(|id| (id, *count)))
        .collect()
}

fn lookup<'a, I: Copy + Ord + From<u32> + Into<u32>, T>(
    interner: &omnis_data::registry::Interner<I>,
    table: &'a alloc::collections::BTreeMap<I, T>,
    id: &str,
) -> Option<(I, &'a T)> {
    let key = interner.get(id)?;
    table.get(&key).map(|value| (key, value))
}

fn with_bonuses(bought: [u8; 6], race: &Race) -> [u8; 6] {
    let mut scores = bought;
    for (ability, bonus) in &race.ability_bonuses {
        let i = ability.index();
        let raised = (i64::from(scores[i]) + i64::from(*bonus)).clamp(1, 30);
        scores[i] = u8::try_from(raised).unwrap_or(1);
    }
    scores
}

/// Background and racial proficiencies, then the class picks: the right number, on the list,
/// and not already held.
fn chosen_skills(
    draft: &Draft,
    class: &Class,
    background: &Background,
    race: &Race,
) -> Result<Vec<Skill>, CreationError> {
    let choose = usize::from(class.skills.choose);
    if draft.skills.len() != choose {
        return Err(CreationError::Skills(format!(
            "choose {choose} from the class list, not {}",
            draft.skills.len()
        )));
    }
    let mut skills: Vec<Skill> = background.skills.clone();
    skills.extend(race.features.iter().filter_map(|f| match f.effect {
        Effect::SkillProficiency(skill) => Some(skill),
        _ => None,
    }));
    for skill in &draft.skills {
        if !class.skills.from.contains(skill) {
            return Err(CreationError::Skills(format!(
                "{skill:?} is not on the class list"
            )));
        }
        let picked_twice = draft.skills.iter().filter(|s| *s == skill).count() > 1;
        if skills.contains(skill) || picked_twice {
            return Err(CreationError::Skills(format!(
                "{skill:?} is already proficient"
            )));
        }
    }
    skills.extend(draft.skills.iter().copied());
    skills.sort();
    skills.dedup();
    Ok(skills)
}

/// The first cantrips and levelled spells of the class list, as many as level 1 allows.
fn known_spells(class: &Class, data: &Data) -> Vec<SpellId> {
    let Some(casting) = &class.casting else {
        return vec![];
    };
    let (mut cantrips, mut spells) = (0, 0);
    let mut known = Vec::new();
    for id in &casting.list {
        let Some(spell_id) = data.registry.spells.get(id) else {
            continue;
        };
        let Some(spell) = data.spells.get(&spell_id) else {
            continue;
        };
        if spell.level == 0 {
            if cantrips < casting.cantrips_at_1 {
                known.push(spell_id);
                cantrips += 1;
            }
        } else if spells < casting.spells_at_1 {
            known.push(spell_id);
            spells += 1;
        }
    }
    known
}
