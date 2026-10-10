//! Class features that act in a fight (M7c): which a member has, how many uses are left, and
//! the rests that give them back. The effects themselves resolve in the simulation's fight;
//! this file holds what is true of a character outside one.

use crate::character::Character;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::{ClassFeature, Data, Recharge};

/// The member's class features with an effect, gained by their level, in the class file's
/// order: the rows a `Feature` command names.
#[must_use]
pub fn combat_features<'a>(character: &Character, data: &'a Data) -> Vec<&'a ClassFeature> {
    data.classes
        .get(&character.class)
        .map(|class| {
            class
                .features
                .iter()
                .filter(|f| f.effect.is_some() && f.level <= character.level)
                .collect()
        })
        .unwrap_or_default()
}

/// Uses spent on a feature since its rest.
#[must_use]
pub fn spent(character: &Character, feature: &ClassFeature) -> u8 {
    character
        .feature_spent
        .binary_search_by(|(name, _)| name.as_str().cmp(&feature.name))
        .map_or(0, |at| character.feature_spent[at].1)
}

/// Uses left before a rest, `None` for an at-will feature.
#[must_use]
pub fn uses_left(character: &Character, feature: &ClassFeature) -> Option<u8> {
    feature
        .uses
        .map(|uses| uses.count.saturating_sub(spent(character, feature)))
}

/// Spend one use; an at-will feature spends nothing.
pub fn spend_use(character: &mut Character, feature: &ClassFeature) {
    if feature.uses.is_none() {
        return;
    }
    match character
        .feature_spent
        .binary_search_by(|(name, _)| name.as_str().cmp(&feature.name))
    {
        Ok(at) => character.feature_spent[at].1 = character.feature_spent[at].1.saturating_add(1),
        Err(at) => character
            .feature_spent
            .insert(at, (String::from(feature.name.as_str()), 1)),
    }
}

/// A rest gives uses back: a long rest every feature's, a short rest those restored by a short
/// rest. A spent entry no class feature names any more is dropped.
pub fn recover_uses(character: &mut Character, data: &Data, long: bool) {
    let features = combat_features(character, data);
    character.feature_spent.retain(|(name, _)| {
        !long
            && features
                .iter()
                .any(|f| f.name == *name && f.uses.is_some_and(|u| u.per == Recharge::LongRest))
    });
}
