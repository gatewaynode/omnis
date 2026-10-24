# M5 Editor v1 (W1): the plan


Saved 2026-10-21 from the approved plan (plan mode, this session). Supersedes `tasks/plans/editor-v1.md` except its §A
and §E, which this plan reuses. Steps are ticked in `tasks/TODO.md` under M5 as they land.
## Context

W1's goal (TODO, owner 2026-10-21): every map and level is authored in the editor and round-trips, so the owner
starts on Resdie with nothing hand-written. Today nothing in `src/` writes a `MapDef`; the four `packs/test` maps
are hand-written RON with comments. The editor was settled 2026-10-14 (ARCHITECTURE §8.2, presentation-PRD X15):
an editor mode of `omnis-vector` in `bevy_ui` in the neon style, a top-down plan one level at a time with the 3D
view as preview and Playtest, maps plus a RON text editor checked by the loader for every other file, a `notes`
text on maps and levels. `tasks/plans/editor-v1.md` §A (document model) and §E (objects) are reused; the rest is
superseded. This plan replaces it as `tasks/plans/m5-editor-v1.md` (written and committed as step 0).

**Owner's choices (2026-10-21, this session):** an editor save reseals the pack and **bumps the patch version**
(`seal::seal(root, None)`, what `omnis-cli pack seal` does); the four maps' `//` comments **move into `notes`**
and the files are rewritten by the editor, the pack resealed and both golden replays rebaselined in that commit;
Playtest uses the **fixed party** (`party::fixed()`), saving dimmed during a playtest.

**Owner's direction (2026-10-21, during M5.1):** notes are a keyed store, `notes/*.ron` in the pack (`NotesFile {
schema, entries: BTreeMap<key, text> }`, the shape of `text/<lang>/`), not a field on maps and levels: a key is a
content id or one followed by `/` and a path (`test:map:town/0/3,1`, `/level/-1`, `/chest.1`, `/terrain/r`), each
text up to 4 KiB, 65,536 entries and 4 MiB per file, any number of files. The loader checks the files and never
hashes them (a note changes no fingerprint and bumps no version); a key whose target is missing is allowed and the
editor marks it orphaned. Where this plan says `notes`, read the store: the editor shows and edits the note of
whatever the cursor is on, the map's and the level's, and lists a file's orphans.

Decisions taken in planning (consistent with the documents, no owner choice needed): the RON editor writes the
checked text **verbatim** ("written as typed" in §8.2 means the author's text; comments survive); the plan view is
a Bevy-free `Raster` in one `ImageNode` (the minimap's path), not UI nodes per cell; the editor is its own layer
built like the fight screen, not a `ScreenId` in the form kit; `/` is a ramp edge (same level), a stair between
levels is a join terrain plus a `stairs` entry the editor keeps consistent; heights rows are normalised to one
emission rule (right-aligned to the level's widest number, one space apart) which reproduces `hills.ron` as is.

## Shape

### Data side (`crates/omnis-data`, Bevy-free; the sim lint applies: integers, `BTreeMap`, no floats)
- `notes: Option<String>` (`#[serde(default)]`, ≤ `MAX_STRING_BYTES`) on `MapDef` and `LevelDef`.
- **In-memory resolve.** The per-map body of `loader::resolve` (`loader.rs:648-732`) moves to `map_data.rs` as
  `resolve_one(def, levels, id, file, data, &dyn MapTargets, errors)`; the loader calls it in its loop (same
  error order, `bad_packs.rs` unchanged). `pub fn MapData::resolve(def, &Data) -> Result<MapData, Vec<DataError>>`
  for the editor, preview and Playtest; `MapTargets` answers portal targets and region owners from `Data`;
  `check_inbound` checks other maps' portals landing on the edited map (shared `landing_errors` body).
  `loader.rs` also exposes `pub(crate) parse_checked`, `check_manifest`, `check_text_file`, `HasSchema`,
  `check_id` so the RON check reuses them (loader shrinks ≈ 120 lines).
