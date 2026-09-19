//! The rules of the crawl, SRD 5.1 structure with the PRD §8 adaptations: ability modifiers,
//! proficiency, hit points, armor class, checks and saves with traced rolls, spell points
//! (D12), leveling thresholds, character creation by point buy, and combat: weapons, attack
//! and damage rolls, initiative, death saves, condition flags, and monster stat block reads.
//! Constants the SRD defines once (the modifier formula, the d20) are Rust; everything expected
//! to change is a rule slot or table in `data/rules` reached through `omnis-expr`.

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

extern crate alloc;

mod attack;
mod character;
mod condition;
mod effect;
mod equip;
mod monster;
mod spell;
mod stats;

pub use attack::{
    AttackBonus, AttackRoll, DamageAdjust, DamageRoll, DeathSaveResult, Weapon, attack_bonus,
    attack_roll, attack_roll_with, best_weapon, damage_roll, death_save, initiative, rejudge,
    weapons, wound_at_zero,
};
pub use character::{Character, CreationError, DeathSaves, Draft, NAME_MAX_BYTES, create};
pub use condition::{ConditionFlags, Defenses, condition_id, flags, member_defenses};
pub use effect::{
    ActiveEffect, EffectKind, Expiry, armor_bonus, bonus_dice, consumable_index, light_depth,
    roll_bonus,
};
pub use equip::{EquipRefusal, Equipped, auto_equip, can_equip, equip, equipped_item, unequip};
pub use monster::{
    choose_target, defenses as monster_defenses, hit_points as monster_hit_points, modifier_of,
    passive_perception, pick_attack,
};
pub use omnis_expr::RuleError;
pub use spell::{
    HealRoll, cantrip_dice, cast_modifier, casting_ability, concentration_dc, heal_roll,
    monster_save, needs_components, save_dc, saved_damage, spell_attack,
};
pub use stats::{
    Roll, RollMode, armor_class, check, kept_d20, level_for_xp, modifier, passive, point_cost,
    proficiency_bonus, save, skill_bonus, spell_cost, spell_point_pool,
};
