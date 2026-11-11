//! The tool table: one MCP tool per protocol op (ARCHITECTURE.md §9.3), each with an input
//! schema built from the op's argument types.

use crate::schema::{Field, object};
use omnis_cli::omnis_sim::Command;
use omnis_cli::omnis_sim::omnis_core::Level;
use omnis_cli::omnis_sim::omnis_data::edit::{Brush, Edit, NewMap, Place};
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
    all.extend(editor_tools());
    all.extend(data_tools());
    all
}

/// The language of a `data.*` answer.
fn lang() -> Field {
    Field::new::<Option<String>>(
        "lang",
        false,
        "Language code of the words, one of data_kinds' languages; en when absent. A key with no text in it gives null, never the key.",
    )
}

/// What the loaded packs define: kinds, listings, one definition, text.
fn data_tools() -> Vec<Tool> {
    let filter = |name: &'static str, description: &'static str| {
        Field::new::<Option<String>>(name, false, description)
    };
    vec![
        tool(
            "data_kinds",
            "data.kinds",
            "The kinds of definition the loaded packs hold (item, monster, spell, map, object, ...), each with how many are loaded, and the languages of their text. Call it first to see what there is. Needs no game, like every data tool; these read everything loaded, whatever the party has met (the author's view).",
            &[],
        ),
        tool(
            "data_list",
            "data.list",
            "List loaded definitions, in kind then id order: each row has id, kind, name (a text key, or null), words (the name's text by its key), pack (the pack that supplied it) and replaces (packs it overrode). Every filter given must hold. refers_to answers 'what uses this': everything that names an id anywhere in it (the classes, services and loot tables that name an item; the maps that place a monster). A page holds at most limit rows; pass the reply's cursor back for the next, until it is null.",
            &[
                filter(
                    "kind",
                    "Only this kind, a word from data_kinds such as monster. A kind this engine lacks lists nothing.",
                ),
                filter(
                    "prefix",
                    "Only ids starting with this, such as base: or test:map:.",
                ),
                filter(
                    "matching",
                    "Only definitions whose id or name text holds this, whatever the case, such as rat.",
                ),
                filter(
                    "refers_to",
                    "Only definitions that name this id anywhere in them, such as base:item:dagger.",
                ),
                lang(),
                filter("cursor", "The cursor of the page before, to go on from it."),
                Field::new::<Option<usize>>(
                    "limit",
                    false,
                    "Rows at most: 100 when absent, never over 500.",
                ),
            ],
        ),
        tool(
            "data_get",
            "data.get",
            "One definition by id: kind, def (the definition exactly as its pack file wrote it; its fields follow the pack format, which can gain fields), name, words (the text of every text key in it), refers (every loaded id and flag it names), pack and replaces. A map's def is the whole map and can be large. Any id a view, an event or another definition carries can be passed.",
            &[
                Field::new::<String>("id", true, "pack:kind:name, such as base:monster:goblin."),
                lang(),
            ],
        ),
        tool(
            "data_text",
            "data.text",
            "The text of text keys, as views carry them in name and label fields (base:text:item.mace.name): words maps each known key to its text (null where the language has none), unknown lists keys no language file has. Text may hold {placeholders} the game fills in play.",
            &[
                Field::new::<Vec<String>>("keys", true, "Text keys, at most 256."),
                lang(),
            ],
        ),
    ]
}

/// A root-relative `.ron` path inside the edited pack.
fn pack_path() -> Field {
    Field::new::<String>(
        "path",
        true,
        "Root-relative .ron path inside the edited pack, such as data/maps/town.ron or text/en/maps.ron.",
    )
}

/// A note's key.
fn note_key() -> Field {
    Field::new::<String>(
        "key",
        true,
        "A content id (base:item:mace, test:map:town), or one followed by / and a path inside it: a cell test:map:town/0/3,1 (level, then x,y), a level /level/-1, a placement /chest.1, a terrain /terrain/r.",
    )
}

