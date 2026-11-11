//! Rule priority (owner, 2026-11-11): every slot, value and table states a priority, the highest
//! stands whatever the file's id or the pack's place in the load order, and what it beat is
//! kept. Two definitions of one rule at one priority, an override with other inputs and a losing
//! formula that does not compile each refuse the load.

use crate::common::{base_pack, scratch};
use omnis_data::rules::{Beaten, RuleKind, Standing};
use omnis_data::{Data, LoadReport, load_packs};
use std::path::{Path, PathBuf};

/// A sealed scratch pack `id` that depends on base and holds these rules files, each given as
/// its file name and the entries after its `schema` and `id`.
fn pack(dir: &str, id: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = scratch(dir);
    std::fs::create_dir_all(root.join("data/rules")).unwrap();
    std::fs::write(
        root.join("pack.ron"),
        format!(
            r#"(schema: 1, id: "{id}", version: "0.1.0", name: "{id}", license: "MIT",
            attribution: [], depends: ["base"], entry: None)"#
        ),
    )
    .unwrap();
    for (name, entries) in files {
        std::fs::write(
            root.join(format!("data/rules/{name}.ron")),
            format!(r#"(schema: 1, id: "{id}:rules:{name}", {entries})"#),
        )
        .unwrap();
    }
    omnis_data::seal::seal(&root, None).unwrap();
    root
}

fn load(root: &Path) -> Result<Data, LoadReport> {
    load_packs(&[base_pack().as_path(), root])
}

fn refused(root: &Path) -> String {
    load(root).expect_err("the load is refused").to_string()
}

fn standing<'a>(data: &'a Data, kind: RuleKind, name: &str) -> &'a Standing {
    data.standings
        .get(&(kind, name.to_owned()))
        .unwrap_or_else(|| panic!("no standing for {name}"))
}

const TOTAL: &str = r#"inputs: ["die", "bonus", "proficiency"]"#;

#[test]
fn the_highest_priority_stands_whatever_the_file_id_or_the_load_order() {
    // `aaa:rules:house` sorts before `base:rules:combat`: in id order base would have won.
    let root = pack(
        "priority-high",
        "aaa",
        &[(
            "house",
            &format!(
                r#"slots: {{"attack.total": (priority: 100, {TOTAL}, expr: "die + bonus + proficiency + 1")}},
                values: {{"death_save_dc": (priority: 100, value: 12)}},
                tables: {{"run_dc": (priority: 100, rows: [1, 2, 3, 4])}}"#
            ),
        )],
    );
    let data = load(&root).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(
        data.rules.slot("attack.total").unwrap().source(),
        "die + bonus + proficiency + 1"
    );
    assert_eq!(data.rules.value("death_save_dc"), Some(12));
    assert_eq!(data.rules.table("run_dc"), Some(&[1, 2, 3, 4][..]));
    for (kind, name) in [
        (RuleKind::Slot, "attack.total"),
        (RuleKind::Value, "death_save_dc"),
        (RuleKind::Table, "run_dc"),
    ] {
        assert_eq!(
            standing(&data, kind, name),
            &Standing {
                file: "aaa:rules:house".into(),
                pack: "aaa".into(),
                priority: 100,
                beats: vec![Beaten {
                    file: "base:rules:combat".into(),
                    pack: "base".into(),
                    priority: 0,
                }],
            },
            "{name}"
        );
    }
}

#[test]
fn a_lower_priority_loses_though_its_pack_loads_later_and_its_file_sorts_later() {
    let root = pack(
        "priority-low",
        "zzz",
        &[(
            "house",
            &format!(
                r#"slots: {{"attack.total": (priority: -5, {TOTAL}, expr: "die")}},
                values: {{"death_save_dc": (priority: -5, value: 12)}}"#
            ),
        )],
    );
    let data = load(&root).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(
        data.rules.slot("attack.total").unwrap().source(),
        "die + bonus + proficiency"
    );
    assert_eq!(data.rules.value("death_save_dc"), Some(10));
    let stands = standing(&data, RuleKind::Slot, "attack.total");
    assert_eq!(
        (stands.file.as_str(), stands.pack.as_str(), stands.priority),
        ("base:rules:combat", "base", 0)
    );
    assert_eq!(
        stands.beats,
        [Beaten {
            file: "zzz:rules:house".into(),
            pack: "zzz".into(),
            priority: -5,
        }]
    );
}

