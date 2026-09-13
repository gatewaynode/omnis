//! Attacks and what follows them: the weapons a member can swing or shoot, the attack roll
//! against armor class, the damage roll with critical hits and defenses, initiative, and death
//! saving throws. Rust rolls the dice; the `combat` rule slots add and compare.

use crate::character::{Character, DeathSaves};
use crate::condition::Defenses;
use crate::stats::{Roll, RollMode, int_result, kept_d20, modifier, proficiency_bonus};
use alloc::format;
use alloc::vec::Vec;
use omnis_core::{Dice, ItemId, Pcg32, RollTrace, StreamName};
use omnis_data::{Ability, DamageType, Data, ItemKind};
use omnis_expr::{RuleError, Value};
use serde::{Deserialize, Serialize};

/// A weapon a member can attack with. Unarmed is a weapon with no dice: one point plus Strength.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weapon {
    /// The item, or none for unarmed.
    pub item: Option<ItemId>,
    /// Damage dice, or none for the unarmed single point.
    pub damage: Option<Dice>,
    /// Damage type.
    pub damage_type: DamageType,
    /// Reaches the back and is used from it.
    pub ranged: bool,
    /// The ability behind the attack: Dexterity for ranged, Strength otherwise.
    pub ability: Ability,
    /// Whether the member's class is proficient with it.
    pub proficient: bool,
}

impl Weapon {
    /// Unarmed: one point of bludgeoning plus Strength, always proficient.
    #[must_use]
    pub const fn unarmed() -> Weapon {
        Weapon {
            item: None,
            damage: None,
            damage_type: DamageType::Bludgeoning,
            ranged: false,
            ability: Ability::Strength,
            proficient: true,
        }
    }

    /// Twice the average damage of the dice (an integer, so ties are exact); one point unarmed.
    #[must_use]
    pub const fn average_twice(&self) -> i32 {
        match self.damage {
            Some(dice) => dice.min() + dice.max(),
            None => 2,
        }
    }
}

/// Every weapon carried, in carrying order, then unarmed.
#[must_use]
pub fn weapons(character: &Character, data: &Data) -> Vec<Weapon> {
    let class = data.classes.get(&character.class);
    let mut out: Vec<Weapon> = character
        .equipment
        .iter()
        .filter_map(|(id, _)| data.items.get(id).map(|item| (*id, item)))
        .filter_map(|(id, item)| match &item.kind {
            ItemKind::Weapon {
                kind,
                damage,
                damage_type,
                ranged,
                ..
            } => {
                let proficient = class.is_some_and(|c| {
                    c.weapons.contains(kind)
                        || data
                            .registry
                            .items
                            .name(id)
                            .is_some_and(|name| c.weapon_ids.iter().any(|w| w == name))
                });
                Some(Weapon {
                    item: Some(id),
                    damage: Some(*damage),
                    damage_type: *damage_type,
                    ranged: *ranged,
                    ability: if *ranged {
                        Ability::Dexterity
                    } else {
                        Ability::Strength
                    },
                    proficient,
                })
            }
            _ => None,
        })
        .collect();
    out.push(Weapon::unarmed());
    out
}

/// The best weapon for a reach: the ranged one with the highest average damage, or the melee
/// one (unarmed at worst). Ties keep the one carried first.
#[must_use]
pub fn best_weapon(character: &Character, data: &Data, ranged: bool) -> Option<Weapon> {
    weapons(character, data)
        .into_iter()
        .filter(|w| w.ranged == ranged)
        .fold(None, |best: Option<Weapon>, w| match best {
            Some(b) if b.average_twice() >= w.average_twice() => Some(b),
            _ => Some(w),
        })
}

/// The bonus a member adds to attack rolls with a weapon, and the proficiency part of it.
pub fn attack_bonus(
    character: &Character,
    data: &Data,
    weapon: &Weapon,
) -> Result<(i64, i64), RuleError> {
    let ability = modifier(character.scores[weapon.ability.index()]);
    let proficiency = if weapon.proficient {
        proficiency_bonus(character.level, data)?
    } else {
        0
    };
    Ok((ability, proficiency))
}

/// An attack roll resolved against an armor class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackRoll {
    /// The d20 and its parts.
    pub roll: Roll,
    /// The armor class it was rolled against.
    pub ac: i64,
    /// Whether it hit.
    pub hit: bool,
    /// Whether it was a critical hit.
    pub crit: bool,
}

/// Roll to hit: `bonus` is the ability modifier or a monster's attack bonus, `proficiency` the
/// bonus for a proficient weapon (zero for monsters, whose stat block folds it in).
pub fn attack_roll(
    data: &Data,
    bonus: i64,
    proficiency: i64,
    ac: i64,
    mode: RollMode,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<AttackRoll, RuleError> {
    let (trace, face) = kept_d20(mode, rng, stream)?;
    let die = Value::Int(i64::from(face));
    let total = data.rules.eval(
        "attack.total",
        &[
            ("die", die),
            ("bonus", Value::Int(bonus)),
            ("proficiency", Value::Int(proficiency)),
        ],
        rng,
        stream,
    )?;
    let total = int_result("attack.total", total.value)?;
    let hit = data.rules.eval(
        "attack.hit",
        &[
            ("die", die),
            ("total", Value::Int(total)),
            ("ac", Value::Int(ac)),
        ],
        rng,
        stream,
    )?;
    let crit = data
        .rules
        .eval("attack.crit", &[("die", die)], rng, stream)?;
    Ok(AttackRoll {
        roll: Roll {
            trace,
            mode,
            face,
            modifier: bonus,
            proficiency,
            total,
        },
        ac,
        hit: bool_result("attack.hit", hit.value)?,
        crit: bool_result("attack.crit", crit.value)?,
    })
}

fn bool_result(slot: &str, value: Value) -> Result<bool, RuleError> {
    match value {
        Value::Bool(b) => Ok(b),
        Value::Int(_) => Err(RuleError::new(slot, "formula must produce true or false")),
    }
}

/// How the target's defenses changed the damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageAdjust {
    /// Taken in full.
    None,
    /// Halved.
    Resisted,
    /// Doubled.
    Vulnerable,
    /// Ignored.
    Immune,
}

