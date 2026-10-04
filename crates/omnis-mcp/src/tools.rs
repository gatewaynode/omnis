//! The tool table: one MCP tool per protocol op (ARCHITECTURE.md §9.3), each with an input
//! schema built from the op's argument types.

use crate::schema::{Field, object};
use omnis_cli::omnis_sim::Command;
use omnis_cli::omnis_sim::omnis_rules::Draft;
use omnis_cli::omnis_sim::ops::ShotTarget;
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

/// The screenshot tool: the canvas, or everything the window shows.
fn screenshot_tool() -> Tool {
    tool(
        "screenshot",
        "screenshot",
        "Save a PNG of the game and return it as an image (game mode only): the canvas at its internal resolution, or the window, which is the canvas as it is scaled with the window-space interface (the Feathers panels) over it.",
        &[
            Field::new::<Option<String>>(
                "path",
                false,
                "A relative .png path; default .omnis/screenshot.png.",
            ),
            Field::new::<Option<ShotTarget>>("target", false, "canvas (the default) or window."),
        ],
    )
}

/// The screen text tool: every open panel as text.
fn screen_text_tool() -> Tool {
    tool(
        "screen_text",
        "screen.text",
        "The text of every open interface panel (game mode only), one line per control, label or text with its rectangle in window pixels: read a panel such as party creation or the enter/leave confirmation without a picture. The canvas screens (the view, the map, the sheet) are not panels; use screenshot for those.",
        &[],
    )
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
            "Mode, turn, party clock, position, map, packs, the world fingerprint, the service the party is inside, and the groups placed once on its map that are cleared out of how many.",
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
            "Apply one command to the game and return its events: steps and turns, Interact, party changes, encounter choices, fight actions (attack, cast with the action or the bonus action, use an item, dodge, exchange, run, a class feature, end the turn), tactics (switch a member's reactions, any time; declare or remove a reaction, outside a fight), rests and town services, a Cast outside a fight, or a Dev edit (items, hit points, points, gold in copper, conditions, flags, teleport, monster hit points) in a devtools world such as the headless driver.",
            &[Field::new::<Command>("command", true, "The command.")],
        ),
        tool(
            "sim_script",
            "sim.script",
            "Apply commands in order, stopping at the first rejection; returns how many applied and every event. A turn in a fight may take several commands: it ends when the budget has nothing left to pay for, or at EndTurn.",
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
        screenshot_tool(),
        screen_text_tool(),
    ]
}

/// The party and the rules.
fn party_tools() -> Vec<Tool> {
    vec![
        tool(
            "party_get",
            "party.get",
            "The party: members with race, class, level, hit and spell points, hit dice and those left, whether a trainer would grant a level and the spell picks owed, armor class, scores, row, conditions, spells, effects, the kit as rows (the numbers the Item commands take) and the worn slots, plus slots, gold and bank (in copper pieces, 100 to the gold piece), gems, food, the stores as rows, party-wide effects, when the last long rest ended and the minutes before the next may begin; and each member's tactics: the reactions switch, the declared reactions as the rows PutReaction and RemoveReaction take (action, trigger, criteria), and the actions the member could declare with the triggers each answers.",
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
            "The encounter or fight in progress: stacks with hit points, front flag, and reach, the initiative order, whose turn it is, the round, dodges, and loot so far; the acting member's turn budget (actions and bonus actions left), the spells cast this turn, and each spell row's reason it cannot be cast with the action and with the bonus action; reactions left by combatant; hidden members; each member's reactions switch and class features with uses left and why each is blocked; each casting monster's points left per individual and the shields it has up. Fails while exploring.",
            &[],
        ),
        tool(
            "service_get",
            "service.get",
            "The service the party is inside (an inn, tavern, temple, smith, bank, trainer or guild in town): gold, bank and food, and every offer as the exact Service command to send with sim_command, its price in copper (or what a sale pays), and the refusal the rules would give. Looking changes nothing. Fails outside a service.",
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
            (
                "screenshot",
                json!({"path": "shot.png", "target": "window"}),
            ),
            ("party_get", json!({})),
            (
                "party_create",
                json!({"character": {"name": "Brenna", "race": "base:race:human", "class": "base:class:fighter", "background": "base:background:acolyte", "alignment": "NeutralGood", "scores": [15, 14, 13, 12, 10, 8], "skills": ["Athletics", "Perception"]}}),
            ),
            ("combat_get", json!({})),
            ("service_get", json!({})),
            ("screen_text", json!({})),
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
        assert_eq!(list()["tools"].as_array().unwrap().len(), 20);
    }
}