#[test]
fn what_a_rule_beat_is_listed_from_the_highest_down() {
    let root = pack(
        "priority-three",
        "mid",
        &[
            (
                "a",
                r#"values: {"death_save_dc": (priority: 100, value: 14)}"#,
            ),
            (
                "b",
                r#"values: {"death_save_dc": (priority: 50, value: 13)}"#,
            ),
            (
                "c",
                r#"values: {"death_save_dc": (priority: -1, value: 9)}"#,
            ),
        ],
    );
    let data = load(&root).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(data.rules.value("death_save_dc"), Some(14));
    let stands = standing(&data, RuleKind::Value, "death_save_dc");
    assert_eq!(stands.file, "mid:rules:a");
    let beaten: Vec<(&str, i32)> = stands
        .beats
        .iter()
        .map(|b| (b.file.as_str(), b.priority))
        .collect();
    assert_eq!(
        beaten,
        [
            ("mid:rules:b", 50),
            ("base:rules:combat", 0),
            ("mid:rules:c", -1)
        ]
    );
}

#[test]
fn two_definitions_at_one_priority_are_refused_with_both_files_named() {
    // Across packs, the later pack does not win a tie.
    let across = pack(
        "priority-tie-across",
        "tie",
        &[(
            "house",
            r#"values: {"death_save_dc": (priority: 0, value: 12)}"#,
        )],
    );
    let report = refused(&across);
    assert!(
        report.contains(
            "value 'death_save_dc': priority 0 in both 'base:rules:combat' and 'tie:rules:house'"
        ),
        "{report}"
    );
    // Inside one pack, and below the definition that would have stood.
    let within = pack(
        "priority-tie-within",
        "tie",
        &[
            ("a", r#"tables: {"run_dc": (priority: -3, rows: [1])}"#),
            ("b", r#"tables: {"run_dc": (priority: -3, rows: [2])}"#),
        ],
    );
    let report = refused(&within);
    assert!(
        report.contains("table 'run_dc': priority -3 in both 'tie:rules:a' and 'tie:rules:b'"),
        "{report}"
    );
}

#[test]
fn an_override_declares_the_inputs_of_the_rule_it_replaces() {
    let root = pack(
        "priority-inputs",
        "house",
        &[(
            "house",
            r#"slots: {"attack.total": (priority: 100, inputs: ["die", "bonus"], expr: "die + bonus")}"#,
        )],
    );
    let report = refused(&root);
    assert!(
        report.contains(
            "slot 'attack.total': inputs [die, bonus] are not the [die, bonus, proficiency] of 'base:rules:combat'"
        ),
        "{report}"
    );
    // The order is part of it: the engine passes them by place.
    let turned = pack(
        "priority-inputs-order",
        "house",
        &[(
            "house",
            r#"slots: {"attack.total": (priority: 100, inputs: ["bonus", "die", "proficiency"], expr: "die")}"#,
        )],
    );
    assert!(refused(&turned).contains("are not the [die, bonus, proficiency]"));
}

#[test]
fn a_losing_formula_that_does_not_compile_is_refused() {
    let root = pack(
        "priority-loser",
        "house",
        &[(
            "house",
            &format!(
                r#"slots: {{"attack.total": (priority: -1, {TOTAL}, expr: "die + nothing")}}"#
            ),
        )],
    );
    let report = refused(&root);
    assert!(report.contains("slot 'attack.total':"), "{report}");
    assert!(report.contains("house.ron"), "{report}");
}

#[test]
fn an_entry_without_a_priority_is_refused() {
    for entries in [
        r#"slots: {"house.rule": (inputs: [], expr: "1")}"#,
        r#"values: {"house_value": (value: 1)}"#,
        r#"tables: {"house_table": (rows: [1])}"#,
        // The shapes before priorities.
        r#"values: {"house_value": 1}"#,
        r#"tables: {"house_table": [1]}"#,
    ] {
        let root = pack("priority-missing", "house", &[("house", entries)]);
        let report = refused(&root);
        assert!(report.contains("house.ron"), "{entries}: {report}");
        assert!(
            report.contains("priority") || report.contains("Expected"),
            "{entries}: {report}"
        );
    }
    let report = refused(&pack(
        "priority-missing",
        "house",
        &[("house", r#"values: {"house_value": (value: 1)}"#)],
    ));
    assert!(report.contains("priority"), "{report}");
}

#[test]
fn the_base_pack_states_every_rule_once_at_priority_zero() {
    let data = load_packs(&[&base_pack()]).unwrap_or_else(|r| panic!("{r}"));
    let count = |kind| data.standings.keys().filter(|(k, _)| *k == kind).count();
    assert_eq!(
        (
            count(RuleKind::Slot),
            count(RuleKind::Value),
            count(RuleKind::Table)
        ),
        (36, 28, 5)
    );
    assert_eq!(count(RuleKind::Slot), data.rules.slot_names().count());
    for ((_, name), stands) in &data.standings {
        assert_eq!(stands.priority, 0, "{name}");
        assert_eq!(stands.pack, "base", "{name}");
        assert!(stands.beats.is_empty(), "{name}");
    }
    assert_eq!(data.rule_files.len(), 10);
    assert!(data.rule_files.contains_key("base:rules:combat"));
}