/// A damage roll: the dice (twice on a critical), the total before defenses, and after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageRoll {
    /// The dice rolled, none for an unarmed hit.
    pub rolls: Vec<RollTrace>,
    /// Dice plus bonus, never below zero.
    pub raw: i64,
    /// After the target's defenses.
    pub amount: i64,
    /// Which defense applied.
    pub adjust: DamageAdjust,
}

/// Roll damage. The dice's own modifier and `bonus` are each added once; a critical hit rolls
/// the dice twice. `None` dice is the unarmed single point.
pub fn damage_roll(
    data: &Data,
    dice: Option<Dice>,
    bonus: i64,
    crit: bool,
    defenses: Defenses,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<DamageRoll, RuleError> {
    let mut rolls = Vec::new();
    let mut sum: i64 = 0;
    let mut flat = bonus;
    match dice {
        Some(dice) => {
            flat += i64::from(dice.modifier);
            let plain = Dice::new(dice.count, dice.sides);
            for _ in 0..if crit { 2 } else { 1 } {
                let trace = plain
                    .roll(rng, stream)
                    .map_err(|e| RuleError::new("damage", format!("{e}")))?;
                sum += i64::from(trace.total);
                rolls.push(trace);
            }
        }
        None => sum = 1,
    }
    let raw = data.rules.eval(
        "damage.total",
        &[("dice", Value::Int(sum)), ("bonus", Value::Int(flat))],
        rng,
        stream,
    )?;
    let raw = int_result("damage.total", raw.value)?;
    let amount = data.rules.eval(
        "damage.adjusted",
        &[
            ("amount", Value::Int(raw)),
            ("resist", Value::Bool(defenses.resist)),
            ("vulnerable", Value::Bool(defenses.vulnerable)),
            ("immune", Value::Bool(defenses.immune)),
        ],
        rng,
        stream,
    )?;
    let adjust = if defenses.immune {
        DamageAdjust::Immune
    } else if defenses.resist {
        DamageAdjust::Resisted
    } else if defenses.vulnerable {
        DamageAdjust::Vulnerable
    } else {
        DamageAdjust::None
    };
    Ok(DamageRoll {
        rolls,
        raw,
        amount: int_result("damage.adjusted", amount.value)?,
        adjust,
    })
}

/// An initiative roll: the total from the `initiative` slot and the die behind it.
pub fn initiative(
    data: &Data,
    dex_mod: i64,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<(i64, RollTrace), RuleError> {
    let (trace, face) = kept_d20(RollMode::Normal, rng, stream)?;
    let total = data.rules.eval(
        "initiative",
        &[
            ("die", Value::Int(i64::from(face))),
            ("dex_mod", Value::Int(dex_mod)),
        ],
        rng,
        stream,
    )?;
    Ok((int_result("initiative", total.value)?, trace))
}

/// What one death saving throw, or a wound at zero hit points, led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathSaveResult {
    /// One more success.
    Success,
    /// One more failure.
    Failure,
    /// Three successes: stable until healed or hurt.
    Stable,
    /// A natural 20: back at one hit point.
    Revived,
    /// Three failures.
    Died,
}

/// A death saving throw against `death_save_dc`: a natural 1 counts two failures, a natural 20
/// revives, three successes stabilize, three failures kill. Counters reset on revival and on
/// stabilizing, as the SRD says.
pub fn death_save(
    data: &Data,
    saves: &mut DeathSaves,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<(RollTrace, DeathSaveResult), RuleError> {
    let dc = data.rules.value("death_save_dc").unwrap_or(10);
    let (trace, face) = kept_d20(RollMode::Normal, rng, stream)?;
    let result = if face == 20 {
        *saves = DeathSaves::default();
        DeathSaveResult::Revived
    } else if i64::from(face) >= dc {
        saves.successes += 1;
        if saves.successes >= 3 {
            *saves = DeathSaves {
                stable: true,
                ..DeathSaves::default()
            };
            DeathSaveResult::Stable
        } else {
            DeathSaveResult::Success
        }
    } else {
        saves.failures += if face == 1 { 2 } else { 1 };
        settle_failures(saves)
    };
    Ok((trace, result))
}

/// Damage taken while at zero hit points: one failure, two for a critical hit; the member is no
/// longer stable.
pub fn wound_at_zero(saves: &mut DeathSaves, crit: bool) -> DeathSaveResult {
    saves.stable = false;
    saves.failures += if crit { 2 } else { 1 };
    settle_failures(saves)
}

fn settle_failures(saves: &mut DeathSaves) -> DeathSaveResult {
    if saves.failures >= 3 {
        saves.failures = 3;
        DeathSaveResult::Died
    } else {
        DeathSaveResult::Failure
    }
}
