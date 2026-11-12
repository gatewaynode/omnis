//! The `rules.*` ops say where each rule came from (owner, 2026-11-11): the file and pack that
//! wrote the definition that stands, its priority, and every definition it beat.

use crate::common;
use omnis_sim::api::{Beaten, Reply, RuleKind, RuleOrigin};
use omnis_sim::omnis_data::rules::Standing;
use omnis_sim::rules_ops::{rule_reply, rules_set, rules_view};

#[test]
fn every_slot_value_and_table_has_an_origin_in_kind_then_name_order() {
    let data = common::data();
    let rules = rules_view(&data);
    assert_eq!(
        rules.origins.len(),
        rules.slots.len() + rules.values.len() + rules.tables.len()
    );
    let of = |kind| -> Vec<&str> {
        (rules.origins.iter())
            .filter(|o| o.kind == kind)
            .map(|o| o.name.as_str())
            .collect()
    };
    let slots: Vec<&str> = rules.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(of(RuleKind::Slot), slots);
    let values: Vec<&str> = rules.values.keys().map(String::as_str).collect();
    assert_eq!(of(RuleKind::Value), values);
    let tables: Vec<&str> = rules.tables.keys().map(String::as_str).collect();
    assert_eq!(of(RuleKind::Table), tables);
    let kinds: Vec<RuleKind> = rules.origins.iter().map(|o| o.kind).collect();
    assert!(kinds.is_sorted(), "slots, then values, then tables");
    let total = (rules.origins.iter())
        .find(|o| o.kind == RuleKind::Slot && o.name == "attack.total")
        .unwrap();
    assert_eq!(
        total,
        &RuleOrigin {
            kind: RuleKind::Slot,
            name: "attack.total".into(),
            file: "base:rules:combat".into(),
            pack: "base".into(),
            priority: 0,
            overrides: vec![],
        }
    );
}

#[test]
fn a_slot_is_given_with_what_it_beat_and_a_hot_swap_keeps_its_origin() {
    let mut data = common::data();
    // As the loader leaves it when a house file outranks the base's.
    let beaten = vec![
        Beaten {
            file: "base:rules:combat".into(),
            pack: "base".into(),
            priority: 0,
        },
        Beaten {
            file: "low:rules:fallback".into(),
            pack: "low".into(),
            priority: -10,
        },
    ];
    data.standings.insert(
        (RuleKind::Slot, "attack.total".into()),
        Standing {
            file: "house:rules:table".into(),
            pack: "house".into(),
            priority: 200,
            beats: beaten.clone(),
        },
    );
    // A value of the same name is another rule: its standing is not the slot's.
    data.standings.insert(
        (RuleKind::Value, "attack.total".into()),
        Standing {
            file: "other:rules:x".into(),
            pack: "other".into(),
            priority: 1,
            beats: vec![],
        },
    );
    let wanted = RuleOrigin {
        kind: RuleKind::Slot,
        name: "attack.total".into(),
        file: "house:rules:table".into(),
        pack: "house".into(),
        priority: 200,
        overrides: beaten,
    };
    let Ok(Reply::Rule { rule, origin }) = rule_reply(&data, "attack.total") else {
        panic!("not a rule")
    };
    assert_eq!(rule.name, "attack.total");
    assert_eq!(origin.as_ref(), Some(&wanted));
    let Ok(Reply::Rule { rule, origin }) = rules_set(&mut data, "attack.total", "die") else {
        panic!("not a rule")
    };
    assert_eq!(rule.source, "die");
    assert_eq!(origin, Some(wanted));
    // A slot no file wrote has none.
    data.standings
        .remove(&(RuleKind::Slot, "attack.hit".into()));
    assert!(matches!(
        rule_reply(&data, "attack.hit"),
        Ok(Reply::Rule { origin: None, .. })
    ));
}
