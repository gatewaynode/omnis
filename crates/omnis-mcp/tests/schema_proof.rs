//! The schema proof (ARCHITECTURE.md §9.3): the hand-written `Command` schema is held against the
//! Rust types in both directions. Every variant, nested variants included, is serialized and
//! must validate and read back; every `oneOf` branch and `enum` value the schema offers must be
//! used by some instance. The instances come from a chain of exhaustive matches ([`next`]), so a
//! new variant fails the build here until it has a successor arm, and then fails the test until
//! the schema has a branch for it. The validator covers the keywords the schema uses and refuses
//! any other, so a new keyword cannot pass unread.

use omnis_cli::omnis_sim::omnis_core::{Direction, Facing, Rotation};
use omnis_cli::omnis_sim::omnis_data::{Ability, Alignment, EquipSlot, Skill};
use omnis_cli::omnis_sim::omnis_rules::Draft;
use omnis_cli::omnis_sim::{
    CombatCommand, Command, DevCommand, EncounterChoice, ItemCommand, PartyCommand, Target,
};
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

fn draft(alignment: Alignment) -> Draft {
    Draft {
        name: "Wren".into(),
        race: "base:race:human".into(),
        class: "base:class:cleric".into(),
        background: "base:background:acolyte".into(),
        alignment,
        scores: [15, 14, 13, 12, 10, 8],
        skills: Skill::ALL.to_vec(),
    }
}

/// The item after `current` in a type's `ALL` list.
fn after<T: Copy + PartialEq>(all: &[T], current: T) -> Option<T> {
    let at = all.iter().position(|one| *one == current)?;
    all.get(at + 1).copied()
}

fn next_party(command: &PartyCommand) -> Option<PartyCommand> {
    Some(match command {
        PartyCommand::Create(made) => match after(&Alignment::ALL, made.alignment) {
            Some(alignment) => PartyCommand::Create(draft(alignment)),
            None => PartyCommand::Reorder { order: vec![1, 0] },
        },
        PartyCommand::Reorder { .. } => PartyCommand::AutoCast {
            member: 0,
            spell: 1,
            on: true,
        },
        PartyCommand::AutoCast { .. } => return None,
    })
}

fn next_combat(command: &CombatCommand) -> Option<CombatCommand> {
    let (spell, item) = (2, 3);
    Some(match command {
        CombatCommand::Attack { .. } => CombatCommand::Cast {
            spell,
            target: Target::Stack(0),
        },
        CombatCommand::Cast { target, .. } => match target {
            Target::Stack(_) => CombatCommand::Cast {
                spell,
                target: Target::Member(1),
            },
            Target::Member(_) => CombatCommand::Use {
                item,
                target: Some(4),
            },
        },
        CombatCommand::Use {
            target: Some(_), ..
        } => CombatCommand::Use { item, target: None },
        CombatCommand::Use { target: None, .. } => CombatCommand::Dodge,
        CombatCommand::Dodge => CombatCommand::Exchange { with: 5 },
        CombatCommand::Exchange { .. } => CombatCommand::Run,
        CombatCommand::Run => return None,
    })
}

fn next_slot(slot: EquipSlot) -> Option<EquipSlot> {
    match slot {
        EquipSlot::MainHand => Some(EquipSlot::OffHand),
        EquipSlot::OffHand => Some(EquipSlot::Ranged),
        EquipSlot::Ranged => Some(EquipSlot::Body),
        EquipSlot::Body => None,
    }
}

fn next_item(command: &ItemCommand) -> Option<ItemCommand> {
    let (member, item, count) = (0, 1, 2);
    Some(match command {
        ItemCommand::Equip { .. } => ItemCommand::Unequip {
            member,
            slot: EquipSlot::MainHand,
        },
        ItemCommand::Unequip { slot, .. } => match next_slot(*slot) {
            Some(slot) => ItemCommand::Unequip { member, slot },
            None => ItemCommand::Give {
                from: 0,
                to: 1,
                item,
                count,
            },
        },
        ItemCommand::Give { .. } => ItemCommand::Stow {
            member,
            item,
            count,
        },
        ItemCommand::Stow { .. } => ItemCommand::Take {
            member,
            item,
            count,
        },
        ItemCommand::Take { .. } => ItemCommand::Use {
            member,
            item,
            target: Some(1),
        },
        ItemCommand::Use {
            target: Some(_), ..
        } => ItemCommand::Use {
            member,
            item,
            target: None,
        },
        ItemCommand::Use { target: None, .. } => return None,
    })
}

fn next_facing(facing: Facing) -> Option<Facing> {
    match facing {
        Facing::North => Some(Facing::East),
        Facing::East => Some(Facing::South),
        Facing::South => Some(Facing::West),
        Facing::West => None,
    }
}

fn teleport(facing: Facing) -> DevCommand {
    DevCommand::Teleport {
        map: "test:map:dungeon".into(),
        x: 3,
        y: 4,
        facing,
    }
}

fn set_score(ability: Ability) -> DevCommand {
    DevCommand::SetScore {
        member: 0,
        ability,
        score: 18,
    }
}