- **`omnis_data::edit`** (new module, files ≤ 600 lines each):
  - `grid.rs`: one `Grid` per level (`tiles: Vec<Option<u8>>`, `_` = `None`; `h_edges (h+1)·w`, `v_edges
    h·(w+1)` of `EdgeKind { Open, Wall, Door, Ramp }`; `heights: Vec<i16>` relative to the level's `base`).
    `from_text` reads edges beside `_` tiles too (the loader skips them), `layout()`/`heights()` emit the
    canonical text: byte-identical on all eight fixture maps (town, meadow, dungeon, depths, hills, tower ×4
    levels, yard ×2). `blank`, `set_tile/edge/height` (boundary only `Wall`), `resize` (keeps top-left),
    `uses`, `reindex_above`, `bytes`.
  - `kinds.rs`: serde wire types `Brush { Tile { glyph, rise: Option<Facing> }, Edge { side, kind }, Height {
    eighths }, Clear }`, `Place { Start, Portal, Encounter, Random, Site, RestEvent, Object, Ground, Lore,
    Remove(Target) }` (upsert by key), `Edit { Resize, Map(MapFields incl. notes), LevelAdd/Set/Remove,
    TerrainAdd/Set/Remove, Rise }`, `NewMap`, `EditorStatus`, `MapEntry`, `EditError` (with `Display`).
  - `document.rs`: `Document { file, state: State { def (layouts emptied), grids: BTreeMap<Level, Arc<Grid>> },
    undo, redo, dirty, in_stroke }`, `UNDO_CAP 100`, snapshots share unchanged levels through `Arc` (a 256×256
    paint copies one grid ≈ 385 KB, not the whole def); `paint(level, x, y, brush, stroke)` with stroke
    coalescing; a stair terrain needs `rise` and upserts its `StairDef`, painting over a stair drops it;
    `TerrainRemove`/`TerrainSet` changing `join` refused while in use; `LevelRemove` drops its placements and
    stairs; `validate(Option<&Data>)` = `MapDef::validate` + `levels::parse` (+ `MapData::resolve`); `def()`.
  - `session.rs` + `files.rs`: `Session { roots, target, document }` (target = last root); `maps()`, `open(id)`,
    `new_map`, `status`, `save` (refused while invalid with the list; `to_string` + `write_atomic`; then
    `seal::seal(root, None)` bumps the patch), `files()` (`pack.ron`, `data/**`, `text/**`, `migrations/**`),
    `read`, `check(rel, text, data)` by directory through the loader's own functions, `write` (check, then the
    text verbatim, atomic, reseal). Paths through `check_asset_path`-style rules, inside the root, `.ron` only.
  - `answer.rs`: `Request`/`Answer` enums and `answer(&mut Session, &Data, &Request)`, so both hosts are thin.
- **Ops** (`omnis-sim`, wire only, `#![no_std]`): 14 `Op` variants `editor.{status,maps,open,new,paint,place,
  edit,undo,redo,save,files,read,check,write}` (all host ops), 5 `Reply` variants (`editor`, `maps`, `files`,
  `file`, `checked`), 1 `OpError` kind `Invalid { errors }`; `bounded`/`client_path` on strings and paths,
  `bounded_file` (≤ `MAX_FILE_BYTES`) on text. New `omnis-sim/src/editor_ops.rs` (`Op::editor() ->
  Option<Request>`, `From<Answer> for Reply`, `From<EditError> for OpError`) keeps `ops.rs` under 1000.
  `Headless` gains `editor: edit::Session`; the client's socket answers editor ops **before** the `NO_GAME` gate.
- **MCP**: 14 tools in `tools.rs` with examples, hand-written schemas for `u16`, `i16`, `char`, `Facing`,
  `EdgeKind`, `Brush`, `Place`, `Edit`, `NewMap`. Counts 27 → 41 ops/tools, replies 20 → 25, error kinds 10 → 11,
  moving: `api_doc.rs` (`ops!`, `every_op`, reply/error lists, "27 ops.", "10 kinds"), `replies.rs` (`tag`,
  `EVERY`), `tools.rs:388`, `bridge.rs:99,434`, `omnis-cli/tests/headless.rs:196`, `omnis-cli/src/schema.rs`
  op dump, `docs/api.md` (§5 table and count, §6, §10, §4 wire forms, §11, §14 drop `editor.*`, changelog),
  `README.md:123` and tool names, `code-map.md:233`, ARCHITECTURE §9.3 table and §8.2 "as built".

