//! Derived numbers and rolls over a character and the loaded data.

use crate::character::{Character, CreationError};
use alloc::format;
use omnis_core::{Dice, Pcg32, RollTrace, StreamName};
use omnis_data::{Ability, ArmorKind, Data, ItemKind, Skill, Spell};
use omnis_expr::{RuleError, Value};
use serde::{Deserialize, Serialize};

/// The SRD ability modifier: `(score - 10) / 2` rounded down.
#[must_use]
pub const fn modifier(score: u8) -> i64 {
    (score as i64 - 10).div_euclid(2)
}

fn table<'a>(data: &'a Data, name: &str) -> Result<&'a [i64], RuleError> {
    data.rules
        .table(name)
        .ok_or_else(|| RuleError::new(name, "table is not defined by any loaded pack"))
}

/// Proficiency bonus at a level, from the `proficiency_bonus` table.
pub fn proficiency_bonus(level: u8, data: &Data) -> Result<i64, RuleError> {
    let table = table(data, "proficiency_bonus")?;
    let index = usize::from(level.max(1) - 1).min(table.len().saturating_sub(1));
    table
        .get(index)
        .copied()
        .ok_or_else(|| RuleError::new("proficiency_bonus", "table is empty"))
}

/// The level a total of experience points has reached, from the `xp_thresholds` table.
pub fn level_for_xp(xp: u32, data: &Data) -> Result<u8, RuleError> {
    let thresholds = table(data, "xp_thresholds")?;
    let reached = thresholds
        .iter()
        .take_while(|t| **t <= i64::from(xp))
        .count();
    Ok(u8::try_from(reached.max(1)).unwrap_or(u8::MAX))
}

/// Points spent on six bought scores, or why they are not allowed. Costs come from the
/// `point_cost` table indexed by `score - score_min`.
pub fn point_cost(scores: [u8; 6], data: &Data) -> Result<i64, CreationError> {
    let min = u8::try_from(data.rules.value("score_min").unwrap_or(8)).unwrap_or(8);
    let max = u8::try_from(data.rules.value("score_max").unwrap_or(15)).unwrap_or(15);
    let costs = table(data, "point_cost").map_err(CreationError::Rule)?;
    let mut spent = 0;
    for score in scores {
        if score < min || score > max {
            return Err(CreationError::ScoreRange { score, min, max });
        }
        let cost = costs
            .get(usize::from(score - min))
            .copied()
            .ok_or(CreationError::ScoreRange { score, min, max })?;
        spent += cost;
    }
    Ok(spent)
}

/// Armor class: 10 plus Dexterity unarmored, else the best armor carried with its Dexterity cap,
/// plus a shield. Everything carried counts as worn until equipment slots arrive (M6).
#[must_use]
pub fn armor_class(character: &Character, data: &Data) -> i64 {
    let dex = modifier(character.scores[Ability::Dexterity.index()]);
    let mut best = 10 + dex;
    let mut shield = 0;
    for (item, _) in &character.equipment {
        let Some(item) = data.items.get(item) else {
            continue;
        };
        if let ItemKind::Armor {
            kind,
            base_ac,
            dex_cap,
            ..
        } = &item.kind
        {
            if *kind == ArmorKind::Shield {
                shield = shield.max(i64::from(*base_ac));
            } else {
                let capped = dex_cap.map_or(dex, |cap| dex.min(i64::from(cap)));
                best = best.max(i64::from(*base_ac) + capped);
            }
        }
    }
    best + shield
}

/// Whether a d20 is rolled once, or twice keeping the better or the worse die.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum RollMode {
    /// One die.
    #[default]
    Normal,
    /// Two dice, the higher counts.
    Advantage,
    /// Two dice, the lower counts.
    Disadvantage,
}

impl RollMode {
    /// The SRD rule: any advantage and any disadvantage cancel to a normal roll.
    #[must_use]
    pub const fn combine(advantage: bool, disadvantage: bool) -> RollMode {
        match (advantage, disadvantage) {
            (true, false) => RollMode::Advantage,
            (false, true) => RollMode::Disadvantage,
            _ => RollMode::Normal,
        }
    }
}

/// A d20 roll with its parts, so a client can show the math. Under advantage or disadvantage
/// the trace holds both dice (its own total is their sum) and `face` is the one kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roll {
    /// The dice.
    pub trace: RollTrace,
    /// How many dice were rolled and which counts.
    pub mode: RollMode,
    /// The die that counts.
    pub face: u32,
    /// Ability modifier (or a monster's attack bonus) added.
    pub modifier: i64,
    /// Proficiency bonus added, zero when not proficient.
    pub proficiency: i64,
    /// Face plus both.
    pub total: i64,
}

