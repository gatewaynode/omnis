//! A small JSON Schema builder driven by the Rust argument types, so the tool schemas cannot
//! drift from what the protocol deserializes. No `schemars`.

use omnis_cli::omnis_sim::Command;
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

impl Schema for Command {
    fn schema() -> Value {
        json!({
            "description": "One player action: a step relative to the facing, a turn in place, or Interact (use the facing edge, such as a door).",
            "oneOf": [
                {"type": "object", "properties": {"Step": {"type": "string", "enum": ["Forward", "Back", "Left", "Right"]}}, "required": ["Step"], "additionalProperties": false},
                {"type": "object", "properties": {"Turn": {"type": "string", "enum": ["Left", "Right", "Around"]}}, "required": ["Turn"], "additionalProperties": false},
                {"type": "string", "enum": ["Interact"]}
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
            3
        );
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(object(&[])["required"], json!([]));
    }
}