### Client side (`crates/omnis-vector`)
- **Entry**: a fourth Title button "Editor" (`Effect::OpenEditor`) and `--editor [map-id]`. The editor runs with
  no `Session`; `Nav.editing` makes `base(None)` `None` (the editor layer is the base, as the fight is); Esc opens
  `ScreenId::EditorMenu` (Save dim with reason, Settings, Back to title / Discard changes and leave, Quit).
  `grab_cursor` gets the editor in its panel condition so a plan click never locks the pointer. Existing gates
  (`playing`, `Session`) already switch off movement, HUD, pad, minimap and the 3D draw.
- **Layer** (`shell/editor.rs`, like `shell/combat.rs`): title line "EDITOR  <root>  *" (dirty), three columns:
  tools (a kit `Screen` built Bevy-free in `editor/view.rs`: map and level choices, Open, brush rows with the
  current one marked, zoom/pan/centre, Undo/Redo, Save, Revert, Playtest, Plan|Files, Menu), centre (the plan
  `ImageNode` + a cursor border node; in Files mode the multi-line text), right (preview image 16:9 over the
  inspector kit `Screen`: cell, terrain, height, edges, placements with Edit/Remove, the brush's form, errors in
  the alert role, map and level notes). `Mark::Editor`, `EditorPress(ControlId::Edit(EditAct))`, columns rebuilt
  only when their `Screen` changed. Bevy-free model `editor/mod.rs`: `Editor { session, map, level, cursor,
  facing, brush, view, mode, files, forms, message, last_valid }`, `apply(data, act, payload)`, `point(target,
  drag)`, `pick(target)`.
- **Plan view** (`editor/plan.rs`, Bevy-free): `View { origin, zoom ∈ [4,8,12,16,24,32] }`, `paint(source,
  level, view, pane) -> Raster` of the cells in view only (never 8192² images), `hit(view, pane, px, py, edges)
  -> Target { Cell | Edge }`, `cursor_rect`, `fit`. Draws terrain colour shaded by height, no-ground, walls,
  doors, ramps, cliffs (from the last valid resolve), join glyphs, placement marks (distinct shapes), with a text
  legend in the tools column. The minimap's `edge_pixels/tick_pixels/join_pixels/line_pixels` move to
  `raster.rs` as `pub`. Pointer: `RelativeCursorPosition` on the node → `hit` → `point`.
- **Input**: left click paints at the target with the brush, drag = one stroke (one undo step), right click
  picks (never destructive), wheel zooms about the pointer; keys as shortcuts only (arrows cursor, Enter apply,
  Cmd/Ctrl+Z/Y, `[`/`]`, Q/E facing, digits for the first nine brushes, Esc menu); keys dropped while a text
  field has focus. No function keys.
- **Preview**: `scene::Occupancy` trait (`door_open`, `objects_on`, `beings_on`; `impl for SimWorld`), geometry
  and fills take `&dyn Occupancy`; `editor/preview.rs::AsPlaced` (doors shut, objects as placed, stands drawn)
  on two small engine helpers `api::placed_on_level` and `api::standing_as_placed` (reuse the private look
  rules, ≈ 40 lines in `omnis-sim`); `render.rs` splits `camera_at` and `draw_scene` out of `follow`/`draw`;
  a `PreviewCamera` + `PreviewGizmos` on render layer 2 rendering into an `Image` shown as the right column's
  `ImageNode`; a `Preview { data: Arc<Data>, map, level, pose, sight, revision, scene }` resource rebuilt when
  the document revision or level changes and `MapData::resolve` succeeds (else the last valid stays, said in
  the inspector). Camera at the cursor cell, eye over the ground, four Face buttons.