/// One d20 under a mode: the trace and the face that counts.
pub fn kept_d20(
    mode: RollMode,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<(RollTrace, u32), RuleError> {
    let count = if mode == RollMode::Normal { 1 } else { 2 };
    let trace = Dice::new(count, 20)
        .roll(rng, stream)
        .map_err(|e| RuleError::new("d20", format!("{e}")))?;
    let faces = trace.rolls.iter().map(|r| r.value);
    let face = match mode {
        RollMode::Normal | RollMode::Advantage => faces.max(),
        RollMode::Disadvantage => faces.min(),
    }
    .unwrap_or(1);
    Ok((trace, face))
}

fn d20(
    character: &Character,
    data: &Data,
    ability: Ability,
    proficient: bool,
    mode: RollMode,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<Roll, RuleError> {
    let (trace, face) = kept_d20(mode, rng, stream)?;
    let modifier = modifier(character.scores[ability.index()]);
    let proficiency = if proficient {
        proficiency_bonus(character.level, data)?
    } else {
        0
    };
    let total = i64::from(face) + modifier + proficiency;
    Ok(Roll {
        trace,
        mode,
        face,
        modifier,
        proficiency,
        total,
    })
}

/// An ability check, with a skill's proficiency when one applies.
pub fn check(
    character: &Character,
    data: &Data,
    skill: Option<Skill>,
    ability: Ability,
    mode: RollMode,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<Roll, RuleError> {
    let proficient = skill.is_some_and(|s| character.skills.contains(&s));
    d20(character, data, ability, proficient, mode, rng, stream)
}

/// A saving throw; the class's two saving throws are proficient.
pub fn save(
    character: &Character,
    data: &Data,
    ability: Ability,
    mode: RollMode,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<Roll, RuleError> {
    let proficient = data
        .classes
        .get(&character.class)
        .is_some_and(|c| c.saving_throws.contains(&ability));
    d20(character, data, ability, proficient, mode, rng, stream)
}

/// A skill's bonus without a die: the ability modifier plus proficiency when proficient.
pub fn skill_bonus(character: &Character, data: &Data, skill: Skill) -> Result<i64, RuleError> {
    let ability = modifier(character.scores[skill.ability().index()]);
    let proficiency = if character.skills.contains(&skill) {
        proficiency_bonus(character.level, data)?
    } else {
        0
    };
    Ok(ability + proficiency)
}

/// A passive score: 10 plus the skill's bonus (SRD passive checks).
pub fn passive(character: &Character, data: &Data, skill: Skill) -> Result<i64, RuleError> {
    Ok(10 + skill_bonus(character, data, skill)?)
}

/// The spell point pool of a caster at their level (D12, slot `spell_points.pool`); zero for a
/// class without casting.
pub fn spell_point_pool(
    character: &Character,
    data: &Data,
    rng: &mut Pcg32,
) -> Result<u32, RuleError> {
    let Some(casting) = data
        .classes
        .get(&character.class)
        .and_then(|c| c.casting.as_ref())
    else {
        return Ok(0);
    };
    let mental = [Ability::Intelligence, Ability::Wisdom, Ability::Charisma];
    let cast_mod = modifier(character.scores[casting.ability.index()]);
    let other: i64 = mental
        .iter()
        .filter(|a| **a != casting.ability)
        .map(|a| modifier(character.scores[a.index()]))
        .sum();
    let outcome = data.rules.eval(
        "spell_points.pool",
        &[
            ("level", Value::Int(i64::from(character.level))),
            ("cast_mod", Value::Int(cast_mod)),
            ("other_mental_mods", Value::Int(other)),
            ("half_caster", Value::Bool(casting.half)),
        ],
        rng,
        &StreamName::new("party"),
    )?;
    int_result("spell_points.pool", outcome.value)
        .map(|v| u32::try_from(v.max(0)).unwrap_or(u32::MAX))
}

/// Points to cast a spell: its declared cost, else the `spell_points.cost` slot.
pub fn spell_cost(spell: &Spell, data: &Data, rng: &mut Pcg32) -> Result<u32, RuleError> {
    if let Some(points) = spell.points {
        return Ok(points);
    }
    let outcome = data.rules.eval(
        "spell_points.cost",
        &[("spell_level", Value::Int(i64::from(spell.level)))],
        rng,
        &StreamName::new("party"),
    )?;
    int_result("spell_points.cost", outcome.value)
        .map(|v| u32::try_from(v.max(0)).unwrap_or(u32::MAX))
}

pub(crate) fn int_result(slot: &str, value: Value) -> Result<i64, RuleError> {
    value
        .as_int()
        .ok_or_else(|| RuleError::new(slot, "formula must produce an integer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modifier_rounds_down() {
        assert_eq!(modifier(1), -5);
        assert_eq!(modifier(8), -1);
        assert_eq!(modifier(9), -1);
        assert_eq!(modifier(10), 0);
        assert_eq!(modifier(11), 0);
        assert_eq!(modifier(15), 2);
        assert_eq!(modifier(20), 5);
        assert_eq!(modifier(30), 10);
    }
}
