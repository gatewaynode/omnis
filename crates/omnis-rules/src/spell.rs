//! Casting arithmetic: the casting ability and its modifier, spell attacks, save DCs and monster
//! saves, damage after a save, healing, cantrip scaling, concentration, and the component
//! threshold (PRD D11, D12, §8.3). Rust rolls; the `casting` slots add and compare.

use crate::attack::{AttackBonus, AttackRoll, attack_roll_with};
use crate::character::Character;
use crate::monster::modifier_of;
use crate::stats::{Roll, RollMode, int_result, kept_d20, modifier, proficiency_bonus};
use alloc::format;
use alloc::vec::Vec;
use omnis_core::{Dice, Pcg32, RollTrace, StreamName};
use omnis_data::{Ability, Data, Monster, Spell};
use omnis_expr::{RuleError, Value};

/// The class's casting ability, when it casts.
#[must_use]
pub fn casting_ability(character: &Character, data: &Data) -> Option<Ability> {
    data.classes
        .get(&character.class)
        .and_then(|c| c.casting.as_ref())
        .map(|c| c.ability)
}

/// The casting ability modifier; zero for a class without casting.
#[must_use]
pub fn cast_modifier(character: &Character, data: &Data) -> i64 {
    casting_ability(character, data).map_or(0, |a| modifier(character.scores[a.index()]))
}

/// A spell attack roll: the casting modifier plus proficiency against an armor class.
pub fn spell_attack(
    character: &Character,
    data: &Data,
    ac: i64,
    mode: RollMode,
    extra: Option<RollTrace>,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<AttackRoll, RuleError> {
    let bonus = AttackBonus {
        modifier: cast_modifier(character, data),
        proficiency: proficiency_bonus(character.level, data)?,
        extra,
    };
    attack_roll_with(data, bonus, ac, mode, rng, stream)
}

/// The difficulty of saving against the caster's spells (slot `spell.save_dc`).
pub fn save_dc(
    character: &Character,
    data: &Data,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<i64, RuleError> {
    let outcome = data.rules.eval(
        "spell.save_dc",
        &[
            ("cast_mod", Value::Int(cast_modifier(character, data))),
            (
                "proficiency",
                Value::Int(proficiency_bonus(character.level, data)?),
            ),
        ],
        rng,
        stream,
    )?;
    int_result("spell.save_dc", outcome.value)
}

/// A monster's saving throw: a d20 plus its ability modifier (stat blocks carry no proficient
/// saves), a success at or above the DC.
pub fn monster_save(
    monster: &Monster,
    ability: Ability,
    dc: i64,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<(Roll, bool), RuleError> {
    let (trace, face) = kept_d20(RollMode::Normal, rng, stream)?;
    let modifier = modifier_of(monster, ability);
    let total = i64::from(face) + modifier;
    let roll = Roll {
        trace,
        mode: RollMode::Normal,
        face,
        modifier,
        proficiency: 0,
        bonus: None,
        total,
    };
    Ok((roll, total >= dc))
}

/// Damage after a saving throw (slot `spell.save_damage`).
pub fn saved_damage(
    data: &Data,
    amount: i64,
    saved: bool,
    half_on_save: bool,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<i64, RuleError> {
    let outcome = data.rules.eval(
        "spell.save_damage",
        &[
            ("amount", Value::Int(amount)),
            ("saved", Value::Bool(saved)),
            ("half_on_save", Value::Bool(half_on_save)),
        ],
        rng,
        stream,
    )?;
    int_result("spell.save_damage", outcome.value)
}

/// A healing roll: the dice and what they came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealRoll {
    /// The dice rolled.
    pub rolls: Vec<RollTrace>,
    /// Hit points regained, never below zero.
    pub amount: i64,
}

/// Roll healing: the dice (with their own modifier) plus the casting modifier when `add_mod`
/// (slot `heal.total`).
pub fn heal_roll(
    character: &Character,
    data: &Data,
    dice: Dice,
    add_mod: bool,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<HealRoll, RuleError> {
    let trace = Dice::new(dice.count, dice.sides)
        .roll(rng, stream)
        .map_err(|e| RuleError::new("heal", format!("{e}")))?;
    let sum = i64::from(trace.total) + i64::from(dice.modifier);
    let outcome = data.rules.eval(
        "heal.total",
        &[
            ("dice", Value::Int(sum)),
            ("cast_mod", Value::Int(cast_modifier(character, data))),
            ("add_mod", Value::Bool(add_mod)),
        ],
        rng,
        stream,
    )?;
    Ok(HealRoll {
        rolls: alloc::vec![trace],
        amount: int_result("heal.total", outcome.value)?.max(0),
    })
}

/// A cantrip's dice at the caster's level: the count multiplied by slot `cantrip.dice`.
pub fn cantrip_dice(
    character: &Character,
    data: &Data,
    dice: Dice,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<Dice, RuleError> {
    let outcome = data.rules.eval(
        "cantrip.dice",
        &[("level", Value::Int(i64::from(character.level)))],
        rng,
        stream,
    )?;
    let times = int_result("cantrip.dice", outcome.value)?.clamp(1, 16);
    let count = u16::try_from(i64::from(dice.count) * times).unwrap_or(u16::MAX);
    Ok(Dice {
        count,
        sides: dice.sides,
        modifier: dice.modifier,
    })
}

/// Whether the spell's level is at or above the `component_threshold` rules value (D11).
#[must_use]
pub fn needs_components(spell: &Spell, data: &Data) -> bool {
    let threshold = data
        .rules
        .value("component_threshold")
        .unwrap_or(omnis_data::DEFAULT_COMPONENT_THRESHOLD);
    i64::from(spell.level) >= threshold
}

/// The DC of the Constitution save to keep concentrating after `damage` (slot
/// `concentration.dc`).
pub fn concentration_dc(
    data: &Data,
    damage: i64,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<i64, RuleError> {
    let outcome = data.rules.eval(
        "concentration.dc",
        &[("damage", Value::Int(damage))],
        rng,
        stream,
    )?;
    int_result("concentration.dc", outcome.value)
}
