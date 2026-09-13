//! Reads over a monster stat block: modifiers, passive perception, defenses, which attack to
//! use, whom to hit, and hit points for one individual.

use crate::condition::Defenses;
use crate::stats::{int_result, modifier};
use omnis_core::{Pcg32, StreamName};
use omnis_data::{Ability, Attack, DamageType, Data, Monster};
use omnis_expr::{RuleError, Value};

/// The monster's modifier for an ability.
#[must_use]
pub const fn modifier_of(monster: &Monster, ability: Ability) -> i64 {
    modifier(monster.abilities[ability.index()])
}

/// Passive Perception: 10 plus the Wisdom modifier (stat blocks carry no skill list).
#[must_use]
pub const fn passive_perception(monster: &Monster) -> i64 {
    10 + modifier_of(monster, Ability::Wisdom)
}

/// The monster's defenses against a damage type.
#[must_use]
pub fn defenses(monster: &Monster, kind: DamageType) -> Defenses {
    Defenses {
        resist: monster.resistances.contains(&kind),
        vulnerable: monster.vulnerabilities.contains(&kind),
        immune: monster.immunities.contains(&kind),
    }
}

/// The attack a monster uses: from the front, its first melee attack (else its first ranged
/// one); from the back, its first ranged attack or none.
#[must_use]
pub fn pick_attack(monster: &Monster, ranged_only: bool) -> Option<&Attack> {
    if ranged_only {
        return monster.attacks.iter().find(|a| a.ranged);
    }
    monster
        .attacks
        .iter()
        .find(|a| !a.ranged)
        .or_else(|| monster.attacks.first())
}

/// One uniform draw among `count` candidates; `None` when there are none.
#[must_use]
pub fn choose_target(count: usize, rng: &mut Pcg32) -> Option<usize> {
    let bound = u32::try_from(count).ok().filter(|c| *c > 0)?;
    usize::try_from(rng.below(bound)).ok()
}

/// Hit points of one individual, from the `monster.hit_points` slot (the SRD average by
/// default; a pack may roll).
pub fn hit_points(
    monster: &Monster,
    data: &Data,
    rng: &mut Pcg32,
    stream: &StreamName,
) -> Result<i32, RuleError> {
    let dice = monster.hit_points;
    let outcome = data.rules.eval(
        "monster.hit_points",
        &[
            ("count", Value::Int(i64::from(dice.count))),
            ("sides", Value::Int(i64::from(dice.sides))),
            ("modifier", Value::Int(i64::from(dice.modifier))),
        ],
        rng,
        stream,
    )?;
    let hp = int_result("monster.hit_points", outcome.value)?;
    Ok(i32::try_from(hp.max(1)).unwrap_or(i32::MAX))
}