/// The editor's tools (M5.3): every one a host op over the last pack loaded.
fn editor_tools() -> Vec<Tool> {
    vec![
        tool(
            "editor_status",
            "editor.status",
            "The editor's state: the pack root, the open map's id, file, size, levels and terrains, whether it differs from its file, the undo and redo depths, and the loader's words for anything that would stop it loading (empty when it would load).",
            &[],
        ),
        tool(
            "editor_maps",
            "editor.maps",
            "The edited pack's map files, each with the id it declares, or why it does not read.",
            &[],
        ),
        tool(
            "editor_open",
            "editor.open",
            "Open a map of the edited pack by id. Unsaved changes to the map open before are dropped.",
            &[Field::new::<String>("map", true, "pack:map:name")],
        ),
        tool(
            "editor_new",
            "editor.new",
            "Start a new map in the edited pack: its id must be <pack>:map:<name> and data/maps/<name>.ron must not exist. Every tile is the first terrain, walls round it, the start at (0, 0) facing East. Open and unsaved; name it in a region file before saving.",
            &[Field::new::<NewMap>("map", true, "The map's shape.")],
        ),
        tool(
            "editor_paint",
            "editor.paint",
            "Paint one tile or edge of the open map: a terrain by glyph (a stair up or down needs its rise), an edge (Open, Wall, Door, Ramp; the boundary stays a wall), a height in eighths above the level's base, or Clear for no ground. With stroke true the paint joins the step the last paint opened, so a drag undoes as one.",
            &[
                Field::new::<Level>(
                    "level",
                    false,
                    "The level; 0, the map's own, when left out.",
                ),
                Field::new::<u16>("x", true, "Column."),
                Field::new::<u16>("y", true, "Row."),
                Field::new::<Brush>("brush", true, "What to paint."),
                Field::new::<bool>(
                    "stroke",
                    false,
                    "Join the open stroke; a new undo step when left out.",
                ),
            ],
        ),
        tool(
            "editor_place",
            "editor.place",
            "Put a placement on the open map, replacing one with the same key (a portal, site or pile by its tile; an encounter, object or lore by id; a rest event by terrain and rest kind; the start; the random table), or Remove one by target.",
            &[Field::new::<Place>(
                "place",
                true,
                "The placement, or Remove with its target.",
            )],
        ),
        tool(
            "editor_edit",
            "editor.edit",
            "Change the open map's structure: Resize (the top-left kept, what falls outside dropped), Map fields, LevelAdd (all no ground) / LevelSet / LevelRemove (with everything on it), TerrainAdd / TerrainSet (a rename carries its rest events; a join change while in use is refused) / TerrainRemove (while unused), or a stair's Rise.",
            &[Field::new::<Edit>("edit", true, "The change.")],
        ),
        tool(
            "editor_undo",
            "editor.undo",
            "Back one step of the open map.",
            &[],
        ),
        tool(
            "editor_redo",
            "editor.redo",
            "Forward one step of the open map.",
            &[],
        ),
        tool(
            "editor_save",
            "editor.save",
            "Write the open map to its file and reseal the pack at its next patch version, so the game loads it; refused with the loader's words, and nothing written, while the map would not load. The running world sees the pack after pack_reload.",
            &[],
        ),
        tool(
            "editor_files",
            "editor.files",
            "Every file of the edited pack the loader reads: pack.ron and the .ron files under data/, text/, notes/ and migrations/.",
            &[],
        ),
        tool(
            "editor_read",
            "editor.read",
            "A pack file's text.",
            &[pack_path()],
        ),
        tool(
            "editor_check",
            "editor.check",
            "What the loader would say of a text as the file at a path, by directory (a map is resolved against the loaded packs); nothing is written. Empty means it loads.",
            &[
                pack_path(),
                Field::new::<String>("text", true, "The whole file, at most 4 MiB."),
            ],
        ),
        tool(
            "editor_write",
            "editor.write",
            "Write a text as a pack file, exactly as typed, once the loader's check passes (refused with its words otherwise, nothing written); then reseal the pack at its next patch version, unless the file is a note, which the seal does not cover.",
            &[
                pack_path(),
                Field::new::<String>("text", true, "The whole file, at most 4 MiB."),
            ],
        ),
        tool(
            "editor_note",
            "editor.note",
            "The author's note on a key, if there is one. Notes are the game master's: the game never reads them.",
            &[note_key()],
        ),
        tool(
            "editor_note_set",
            "editor.note_set",
            "Write the author's note on a key (at most 4 KiB), or take it away by leaving text out. Written at once to notes/<map name>.ron for a map's keys, notes/<kind>.ron otherwise; never sealed. A key whose target is not built yet is allowed.",
            &[
                note_key(),
                Field::new::<Option<String>>(
                    "text",
                    false,
                    "The note; leave out to take the note away.",
                ),
            ],
        ),
        tool(
            "editor_notes",
            "editor.notes",
            "The keys that carry notes, those starting with a prefix when given (a map id lists the map's), and the orphans among all notes: keys whose target is nowhere in the loaded packs nor in the open map.",
            &[Field::new::<Option<String>>(
                "prefix",
                false,
                "Only keys starting with this.",
            )],
        ),
    ]
}

