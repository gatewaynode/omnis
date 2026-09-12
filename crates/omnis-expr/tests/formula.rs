//! The Formula profile as a pack author meets it: what compiles, what is refused, what a
//! formula can do at runtime, and that two hosts agree on every result and every roll.

use omnis_core::{Dice, Pcg32, StreamName};
use omnis_expr::{Limits, Rules, Value};

/// The spell point pool of ARCHITECTURE.md §5.5 (PRD D12).
const POOL: &str =
    "max(level, (if half_caster { level / 2 } else { level }) * cast_mod + other_mental_mods)";

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

fn stream() -> StreamName {
    StreamName::new("party")
}

fn rules_with_pool() -> Rules {
    let mut rules = Rules::new();
    rules
        .add_slot(
            "spell_points.pool",
            names(&["level", "cast_mod", "other_mental_mods", "half_caster"]),
            POOL,
        )
        .unwrap();
    rules
}

fn pool(rules: &Rules, level: i64, cast_mod: i64, other: i64, half: bool) -> i64 {
    let mut rng = Pcg32::for_stream(1, &stream());
    rules
        .eval(
            "spell_points.pool",
            &[
                ("level", Value::Int(level)),
                ("cast_mod", Value::Int(cast_mod)),
                ("other_mental_mods", Value::Int(other)),
                ("half_caster", Value::Bool(half)),
            ],
            &mut rng,
            &stream(),
        )
        .unwrap()
        .value
        .as_int()
        .unwrap()
}

#[test]
fn the_spell_point_formula_reproduces_the_prd_table() {
    let rules = rules_with_pool();
    // PRD §8.3, F3 focused caster: +3 then +4 at 8 and +5 at 16; others +1 and +0.
    assert_eq!(pool(&rules, 1, 3, 1, false), 4);
    assert_eq!(pool(&rules, 5, 3, 1, false), 16);
    assert_eq!(pool(&rules, 10, 4, 1, false), 41);
    assert_eq!(pool(&rules, 20, 5, 1, false), 101);
    // Dump-stat caster at level 1 hits the floor of the character level.
    assert_eq!(pool(&rules, 1, 3, -2, false), 1);
    // Gifted caster.
    assert_eq!(pool(&rules, 5, 3, 4, false), 19);
    // A half caster uses half its level rounded down.
    assert_eq!(pool(&rules, 5, 3, 1, true), 7);
}

#[test]
fn statements_and_escapes_are_rejected_at_compile_time() {
    let mut rules = Rules::new();
    for source in [
        "let x = 1; x",
        "fn f() { 1 } f()",
        "loop { 1 }",
        "while true { 1 }",
        "for i in 1..2 { i }",
        "do { 1 } while false",
        "\"abc\"",
        "'c'",
        "`text ${1}`",
        "import \"x\" as x; 1",
        "eval(\"1\")",
        "throw 1",
        "try { 1 } catch { 2 }",
        "switch 1 { _ => 2 }",
        "print(1)",
        "[1, 2]",
        "#{a: 1}",
        "1.5 + 1",
        "x = 5",
    ] {
        let error = rules
            .add_slot("s", names(&["x"]), source)
            .expect_err(&format!("{source:?} must not compile"));
        assert!(!error.message.is_empty(), "{source:?}");
    }
    assert!(rules.slot("s").is_none(), "a rejected slot is not stored");
}

#[test]
fn unknown_names_are_reported_with_their_position() {
    let mut rules = Rules::new();
    let error = rules
        .add_slot("s", names(&["level"]), "level + bonus")
        .unwrap_err();
    assert_eq!((error.line, error.column), (Some(1), Some(9)));
    assert!(error.message.contains("bonus"), "{error}");
    assert_eq!(error.to_string(), "1:9: unknown input 'bonus'");

    let error = rules
        .add_slot("s", names(&["level"]), "level +\n  sqrt(level)")
        .unwrap_err();
    assert_eq!((error.line, error.column), (Some(2), Some(3)));
    assert_eq!(error.message, "unknown function 'sqrt'");

    let error = rules.add_slot("s", names(&[]), "1 +").unwrap_err();
    assert!(
        error.line.is_some(),
        "a parse error has a position: {error}"
    );
}

#[test]
fn arithmetic_is_checked_and_the_helpers_work() {
    let mut rules = Rules::new();
    let mut rng = Pcg32::for_stream(1, &stream());
    let mut eval = |source: &str| {
        rules.add_slot("s", names(&[]), source).unwrap();
        rules.eval("s", &[], &mut rng, &stream())
    };
    assert!(
        eval("1 / 0")
            .unwrap_err()
            .message
            .contains("Division by zero")
    );
    assert!(
        eval("9223372036854775807 + 1")
            .unwrap_err()
            .message
            .contains("overflow")
    );
    assert_eq!(eval("floor_div(-7, 2)").unwrap().value, Value::Int(-4));
    assert_eq!(eval("-7 / 2").unwrap().value, Value::Int(-3));
    assert_eq!(eval("clamp(50, 0, 10)").unwrap().value, Value::Int(10));
    assert_eq!(
        eval("abs(-3) + min(2, 9) * max(1, 4)").unwrap().value,
        Value::Int(11)
    );
    assert_eq!(eval("3 > 2 && !(1 == 2)").unwrap().value, Value::Bool(true));
    assert_eq!(eval("7 % 3").unwrap().value, Value::Int(1));
}

