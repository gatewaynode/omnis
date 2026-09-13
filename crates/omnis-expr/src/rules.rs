//! The rule set: named formula slots plus plain values and tables, compiled once and evaluated
//! against inputs and a named RNG stream.

use crate::check;
use crate::engine::{self, DiceSlot, DiceState, Limits};
use crate::error::{CompileError, RuleError};
use core::fmt;
use omnis_core::{Pcg32, RollTrace, StreamName};
use rhai::{AST, Dynamic, Engine, Scope};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

/// A formula input or result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    /// An integer.
    Int(i64),
    /// A boolean.
    Bool(bool),
}

impl Value {
    /// The integer, if it is one.
    #[must_use]
    pub const fn as_int(self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(v),
            Value::Bool(_) => None,
        }
    }

    fn into_dynamic(self) -> Dynamic {
        match self {
            Value::Int(v) => Dynamic::from_int(v),
            Value::Bool(v) => Dynamic::from_bool(v),
        }
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

/// What an evaluation produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The formula's value.
    pub value: Value,
    /// Every die rolled, in order.
    pub rolls: Vec<RollTrace>,
}

/// One compiled formula.
#[derive(Clone)]
pub struct Slot {
    inputs: Vec<String>,
    source: String,
    ast: AST,
}

impl Slot {
    /// The declared input names.
    #[must_use]
    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    /// The formula text.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// A rule set: one engine, its slots, and the plain numbers and tables that ride along.
pub struct Rules {
    limits: Limits,
    engine: Engine,
    dice: DiceSlot,
    slots: BTreeMap<String, Slot>,
    values: BTreeMap<String, i64>,
    tables: BTreeMap<String, Vec<i64>>,
}

impl Rules {
    /// An empty rule set with the Formula profile.
    #[must_use]
    pub fn new() -> Self {
        Rules::with_limits(Limits::FORMULA)
    }

    /// An empty rule set with custom limits (tests, and the later Script profile).
    #[must_use]
    pub fn with_limits(limits: Limits) -> Self {
        let dice: DiceSlot = Arc::new(Mutex::new(None));
        Rules {
            limits,
            engine: engine::build(limits, dice.clone()),
            dice,
            slots: BTreeMap::new(),
            values: BTreeMap::new(),
            tables: BTreeMap::new(),
        }
    }

    /// The limits this set was built with.
    #[must_use]
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    fn compile(&self, inputs: &[String], source: &str) -> Result<AST, CompileError> {
        let ast = self.engine.compile_expression(source).map_err(|e| {
            CompileError::at(e.position().line(), e.position().position(), e.to_string())
        })?;
        check::check(&ast, inputs)?;
        Ok(ast)
    }

    /// Add or replace a slot. Inputs are the only names the formula may use.
    pub fn add_slot(
        &mut self,
        name: &str,
        inputs: Vec<String>,
        source: &str,
    ) -> Result<(), CompileError> {
        if name.is_empty() {
            return Err(CompileError::plain("slot name is empty"));
        }
        let ast = self.compile(&inputs, source)?;
        self.slots.insert(
            name.to_owned(),
            Slot {
                inputs,
                source: source.to_owned(),
                ast,
            },
        );
        Ok(())
    }

    /// Replace a slot's formula in place, keeping its declared inputs (hot swap).
    pub fn set_slot(&mut self, name: &str, source: &str) -> Result<(), CompileError> {
        let Some(slot) = self.slots.get(name) else {
            return Err(CompileError::plain(format!("no slot named '{name}'")));
        };
        let ast = self.compile(&slot.inputs, source)?;
        let slot = self.slots.get_mut(name).expect("checked above");
        slot.source = source.to_owned();
        slot.ast = ast;
        Ok(())
    }

    /// A slot by name.
    #[must_use]
    pub fn slot(&self, name: &str) -> Option<&Slot> {
        self.slots.get(name)
    }

    /// Slot names in order.
    pub fn slot_names(&self) -> impl Iterator<Item = &str> {
        self.slots.keys().map(String::as_str)
    }

