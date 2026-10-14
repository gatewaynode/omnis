//! Checks on object kinds that need other files: the items they name, the rules slots, the
//! text keys and the world flags, which are interned here (objects are their first source).

use super::{Effect, ObjectKind, Requirement};
use crate::content::Files;
use crate::error::DataError;
use crate::item::Item;
use crate::loader::Data;
use crate::manifest::is_content_id;

/// The inputs an object's rules slot receives.
pub const OBJECT_RULE_INPUTS: [&str; 2] = ["party_level", "party_size"];

/// Check every kind's references, then intern the flags the kinds name.
pub(crate) fn check_objects(
    objects: &Files<ObjectKind>,
    items: &Files<Item>,
    data: &mut Data,
    errors: &mut Vec<DataError>,
) {
    for (file, kind) in objects.values() {
        let mut error = |message: String| errors.push(DataError::new(file, message));
        for key in text_keys(kind) {
            if data.registry.text.get(key).is_none() {
                error(format!("text key '{key}' is not defined in any language"));
            }
        }
        for interaction in &kind.interactions {
            for requirement in &interaction.requires {
                check_requirement(requirement, items, data, &mut error);
            }
            for effect in &interaction.effects {
                check_effect(effect, items, &mut error);
            }
        }
    }
    for kind in objects.values().map(|(_, kind)| kind) {
        for name in flags(kind) {
            if is_content_id(name) {
                data.registry.flags.intern(name);
            }
        }
    }
}

fn check_requirement(
    requirement: &Requirement,
    items: &Files<Item>,
    data: &Data,
    error: &mut impl FnMut(String),
) {
    match requirement {
        Requirement::Item { id, .. } if !items.contains_key(id) => {
            error(format!("item '{id}' is not defined by any loaded pack"));
        }
        Requirement::Flag { name, .. } => check_flag(name, error),
        Requirement::Rule { slot } => match data.rules.slot(slot) {
            None => error(format!("rule '{slot}' is not a rules slot")),
            Some(found) => {
                for input in found.inputs() {
                    if !OBJECT_RULE_INPUTS.contains(&input.as_str()) {
                        error(format!(
                            "rule '{slot}' declares input '{input}'; an object's rule takes only {}",
                            OBJECT_RULE_INPUTS.join(", ")
                        ));
                    }
                }
            }
        },
        _ => {}
    }
}

fn check_effect(effect: &Effect, items: &Files<Item>, error: &mut impl FnMut(String)) {
    match effect {
        Effect::GiveItem { id, .. } if !items.contains_key(id) => {
            error(format!("item '{id}' is not defined by any loaded pack"));
        }
        Effect::SetFlag { name, .. } => check_flag(name, error),
        _ => {}
    }
}

fn check_flag(name: &str, error: &mut impl FnMut(String)) {
    if !is_content_id(name) || name.split(':').nth(1) != Some("flag") {
        error(format!("flag '{name}' is not of the form pack:flag:name"));
    }
}

/// Every text key a kind names: its name, labels, failures and messages.
fn text_keys(kind: &ObjectKind) -> impl Iterator<Item = &str> {
    let per_interaction = kind.interactions.iter().flat_map(|it| {
        let messages = it.effects.iter().filter_map(|e| match e {
            Effect::Message(key) => Some(key.as_str()),
            _ => None,
        });
        it.label
            .as_deref()
            .into_iter()
            .chain(it.failure.as_deref())
            .chain(messages)
    });
    core::iter::once(kind.name.as_str()).chain(per_interaction)
}

/// Every flag a kind names, in requirements and effects.
fn flags(kind: &ObjectKind) -> impl Iterator<Item = &str> {
    kind.interactions.iter().flat_map(|it| {
        let required = it.requires.iter().filter_map(|r| match r {
            Requirement::Flag { name, .. } => Some(name.as_str()),
            _ => None,
        });
        let set = it.effects.iter().filter_map(|e| match e {
            Effect::SetFlag { name, .. } => Some(name.as_str()),
            _ => None,
        });
        required.chain(set)
    })
}