fn give(member: Option<u8>) -> DevCommand {
    DevCommand::GiveItem {
        member,
        item: "base:item:gem".into(),
        count: 2,
    }
}

fn next_dev(command: &DevCommand) -> Option<DevCommand> {
    let member = 0;
    Some(match command {
        DevCommand::GiveItem {
            member: Some(_), ..
        } => give(None),
        DevCommand::GiveItem { member: None, .. } => DevCommand::SetHp { member, hp: -1 },
        DevCommand::SetHp { .. } => DevCommand::SetSpellPoints { member, points: 4 },
        DevCommand::SetSpellPoints { .. } => DevCommand::SetGold { gold: 50 },
        DevCommand::SetGold { .. } => DevCommand::SetFood { food: 7 },
        DevCommand::SetFood { .. } => DevCommand::SetXp { member, xp: 300 },
        DevCommand::SetXp { .. } => set_score(Ability::ALL[0]),
        DevCommand::SetScore { ability, .. } => match after(&Ability::ALL, *ability) {
            Some(ability) => set_score(ability),
            None => DevCommand::SetCondition {
                member,
                condition: "base:condition:poisoned".into(),
                applied: true,
            },
        },
        DevCommand::SetCondition { .. } => DevCommand::SetFlag {
            flag: "test:flag:door".into(),
            value: -3,
        },
        DevCommand::SetFlag { .. } => teleport(Facing::North),
        DevCommand::Teleport { facing, .. } => match next_facing(*facing) {
            Some(facing) => teleport(facing),
            None => DevCommand::SetMonsterHp {
                stack: 0,
                index: 1,
                hp: 0,
            },
        },
        DevCommand::SetMonsterHp { .. } => DevCommand::KillStack { stack: 1 },
        DevCommand::KillStack { .. } => return None,
    })
}

fn next_step(direction: Direction) -> Command {
    match direction {
        Direction::Forward => Command::Step(Direction::Back),
        Direction::Back => Command::Step(Direction::Left),
        Direction::Left => Command::Step(Direction::Right),
        Direction::Right => Command::Turn(Rotation::Left),
    }
}

fn next_turn(rotation: Rotation) -> Command {
    match rotation {
        Rotation::Left => Command::Turn(Rotation::Right),
        Rotation::Right => Command::Turn(Rotation::Around),
        Rotation::Around => Command::Interact,
    }
}

fn next_encounter(choice: EncounterChoice) -> Command {
    match choice {
        EncounterChoice::Attack => Command::Encounter(EncounterChoice::Bribe),
        EncounterChoice::Bribe => Command::Encounter(EncounterChoice::Hide),
        EncounterChoice::Hide => Command::Encounter(EncounterChoice::Run),
        EncounterChoice::Run => Command::Combat(CombatCommand::Attack { stack: 0 }),
    }
}

/// The instance after `command`, from `Step(Forward)` to the last `Dev` edit. Every match here
/// and in the helpers is exhaustive: a new variant gets an arm, and a link from its neighbour.
fn next(command: &Command) -> Option<Command> {
    let cast = |target| Command::Cast {
        caster: 0,
        spell: 1,
        target,
    };
    Some(match command {
        Command::Step(direction) => next_step(*direction),
        Command::Turn(rotation) => next_turn(*rotation),
        Command::Interact => Command::Party(PartyCommand::Create(draft(Alignment::ALL[0]))),
        Command::Party(party) => {
            next_party(party).map_or(Command::Encounter(EncounterChoice::Attack), Command::Party)
        }
        Command::Encounter(choice) => next_encounter(*choice),
        Command::Combat(combat) => {
            next_combat(combat).map_or(cast(Target::Stack(2)), Command::Combat)
        }
        Command::Cast { target, .. } => match target {
            Target::Stack(_) => cast(Target::Member(3)),
            Target::Member(_) => Command::Item(ItemCommand::Equip { member: 0, item: 1 }),
        },
        Command::Item(item) => next_item(item).map_or(Command::Dev(give(Some(0))), Command::Item),
        Command::Dev(dev) => return next_dev(dev).map(Command::Dev),
    })
}

fn instances() -> Vec<Command> {
    let mut all = vec![Command::Step(Direction::Forward)];
    while let Some(following) = next(&all[all.len() - 1]) {
        all.push(following);
    }
    all
}

#[test]
fn every_command_variant_validates_reads_back_and_uses_the_whole_schema() {
    let schema = Command::schema();
    let all = instances();
    assert_eq!(
        all.len(),
        64,
        "4 steps, 3 turns, interact, 11 party, 4 encounter, 8 combat, 2 casts, 10 item, 21 dev"
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
    assert_eq!(every.len(), 94, "oneOf branches and enum values offered");
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
        .push(json!({"type": "string", "enum": ["Rest"]}));
    let (mut used, mut every) = (Used::new(), Used::new());
    for command in instances() {
        let wire = serde_json::to_value(&command).unwrap();
        check(&padded, &wire, "", &mut used).unwrap();
    }
    offered(&padded, "", &mut every);
    let unused: Vec<&String> = every.difference(&used).collect();
    assert_eq!(unused, ["/oneOf/9", "/oneOf/9/enum/\"Rest\""]);
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
