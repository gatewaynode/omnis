//! A small JSON Schema builder. The tool arguments are built from their Rust types; the
//! `Command` schema is written by hand and held against the Rust types by `tests/schema_proof.rs`
//! (every variant validates and reads back, every branch and enum value is used), so it cannot
//! drift from what the protocol deserializes. No `schemars`.

use omnis_cli::omnis_sim::Command;
use omnis_cli::omnis_sim::omnis_data::{Alignment, Skill};
use omnis_cli::omnis_sim::omnis_rules::Draft;
use serde_json::{Value, json};

/// A type that can describe itself as JSON Schema.
pub trait Schema {
    /// The schema.
    fn schema() -> Value;
}

impl Schema for String {
    fn schema() -> Value {
        json!({"type": "string"})
    }
}

impl Schema for bool {
    fn schema() -> Value {
        json!({"type": "boolean"})
    }
}

impl Schema for usize {
    fn schema() -> Value {
        json!({"type": "integer", "minimum": 0})
    }
}

impl Schema for omnis_cli::omnis_sim::ops::ShotTarget {
    fn schema() -> Value {
        json!({"type": "string", "enum": ["canvas", "window"]})
    }
}

impl<T: Schema> Schema for Option<T> {
    fn schema() -> Value {
        T::schema()
    }
}

impl<T: Schema> Schema for Vec<T> {
    fn schema() -> Value {
        json!({"type": "array", "items": T::schema()})
    }
}

impl Schema for Draft {
    fn schema() -> Value {
        let names = |list: &[&dyn std::fmt::Debug]| -> Vec<String> {
            list.iter().map(|v| format!("{v:?}")).collect()
        };
        let alignments: Vec<&dyn std::fmt::Debug> = Alignment::ALL
            .iter()
            .map(|a| a as &dyn std::fmt::Debug)
            .collect();
        let skills: Vec<&dyn std::fmt::Debug> = Skill::ALL
            .iter()
            .map(|s| s as &dyn std::fmt::Debug)
            .collect();
        json!({
            "description": "A character draft: ids as pack:type:name, six bought scores in SRD order (8..=15, 27 points), and the class's skill picks.",
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1, "maxLength": 32},
                "race": {"type": "string"},
                "class": {"type": "string"},
                "background": {"type": "string"},
                "alignment": {"type": "string", "enum": names(&alignments)},
                "scores": {"type": "array", "items": {"type": "integer", "minimum": 1, "maximum": 30}, "minItems": 6, "maxItems": 6},
                "skills": {"type": "array", "items": {"type": "string", "enum": names(&skills)}}
            },
            "required": ["name", "race", "class", "background", "alignment", "scores"],
            "additionalProperties": false
        })
    }
}

/// A non-negative integer.
fn index() -> Value {
    json!({"type": "integer", "minimum": 0})
}

/// A spell's target: a stack or a member by index.
fn target() -> Value {
    json!({"oneOf": [
        {"type": "object", "properties": {"Stack": index()}, "required": ["Stack"], "additionalProperties": false},
        {"type": "object", "properties": {"Member": index()}, "required": ["Member"], "additionalProperties": false}
    ]})
}

/// One `{Variant: {fields}}` object.
fn variant(name: &str, fields: &[(&str, Value)]) -> Value {
    let properties: serde_json::Map<String, Value> = fields
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect();
    let required: Vec<&str> = fields.iter().map(|(k, _)| *k).collect();
    json!({"type": "object", "properties": {name: {"type": "object", "properties": properties, "required": required, "additionalProperties": false}}, "required": [name], "additionalProperties": false})
}

