//! A small JSON Schema builder driven by the Rust argument types, so the tool schemas cannot
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

impl Schema for Command {
    fn schema() -> Value {
        json!({
            "description": "One player action: a step relative to the facing, a turn in place, Interact (use the facing edge, such as a door), or a party change.",
            "oneOf": [
                {"type": "object", "properties": {"Step": {"type": "string", "enum": ["Forward", "Back", "Left", "Right"]}}, "required": ["Step"], "additionalProperties": false},
                {"type": "object", "properties": {"Turn": {"type": "string", "enum": ["Left", "Right", "Around"]}}, "required": ["Turn"], "additionalProperties": false},
                {"type": "string", "enum": ["Interact"]},
                {"type": "object", "properties": {"Party": {"oneOf": [
                    {"type": "object", "properties": {"Create": Draft::schema()}, "required": ["Create"], "additionalProperties": false},
                    {"type": "object", "properties": {"Reorder": {"type": "object", "properties": {"order": {"type": "array", "items": {"type": "integer", "minimum": 0}}}, "required": ["order"], "additionalProperties": false}}, "required": ["Reorder"], "additionalProperties": false}
                ]}}, "required": ["Party"], "additionalProperties": false}
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
            4
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
