//! The rules of the crawl, SRD 5.1 structure with the PRD §8 adaptations: ability modifiers,
//! proficiency, hit points, armor class, checks and saves with traced rolls, spell points
//! (D12), leveling thresholds, and character creation by point buy. Constants the SRD defines
//! once (the modifier formula, the d20) are Rust; everything expected to change is a rule slot
//! or table in `data/rules` reached through `omnis-expr`.

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

extern crate alloc;

mod character;
mod stats;

pub use character::{Character, CreationError, Draft, NAME_MAX_BYTES, create};
pub use omnis_expr::RuleError;
pub use stats::{
    Roll, armor_class, check, level_for_xp, modifier, point_cost, proficiency_bonus, save,
    spell_cost, spell_point_pool,
};