/// The screenshot tool: what the game's window shows.
fn screenshot_tool() -> Tool {
    tool(
        "screenshot",
        "screenshot",
        "Save a PNG of the game and return it as an image (game mode only): what the window shows, the view or the fight with every open panel over it.",
        &[
            Field::new::<Option<String>>(
                "path",
                false,
                "A relative .png path; default .omnis/screenshot.png.",
            ),
            Field::new::<Option<ShotTarget>>(
                "target",
                false,
                "canvas (the default) or window; the game captures what its window shows for either.",
            ),
        ],
    )
}

/// The screen text tool: the text on screen.
fn screen_text_tool() -> Tool {
    tool(
        "screen_text",
        "screen.text",
        "The text on the game's screen (game mode only), one line per control, label or text with its rectangle in window pixels: read a panel such as party creation or a town service without a picture. The view's lines and the minimap are not text; use screenshot for those.",
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
    let level = || {
        Field::new::<Option<omnis_cli::omnis_sim::omnis_core::Level>>(
            "level",
            false,
            "Level of the map (0 the ground, below negative, above positive); the party's level on its map, else 0, when absent.",
        )
    };
    let path = |what: &'static str| Field::new::<String>("path", true, what);
    vec![
        tool(
            "game_status",
            "game.status",
            "Mode, turn, party clock, position (its map by string id), packs, the world fingerprint, the service the party is inside, and the groups placed once on its map that are cleared out of how many.",
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
                "How many, default 32, at most 16384.",
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
            "A level of a map as text: walls - |, closed doors = :, open doors _ ', ramps /, portals *, no ground _ on a tile, the party as ^ > v <.",
            &[map(), level()],
        ),
        tool(
            "automap_get",
            "automap.get",
            "What the party knows of a level of a map: tiles with terrain, walls, doors, layers, and when they were seen.",
            &[map(), level()],
        ),
        tool(
            "save_write",
            "save.write",
            "Write a character save (the party as it stands, the world's changes since its base, the party's log) to a relative .ron path, and the world's files under .omnis/worlds/<id>/. Refused if the save would be over 8 MiB or a world file over 4 MiB.",
            &[path("A relative .ron path without . or .. components.")],
        ),
        tool(
            "save_read",
            "save.read",
            "Replace the world with a save from a relative .ron path: its world's bases with the save's changes laid over them (no command is replayed), the party and its log as saved. An older whole-world save is converted.",
            &[
                path("A relative .ron path without . or .. components."),
                Field::new::<bool>(
                    "force",
                    false,
                    "Load a save made with other packs; skips the pack and fingerprint checks.",
                ),
                Field::new::<bool>(
                    "migrate",
                    false,
                    "Apply the packs' migrations when they change the save (save_check's verdict needs_migration lists them).",
                ),
            ],
        ),
        tool(
            "save_check",
            "save.check",
            "What the loaded packs make of a save at a relative .ron path, changing nothing: a verdict (same, content, renamed, unresolved, unnamed), each pack as saved and as loaded, how many names moved number, the names the packs lack with where the save holds them, and the places the loaded maps no longer allow with what loading does with each.",
            &[path("A relative .ron path without . or .. components.")],
        ),
        tool(
            "pack_reload",
            "pack.reload",
            "Reload the packs from disk under the running world, fitted by name through the packs' migrations; answers done with the check, or refuses with each name the packs lack, or with the migrations that would change the world unless migrate, nothing changed.",
            &[Field::new::<bool>(
                "migrate",
                false,
                "Apply the packs' migrations when they change the running world.",
            )],
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
            "The party: members with race, class, level, hit and spell points, hit dice and those left, whether a trainer would grant a level and the spell picks owed, armor class, scores, row, conditions, spells, effects with the caster and minutes left, the sheet (background, alignment, age in years, proficiency, the next level's experience, the hit die, saving throws and proficient skills with their bonus, the casting ability), the kit (each item's id, the `item` the Item commands take) and the worn slots, plus slots, gold and bank (in copper pieces, 100 to the gold piece), gems, food, the stores, party-wide effects, when the last long rest ended and the minutes before the next may begin; and each member's tactics: the reactions switch, the declared reactions with the `entry` PutReaction and RemoveReaction take (action, trigger, criteria, naming spells and items by string id), and the actions the member could declare with the triggers each answers.",
            &[],
        ),
        tool(
            "party_create",
            "party.create",
            "Create a character from a draft and add it to the party; returns the events. Use creation_get for what a draft may be (point buy, races, classes and their skill picks, backgrounds) and party_get to see the result.",
            &[Field::new::<Draft>("character", true, "The draft.")],
        ),
        tool(
            "combat_get",
            "combat.get",
            "The encounter or fight in progress: stacks with hit points, front flag, and reach, the initiative order, whose turn it is, the round, dodges, and loot so far; the acting member's turn budget (actions and bonus actions left), the spells cast this turn, each known spell by id (the `spell` Cast takes) with its reason it cannot be cast with the action and with the bonus action; reactions left by combatant; hidden members; each member's reactions switch and class features by name key (the `feature` Feature takes) with uses left and why each is blocked; each casting monster's points left per individual and the shields it has up; why the acting member cannot attack each stack; the bribe's price before the fight; the choices each feature takes. Fails while exploring.",
            &[],
        ),
        tool(
            "service_get",
            "service.get",
            "The service the party is inside (an inn, tavern, temple, smith, bank, trainer or guild in town): gold, bank and food, and every offer as the exact Service command to send with sim_command, its price in copper (or what a sale pays), and the refusal the rules would give. Looking changes nothing. Fails outside a service.",
            &[],
        ),
        tool(
            "rest_get",
            "rest.get",
            "The camp: each member's hit dice in all and left, how many a short rest may spend and why not, and why a long rest would be refused now. Looking changes nothing.",
            &[],
        ),
        tool(
            "cast_get",
            "cast.get",
            "The spells each member may cast outside a fight: the caster's id and the spell's id (the `caster` and `spell` a Cast command takes), its cost in points, whether it is aimed at a member, and the refusal the rules would give now. Looking changes nothing.",
            &[],
        ),
        tool(
            "objects_get",
            "objects.get",
            "The objects the party reaches (its own cell, and the one it faces across an open edge), each with its id (the `object` an Act command takes), kind, name key, state, capacity and, when it is an open container, its contents; and every interaction its kind offers in that state as the `verb` and `option` Act takes, with its label key, the checks it rolls (skill or ability and difficulty), the minutes an attempt takes and the refusal the rules would give now (a spent one-time interaction stays listed, refused). Also the pile on the party's cell, what PickUp takes from. Hidden objects appear once found. Looking changes nothing.",
            &[],
        ),
        tool(
            "creation_get",
            "creation.get",
            "What a party member's draft may be, as party_create checks it: the party's slots, the longest name in bytes, the score range and the points the six bought scores may cost with each score's cost, every race (its id, the bonus it adds to each score, the skills it grants), class (its id, how many skills a draft picks, `choose`, from its `skill_list`) and background (its id, the skills it grants, which a draft may not pick again), and the nine alignments. Looking changes nothing.",
            &[],
        ),
        tool(
            "time_clocks",
            "time.clocks",
            "Subjective time: the party's age and shared time (minutes lived weighted by the company of each region), the date it believes (year, day, minute, night), every holder's clock (the party and each region met) and every contact (each side's clock when they last met). Looking changes nothing.",
            &[],
        ),
        tool(
            "time_reconcile",
            "time.reconcile",
            "Dev world only, outside a fight: the party meets a region as on entering it. The region catches up by the party's shared time (a settlement, which then sets the party's date) or lived time (a wild region), its coupled regions follow, and the Reconciled events come back. Sent as a dev command, so a replay holds it.",
            &[Field::new::<String>(
                "region",
                true,
                "Region id such as test:region:town.",
            )],
        ),
        tool(
            "rules_list",
            "rules.list",
            "Every rule slot with its inputs and source, the rule values and tables, and in origins where each came from: the rules file, its pack, its priority and the definitions it beat.",
            &[],
        ),
        tool(
            "rules_get",
            "rules.get",
            "One rule slot's inputs and source, with its origin: the rules file that defined it, its pack, its priority and the definitions it beat.",
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
            (
                "save_read",
                json!({"path": "a.ron", "force": true, "migrate": true}),
            ),
            ("save_check", json!({"path": "a.ron"})),
            ("pack_reload", json!({"migrate": true})),
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
            ("rest_get", json!({})),
            ("cast_get", json!({})),
            ("objects_get", json!({})),
            ("creation_get", json!({})),
            ("time_clocks", json!({})),
            ("time_reconcile", json!({"region": "test:region:town"})),
            ("screen_text", json!({})),
            ("rules_list", json!({})),
            ("rules_get", json!({"slot": "spell_points.pool"})),
            (
                "rules_set",
                json!({"slot": "spell_points.pool", "source": "level * 10"}),
            ),
            ("editor_status", json!({})),
            ("editor_maps", json!({})),
            ("editor_open", json!({"map": "test:map:town"})),
            (
                "editor_new",
                json!({"map": {"id": "test:map:hamlet", "name": "test:text:map.town.name", "kind": "Town", "tileset": "test:tileset:outdoor", "width": 4, "height": 3, "wall": {"front": "hedge", "left": "hedge.left", "right": "hedge.right"}, "door": "gate", "terrain": {"glyph": "r", "name": "street", "floor": "road", "passable": true, "visibility_depth": 12, "step_minutes": 1}}}),
            ),
            (
                "editor_paint",
                json!({"level": 0, "x": 1, "y": 1, "brush": {"Edge": {"side": "East", "kind": "Door"}}, "stroke": false}),
            ),
            (
                "editor_place",
                json!({"place": {"Site": {"x": 1, "y": 1, "service": "base:service:inn"}}}),
            ),
            (
                "editor_edit",
                json!({"edit": {"Resize": {"width": 12, "height": 6}}}),
            ),
            ("editor_undo", json!({})),
            ("editor_redo", json!({})),
            ("editor_save", json!({})),
            ("editor_files", json!({})),
            ("editor_read", json!({"path": "data/maps/town.ron"})),
            (
                "editor_check",
                json!({"path": "data/maps/town.ron", "text": "(schema: 1)"}),
            ),
            (
                "editor_write",
                json!({"path": "notes/town.ron", "text": "(schema: 1, entries: {})"}),
            ),
            ("editor_note", json!({"key": "test:map:town/0/1,1"})),
            (
                "editor_note_set",
                json!({"key": "test:map:town/0/1,1", "text": "The inn's front room."}),
            ),
            ("editor_notes", json!({"prefix": "test:map:town"})),
            ("data_kinds", json!({})),
            (
                "data_list",
                json!({"kind": "item", "prefix": "base:", "matching": "sword",
                    "refers_to": "base:item:dagger", "lang": "en", "cursor": "base:item:mace",
                    "limit": 20}),
            ),
            ("data_get", json!({"id": "base:item:mace", "lang": "en"})),
            (
                "data_text",
                json!({"keys": ["base:text:item.mace.name"], "lang": "en"}),
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
        assert_eq!(list()["tools"].as_array().unwrap().len(), 48);
    }
}
