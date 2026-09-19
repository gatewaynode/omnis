# Editor v1 on `bevy_egui`, then objects

Saved 2026-09-19. Planned on 2026-09-13 as M5 and deferred by the owner on 2026-09-19 ("the editor can wait"); it now sits in the TODO's Phases 2–5 block ahead of the procgen panel that extends it. Dates and version numbers below are as of the planning day; re-check the dependency table (30-day rule, Socket scores) and the code facts (function lengths, file lines, `Title::ITEMS`) before starting.

## Context
M0–M4 are merged and the wide canvas is accepted on branch `m5-tasks` (pushed at `024772c`; CI runs on
pull requests only, so the branch's run starts with the PR; main is green). M5 is the first milestone of the
editor, which PRD D4 makes the tool every later map is built with. Owner decisions 2026-09-13:
- **Toolkit**: `bevy_egui` `=0.41.1` (published 2026-07-18, egui 0.35, targets Bevy 0.19). 0.42.0 is 28 days
  old, resolves egui 0.36.2 (5 days old) and harfrust 0.13.3 (19 days) unless pinned, and scores identically
  on Socket (every crate 90–100; `egui` supply-chain 35 at 0.35.0, 0.36.0, 0.36.1 and 0.36.2 alike, so a
  package-level heuristic, reason not retrievable by tooling). The whole 0.42.0 tree clears the 30-day rule
  on **2026-10-08**; noted as a one-line bump then. Features `render, manage_clipboard, default_fonts` only
  (no `bevy_ui`, `picking`, `open_url`). Exactly one new duplicate, `harfrust` 0.6.2 (Bevy's parley) beside
  0.7.0 (epaint), unavoidable at any Bevy 0.19 version: allow-listed with an ARCH §13 exception.
- **Data tables and text keys**: one validated RON editor over the pack's files (forms are a horizon).
- **Dev-socket ops** `editor.status/open/paint/place/save`, mirrored as MCP tools, ship with the editor.
- **Objects**: a real schema (many per tile, look, position, attributes, interactivity points, migration
  path). Sequenced **after** the editor core as **M5b** in the same milestone; the schema below is reviewed
  with this plan.

Done when (the TODO's editor item): the M1 test maps (`packs/test/data/maps/dungeon.ron`, `meadow.ron`) are re-authored
in the editor and every later map is built with it.

## Verified facts that shape the design
- `omnis-app` may not import `omnis-data`/`omnis-core` (Sentrux boundary); it reaches them via
  `omnis_sim::omnis_data`. `omnis-sim` is `#![no_std]`. `omnis-cli`'s `Headless` must reach the editor, so
  the document model lives in **`omnis-data`** (the one crate allowed file I/O; integers and BTreeMap only).
- The writer exists: `omnis-data/src/ron_io.rs` (`to_string` with `PrettyConfig` `struct_names(false)`,
  `write_ron`, `from_str`, `read_ron`, `read_text` refusing symlinks). Round-trip tests exist
  (`tests/round_trip.rs`). **`PrettyConfig` must not change**: `World::fingerprint` hashes pretty RON.
- `MapDef` (`map.rs`): text grid `layout` of `2h+1` rows × `2w+1` chars; `terrains` (visibility depth is a
  *terrain* field), `wall`, `door`, `door_open`, `portals`, `encounters`, `random`; `cells()` and
  `validate()` collect `DataError`s; the loader refuses a whole pack on any error and clears `maps`.
- Rewriting a `packs/test` file changes its fingerprint → both golden replays
  (`crates/omnis-sim/tests/replays/{walk,fight}.ron`) rebaseline in the same commit
  (`cargo test -p omnis-sim rebaseline -- --ignored`). Save fixtures load with `force`.
- `AppState { Boot, MainMenu, Playing }`; `Where::screen()`/`Active` exhaustive; `combat_keys`,
  `menu_keys`, `click_keys` key on `Active`, so an `Active::Editor` arm falls into their no-op branches;
  `map_keys`/`map_pad` are gated on `Explore`; `ui::hit` finds no widgets on an empty frame.
- `pixel.rs` spawns `InnerCamera` (order −1, `RenderTarget::Image`, layer 0) **before** `OuterCamera`
  (order 0, window, layer 1). `bevy_egui` binds its primary context to the *first* camera unless
  `EguiGlobalSettings.auto_create_primary_context = false` (PreStartup); `PrimaryEguiContext` must sit on
  `OuterCamera`; UI systems run in `EguiPrimaryContextPass`; `native_pixels_per_point` = the camera target's
  scale factor (the true window factor; no override needed; zoom stays 1.0).
- egui draws outside the canvas: not in `--screenshot-canvas` nor the MCP `screenshot`; window captures are
  black on the owner's machine. Verification of views is by Bevy-free model tests, bare-`egui::Context`
  smoke runs, and the socket ops; the visual acceptance is the owner's run.
- No teleport command; `Replay` runs through `World::new`, so a `World::new_at(start)` for playtest leaves
  replays untouched. `socket::handle`'s `PackReload` reloads every root, swaps `PackData`, writes
  `WorldReplaced` (factor it into `sim::reload_packs`). The socket refuses every op without a `SimWorld`;
  editor ops must answer before that check.
- Sentrux: fn ≤ 100 lines incl. tests (`plan::viewport` = 100, `parse_args` 91, `socket::serve` 72,
  `menu_keys` 75, `loader::resolve` 82, `apply::r#move` 72), `max_cycles 0`, files ≲ 1000
  (`ops.rs` 663, `socket.rs` 484). `Title::ITEMS` is 3 items; a fourth fits the box.
- `scripts/check-duplicates.sh` diffs `cargo tree --duplicates` against `scripts/duplicates.allow`.

## Design

### A. `omnis_data::edit` (Bevy-free document model; `crates/omnis-data/src/edit/`)
- `grid.rs`: `EdgeKind { Open, Wall, Door }`; `Grid { width, height, tiles: Vec<u8>, h_edges, v_edges }`
  (`(h+1)·w` and `h·(w+1)` entries: a shared edge is one cell by construction). `blank(w,h)` (terrain 0,
  boundary walls), `from_def(def, file, errors)` via `MapDef::cells`, `layout(&[Terrain]) -> Vec<String>`,
  `tile/set_tile`, `edge/set_edge` (boundary only `Wall`), `resize` (keeps top-left, re-walls the boundary).
  Round trip tested both ways on both real maps byte for byte.
- `document.rs`: `Document { file, def (layout empty), grid, dirty, undo, redo }`, `UNDO_CAP 100`
  snapshots (32×32 ≈ 4 KB, 256×256 ≈ 200 KB each; a test asserts the size), `begin_stroke/end_stroke`
  coalescing; ops `paint_tile(x,y,glyph)`, `paint_edge(x,y,side,kind)`, `resize`, `set_start`,
  `add/remove_portal`, `add/remove_encounter`, `set_random`, `add/set/remove_terrain` (remove refused
  while in use), `clear_tile`, `undo/redo`, `validate() -> Vec<DataError>` (`MapDef::validate` + `cells`),
  `save()` refused when invalid, else `write_ron`. `EditError { Outside, UnknownGlyph, BoundaryEdge,
  TerrainInUse, Occupied, TooMany }`. Object ops arrive in M5b.
- `session.rs` + `files.rs`: `Session { root, document }`: `maps()` (`data/maps/*.ron` with ids or parse
  errors), `open(id)` (by id, never a path), `new_map(file, def)`, `status() -> Status { root, map, file,
  width, height, dirty, errors }`, `files()` (`pack.ron`, `data/**`, `text/**` root-relative), `read(rel)`
  (`check_asset_path(rel, ["ron"])` + `read_text`), `check(rel, text)` parses by directory (`data/maps` →
  `MapDef` + validate + cells; `data/tiles` → `Tileset`; races/classes/backgrounds/items/conditions/
  spells/monsters/rules → their types + validate; `text/<lang>` → `TextFile`; `pack.ron` → manifest; unknown
  dirs → syntax only via `ron::Value`), `write(rel, text)` = check then write **as typed** (comments
  survive; "Reformat" is a horizon). Needs `loader::HasSchema` `pub(crate)`.
- Serde types for the ops: `Brush { Tile { glyph }, Edge { side, kind } }`, `Placement { Start{x,y,facing},
  Portal(Portal), Encounter(FixedEncounter), Random(Option<RandomEncounters>), Clear{x,y} }` (+ `Object` in
  M5b), `Status`.

### B. Sim and protocol
- `World::new_at(data, seed, settings, start: Position)`; `new` = `new_at` at the entry;
  `NewGameError::BadStart(Position)` (unknown map, outside, impassable).
- `ops.rs`: `Op::{EditorStatus, EditorOpen{map}, EditorPaint{x,y,brush}, EditorPlace{placement},
  EditorSave}`, all `is_host`; `Reply::Editor { editor: Status }` inserted before `Value`; `editor.save` →
  `Reply::Written`; `OpError::Invalid { errors: Vec<String> }`. Glue `ops/editor.rs`:
  `handle(&mut Session, &Op) -> Option<Result<Reply, OpError>>`, `no_std`-clean (`&str` only).
- `Headless` gains `editor: Session` on the last pack root; `schema.rs` dumps the five ops by example.
- MCP: `editor_status/open/paint/place/save`; `Schema` for `u16`, `char`, `Brush`, `Placement`; tool count
  18 → 23 (tests in two places).

### C. App
- `AppState::Editor` (sibling of `MainMenu`/`Playing`); `Active::Editor`; `menu_for` arm with an empty
  help line; `Title::ITEMS = ["New game", "Load quick save", "Editor", "Quit"]`, `TitleAction::Editor`
  (a `Notice` "this build has no editor" when the `editor` feature is off; the Bevy-free menu stays
  feature-free). `--editor [map-id]` (`AppConfig.editor: Option<EditorLaunch>`; `parse_args` split first:
  `parse_dev_flag`). `sim::boot`: with `config.editor` and no autostart → `Editor`; a pack load failure
  keeps the app running with a `Notice` so the RON view can fix it (playtest refused until it loads).
- `editor.rs` `EditorPlugin` (always compiled): `EditorSession` resource from `config.packs.last()`,
  `ReturnTo(AppState)`, `Playtest { x, y, facing }` message, the `--editor` map opened on enter, canvas
  sprite hidden on `OnEnter(Editor)` / shown on exit (`Option<Single<..>>`, headless-safe).
  `socket::serve` split into `serve` + `answer`; editor ops answered via `Option<ResMut<EditorSession>>`
  before the "no game is running" check.
- `editor/ui.rs` `EditorUiPlugin` (feature `editor`, default on; `bevy_egui` optional dep):
  `EguiPlugin::default()`, PreStartup `auto_create_primary_context = false`, PostStartup
  `PrimaryEguiContext` onto the `OuterCamera` entity (keeps `pixel.rs` free of egui), views in
  `EguiPrimaryContextPass` `run_if(in_state(AppState::Editor))`. `cargo clippy -p omnis-app
  --no-default-features --lib` stays green.
- Views (`editor/model.rs` Bevy-free `Tool`, `View { zoom, pan }`, `hit`, `apply`, stroke coalescing;
  `canvas.rs` painter + interaction; `views.rs` tool panel + status strip; `props.rs` terrain form incl.
  `visibility_depth`, portal/encounter forms, random table; `dialogs.rs` open/new/unsaved guard;
  `ron_view.rs` file tree + monospace `TextEdit` + error list). Menu bar: File (Open, New, Save, Revert),
  Playtest at cursor, View (Map | RON files), Back to title. Shortcuts from `ctx.input` (Cmd/Ctrl Z, Y, S).
- Playtest: save if dirty (refused with errors if invalid) → `sim::reload_packs` → swap `PackData` →
  `World::new_at` at the cursor → party of four `dev::recruit`s under `devtools` (M4 measured two members
  at 94 % wipes), else empty → `SimWorld`, `StartIn(Explore)`, `ReturnTo(Editor)`, `WorldReplaced`,
  `Playing`. `leave_game` returns to `ReturnTo` when set; the pause item reads "Quit to editor".

### D. Re-authoring the M1 maps (the done-when)
`crates/omnis-data/tests/edit_authoring.rs`: `author_dungeon()` from a blank 24×24 with the two terrains
builds the sixteen rooms (`room(doc, x0, y0, 6, 6)`), doors, pillars, portal, three encounters, random
table; asserts `doc.def() == data.maps[dungeon].def` including the layout text; `author_meadow()` likewise
(road, pond, tree clumps). An `#[ignore]` `rewrite_test_maps` writes both through `Document::save`; that
commit rebaselines both replays. Comments in the map files are lost (verbose pretty RON); the M4 owner note
moves to the TODO. Owner acceptance: open both maps in the running editor, edit, save, playtest from a tile.

### E. M5b: objects (after step 15; schema reviewed with this plan)
Kinds in `data/objects/*.ron` (`pack:object:name`), placements on the map. Chosen: string placement ids
(`chest.1`, editor-generated, renameable) keyed in the save; Interact addresses the tile ahead then the
party's tile; own-tile objects are used but not drawn in v1; a named surface must exist in the tileset
(load error), `surface: None` opts into a placeholder box; v1 takes the first available interaction
(`Command::InteractWith { object, verb }` is the M6/M7 hook); object art is **baked** as billboards
(uniform scale to `height_tenths` at `z = d + 0.5`, no shear, pink colour-keyed from the OpenRTP upper-layer
columns) so the tileset stays the whole art contract.
```ron
// data/objects/chest.ron
( schema: 1, id: "test:object:chest", name: "test:text:object.chest.name",
  look: (surface: Some("chest"), height_tenths: 5, width_tenths: 6, placeholder: (140, 90, 40)),
  initial_state: "closed",
  states: { "closed": (), "open": (surface: Some("chest.open")) },
  attributes: (blocks_movement: true, blocks_sight: false, size: Small, weight_tenths: 250,
               material: Wood, capacity: 8, light: 0, noise: 0, smell: 0, value_cp: 500,
               tags: ["container", "furniture", "lootable"]),
  interactions: [
    (verb: Open, when_state: Some("closed"), effects: [SetState("open"), Message("test:text:object.chest.opened")]),
    (verb: Search, when_state: Some("open"), requires: [Skill(skill: Investigation, dc: 10)],
     effects: [GiveGold(25), Message("test:text:object.chest.gold")],
     failure: Some("test:text:object.chest.empty"), once: true),
    (verb: Close, when_state: Some("open"), effects: [SetState("closed")]),
  ] )
// in a map: objects: [ (id: "chest.1", kind: "test:object:chest", x: 4, y: 2, pos: (0, -3), facing: Some(South)),
//                      (id: "fern.1", kind: "test:object:fern_cluster", x: 4, y: 2, pos: (-3, 3)) ]
```
Types (`omnis-data/src/object.rs`): `Verb { Use, Open, Close, Search, Examine, Take, Push, Pull, Break,
Read, Drink, Light, Extinguish }`, `Material`, `Requirement { Skill{skill,dc}, Ability{ability,dc},
Item{id,consume}, Flag{name,min}, School(School), Rule{slot} }`, `Effect { SetState, SetFlag, Message,
GiveGold, GiveItem, Remove }`, `Interaction { verb, label, when_state, requires, effects, failure, once }`,
`Look { surface, height_tenths, width_tenths, placeholder }`, `StateDef { surface, blocks_movement,
blocks_sight }` (overrides), `Attributes` (all integers, `tags: BTreeSet<String>`), `ObjectKind { schema,
id, name, look, initial_state, states: BTreeMap, attributes, interactions }`, `Placement { id, kind, x, y,
pos: (i8,i8) tenths (east, south) in −5..=5, facing, state, hidden: Option<u8>, name }`, resolved twins,
`ObjectKindId`, `Registry.objects`, `Data.objects`, `MapDef.objects` `#[serde(default)]` (no schema
bump), `MapData.objects` + `objects_at`. Limits: `MAX_OBJECTS_PER_TILE 16`, `MAX_OBJECTS_PER_MAP 4096`,
`MAX_STATES 16`, `MAX_INTERACTIONS 16`, `MAX_TAGS 32`. Validation: kind exists, surfaces exist with
`SlotKind::Object`, placements inside, unique ids `[a-z0-9_.-]+`, counts, text keys, rule slots' inputs ⊆
`RULE_INPUTS`; bad-pack corpus rows. Save: `MapState.objects: BTreeMap<String, ObjectState { state,
removed, used: BTreeSet<u8>, noticed }>`, `SAVE_SCHEMA 3 → 4` (trivial `v3_to_v4`; orphaned ids dropped).
Sim v1: blocking (`BlockReason::Object`), opacity (`map.opaque_at`), Interact → `objects::interact`
(first eligible interaction; `Skill` rolls `stats::check` with the best member on the `party` stream,
`Event::Check` with `CheckKind::Skill/Ability`; effects in order; `Event::Object { map, index, kind, verb,
success, messages }`, `Event::ObjectState`). Schema-only in v1 (stated in docs): `School`, `hidden`,
`capacity`, weight/light/noise/smell/material/size/value/facing/tags, verb choice. Draw: split
`plan::viewport` (horizon band out) first, then `objects::row(...)` per depth after the blocks pass
(baked slot shifted by the rotated sub-tile offset; placeholder box otherwise). Bake: `BakeSpec.color_key`,
`SurfaceSpec.size`/`height_tenths`, Object arm in `slot_positions` and `render` (`billboard`). Migration
pattern: `omnis-data/src/migrate.rs` with frozen old shapes and `map_v1_to_v2`, `read_checked` reads a
`Header { schema }` first; fields are only ever added with defaults, renames bump `SCHEMA`.
Editor: per-tile object list (add from kinds, remove, reorder, 3×3 anchor pad + fine `pos`, facing, state,
hidden DC, name key), `Placement::Object` op, kind files through the RON view. Size ≈ 1.5 k lines.

## Steps (one commit each, green on its own; stage by name; gate with `&&`; never push)
`V` = fresh `cargo metadata` when manifests change; `cargo fmt --all --check && cargo clippy --workspace
--all-targets --locked -- -D warnings && cargo clippy -p omnis-app --no-default-features --lib --locked
-- -D warnings && cargo test --workspace --locked && scripts/lint-sim.sh --self-test && scripts/lint-sim.sh
&& scripts/check-duplicates.sh`; Sentrux `check_rules` on `crates/`; awk long-fn check.
1. **data: grid and document** (`edit.rs`, `edit/grid.rs`, `edit/document.rs`, tests on both real maps:
   layout round trip, cells equality, paint/undo/redo, boundary and terrain-in-use refusals, resize drops,
   save refused when invalid, saved scratch pack loads equal, snapshot size under cap). ~900 lines.
2. **data: session and files** (`session.rs`, `files.rs`, `HasSchema` pub(crate); tests: maps of
   `packs/test`, `files()`, `check` empty on every real file, a broken map text yields the loader's
   messages, `write` refuses `../`, absolute, `.txt`; written scratch pack loads). ~600.
3. **sim: `World::new_at`** (+ tests: start at (3, 8) faces the rats; `BadStart`; `new == new_at`). ~60.
4. **sim: editor ops** (`ops.rs` variants/`Reply::Editor`/`OpError::Invalid`, `ops/editor.rs`, serde
   `Brush`/`Placement`/`Status`; tests: open a scratch copy of the dungeon, paint, place, status errors,
   save, `dispatch` → `HostOnly`, JSON round trips). ~400.
5. **cli: headless editor + schema dump** (+ tests). ~150.
6. **mcp: editor tools** (`tools.rs`, `schema.rs`, bridge tests, counts 23). ~200.
7. **app: editor state and core plugin** (`AppState::Editor`, `Active::Editor`, title item, `--editor`,
   `parse_dev_flag` split, `editor.rs`, `sim::reload_packs`, `socket::serve` split + editor arm; tests:
   title "Editor" → `Editor`; `--editor` boots there; socket `editor.status` with no game). ~600.
8. **app: bevy_egui skeleton** (`Cargo.toml` feature + dep, `duplicates.allow` + `harfrust`,
   `editor/ui.rs` with the menu bar and Back to title). Check duplicates **before** writing code: anything
   beyond `harfrust` is stop-and-ask. Owner confirms the panel with `--editor --window medium`. ~150.
9. **app: model, canvas, tools** (`model.rs` with tests on the real dungeon, `canvas.rs`, `views.rs`,
   bare-`egui::Context` smoke tests). ~900.
10. **app: properties and dialogs** (`props.rs`, `dialogs.rs`). ~600.
11. **app: RON view** (`ron_view.rs`; a model test that a bad edit lists every loader error). ~250.
12. **app: playtest** (reload, `new_at`, recruits, `ReturnTo`, "Quit to editor"; test: open the dungeon,
    `Playtest { 3, 8, South }` → `Explore` at (3, 8); escape → Paused; quit → `Editor`, no `SimWorld`). ~250.
13. **maps re-authored** (`edit_authoring.rs`, both files rewritten through `Document::save`, both replays
    rebaselined, the M4 note moved to the TODO; `omnis-cli validate` and both CLI replays). ~350.
14. **docs** (ARCH §8.1 states/plugins, §8.4 as built, §9.3 `editor.*`, §13 `bevy_egui` row + `harfrust`
    exception + the 2026-10-08 note; PRD §11.1 Feathers sentence reworded "which the editor does not use
    (A11)"; TODO M5 block with these items checked and a review paragraph; LESSONS if corrected; CONTINUITY).
15. **Owner acceptance of the editor**, then **M5b objects** in its own commits: (a) `object.rs` types,
    loader, limits, validation, bad-pack rows, schema dump, `packs/test` kinds + text; (b) `MapState.objects`,
    `SAVE_SCHEMA 4`, sim blocking/opacity/interact, events, tests on `packs/test`; (c) `plan::viewport`
    split + `objects::row` + placeholder; (d) bake `billboard` + colour key, Object slots for the test
    tilesets, sprites committed; (e) document/session/ops/MCP `Placement::Object`, the editor's object
    panel, authoring tests extended, replays rebaselined; (f) docs (ARCH §6.1/§7.1/§8.3, PRD §10 wording if
    drift: ask). Measured before the owner plays: a chest on the golden walk does not change the fight.

## Verification (after the last tree change of each step, and in full before the PR)
`V` above; Sentrux rules pass (`max_fn_lines 100`, `max_cycles 0`, boundaries); `scripts/check-duplicates.sh`
shows only `harfrust` new; `cargo run -p omnis-cli -- validate packs/base packs/test`; both replays via the
CLI; socket test suite; `OMNIS_DUMP_SCREENS` dumps unchanged for the game screens; on the owner's machine
`cargo run -p omnis-app -- --editor test:map:dungeon --window medium` then fullscreen: open both maps,
paint a tile and a door, place a portal and an encounter, edit a terrain's visibility depth, save, reopen
(bytes as written), edit `text/en/maps.ron` in the RON view with one deliberate error (listed) then fixed,
playtest from (3, 8) facing South, quit to editor. MCP: restart the server on the new build, then
`editor_open` → `editor_paint` → `editor_status` (dirty, no errors) → `editor_save` headless and live.

## Risks and horizons
- egui invisible to captures (owner's run is the visual acceptance; `render_egui_to_image` is a horizon).
- Rewritten maps lose comments; a compact map-only writer is a horizon; `PrettyConfig` is frozen.
- Window drags and zoom untouched (zoom 1.0; egui uses the true scale factor).
- Horizons: forms per data type, first-person preview in the editor (`render_to_image_widget`), docking,
  minimap, region/quest/procgen views, object verb chooser, containers, per-facing art, `bevy_egui` 0.42.0 on
  2026-10-08, Feathers revisited at Bevy 0.20/0.21.
- The PRD's two rows numbered D20 and the stale "detail depth stays at 4" in §14: the owner's call.
