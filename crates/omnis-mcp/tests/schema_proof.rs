//! The schema proof (ARCHITECTURE.md §9.3): the hand-written `Command` schema is held against the
//! Rust types in both directions. Every variant, nested variants included, is serialized and
//! must validate and read back; every `oneOf` branch and `enum` value the schema offers must be
//! used by some instance. The instances come from a chain of exhaustive matches ([`next`]), so a
//! new variant fails the build here until it has a successor arm, and then fails the test until
//! the schema has a branch for it. The validator covers the keywords the schema uses and refuses
//! any other, so a new keyword cannot pass unread.

mod common;

use common::commands::{draft, instances};
use omnis_cli::omnis_sim::omnis_core::Rotation;
use omnis_cli::omnis_sim::omnis_data::Alignment;
use omnis_cli::omnis_sim::omnis_rules::Draft;
use omnis_cli::omnis_sim::{CombatCommand, Command};
use omnis_mcp::schema::Schema;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// The schema branches and enum values a validation used, as schema paths.
type Used = BTreeSet<String>;

fn type_matches(name: &str, value: &Value) -> bool {
    match name {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "integer" => value.is_i64() || value.is_u64(),
        "null" => value.is_null(),
        _ => false,
    }
}

fn check_type(rule: &Value, value: &Value, at: &str) -> Result<(), String> {
    let names: Vec<&str> = match rule {
        Value::String(one) => vec![one.as_str()],
        Value::Array(many) => many.iter().filter_map(Value::as_str).collect(),
        _ => return Err(format!("{at}: type is neither a name nor a list")),
    };
    if names.iter().any(|name| type_matches(name, value)) {
        Ok(())
    } else {
        Err(format!("{at}: {value} is not {names:?}"))
    }
}

/// `minimum`, `maximum` on integers; `minLength`, `maxLength` on strings; `minItems`, `maxItems`
/// on arrays. A bound on a value of another type (a null member) does not apply.
fn check_bound(key: &str, rule: &Value, value: &Value, at: &str) -> Result<(), String> {
    let bound = rule.as_i64().ok_or(format!("{at}: {key} is no integer"))?;
    let size = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
    let measured = match key {
        "minimum" | "maximum" => value.as_i64(),
        "minLength" | "maxLength" => value.as_str().map(|s| size(s.chars().count())),
        _ => value.as_array().map(|a| size(a.len())),
    };
    let Some(measured) = measured else {
        return Ok(());
    };
    let holds = if key.starts_with("min") {
        measured >= bound
    } else {
        measured <= bound
    };
    if holds {
        Ok(())
    } else {
        Err(format!("{at}: {value} breaks {key} {bound}"))
    }
}

fn check_object(schema: &Value, value: &Value, at: &str, used: &mut Used) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    let none = serde_json::Map::new();
    let properties = schema["properties"].as_object().unwrap_or(&none);
    for name in schema["required"].as_array().into_iter().flatten() {
        let name = name.as_str().unwrap_or_default();
        if !object.contains_key(name) {
            return Err(format!("{at}: required {name} is missing"));
        }
    }
    for (name, member) in object {
        match properties.get(name) {
            Some(rule) => check(rule, member, &format!("{at}/properties/{name}"), used)?,
            None if schema["additionalProperties"] == json!(false) => {
                return Err(format!("{at}: {name} is not a property"));
            }
            None => {}
        }
    }
    Ok(())
}

/// Exactly one branch must hold; its uses are kept, the failed branches' uses are dropped.
fn check_one_of(branches: &Value, value: &Value, at: &str, used: &mut Used) -> Result<(), String> {
    let branches = branches
        .as_array()
        .ok_or(format!("{at}: oneOf is no list"))?;
    let mut matched = Vec::new();
    for (i, branch) in branches.iter().enumerate() {
        let mut scratch = Used::new();
        let path = format!("{at}/oneOf/{i}");
        if check(branch, value, &path, &mut scratch).is_ok() {
            scratch.insert(path);
            matched.push(scratch);
        }
    }
    if matched.len() == 1 {
        used.append(&mut matched[0]);
        Ok(())
    } else {
        Err(format!(
            "{at}: {value} matches {} oneOf branches",
            matched.len()
        ))
    }
}

/// Validate `value` against `schema`, recording what was used. Unknown keywords are errors.
fn check(schema: &Value, value: &Value, at: &str, used: &mut Used) -> Result<(), String> {
    let rules = schema
        .as_object()
        .ok_or(format!("{at}: the schema is no object"))?;
    for (key, rule) in rules {
        match key.as_str() {
            "description" | "properties" | "required" | "additionalProperties" => {}
            "type" => check_type(rule, value, at)?,
            "enum" => {
                let values = rule.as_array().ok_or(format!("{at}: enum is no list"))?;
                if !values.contains(value) {
                    return Err(format!("{at}: {value} is not in the enum"));
                }
                used.insert(format!("{at}/enum/{value}"));
            }
            "oneOf" => check_one_of(rule, value, at, used)?,
            "items" => {
                for member in value.as_array().into_iter().flatten() {
                    check(rule, member, &format!("{at}/items"), used)?;
                }
            }
            "minimum" | "maximum" | "minLength" | "maxLength" | "minItems" | "maxItems" => {
                check_bound(key, rule, value, at)?;
            }
            other => return Err(format!("{at}: the proof does not read the keyword {other}")),
        }
    }
    check_object(schema, value, at, used)
}

