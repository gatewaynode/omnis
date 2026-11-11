//! `data/rules`: formula slots, plain values, and tables (ARCHITECTURE.md §5). Every file adds
//! to one rule set. Each slot, value and table states a priority; where two files define one
//! name the higher priority stands, whatever the files' ids or their packs' load order, and what
//! it beat is kept ([`Standings`]). Two definitions at one priority are refused.

use crate::content::Files;
use crate::error::DataError;
use crate::limits::{MAX_COLLECTION, string_fits};
use omnis_expr::Rules;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// One formula: its priority, its declared inputs and its Rhai expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotDef {
    /// Which definition of this slot stands: the highest among the loaded files.
    pub priority: i32,
    /// The only names the expression may use. An override declares the same, in the same order.
    pub inputs: Vec<String>,
    /// The expression.
    pub expr: String,
}

/// One plain integer and its priority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueDef {
    /// Which definition of this value stands: the highest among the loaded files.
    pub priority: i32,
    /// The integer.
    pub value: i64,
}

/// One integer table and its priority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDef {
    /// Which definition of this table stands: the highest among the loaded files.
    pub priority: i32,
    /// The integers.
    pub rows: Vec<i64>,
}

/// One rules file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesFile {
    /// Schema version.
    pub schema: u32,
    /// `pack:rules:name`.
    pub id: String,
    /// Formula slots by name.
    #[serde(default)]
    pub slots: BTreeMap<String, SlotDef>,
    /// Plain integers by name.
    #[serde(default)]
    pub values: BTreeMap<String, ValueDef>,
    /// Integer tables by name.
    #[serde(default)]
    pub tables: BTreeMap<String, TableDef>,
}

/// What a rule is: slots, values and tables are named apart, so one name may be each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    /// A formula.
    Slot,
    /// A plain integer.
    Value,
    /// An integer table.
    Table,
}

impl RuleKind {
    /// The kind as a rules file and a message write it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            RuleKind::Slot => "slot",
            RuleKind::Value => "value",
            RuleKind::Table => "table",
        }
    }
}

impl fmt::Display for RuleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// A definition of a rule that a higher priority beat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beaten {
    /// The rules file that wrote it (`pack:rules:name`).
    pub file: String,
    /// The pack that supplied that file.
    pub pack: String,
    /// Its priority.
    pub priority: i32,
}

/// The definition of a rule that stands, and every one it beat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    /// The rules file that wrote it (`pack:rules:name`).
    pub file: String,
    /// The pack that supplied that file.
    pub pack: String,
    /// Its priority.
    pub priority: i32,
    /// The definitions it beat, from the highest priority down.
    pub beats: Vec<Beaten>,
}

/// Where every loaded rule came from, by kind and name.
pub type Standings = BTreeMap<(RuleKind, String), Standing>;

/// One definition of a rule in one file, as the merge weighs it.
struct Claim<'a> {
    priority: i32,
    /// The rules file's id.
    id: &'a str,
    /// The rules file's path, for errors.
    path: &'a Path,
    def: Def<'a>,
}

