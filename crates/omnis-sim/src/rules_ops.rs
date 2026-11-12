//! The `rules.*` ops: the compiled rule set over Tier 2, and where each rule came from. Every
//! slot, value and table states a priority in its file; the highest stands, and `origins` names
//! the file and pack that wrote it with every definition it beat (`omnis_data::rules`).

use crate::ops::{OpError, Reply, bounded};
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::Data;
use omnis_data::rules::{Beaten, RuleKind};
use serde::{Deserialize, Serialize};

/// One rule slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotView {
    /// Slot name.
    pub name: String,
    /// Declared inputs.
    pub inputs: Vec<String>,
    /// The expression.
    pub source: String,
}

/// The loaded rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesView {
    /// Every slot.
    pub slots: Vec<SlotView>,
    /// Plain values.
    pub values: BTreeMap<String, i64>,
    /// Tables.
    pub tables: BTreeMap<String, Vec<i64>>,
    /// Where each slot, value and table came from, in kind then name order.
    pub origins: Vec<RuleOrigin>,
}

/// Where one rule came from: the definition that stands, and every one it beat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleOrigin {
    /// What the rule is: `slot`, `value` or `table`. A name may be one of each.
    pub kind: RuleKind,
    /// The rule's name.
    pub name: String,
    /// The rules file that wrote the definition that stands (`pack:rules:name`).
    pub file: String,
    /// The pack that supplied that file.
    pub pack: String,
    /// Its priority: the highest among the definitions of this rule.
    pub priority: i32,
    /// The definitions it beat, each with its file, pack and priority, from the highest down.
    pub overrides: Vec<Beaten>,
}

/// The origin of one rule, when a loaded file wrote it.
#[must_use]
pub fn origin(data: &Data, kind: RuleKind, name: &str) -> Option<RuleOrigin> {
    let stands = data.standings.get(&(kind, name.to_owned()))?;
    Some(RuleOrigin {
        kind,
        name: name.to_owned(),
        file: stands.file.clone(),
        pack: stands.pack.clone(),
        priority: stands.priority,
        overrides: stands.beats.clone(),
    })
}

/// `rules.list`.
#[must_use]
pub fn rules_view(data: &Data) -> RulesView {
    RulesView {
        slots: data
            .rules
            .slot_names()
            .filter_map(|name| slot_view(data, name).ok())
            .collect(),
        values: data.rules.values().clone(),
        tables: data.rules.tables().clone(),
        origins: (data.standings.keys())
            .filter_map(|(kind, name)| origin(data, *kind, name))
            .collect(),
    }
}

/// `rules.get`, and what `rules.set` answers with.
pub fn slot_view(data: &Data, name: &str) -> Result<SlotView, OpError> {
    bounded(name)?;
    let slot = data.rules.slot(name).ok_or_else(|| OpError::UnknownSlot {
        slot: name.to_owned(),
    })?;
    Ok(SlotView {
        name: name.to_owned(),
        inputs: slot.inputs().to_vec(),
        source: slot.source().to_owned(),
    })
}

/// `rules.get`, and what `rules.set` answers with: the slot and its origin.
pub fn rule_reply(data: &Data, name: &str) -> Result<Reply, OpError> {
    slot_view(data, name).map(|rule| Reply::Rule {
        rule,
        origin: origin(data, RuleKind::Slot, name),
    })
}

/// `rules.set`: hot-swap a slot's expression, checked as the loader checks it, and answer with
/// the slot.
pub fn rules_set(data: &mut Data, slot: &str, source: &str) -> Result<Reply, OpError> {
    bounded(source)?;
    slot_view(data, slot)?;
    data.rules
        .set_slot(slot, source)
        .map_err(OpError::bad_request)?;
    rule_reply(data, slot)
}