/// Every `oneOf` branch and `enum` value a schema offers, as the paths [`check`] records.
fn offered(schema: &Value, at: &str, out: &mut Used) {
    for value in schema["enum"].as_array().into_iter().flatten() {
        out.insert(format!("{at}/enum/{value}"));
    }
    for (i, branch) in schema["oneOf"].as_array().into_iter().flatten().enumerate() {
        let path = format!("{at}/oneOf/{i}");
        offered(branch, &path, out);
        out.insert(path);
    }
    for (name, rule) in schema["properties"].as_object().into_iter().flatten() {
        offered(rule, &format!("{at}/properties/{name}"), out);
    }
    if schema.get("items").is_some() {
        offered(&schema["items"], &format!("{at}/items"), out);
    }
}

#[test]
fn every_command_variant_validates_reads_back_and_uses_the_whole_schema() {
    let schema = Command::schema();
    let all = instances();
    assert_eq!(
        all.len(),
        94,
        "4 steps, 3 turns, interact, 20 party, 4 encounter, 12 combat, 2 casts, 10 item, \
         14 service, 2 rest, 22 dev"
    );
    let mut used = Used::new();
    for command in &all {
        let wire = serde_json::to_value(command).unwrap();
        if let Err(why) = check(&schema, &wire, "", &mut used) {
            panic!("{command:?} as {wire}: {why}");
        }
        let back: Command = serde_json::from_value(wire).unwrap();
        assert_eq!(&back, command);
    }
    let mut every = Used::new();
    offered(&schema, "", &mut every);
    let unused: Vec<&String> = every.difference(&used).collect();
    assert!(unused.is_empty(), "no instance uses {unused:?}");
    assert_eq!(every.len(), 142, "oneOf branches and enum values offered");
}

#[test]
fn a_draft_without_skills_validates_and_deserializes() {
    let mut wire = serde_json::to_value(draft(Alignment::ALL[0])).unwrap();
    wire.as_object_mut().unwrap().remove("skills");
    check(&Draft::schema(), &wire, "", &mut Used::new()).unwrap();
    let back: Draft = serde_json::from_value(wire).unwrap();
    assert!(back.skills.is_empty());
}

/// The `Attack` arm of the fight's schema.
fn attack_arm(schema: &mut Value) -> &mut Value {
    &mut schema["oneOf"][5]["properties"]["Combat"]["oneOf"][0]["properties"]["Attack"]
}

#[test]
fn the_proof_catches_a_schema_that_drifted() {
    let attack = serde_json::to_value(Command::Combat(CombatCommand::Attack { stack: 0 })).unwrap();
    let around = serde_json::to_value(Command::Turn(Rotation::Around)).unwrap();
    let pristine = Command::schema();
    check(&pristine, &attack, "", &mut Used::new()).unwrap();

    // A renamed field.
    let mut renamed = pristine.clone();
    let arm = attack_arm(&mut renamed);
    arm["required"] = json!(["stak"]);
    let properties = arm["properties"].as_object_mut().unwrap();
    let rule = properties.remove("stack").unwrap();
    properties.insert("stak".into(), rule);
    assert!(check(&renamed, &attack, "", &mut Used::new()).is_err());

    // A changed type.
    let mut retyped = pristine.clone();
    attack_arm(&mut retyped)["properties"]["stack"] = json!({"type": "string"});
    assert!(check(&retyped, &attack, "", &mut Used::new()).is_err());

    // An enum value the schema forgot.
    let mut forgetful = pristine.clone();
    forgetful["oneOf"][1]["properties"]["Turn"]["enum"] = json!(["Left", "Right"]);
    assert!(check(&forgetful, &around, "", &mut Used::new()).is_err());

    // A branch no Rust variant stands behind.
    let mut padded = pristine.clone();
    padded["oneOf"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type": "string", "enum": ["Nap"]}));
    let (mut used, mut every) = (Used::new(), Used::new());
    for command in instances() {
        let wire = serde_json::to_value(&command).unwrap();
        check(&padded, &wire, "", &mut used).unwrap();
    }
    offered(&padded, "", &mut every);
    let unused: Vec<&String> = every.difference(&used).collect();
    assert_eq!(unused, ["/oneOf/11", "/oneOf/11/enum/\"Nap\""]);
}

#[test]
fn the_validator_refuses_what_a_schema_forbids() {
    let refuses =
        |schema: Value, value: Value| check(&schema, &value, "", &mut Used::new()).is_err();
    let object = json!({"type": "object", "properties": {"n": {"type": "integer", "minimum": 0}},
        "required": ["n"], "additionalProperties": false});
    assert!(!refuses(object.clone(), json!({"n": 3})));
    assert!(refuses(object.clone(), json!({"n": "3"})));
    assert!(refuses(object.clone(), json!({"n": -1})));
    assert!(refuses(object.clone(), json!({})));
    assert!(refuses(object.clone(), json!({"n": 3, "m": 1})));
    assert!(refuses(object, json!([3])));
    let either = json!({"oneOf": [{"type": "integer"}, {"type": "integer", "minimum": 5}]});
    assert!(!refuses(either.clone(), json!(2)));
    assert!(refuses(either.clone(), json!(7)), "two branches match");
    assert!(refuses(either, json!("seven")), "no branch matches");
    let list = json!({"type": "array", "items": {"type": "string", "maxLength": 2}, "minItems": 1});
    assert!(!refuses(list.clone(), json!(["ab"])));
    assert!(refuses(list.clone(), json!([])));
    assert!(refuses(list, json!(["abc"])));
    assert!(!refuses(
        json!({"type": ["integer", "null"], "minimum": 0}),
        json!(null)
    ));
    assert!(
        refuses(json!({"type": "integer", "pattern": "x"}), json!(1)),
        "an unread keyword"
    );
    assert!(refuses(json!({"type": "integer"}), json!(true)));
}
