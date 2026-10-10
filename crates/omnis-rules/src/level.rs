//! Gaining a level at a trainer (PRD §8.2: experience accrues anywhere, the level is granted in
//! town) and the spells a member may add to their list (§8.3: prepared-list classes gain a
//! fixed number per level and buy or find the rest). Every number is a rule slot or table:
//! `xp_thresholds`, `hit_points.per_level`, `spell_points.pool`, `proficiency_bonus`,
//! `max_spell_level`, and the class's `spells_per_level`.

use crate::character::{Character, hp_bonus_per_level};
use crate::stats::{int_result, level_for_xp, modifier, proficiency_bonus, spell_point_pool};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{Pcg32, SpellId, StreamName};
use omnis_data::{Ability, Data};
use omnis_expr::{RuleError, Value};
use serde::{Deserialize, Serialize};

/// The highest level a character can reach (SRD).
pub const MAX_LEVEL: u8 = 20;

/// What one level brought: the new totals' increases and what the member may now do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gains {
    /// Hit points added to the maximum and to the current total.
    pub hp: i32,
    /// Spell points added to the maximum and to the current total.
    pub spell_points: u32,
    /// Spell picks added (the class's `spells_per_level`).
    pub picks: u8,
    /// The proficiency bonus at the new level.
    pub proficiency: i64,
    /// Text keys of the class features gained at the new level (labels until M7c).
    pub features: Vec<String>,
}

/// Why a spell cannot be added to a member's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellRefusal {
    /// The member's class has no casting, or the spell is not on its list.
    NotOnList,
    /// Cantrips come with the class, never by a pick or a purchase.
    Cantrip,
    /// Above the highest spell level the member's level allows.
    TooHigh {
        /// The spell's level.
        level: u8,
        /// The highest the member may learn.
        max: u8,
    },
    /// Already on the member's list.
    Known,
}

/// The experience a member needs for their next level, or `None` at the top.
pub fn next_threshold(character: &Character, data: &Data) -> Result<Option<u32>, RuleError> {
    if character.level >= MAX_LEVEL {
        return Ok(None);
    }
    let thresholds = data.rules.table("xp_thresholds").ok_or_else(|| {
        RuleError::new("xp_thresholds", "table is not defined by any loaded pack")
    })?;
    Ok(thresholds
        .get(usize::from(character.level))
        .and_then(|t| u32::try_from(*t).ok()))
}

/// The member's experience has reached a level they have not been granted.
pub fn ready(character: &Character, data: &Data) -> Result<bool, RuleError> {
    Ok(character.level < MAX_LEVEL && level_for_xp(character.xp, data)? > character.level)
}

/// Grant one level: the average hit points of `hit_points.per_level`, the pool at the new level
/// (current points rise by the same amount), one more hit die (the total is the level), the
/// class's spell picks, and its features for the level. The caller has checked [`ready`].
pub fn level_up(
    character: &mut Character,
    data: &Data,
    rng: &mut Pcg32,
) -> Result<Gains, RuleError> {
    if character.level >= MAX_LEVEL {
        return Err(RuleError::new("level", "already at the highest level"));
    }
    let class = data
        .classes
        .get(&character.class)
        .ok_or_else(|| RuleError::new("level", "the member's class is not loaded"))?;
    let race = data
        .races
        .get(&character.race)
        .ok_or_else(|| RuleError::new("level", "the member's race is not loaded"))?;
    let outcome = data.rules.eval(
        "hit_points.per_level",
        &[
            ("hit_die", Value::Int(i64::from(class.hit_die))),
            (
                "con_mod",
                Value::Int(modifier(character.scores[Ability::Constitution.index()])),
            ),
            ("per_level", Value::Int(hp_bonus_per_level(race))),
        ],
        rng,
        &StreamName::new("party"),
    )?;
    let hp = i32::try_from(int_result("hit_points.per_level", outcome.value)?.max(0))
        .unwrap_or(i32::MAX);
    let picks = class.casting.as_ref().map_or(0, |c| c.spells_per_level);
    character.level += 1;
    let pool = spell_point_pool(character, data, rng)?;
    let spell_points = pool.saturating_sub(character.spell_points_max);
    character.hp_max = character.hp_max.saturating_add(hp);
    character.hp = character.hp.saturating_add(hp);
    character.spell_points_max = pool;
    character.spell_points = character.spell_points.saturating_add(spell_points);
    character.spell_picks = character.spell_picks.saturating_add(picks);
    let features = class
        .features
        .iter()
        .filter(|f| f.level == character.level)
        .map(|f| f.name.clone())
        .collect();
    Ok(Gains {
        hp,
        spell_points,
        picks,
        proficiency: proficiency_bonus(character.level, data)?,
        features,
    })
}

/// The highest spell level a member may learn at a character level (`max_spell_level`).
pub fn max_spell_level(level: u8, data: &Data) -> Result<u8, RuleError> {
    let table = data.rules.table("max_spell_level").ok_or_else(|| {
        RuleError::new("max_spell_level", "table is not defined by any loaded pack")
    })?;
    let index = usize::from(level.max(1) - 1).min(table.len().saturating_sub(1));
    let max = table
        .get(index)
        .copied()
        .ok_or_else(|| RuleError::new("max_spell_level", "table is empty"))?;
    Ok(u8::try_from(max.clamp(0, 9)).unwrap_or(0))
}

/// Whether a spell may be added to the member's list, by a pick or a purchase.
pub fn may_learn(
    character: &Character,
    data: &Data,
    spell: SpellId,
) -> Result<Option<SpellRefusal>, RuleError> {
    let on_list = data
        .classes
        .get(&character.class)
        .and_then(|c| c.casting.as_ref())
        .is_some_and(|c| {
            c.list
                .iter()
                .any(|id| data.registry.spells.get(id) == Some(spell))
        });
    let Some(def) = data.spells.get(&spell).filter(|_| on_list) else {
        return Ok(Some(SpellRefusal::NotOnList));
    };
    if def.level == 0 {
        return Ok(Some(SpellRefusal::Cantrip));
    }
    let max = max_spell_level(character.level, data)?;
    if def.level > max {
        return Ok(Some(SpellRefusal::TooHigh {
            level: def.level,
            max,
        }));
    }
    if character.known_spells.contains(&spell) {
        return Ok(Some(SpellRefusal::Known));
    }
    Ok(None)
}

/// The spells of the member's class list they may pick now, in list order.
pub fn eligible(character: &Character, data: &Data) -> Result<Vec<SpellId>, RuleError> {
    let Some(casting) = data
        .classes
        .get(&character.class)
        .and_then(|c| c.casting.as_ref())
    else {
        return Ok(Vec::new());
    };
    let mut spells = Vec::new();
    for id in &casting.list {
        let Some(spell) = data.registry.spells.get(id) else {
            continue;
        };
        if may_learn(character, data, spell)?.is_none() {
            spells.push(spell);
        }
    }
    Ok(spells)
}