enum Def<'a> {
    Slot(&'a SlotDef),
    Value(i64),
    Table(&'a [i64]),
}

/// One rule set from every rules file: of the definitions of one name the highest priority
/// stands. Pushed as errors: two definitions at one priority, a slot whose inputs are not those
/// of the lowest definition of its name, and any formula that does not compile, standing or not.
/// `sources` gives the pack that supplied each file ([`crate::Data::sources`]).
pub(crate) fn merge(
    files: &Files<RulesFile>,
    sources: &BTreeMap<String, Vec<String>>,
    errors: &mut Vec<DataError>,
) -> (Rules, Standings) {
    let mut claims: BTreeMap<(RuleKind, &str), Vec<Claim<'_>>> = BTreeMap::new();
    for (id, (path, file)) in files {
        let slots = file
            .slots
            .iter()
            .map(|(name, slot)| (RuleKind::Slot, name, slot.priority, Def::Slot(slot)));
        let values = file.values.iter().map(|(name, value)| {
            (
                RuleKind::Value,
                name,
                value.priority,
                Def::Value(value.value),
            )
        });
        let tables = file.tables.iter().map(|(name, table)| {
            (
                RuleKind::Table,
                name,
                table.priority,
                Def::Table(&table.rows),
            )
        });
        for (kind, name, priority, def) in slots.chain(values).chain(tables) {
            claims
                .entry((kind, name.as_str()))
                .or_default()
                .push(Claim {
                    priority,
                    id,
                    path,
                    def,
                });
        }
    }
    let pack = |id: &str| -> String {
        sources
            .get(id)
            .and_then(|packs| packs.last())
            .cloned()
            .unwrap_or_default()
    };
    let mut rules = Rules::new();
    let mut standings = Standings::new();
    for ((kind, name), mut claims) in claims {
        // Rising priority, so the definition that stands is applied last; the sort is stable, so
        // a tie keeps the files in id order for its message.
        claims.sort_by_key(|claim| claim.priority);
        for pair in claims.windows(2) {
            if pair[0].priority == pair[1].priority {
                errors.push(DataError::new(
                    pair[1].path,
                    format!(
                        "{kind} '{name}': priority {} in both '{}' and '{}'; two definitions of \
                         one rule need different priorities",
                        pair[0].priority, pair[0].id, pair[1].id
                    ),
                ));
            }
        }
        for claim in &claims {
            apply(&mut rules, kind, name, claim, &claims[0], errors);
        }
        let mut beaten = claims.iter().rev().map(|claim| Beaten {
            file: claim.id.to_owned(),
            pack: pack(claim.id),
            priority: claim.priority,
        });
        if let Some(top) = beaten.next() {
            standings.insert(
                (kind, name.to_owned()),
                Standing {
                    file: top.file,
                    pack: top.pack,
                    priority: top.priority,
                    beats: beaten.collect(),
                },
            );
        }
    }
    (rules, standings)
}

/// Put one definition into `rules`, over whatever stood under its name. `first` is the lowest
/// definition of the name: a slot's inputs are held to its.
fn apply(
    rules: &mut Rules,
    kind: RuleKind,
    name: &str,
    claim: &Claim<'_>,
    first: &Claim<'_>,
    errors: &mut Vec<DataError>,
) {
    match claim.def {
        Def::Value(value) => rules.insert_value(name, value),
        Def::Table(rows) => rules.insert_table(name, rows.to_vec()),
        Def::Slot(slot) => {
            if let Def::Slot(held) = first.def
                && held.inputs != slot.inputs
            {
                errors.push(DataError::new(
                    claim.path,
                    format!(
                        "{kind} '{name}': inputs [{}] are not the [{}] of '{}'; an override \
                         declares the inputs of the rule it replaces",
                        slot.inputs.join(", "),
                        held.inputs.join(", "),
                        first.id
                    ),
                ));
            }
            if let Err(error) = rules.add_slot(name, slot.inputs.clone(), &slot.expr) {
                errors.push(DataError::new(
                    claim.path,
                    format!("{kind} '{name}': {error}"),
                ));
            }
        }
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl RulesFile {
    /// Self-contained checks; every problem is pushed. Compilation happens at resolution,
    /// once every file is in.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.slots.len() > MAX_COLLECTION
            || self.values.len() > MAX_COLLECTION
            || self.tables.len() > MAX_COLLECTION
            || self.tables.values().any(|t| t.rows.len() > MAX_COLLECTION)
        {
            errors.push(DataError::new(file, "too many entries"));
        }
        for (name, slot) in &self.slots {
            if name.is_empty() || !string_fits(name) {
                errors.push(DataError::new(file, "a slot name is empty or too long"));
            }
            for input in &slot.inputs {
                if !is_identifier(input) {
                    errors.push(DataError::new(
                        file,
                        format!("slot '{name}': input '{input}' is not an identifier"),
                    ));
                }
            }
            if slot.expr.trim().is_empty() || !string_fits(&slot.expr) {
                errors.push(DataError::new(
                    file,
                    format!("slot '{name}': expression is empty or too long"),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_lowercase_snake_case() {
        assert!(is_identifier("level"));
        assert!(is_identifier("cast_mod2"));
        assert!(is_identifier("_x"));
        assert!(!is_identifier(""));
        assert!(!is_identifier("1x"));
        assert!(!is_identifier("Level"));
        assert!(!is_identifier("a-b"));
    }
}
