//! What a member's conditions and race do to the numbers: the mechanical flags the condition
//! data declares, folded over everything in effect, and damage defenses.

use crate::character::Character;
use omnis_core::ConditionId;
use omnis_data::{DamageType, Data, Effect};

/// The condition a pack defines under `<pack>:condition:<name>`: the first interned id whose
/// name ends in `:<name>`, so the simulation finds `dead` and `unconscious` in any pack.
#[must_use]
pub fn condition_id(data: &Data, name: &str) -> Option<ConditionId> {
    let registry = &data.registry.conditions;
    (0..u32::try_from(registry.len()).unwrap_or(u32::MAX))
        .map(ConditionId)
        .find(|id| {
            registry
                .name(*id)
                .is_some_and(|n| n.rsplit(':').next() == Some(name) && n.contains(':'))
        })
}

/// Every condition flag in effect, OR-ed over the conditions a combatant carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConditionFlags {
    /// Cannot act.
    pub incapacitated: bool,
    /// Attack rolls against the bearer have advantage.
    pub attacks_against_advantage: bool,
    /// The bearer's own attack rolls have disadvantage.
    pub own_attacks_disadvantage: bool,
    /// Strength and Dexterity saves fail without a roll.
    pub auto_fail_str_dex_saves: bool,
    /// A melee hit on the bearer is a critical hit.
    pub melee_hits_crit: bool,
    /// Every damage type is resisted.
    pub resist_all: bool,
}

/// The flags in effect for a list of conditions; unknown ids contribute nothing.
#[must_use]
pub fn flags(conditions: &[ConditionId], data: &Data) -> ConditionFlags {
    let mut out = ConditionFlags::default();
    for c in conditions.iter().filter_map(|id| data.conditions.get(id)) {
        out.incapacitated |= c.incapacitated;
        out.attacks_against_advantage |= c.attacks_against_advantage;
        out.own_attacks_disadvantage |= c.own_attacks_disadvantage;
        out.auto_fail_str_dex_saves |= c.auto_fail_str_dex_saves;
        out.melee_hits_crit |= c.melee_hits_crit;
        out.resist_all |= c.resist_all;
    }
    out
}

/// How a target takes one damage type. Immunity wins over resistance, resistance over
/// vulnerability, as the `damage.adjusted` formula reads them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Defenses {
    /// Halved.
    pub resist: bool,
    /// Doubled.
    pub vulnerable: bool,
    /// Ignored.
    pub immune: bool,
}

/// A member's defenses against a damage type: racial resistances and `resist_all` conditions.
#[must_use]
pub fn member_defenses(character: &Character, data: &Data, kind: DamageType) -> Defenses {
    let racial = data.races.get(&character.race).is_some_and(|race| {
        race.features
            .iter()
            .any(|f| f.effect == Effect::Resistance(kind))
    });
    Defenses {
        resist: racial || flags(&character.conditions, data).resist_all,
        vulnerable: false,
        immune: false,
    }
}