#[test]
fn the_limits_hold() {
    let mut rules = Rules::with_limits(Limits {
        max_operations: 8,
        max_expr_depth: 32,
    });
    rules
        .add_slot("s", names(&[]), "1+2+3+4+5+6+7+8+9+10+11+12+13+14+15+16")
        .unwrap();
    let mut rng = Pcg32::for_stream(1, &stream());
    let error = rules.eval("s", &[], &mut rng, &stream()).unwrap_err();
    assert!(error.message.contains("operations"), "{error}");

    let mut shallow = Rules::with_limits(Limits {
        max_operations: 10_000,
        max_expr_depth: 4,
    });
    let error = shallow
        .add_slot("s", names(&[]), "1+(1+(1+(1+(1+(1+(1+(1+1)))))))")
        .unwrap_err();
    assert!(error.message.contains("maximum complexity"), "{error}");
    assert_eq!((error.line, error.column), (Some(1), Some(7)));
    assert_eq!(Rules::new().limits(), Limits::FORMULA);
}

#[test]
fn dice_draw_from_the_callers_stream_and_two_hosts_agree() {
    let mut a = Rules::new();
    a.add_slot("hp", names(&["con"]), "d(3, 6) + con").unwrap();
    let b = a.clone();
    assert_eq!(a, b);

    let mut rng_a = Pcg32::for_stream(7, &stream());
    let mut rng_b = Pcg32::for_stream(7, &stream());
    let out_a = a
        .eval("hp", &[("con", Value::Int(2))], &mut rng_a, &stream())
        .unwrap();
    let out_b = b
        .eval("hp", &[("con", Value::Int(2))], &mut rng_b, &stream())
        .unwrap();
    assert_eq!(out_a, out_b);
    assert_eq!(rng_a, rng_b);
    assert_eq!(rng_a.draws(), 3, "three dice, three draws");
    assert_eq!(out_a.rolls.len(), 1);
    assert_eq!(out_a.rolls[0].dice, Dice::new(3, 6));
    assert_eq!(out_a.rolls[0].stream, stream());
    assert_eq!(out_a.value, Value::Int(i64::from(out_a.rolls[0].total) + 2));

    // A different stream gives a different sequence and never touches the first one.
    let mut other = Pcg32::for_stream(7, &StreamName::new("combat"));
    let out_c = a
        .eval(
            "hp",
            &[("con", Value::Int(2))],
            &mut other,
            &StreamName::new("combat"),
        )
        .unwrap();
    assert_eq!(out_c.rolls[0].stream, StreamName::new("combat"));
    assert_ne!(out_c.rolls[0].rolls, out_a.rolls[0].rolls);

    // Bad dice are errors, not draws.
    a.set_slot("hp", "d(0, 6)").unwrap();
    let before = rng_a;
    assert!(
        a.eval("hp", &[("con", Value::Int(0))], &mut rng_a, &stream())
            .is_err()
    );
    assert_eq!(rng_a, before);
}

#[test]
fn hot_swap_replaces_a_formula_and_keeps_the_old_one_on_failure() {
    let mut rules = rules_with_pool();
    assert_eq!(pool(&rules, 5, 3, 1, false), 16);
    rules.set_slot("spell_points.pool", "level * 10").unwrap();
    assert_eq!(pool(&rules, 5, 3, 1, false), 50);
    let error = rules
        .set_slot("spell_points.pool", "level * wisdom")
        .unwrap_err();
    assert_eq!(error.message, "unknown input 'wisdom'");
    assert_eq!(
        pool(&rules, 5, 3, 1, false),
        50,
        "the last good formula stays"
    );
    assert_eq!(
        rules.slot("spell_points.pool").unwrap().source(),
        "level * 10"
    );
    assert!(rules.set_slot("nothing", "1").is_err());
}

#[test]
fn inputs_are_checked_and_values_and_tables_ride_along() {
    let mut rules = rules_with_pool();
    let mut rng = Pcg32::for_stream(1, &stream());
    let error = rules
        .eval(
            "spell_points.pool",
            &[("level", Value::Int(1))],
            &mut rng,
            &stream(),
        )
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "rule 'spell_points.pool': input 'cast_mod' not given"
    );
    assert!(rules.eval("nope", &[], &mut rng, &stream()).is_err());

    let copy = rules.clone();
    rules.insert_value("component_threshold", 5);
    rules.insert_table("point_cost", vec![0, 1, 2, 3, 4, 5, 7, 9]);
    assert_eq!(rules.value("component_threshold"), Some(5));
    assert_eq!(rules.table("point_cost").unwrap()[7], 9);
    assert_ne!(rules, copy, "values are part of equality");
    assert_eq!(
        rules.slot_names().collect::<Vec<_>>(),
        ["spell_points.pool"]
    );
    assert!(format!("{rules:?}").contains("spell_points.pool"));
}
