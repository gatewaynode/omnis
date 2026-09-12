//! The tool table: one MCP tool per protocol op (ARCHITECTURE.md §9.3), each with an input
//! schema built from the op's argument types.

use crate::schema::{Field, object};
use omnis_cli::omnis_sim::Command;
use serde_json::{Value, json};

/// One tool.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    /// The MCP tool name (letters, digits, underscores).
    pub name: &'static str,
    /// The protocol op it becomes.
    pub op: &'static str,
    /// For the model.
    pub description: &'static str,
    /// Whether the op takes arguments at all; unit ops are sent without `args`.
    pub takes_args: bool,
    /// The input schema.
    pub input_schema: Value,
}

fn tool(name: &'static str, op: &'static str, description: &'static str, fields: &[Field]) -> Tool {
    Tool {
        name,
        op,
        description,
        takes_args: !fields.is_empty(),
        input_schema: object(fields),
    }
}

/// Every tool, in the order `tools/list` reports them.
#[must_use]
pub fn tools() -> Vec<Tool> {
    let map = || {
        Field::new::<Option<String>>(
            "map",
            false,
            "Map id such as test:map:dungeon; the party's map when absent.",
        )
    };
    let path = |what: &'static str| Field::new::<String>("path", true, what);
    vec![
        tool(
            "game_status",
            "game.status",
            "Mode, turn, party clock, position, map, packs, and the world fingerprint.",
            &[],
        ),
        tool(
            "world_query",
            "world.query",
            "Read one value of the world by dotted path, e.g. position.x, clocks.Party(0).elapsed, automap.maps.1.16,15.layers.",
            &[Field::new::<String>("path", true, "The dotted path.")],
        ),
        tool(
            "sim_command",
            "sim.command",
            "Apply one command to the game and return its events.",
            &[Field::new::<Command>("command", true, "The command.")],
        ),
        tool(
            "sim_script",
            "sim.script",
            "Apply commands in order, stopping at the first rejection; returns how many applied and every event.",
            &[Field::new::<Vec<Command>>(
                "commands",
                true,
                "The commands, in order.",
            )],
        ),
        tool(
            "events_tail",
            "events.tail",
            "The last N events the game produced.",
            &[Field::new::<usize>(
                "count",
                false,
                "How many, default 32, at most 4096.",
            )],
        ),
        tool(
            "viewport_get",
            "viewport.get",
            "The first-person viewport model: visible tiles with terrain and edges, nearest first.",
            &[],
        ),
        tool(
            "map_text",
            "map.text",
            "A map as text: walls - |, closed doors = :, open doors _ ', the party as ^ > v <.",
            &[map()],
        ),
        tool(
            "automap_get",
            "automap.get",
            "What the party knows of a map: tiles with terrain, walls, doors, layers, and when they were seen.",
            &[map()],
        ),
        tool(
            "save_write",
            "save.write",
            "Write the save to a relative .ron path.",
            &[path("A relative .ron path without . or .. components.")],
        ),
        tool(
            "save_read",
            "save.read",
            "Replace the world with a save from a relative .ron path.",
            &[
                path("A relative .ron path without . or .. components."),
                Field::new::<bool>("force", false, "Skip the pack fingerprint check."),
            ],
        ),
        tool(
            "pack_reload",
            "pack.reload",
            "Reload the packs from disk, keeping the world where it is.",
            &[],
        ),
        tool(
            "screenshot",
            "screenshot",
            "Save a PNG of the game canvas and return it as an image (game mode only).",
            &[Field::new::<Option<String>>(
                "path",
                false,
                "A relative .png path; default .omnis/screenshot.png.",
            )],
        ),
    ]
}

/// The `tools/list` result.
#[must_use]
pub fn list() -> Value {
    let tools: Vec<Value> = tools()
        .iter()
        .map(|t| json!({"name": t.name, "description": t.description, "inputSchema": t.input_schema}))
        .collect();
    json!({"tools": tools})
}

/// The tool by name.
#[must_use]
pub fn find(name: &str) -> Option<Tool> {
    tools().into_iter().find(|t| t.name == name)
}

/// Something `Schema` must cover for every argument type above.
#[cfg(test)]
mod tests {
    use super::*;
    use omnis_cli::omnis_sim::Op;

    #[test]
    fn every_tool_is_an_op_and_its_schema_matches_the_protocol() {
        let examples = [
            ("game_status", json!({})),
            ("world_query", json!({"path": "turn"})),
            ("sim_command", json!({"command": {"Step": "Forward"}})),
            (
                "sim_script",
                json!({"commands": [{"Turn": "Left"}, "Interact"]}),
            ),
            ("events_tail", json!({"count": 3})),
            ("viewport_get", json!({})),
            ("map_text", json!({"map": "test:map:dungeon"})),
            ("automap_get", json!({})),
            ("save_write", json!({"path": "a.ron"})),
            ("save_read", json!({"path": "a.ron", "force": true})),
            ("pack_reload", json!({})),
            ("screenshot", json!({"path": "shot.png"})),
        ];
        assert_eq!(tools().len(), examples.len());
        for (name, arguments) in examples {
            let tool = find(name).unwrap();
            assert!(
                tool.name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            );
            let request = if tool.takes_args {
                json!({"op": tool.op, "args": arguments})
            } else {
                json!({"op": tool.op})
            };
            serde_json::from_value::<Op>(request.clone())
                .unwrap_or_else(|e| panic!("{name}: {e}: {request}"));
            for field in tool.input_schema["required"].as_array().unwrap() {
                assert!(
                    arguments.get(field.as_str().unwrap()).is_some(),
                    "{name} example lacks {field}"
                );
            }
            for key in arguments.as_object().unwrap().keys() {
                assert!(
                    tool.input_schema["properties"].get(key).is_some(),
                    "{name} schema lacks {key}"
                );
            }
        }
        assert_eq!(list()["tools"].as_array().unwrap().len(), 12);
    }
}