- **Playtest**: lit when valid, the id is interned, devtools built in, and the cursor cell has ground and is
  passable. Clone `Packs.data` (`Arc::make_mut`), insert the resolved map, `Session::with_party(&temp, seed,
  settings, &party::fixed())`, `DevCommand::Teleport` to the cursor, insert `Session` + `Playtesting`,
  `nav.edit(false)`. `Ctx.playtest` dims Save ("a playtest is not saved") and Load on Pause, and Pause/Defeat
  show "Back to editor" (`Effect::ToEditor`: remove `Session`, `nav.edit(true)`); the `Editor` resource and its
  document are untouched. After an editor Save, `Packs.data` is reloaded from disk.
- **RON editor** (`editor/files.rs`, `shell/editor_text.rs`): `Mode::Files`; file list buttons; one tall
  `EditableText` with `allow_newlines`, `visible_lines`, byte limit `MAX_FILE_BYTES`; Check lists the loader's
  errors; Save writes verbatim and reseals; wheel scroll on `TextScroll`. Notes use the same widget at 4 lines
  and `MAX_STRING_BYTES`.

## Steps (one commit each, green on its own; stage by name; never push)

Each ends with something the owner can try; an acceptance script `tasks/acceptance/m5-N.md` per step as the
project does. Gate per step: `scripts/verify.sh`, Sentrux `rescan` + `check_rules`, lint list unchanged (no new
simulation crate). Tests red first, on real packs and the fixtures `levels`, `heights`, `objects`.

