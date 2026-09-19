//! The tool table: one MCP tool per protocol op (ARCHITECTURE.md §9.3), each with an input
//! schema built from the op's argument types.

use crate::schema::{Field, object};
use omnis_cli::omnis_sim::Command;
use omnis_cli::omnis_sim::omnis_rules::Draft;
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
    let mut all = game_tools();
    all.extend(party_tools());
    all
}

/// The game tools: status, commands, views, saves, packs, screenshot.
fn game_tools() -> Vec<Tool> {
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
            "Apply one command to the game and return its events: steps and turns, Interact, party changes, encounter choices, fight actions (attack, cast, dodge, exchange, run), a Cast outside a fight, or a Dev edit (items, hit points, points, gold, conditions, flags, teleport, monster hit points) in a devtools world such as the headless driver.",
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

/// The party and the rules.
fn party_tools() -> Vec<Tool> {
    vec![
        tool(
            "party_get",
            "party.get",
            "The party: members with race, class, level, hit and spell points, armor class, scores, row, conditions, spells, effects, the kit as rows (the numbers the Item commands take) and the worn slots, plus slots, gold, gems, food, the stores as rows, and party-wide effects.",
            &[],
        ),
        tool(
            "party_create",
            "party.create",
            "Create a character from a draft and add it to the party; returns the events. Use rules_list for the point budget and party_get to see the result.",
            &[Field::new::<Draft>("character", true, "The draft.")],
        ),
        tool(
            "combat_get",
            "combat.get",
            "The encounter or fight in progress: stacks with hit points, front flag, and reach, the initiative order, whose turn it is, the round, dodges, and loot so far. Fails while exploring.",
            &[],
        ),
        tool(
            "rules_list",
            "rules.list",
            "Every rule slot with its inputs and source, plus the rule values and tables.",
            &[],
        ),
        tool(
            "rules_get",
            "rules.get",
            "One rule slot's inputs and source.",
            &[Field::new::<String>(
                "slot",
                true,
                "Slot name such as spell_points.pool.",
            )],
        ),
        tool(
            "rules_set",
            "rules.set",
            "Replace one rule slot's formula in the running game (hot swap); a bad formula is refused with its line and column. Packs on disk are untouched.",
            &[
                Field::new::<String>("slot", true, "Slot name such as spell_points.pool."),
                Field::new::<String>(
                    "source",
                    true,
                    "A Rhai expression over the slot's declared inputs.",
                ),
            ],
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
            ("party_get", json!({})),
            (
                "party_create",
                json!({"character": {"name": "Brenna", "race": "base:race:human", "class": "base:class:fighter", "background": "base:background:acolyte", "alignment": "NeutralGood", "scores": [15, 14, 13, 12, 10, 8], "skills": ["Athletics", "Perception"]}}),
            ),
            ("combat_get", json!({})),
            ("rules_list", json!({})),
            ("rules_get", json!({"slot": "spell_points.pool"})),
            (
                "rules_set",
                json!({"slot": "spell_points.pool", "source": "level * 10"}),
            ),
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
        assert_eq!(list()["tools"].as_array().unwrap().len(), 18);
    }
}