/// The `Dev` command's variants: debugging edits, accepted only in a devtools world.
fn dev_schema() -> Value {
    let member = ("member", index());
    let abilities = json!({"type": "string", "enum": ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"]});
    let facing = json!({"type": "string", "enum": ["North", "East", "South", "West"]});
    json!({"oneOf": [
        variant("GiveItem", &[("member", json!({"type": ["integer", "null"], "minimum": 0})), ("item", String::schema()), ("count", index())]),
        variant("SetHp", &[member.clone(), ("hp", json!({"type": "integer"}))]),
        variant("SetSpellPoints", &[member.clone(), ("points", index())]),
        variant("SetGold", &[("gold", index())]),
        variant("SetFood", &[("food", index())]),
        variant("SetXp", &[member.clone(), ("xp", index())]),
        variant("SetScore", &[member.clone(), ("ability", abilities), ("score", index())]),
        variant("SetCondition", &[member.clone(), ("condition", String::schema()), ("applied", bool::schema())]),
        variant("SetFlag", &[("flag", String::schema()), ("value", json!({"type": "integer"}))]),
        variant("Teleport", &[("map", String::schema()), ("x", index()), ("y", index()), ("facing", facing)]),
        variant("SetMonsterHp", &[("stack", index()), ("index", index()), ("hp", json!({"type": "integer"}))]),
        variant("KillStack", &[("stack", index())])
    ]})
}

/// A member's slot, or null for the actor.
fn member_or_null() -> Value {
    json!({"type": ["integer", "null"], "minimum": 0})
}

/// The `Item` command's variants: every `item` is a row of the kit or the stores it names.
fn item_schema() -> Value {
    let member = ("member", index());
    let item = ("item", index());
    let count = ("count", json!({"type": "integer", "minimum": 1}));
    let slot = json!({"type": "string", "enum": ["MainHand", "OffHand", "Ranged", "Body"]});
    json!({"oneOf": [
        variant("Equip", &[member.clone(), item.clone()]),
        variant("Unequip", &[member.clone(), ("slot", slot)]),
        variant("Give", &[("from", index()), ("to", index()), item.clone(), count.clone()]),
        variant("Stow", &[member.clone(), item.clone(), count.clone()]),
        variant("Take", &[member.clone(), item.clone(), count]),
        variant("Use", &[member, item, ("target", member_or_null())])
    ]})
}

/// The `Service` command's variants, inside a service only: `member` is a party slot, `Buy`'s
/// `item` a row of the service's stock, `Sell`'s a row of the stores; amounts are copper.
/// `Choose`'s `spell` is a row of the member's class list, `Learn`'s a row of the service's
/// spells.
fn service_schema() -> Value {
    let member = ("member", index());
    let spell = ("spell", index());
    let item = ("item", index());
    let count = ("count", json!({"type": "integer", "minimum": 1}));
    let amount = ("amount", json!({"type": "integer", "minimum": 1}));
    json!({"oneOf": [
        {"type": "string", "enum": ["Leave", "Room", "Rumor"]},
        variant("BuyFood", std::slice::from_ref(&count)),
        variant("Heal", std::slice::from_ref(&member)),
        variant("Cure", std::slice::from_ref(&member)),
        variant("Raise", std::slice::from_ref(&member)),
        variant("Buy", &[item.clone(), count.clone()]),
        variant("Sell", &[item, count]),
        variant("Deposit", std::slice::from_ref(&amount)),
        variant("Withdraw", &[amount]),
        variant("Train", std::slice::from_ref(&member)),
        variant("Choose", &[member.clone(), spell.clone()]),
        variant("Learn", &[member, spell])
    ]})
}

/// A newtype variant `{name: value}`.
fn newtype(name: &str, value: Value) -> Value {
    json!({"type": "object", "properties": {name: value}, "required": [name], "additionalProperties": false})
}

/// The tactics commands (M7c): the reactions switch, any time; a declared reaction put in the
/// default runbook (`at` replaces that entry, `null` appends) or removed, outside a fight. Ids
/// in an action or a predicate are the numbers `party_get` and `world_query` show. `when` is
/// `"Always"` or one object node, `{"All": [..]}`, `{"Any": [..]}` or `{"Is": predicate}`, the
/// simulation checking the whole tree (depth 4, 16 nodes, percentages 0..=100, known ids).
fn tactics_schema() -> Value {
    let member = ("member", index());
    let set = json!({"type": "object", "properties": {
        "name": {"type": "string", "minLength": 1, "maxLength": 32},
        "action": {"oneOf": [
            {"type": "string", "enum": ["Attack"]},
            newtype("Spell", index()),
            newtype("Item", index()),
            newtype("Feature", json!({"type": "string"}))
        ]},
        "trigger": {"type": "string", "enum": ["SpellCast", "Attacked", "MemberAttacked", "MemberWounded", "MemberDying", "EnemyFlees", "EnemyCasts", "OwnTurn"]},
        "when": {"oneOf": [
            {"type": "string", "enum": ["Always"]},
            {"type": "object", "description": "One node: All or Any with a list of trees, or Is with a predicate: MonsterCount {monster, cmp, n}, MonsterShare {monster, cmp, percent}, Hp or SpellPoints {who: Me|Subject, cmp, percent}, HasCondition {who, condition}, Row {who, row: Front|Back}, Round {cmp, n}, or \"WouldChangeOutcome\"; cmp is Lt, Le, Eq, Ge or Gt"}
        ]}
    }, "required": ["name", "action", "trigger", "when"], "additionalProperties": false});
    json!({"oneOf": [
        variant("SetReactions", &[member.clone(), ("on", bool::schema())]),
        variant("PutReaction", &[member.clone(), ("at", member_or_null()), ("set", set)]),
        variant("RemoveReaction", &[member, ("at", index())])
    ]})
}

/// The `Rest` command's variants, outside a service only: `Long` is the night (food, once a
/// day); `Short` is an hour, `dice[i]` the hit dice member `i` spends (missing members none).
fn rest_schema() -> Value {
    let dice = json!({"type": "array", "items": {"type": "integer", "minimum": 0, "maximum": 255}});
    json!({"oneOf": [
        {"type": "string", "enum": ["Long"]},
        variant("Short", &[("dice", dice)])
    ]})
}

impl Schema for Command {
    fn schema() -> Value {
        json!({
            "description": "One player action: a step relative to the facing, a turn in place, Interact (use the facing edge, such as a door), a party change (create, reorder, or Tactics: a member's reactions switch, any time, and declared reactions, outside a fight), an Encounter choice before a fight, a Combat action on the acting member's turn, which takes commands until its budget has nothing left to pay for or EndTurn (attack, cast a known spell by index at a stack or member with the action or the bonus action, use a kit row on a member, dodge, exchange, run, a class feature by its row with its choice, end the turn), a Cast outside a fight (healing, a buff, light, mage hand), an Item command outside a fight (equip, unequip, give, stow, take, use; rows as party_get lists them), a Service command inside a town service (leave, a room at the inn, food and rumors at the tavern, healing, curing and raising at the temple, buying from the smith's stock and selling from the stores, banking copper, a level at the trainer and the spell picks it owes, spells bought at a guild or a temple), a Rest outside a service (Short spends hit dice per member for an hour, Long is the night for food, once a day; either may be ambushed), or a Dev edit in a devtools world. A step onto a service's tile goes inside it; a step off leaves.",
            "oneOf": [
                {"type": "object", "properties": {"Step": {"type": "string", "enum": ["Forward", "Back", "Left", "Right"]}}, "required": ["Step"], "additionalProperties": false},
                {"type": "object", "properties": {"Turn": {"type": "string", "enum": ["Left", "Right", "Around"]}}, "required": ["Turn"], "additionalProperties": false},
                {"type": "string", "enum": ["Interact"]},
                {"type": "object", "properties": {"Party": {"oneOf": [
                    {"type": "object", "properties": {"Create": Draft::schema()}, "required": ["Create"], "additionalProperties": false},
                    {"type": "object", "properties": {"Reorder": {"type": "object", "properties": {"order": {"type": "array", "items": {"type": "integer", "minimum": 0}}}, "required": ["order"], "additionalProperties": false}}, "required": ["Reorder"], "additionalProperties": false},
                    newtype("Tactics", tactics_schema())
                ]}}, "required": ["Party"], "additionalProperties": false},
                {"type": "object", "properties": {"Encounter": {"type": "string", "enum": ["Attack", "Bribe", "Hide", "Run"]}}, "required": ["Encounter"], "additionalProperties": false},
                {"type": "object", "properties": {"Combat": {"oneOf": [
                    {"type": "object", "properties": {"Attack": {"type": "object", "properties": {"stack": {"type": "integer", "minimum": 0}}, "required": ["stack"], "additionalProperties": false}}, "required": ["Attack"], "additionalProperties": false},
                    {"type": "object", "properties": {"Cast": {"type": "object", "properties": {"spell": index(), "target": target(), "pay": {"type": "string", "enum": ["Action", "BonusAction"], "description": "Paid with the action (the default) or, for a spell that allows it, the bonus action"}}, "required": ["spell", "target"], "additionalProperties": false}}, "required": ["Cast"], "additionalProperties": false},
                    variant("Use", &[("item", index()), ("target", member_or_null())]),
                    {"type": "string", "enum": ["Dodge", "Run", "EndTurn"]},
                    {"type": "object", "properties": {"Exchange": {"type": "object", "properties": {"with": {"type": "integer", "minimum": 0}}, "required": ["with"], "additionalProperties": false}}, "required": ["Exchange"], "additionalProperties": false},
                    {"type": "object", "properties": {"Feature": {"type": "object", "properties": {"feature": index(), "choice": {"oneOf": [
                        {"type": "string", "enum": ["None", "Hide"]},
                        {"type": "object", "properties": {"Exchange": {"type": "object", "properties": {"with": {"type": "integer", "minimum": 0}}, "required": ["with"], "additionalProperties": false}}, "required": ["Exchange"], "additionalProperties": false}
                    ]}}, "required": ["feature"], "additionalProperties": false}}, "required": ["Feature"], "additionalProperties": false}
                ]}}, "required": ["Combat"], "additionalProperties": false},
                variant("Cast", &[("caster", index()), ("spell", index()), ("target", target())]),
                {"type": "object", "properties": {"Item": item_schema()}, "required": ["Item"], "additionalProperties": false},
                {"type": "object", "properties": {"Service": service_schema()}, "required": ["Service"], "additionalProperties": false},
                {"type": "object", "properties": {"Rest": rest_schema()}, "required": ["Rest"], "additionalProperties": false},
                {"type": "object", "properties": {"Dev": dev_schema()}, "required": ["Dev"], "additionalProperties": false}
            ]
        })
    }
}

/// One field of an object schema.
pub struct Field {
    /// The property name, as the protocol's serde field.
    pub name: &'static str,
    /// Its schema.
    pub schema: Value,
    /// Whether the caller must supply it.
    pub required: bool,
    /// What it is for.
    pub description: &'static str,
}

impl Field {
    /// A field of type `T`.
    #[must_use]
    pub fn new<T: Schema>(name: &'static str, required: bool, description: &'static str) -> Field {
        Field {
            name,
            schema: T::schema(),
            required,
            description,
        }
    }
}

/// An object schema from its fields; no other properties are allowed.
#[must_use]
pub fn object(fields: &[Field]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for field in fields {
        let mut schema = field.schema.clone();
        if let Some(map) = schema.as_object_mut() {
            map.insert("description".into(), json!(field.description));
        }
        properties.insert(field.name.into(), schema);
        if field.required {
            required.push(json!(field.name));
        }
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_list_their_required_fields() {
        let schema = object(&[
            Field::new::<String>("path", true, "where"),
            Field::new::<bool>("force", false, "skip the check"),
            Field::new::<Vec<Command>>("commands", true, "in order"),
        ]);
        assert_eq!(schema["required"], json!(["path", "commands"]));
        assert_eq!(schema["properties"]["path"]["type"], json!("string"));
        assert_eq!(
            schema["properties"]["force"]["description"],
            json!("skip the check")
        );
        assert_eq!(
            schema["properties"]["commands"]["items"]["oneOf"]
                .as_array()
                .unwrap()
                .len(),
            11,
            "step, turn, interact, party, encounter, combat, cast, item, service, rest, dev"
        );
        assert_eq!(dev_schema()["oneOf"].as_array().unwrap().len(), 12);
        assert_eq!(rest_schema()["oneOf"].as_array().unwrap().len(), 2);
        assert_eq!(service_schema()["oneOf"].as_array().unwrap().len(), 12);
        assert_eq!(item_schema()["oneOf"].as_array().unwrap().len(), 6);
        let combat =
            &schema["properties"]["commands"]["items"]["oneOf"][5]["properties"]["Combat"]["oneOf"];
        assert_eq!(combat.as_array().unwrap().len(), 6);
        assert_eq!(
            combat[1]["properties"]["Cast"]["required"],
            json!(["spell", "target"])
        );
        assert_eq!(
            combat[2]["properties"]["Use"]["properties"]["target"]["type"],
            json!(["integer", "null"])
        );
        let item = &schema["properties"]["commands"]["items"]["oneOf"][7]["properties"]["Item"];
        assert_eq!(
            item["oneOf"][2]["properties"]["Give"]["required"],
            json!(["from", "to", "item", "count"])
        );
        assert_eq!(Draft::schema()["required"].as_array().unwrap().len(), 6);
        assert_eq!(
            Draft::schema()["properties"]["alignment"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            9
        );
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(object(&[])["required"], json!([]));
    }
}