| # | Step | Where | ≈ lines |
|---|---|---|---|
| 0 | This plan as `tasks/plans/m5-editor-v1.md`; TODO M5 block lists the steps; `editor-v1.md` header points at it | tasks | docs |
| 1 | `notes` on `MapDef`/`LevelDef` (+ corpus row, schema dump); loader extractions; `resolve_one`, `MapTargets`, `check_inbound`, `MapData::resolve`; `tests/resolve.rs` (every fixture map resolves equal to the loader's) | omnis-data | 380 |
| 2 | `edit::{grid, kinds, document, session, files}`; tests: byte-identical layouts and heights on all eight maps; undo/redo, strokes, cap, snapshot shares grids; stairs consistency; terrain and level rules; session on a scratch copy of `packs/test` (open, check, write refusals, save refused while invalid, save → seal bumps patch → `load_packs` accepts, `notes` round-trip). Try: `cargo test -p omnis-data edit` | omnis-data | 1900 |
| 3 | `edit::answer`; 14 ops, 5 replies, `OpError::Invalid`, `editor_ops.rs`; `Headless.editor`; MCP tools, schemas, examples; every pinned count and document moved. Try: on `omnis-mcp --headless`, `editor_open test:map:town` → `editor_paint` → `editor_status` → `editor_save`, then `pack_reload` | omnis-sim, cli, mcp, docs | 800 + docs |
| 4 | Client entry and plan view: Title "Editor", `--editor`, `Nav.editing`, `EditorMenu`, the layer with three columns, map/level choice + Open, `editor/plan.rs` raster + cursor + hit, zoom/pan, inspector cell lines; socket answers `editor.*` with no game. Try: `just run --windowed --editor test:map:dungeon`, click cells, switch levels on `--pack crates/omnis-sim/tests/packs/levels` | omnis-vector | 1000 |
| 5 | Brushes and painting: terrain, edge, height, clear; strokes; pick; Undo/Redo; dirty mark; Save (dim with "N errors", list) and Revert; `Packs.data` reload after save. Try: paint a room on a scratch copy, undo, save, New game walks it | omnis-vector | 500 |
| 6 | Placements, terrains, levels, notes: forms per kind (start, portal, encounter + random, site, rest event, object with pos pad, ground, lore, stairs rise), terrain editor, level add/remove/base/name, notes (multi-line widget). Try: place a portal to the meadow and a stair with its rise; edit a terrain colour. *As built (2026-10-24, three commits): an attribute bar over the plan for the cell's terrain, height and rise (owner's direction after M5.5); list fields as row editors, not RON text (owner's choice); a column's words rewritten in place so a field keeps its caret.* | omnis-vector | 650 |
| 7 | Preview: `Occupancy`, `AsPlaced`, the two engine helpers, `render.rs` split, preview camera and image, Face buttons. Try: move the cursor, face each way, paint a wall and see it | omnis-vector, omnis-sim | 500 |
| 8 | Playtest: button and dim reasons, `Playtesting`, `Ctx.playtest`, Pause/Defeat "Back to editor", `Effect::ToEditor`. Try: Playtest from a cell, walk, Esc, Back to editor; the plan is as left | omnis-vector | 350 |
| 9 | RON editor: Files mode, file list, multi-line text, Check, Save (verbatim, reseal), Revert, wheel scroll. Try: open `text/en/maps.ron`, break it, Check, fix, Save | omnis-vector | 500 |
| 10 | Re-authoring and close: `tests/edit_authoring.rs` builds each `packs/test` map from blank with ops only and matches the loaded def; the comments of the four maps moved to `notes` and the files rewritten through the editor; `packs/test` resealed; both replays rebaselined; documents as built (ARCHITECTURE §8.2, presentation-ARCHITECTURE §6/§8, PRD §10 status, code-map, README); TODO M5 ticked. Try: open each map in the editor, read its notes, edit, undo, save, play | all | 350 + docs |

Total ≈ 6,900 lines including tests, no file over 800. No new dependency (`EditableText.allow_newlines` is Bevy's).

## Critical files
- `crates/omnis-data/src/loader.rs`, `map_data.rs`, `levels.rs`, `map.rs`, `seal.rs`, `ron_io.rs` (extractions, `notes`, resolve, save path)
- `crates/omnis-data/src/edit/` (new: `grid.rs`, `kinds.rs`, `document.rs`, `session.rs`, `files.rs`, `answer.rs`)
- `crates/omnis-sim/src/ops.rs`, new `editor_ops.rs`, `api.rs`; `crates/omnis-cli/src/headless.rs`, `schema.rs`
- `crates/omnis-mcp/src/{tools,schema}.rs`, `tests/{api_doc,replies,bridge}.rs`
- `crates/omnis-vector/src/screen/{nav,title,mod}.rs`, new `screen/editor_menu.rs`; `shell/{screen,socket,host,movement,render}.rs`, `scene.rs`, `minimap.rs`, `raster.rs`; new `editor/{mod,view,plan,files,preview}.rs`, `shell/{editor,editor_plan,editor_text}.rs`
- Reused: `screen_draw::{build,boxed,colour,text_field,cursor_style}`, `style`, `labels`, `minimap::paint` pattern, `shell/minimap.rs::image`, `shell/combat.rs` (the layer pattern), `party::fixed`, `DevCommand::Teleport`, `Session::with_party`, `seal::seal`, `ron_io::write_atomic`

## Verification
- Per step: red test first, then `scripts/verify.sh > <scratchpad>/gate-m5-N.txt 2>&1` (VERIFY-GREEN), Sentrux
  `rescan` + `check_rules`, `cargo fmt --all` after any appended test; `api_doc`, `replies`, `bridge`, `headless`
  pins green with the moved counts; `omnis-cli validate packs/base packs/test`.
- Model proofs: byte-identical `layout()`/`heights()` on the eight fixture maps; `MapData::resolve` equals the
  loader's `MapData` for every loaded map; save → seal → `load_packs` accepts and the loaded def equals the
  document's; the authoring test rebuilds the four maps from blank with ops only.
- Client proofs under `MinimalPlugins`: opening the editor gives no `Session` and no HUD; a click paints and moves
  the cursor; a drag is one undo step; Save refused lists the loader's line in `screen_text`; Playtest yields a
  `Session` at the cursor with the fixed party and "Back to editor" restores the editor unchanged; the RON editor
  Check names the broken file and Save writes the typed bytes.
- End-to-end on the owner's machine (acceptance scripts): `just run --windowed --editor test:map:dungeon`;
  the `editor_*` MCP tools against `omnis-mcp --headless`; after step 10, `New game` on the rewritten `packs/test`
  and both CLI replays (`omnis-cli replay walk.ron / fight.ron`) green.
