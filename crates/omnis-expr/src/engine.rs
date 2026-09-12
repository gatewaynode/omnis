//! Building the Formula-profile engine: a raw engine with integer arithmetic and logic, our
//! own `min`, `max`, `abs`, `clamp`, `floor_div`, and `d`, every statement keyword disabled,
//! and hard limits on work and depth.

use omnis_core::{Dice, Pcg32, RollTrace, StreamName};
use rhai::packages::{ArithmeticPackage, LogicPackage, Package};
use rhai::{Engine, EvalAltResult, OptimizationLevel};
use std::sync::{Arc, Mutex, Once, PoisonError};

/// Work and depth limits of a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Operations one evaluation may perform before it is aborted.
    pub max_operations: u64,
    /// Nesting depth an expression may reach before it is rejected at compile time.
    pub max_expr_depth: usize,
}

impl Limits {
    /// The Formula profile (ARCHITECTURE.md §5.2).
    pub const FORMULA: Limits = Limits {
        max_operations: 10_000,
        max_expr_depth: 32,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Limits::FORMULA
    }
}

/// Keywords a formula may not use. `no_function` and friends remove most of them from the
/// language already; disabling them as symbols keeps the guarantee independent of features.
pub(crate) const DISABLED: [&str; 14] = [
    "fn", "let", "const", "loop", "while", "for", "do", "import", "export", "eval", "throw", "try",
    "return", "switch",
];

/// Functions a formula may call by name. Operators are separate and always allowed.
pub(crate) const ALLOWED_CALLS: [&str; 6] = ["d", "min", "max", "abs", "clamp", "floor_div"];

/// The dice state of the evaluation in progress, shared with the registered `d` function.
#[derive(Debug)]
pub(crate) struct DiceState {
    pub rng: Pcg32,
    pub stream: StreamName,
    pub rolls: Vec<RollTrace>,
}

pub(crate) type DiceSlot = Arc<Mutex<Option<DiceState>>>;

/// Rhai hashes function names with a per-process seed unless told otherwise. Fixing it is
/// hygiene, not correctness: script results never depend on it.
fn seed_hashing() {
    static SEED: Once = Once::new();
    SEED.call_once(|| {
        // Ignored when something else in the process already chose a seed.
        let _ = rhai::config::hashing::set_hashing_seed(Some([
            0x6f6d_6e69_7320_7275,
            0x6c65_7320_7365_6564,
            0x2d31_2d32_3032_362d,
            0x3039_2d31_3200_0001,
        ]));
    });
}

/// Build the engine of one [`Rules`](crate::Rules) value.
pub(crate) fn build(limits: Limits, dice: DiceSlot) -> Engine {
    seed_hashing();
    let mut engine = Engine::new_raw();
    engine.register_global_module(ArithmeticPackage::new().as_shared_module());
    engine.register_global_module(LogicPackage::new().as_shared_module());
    engine.register_fn("clamp", |x: i64, lo: i64, hi: i64| x.max(lo).min(hi));
    engine.register_fn("floor_div", floor_div);
    engine.register_fn("d", move |count: i64, sides: i64| roll(&dice, count, sides));
    for symbol in DISABLED {
        engine.disable_symbol(symbol);
    }
    engine.set_allow_looping(false);
    engine.set_max_operations(limits.max_operations);
    engine.set_max_expr_depths(limits.max_expr_depth);
    engine.set_optimization_level(OptimizationLevel::None);
    engine
}

/// Division rounding toward negative infinity; refuses a zero divisor and the one overflow.
fn floor_div(a: i64, b: i64) -> Result<i64, Box<EvalAltResult>> {
    if b == 0 {
        return Err(format!("Division by zero: floor_div({a}, {b})").into());
    }
    let Some(q) = a.checked_div(b) else {
        return Err(format!("Division overflow: floor_div({a}, {b})").into());
    };
    if a % b != 0 && ((a < 0) != (b < 0)) {
        Ok(q - 1)
    } else {
        Ok(q)
    }
}

/// `d(n, sides)`: n dice of the given sides on the caller's stream, traced.
fn roll(dice: &DiceSlot, count: i64, sides: i64) -> Result<i64, Box<EvalAltResult>> {
    let (Ok(count), Ok(sides)) = (u16::try_from(count), u16::try_from(sides)) else {
        return Err(format!("d({count}, {sides}): count and sides must be 1..=65535").into());
    };
    let mut guard = dice.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(state) = guard.as_mut() else {
        return Err("d() called outside an evaluation".into());
    };
    let trace = Dice::new(count, sides)
        .roll(&mut state.rng, &state.stream)
        .map_err(|e| format!("d({count}, {sides}): {e}"))?;
    let total = i64::from(trace.total);
    state.rolls.push(trace);
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_div_rounds_toward_negative_infinity() {
        assert_eq!(floor_div(7, 2).unwrap(), 3);
        assert_eq!(floor_div(-7, 2).unwrap(), -4);
        assert_eq!(floor_div(7, -2).unwrap(), -4);
        assert_eq!(floor_div(-8, 2).unwrap(), -4);
        assert!(floor_div(1, 0).is_err());
        assert!(floor_div(i64::MIN, -1).is_err());
    }

    #[test]
    fn dice_outside_an_evaluation_are_refused() {
        let slot: DiceSlot = Arc::new(Mutex::new(None));
        assert!(roll(&slot, 1, 6).is_err());
        assert!(roll(&slot, 0, 6).is_err());
        assert!(roll(&slot, 1, -6).is_err());
    }
}