    /// Set a plain value.
    pub fn insert_value(&mut self, name: &str, value: i64) {
        self.values.insert(name.to_owned(), value);
    }

    /// A plain value by name.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<i64> {
        self.values.get(name).copied()
    }

    /// The plain values in order.
    #[must_use]
    pub const fn values(&self) -> &BTreeMap<String, i64> {
        &self.values
    }

    /// Set a table.
    pub fn insert_table(&mut self, name: &str, table: Vec<i64>) {
        self.tables.insert(name.to_owned(), table);
    }

    /// A table by name.
    #[must_use]
    pub fn table(&self, name: &str) -> Option<&[i64]> {
        self.tables.get(name).map(Vec::as_slice)
    }

    /// The tables in order.
    #[must_use]
    pub const fn tables(&self) -> &BTreeMap<String, Vec<i64>> {
        &self.tables
    }

    /// Evaluate a slot. Every declared input must be given; `rng` advances by the dice the
    /// formula rolled on `stream`, and those rolls come back traced.
    pub fn eval(
        &self,
        name: &str,
        inputs: &[(&str, Value)],
        rng: &mut Pcg32,
        stream: &StreamName,
    ) -> Result<Outcome, RuleError> {
        let Some(slot) = self.slots.get(name) else {
            return Err(RuleError::new(name, "no such slot"));
        };
        let mut scope = Scope::new();
        for declared in &slot.inputs {
            let Some((_, value)) = inputs.iter().find(|(n, _)| n == declared) else {
                return Err(RuleError::new(
                    name,
                    format!("input '{declared}' not given"),
                ));
            };
            scope.push_constant(declared.clone(), value.into_dynamic());
        }
        self.put_dice(Some(DiceState {
            rng: *rng,
            stream: stream.clone(),
            rolls: Vec::new(),
        }));
        let result = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &slot.ast);
        let state = self.put_dice(None);
        let rolls = match state {
            Some(state) => {
                *rng = state.rng;
                state.rolls
            }
            None => Vec::new(),
        };
        let value = result.map_err(|e| RuleError::new(name, e.to_string()))?;
        let value = if value.is_int() {
            Value::Int(value.as_int().unwrap_or_default())
        } else if value.is_bool() {
            Value::Bool(value.as_bool().unwrap_or_default())
        } else {
            return Err(RuleError::new(
                name,
                format!(
                    "result is {}, not an integer or a boolean",
                    value.type_name()
                ),
            ));
        };
        Ok(Outcome { value, rolls })
    }

    fn put_dice(&self, state: Option<DiceState>) -> Option<DiceState> {
        let mut guard = self.dice.lock().unwrap_or_else(PoisonError::into_inner);
        core::mem::replace(&mut guard, state)
    }
}

impl Default for Rules {
    fn default() -> Self {
        Rules::new()
    }
}

/// Equality is by source: two sets with the same limits, slots, values, and tables behave the
/// same, and that is what pack loading compares.
impl PartialEq for Rules {
    fn eq(&self, other: &Self) -> bool {
        self.limits == other.limits
            && self.slots.len() == other.slots.len()
            && self
                .slots
                .iter()
                .zip(&other.slots)
                .all(|((a, x), (b, y))| a == b && x.inputs == y.inputs && x.source == y.source)
            && self.values == other.values
            && self.tables == other.tables
    }
}

impl Eq for Rules {}

impl Clone for Rules {
    fn clone(&self) -> Self {
        let dice: DiceSlot = Arc::new(Mutex::new(None));
        Rules {
            limits: self.limits,
            engine: engine::build(self.limits, dice.clone()),
            dice,
            slots: self.slots.clone(),
            values: self.values.clone(),
            tables: self.tables.clone(),
        }
    }
}

impl fmt::Debug for Rules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rules")
            .field("limits", &self.limits)
            .field("slots", &self.slots.keys().collect::<Vec<_>>())
            .field("values", &self.values)
            .field("tables", &self.tables.keys().collect::<Vec<_>>())
            .finish()
    }
}
