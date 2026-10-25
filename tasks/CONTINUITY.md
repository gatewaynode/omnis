# Continuity notes

Written 2026-10-25, after step 2 of the editor's column as a drawing GUI (Fill and Move; `54161815`, the
start-marker fix `40863b52`, the open report recorded in the commit after it) and the owner's hand test of it:
"Most things are working as expected" but Move does not carry the start marker in the owner's window, left for
later with the entrances and exits work (owner: "we are going to have to enhance the entrance/exits later
anyway"). Step 1 was accepted the same day. Rewrite this file every time it is used. Durable knowledge lives in
`tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, not pushed (the owner pushes). **M5 Editor v1 is closed** (M5.0 to M5.10,
  every step accepted; `tasks/acceptance/m5-10.md`); the **boundary tools** followed and were accepted the same
  day (`9486183b`, `tasks/acceptance/boundary-tools.md`). Untracked and never staged: `assets/` and
  `docs/dnd_supplemental/` (the owner's). Owner world docs under `docs/background` and `docs/locations` that
  appear modified or new are committed at once, unedited, in their own commit (`3d5e6e5`, `e62012a` today).
- **The editor's column as a drawing GUI, step 1 (2026-10-25, accepted; `tasks/acceptance/editor-column-1.md`;
  the owner's direction captured in TODO and memory `editor-column-as-drawing-gui`):** `96237cbf` the kit draws icons
  (`Kind::Icon { name, chosen }`, `shell/icons.rs`: `IconImages` cache by name, side and colour, `NeedsIcon` →
  `fill`, `IconOf` printed by `screen_text` as `[icon] "name"`; `controls::shade` on `Button`s also under
  `Editing`; `screen_draw::least_width` 120 under `Mark::Editor`; the bar's view icons). This commit: the
  column (`brush.rs`: `Tool { Select, Brush, Corners, Freehand, Eraser }` times `Entry { Terrain(u8),
  Edge(EdgeKind), Height }`; `Kind::Swatch`; `view::tool_rows`/`palette_rows`/`tools`; the bar's level choice;
  the inspector's Add level and Level...; `Editor::paint` dispatches by tool, `here_at(cell)`, `pick` → brush +
  entry or the eraser; `EditAct::Tool`; `run::paints(size, sides, kind)`; keys V B P N X). Owner's choices in the
  planning: view controls on the bar, details under the palette, no Fill yet, the eraser by what is under the
  pointer.
- **Step 2, Fill and Move (2026-10-25, built and hand-tested by the owner: fill, the services' moves, the pan
  work; `tasks/acceptance/editor-column-2.md`). One open report: Move does not carry the start marker in the
  owner's window.** `40863b52` made the press take the start first (`grab::Held { Start, Place(Here) }` through
  `Editor::under`, the inspector's order; "Start moved to (5, 2)"); the owner's second run: "Still not working,
  set start here works, but not the move function. The move function does work with the town services." Not
  reproduced: the headless move test and `editor_pointer.rs::a_drag_in_the_window_moves_the_start_marker_from_
  under_the_cursor` (Bevy's real UI focus path, the press on the start cell where the cursor stands after Open)
  both pass; the binary the owner ran (`target/debug/deps/omnis-1e1d97ae7ba05c9a`, 16:44) was built after the
  fix; Bevy's `ui_focus_system` fills the pane's `RelativeCursorPosition` under the cursor and hover nodes
  (`Pickable::IGNORE`). The owner's scratch pack `/tmp/omnis-scratch` is the 0.1.4 town (2026-10-24) with a
  nested `test/` copy inside it (a second `cp -R`); its start is (10, 2, West) like the shipped one. **When it is
  taken up:** ask the one question (does the plan pan, or does the cursor follow and nothing land?) or, with
  the game left open after the attempt, read `editor_status` (the undo depth) and `screen_text` (the message)
  over the dev socket (`.omnis/dev.addr`). Lesson recorded (`tasks/LESSONS.md`, 2026-10-25, a window report).
  Step 2's other details:**
  `editor/fill.rs` (`region(grid, start)`: one ground joined through open sides, every closed side a bound;
  `Editor::fill` → `paint_many`, "N cells filled with r street"), `editor/grab.rs` (`Grab`, `Editor.grab`,
  `grabbing` read first by `pointer`; a press on a placement carries it, `moved` → `Document::place_many`
  (new, beside `paint_many`), refused where a tile-keyed kind already is; a press elsewhere or the middle
  button pans `View::origin` by whole cells); `Tool::{Move, Fill}` (icons `move`, `paint-bucket`; keys M, F);
  `apply` clears `grab`; `Tool(Fill)` with an edge chooses the first terrain; `Brush(i)` keeps Fill for a
  terrain or the height. Owner's choices: Move grabs what is under the pointer, any closed side bounds, whole
  cells. Later steps in TODO: placements as stamps (3); Lasso and terrain runs (4); a history list (5).
- **Observed, not fixed (told the owner 2026-10-25):** with the map's own file open as text and typed into, a
  paint and Save from the plan leave the text view older than the file; Save file would then write the old text
  over the paint. Narrow (needs a dirty text and a map save); a rule if the owner wants one.

## Next
1. **The owner says what comes next** ("Let's prepare for a compact before moving on"). Step 2 is hand-tested
   with the one open report above (no acceptance commit yet; the owner has not said "accepted"). Likely: **step 3
   (placements as palette stamps) in plan mode** (TODO: steps 3 to 5), or the entrances and exits enhancement
   the owner named, which is where the start marker's move is taken up again. The owner's order of 2026-10-24 otherwise: the activities schema when a second
   kind has engine rules; then the plan in `tasks/TODO.md`: the look in data, M9.
2. Later: a smooth (sub-cell) pan; a fill's region previewed on hover; the map's own fields (`Edit::Map`),
   Resize and New map as forms; a random entry's creatures as rows; the slider's thumb re-synced after a
   refused value; plan edges one pixel at every zoom (may read thin at 128); the stale-text-over-paint rule
   above; the 4 MiB file cap stays soft (owner); an `editor.run` op (and now a fill and a move); the icons'
   keys shown somewhere (a dim icon shows no reason).

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "tested" or "accepted" and asks for the next.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`. The editor: Title → Editor, or `--editor test:map:town`. A Save reseals the pack on disk: hand
  tests of the editor run on a scratch copy (`cp -R packs/test /tmp/omnis-scratch`, named as the last pack).
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (`cargo build -p omnis-vector` first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode. A study's
  recommended answers become the owner's choices when the owner says "settle" (2026-10-21).
- Owner UI choices (memories `editor-pickers-are-modals`, `editor-column-as-drawing-gui`): pickers as modals
  over the editor; a decision asked at the action, not spread as dim reasons; wheel zoom about the pointer to
  room scale; the column is a tool grid times a palette grid, never another row of text buttons; icons where
  they beat the word, words where the reason matters (Save, Revert).
- The owner edits and saves through the editor and commits checkpoints of their own: tests never pin what those
  saves move (the pack's version, its notes); a content change the owner did not intend is asked about, not fixed.

## Pins
- **Replays:** walk `c5ce78d4a5d9655a`, fight `f90c92e47e6b2777` (rebaselined in M5.10a, `packs/test` 0.2.0, hash
  2508070832228001662). `SAVE_SCHEMA` 9, `PROTOCOL` 2, **ops 44, MCP tools 44, `Reply` 27 (`EVERY`), `OpError` 11**
  (since M5.3), `MAX_MIGRATIONS` 1024, `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as
  `--test integration api_doc`; the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`,
  README, code-map).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12. The palette is the
  terrains plus 4 (`brush::palette`: Wall, Door, Ramp, Height); `Tool::ALL` 7 (Select, Brush, Move, Fill, Corners,
  Freehand, Eraser); the kit's `Kind` has 7 members.
- **Fixtures:** `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}`; a test that edits a
  pack works on a scratch copy (`omnis-data/tests/common::copy_pack`, `omnis-cli/tests/headless.rs::scratch_pack`,
  `omnis-vector/tests/common/editor.rs::in_editor_scratch`) and reseals it; the bad-pack corpus `broken` carries
  `notes/bad.ron` (not hashed: no reseal was needed). `packs/test` ships notes on all four maps and on the town's
  cell (11,2), which also carries the portal (the eraser tests remove it); the town's sites sit on (1,1), (4,1),
  (7,1), (10,1), (1,3), (4,3), (7,3): an eraser test that wants bare ground uses the street, row 2.
  `crates/omnis-sim/tests/saves/v9/.../manifest.ron` still names 0.1.1 and loads with `force`.
- **Limits the editor uses:** `MAX_STRING_BYTES` 4 KiB (a note, a key), `MAX_FILE_BYTES` 4 MiB (`bounded_file`, a
  check or write text; soft cap by the owner's decision), `MAX_SAVE_BYTES` 8 MiB, `MAX_COLLECTION` 65,536,
  `MAX_MAP_SIDE` 256, `MAX_LEVELS` 16, `UNDO_CAP` 100. `Session::files()` lists `pack.ron` and `.ron` under
  `data`, `text`, `notes`, `migrations` only (normal components, no symlinks, depth 8).
- **Client test helpers:** `tests/common/app.rs` (`keyboard_on`, `keyboard`, `tap`, `set`, `press_ui`, `top`,
  `churn_fields`, `wheel`, `ui_editor_app` with `UI_PHYSICAL` 2400×1400 at scale 2; its window cannot be resized
  for a layout test: crowd the column with content instead), `tests/common/editor.rs` (`in_editor`,
  `in_editor_scratch`, `press_edit`, `arrow`, `point_at`, `point_frac`, `leave_plan`, `mouse`, `type_into`,
  `field_entity`, `field_of`, `panel`); `editor_pointer.rs` has `over` (a window cursor position over a cell),
  `click` and `drag_in_window` (the real UI path: set the `Window` cursor, write `MouseButtonInput`). A written `RelativeCursorPosition` reaches the model on the next
  `app.update()`: point, then update, then read; `mouse(app, button, state)` writes the button and runs a frame
  (the middle button too). Under Bevy picking a faked `Interaction::Pressed` is reset
  before the route reads it: drive the model through `shell::editor::act` in `ui_editor_app` tests. Dim kit
  controls carry no press marker (an Undo at depth 0 cannot be pressed). `tap` sends no character: dirty a field
  with `keyboard`. `omnis-cli/tests/headless.rs` is its own test binary (`--test headless`). `screen_text`
  prints an icon as `[button] [icon] "name"` and a swatch as its glyph.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit (it has no row for `svg-icons/`).
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (built by the advisory Linux CI job; informational); Bevy 0.20 not before 2026-11-07.

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both (git refuses without it:
  the Xcode licence). `cargo fmt --all` first and again after any appended test. Workspace clippy with
  `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Sim tests:
  `cargo test -p omnis-sim --test integration <filter>`; data `-p omnis-data --test integration <filter>`; client
  `-p omnis-vector --test integration <filter>` (one filter per invocation); client unit tests `--lib`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no source or pinned-document
  edits while it runs (about seven minutes; it fails fast on a clippy error, so run the workspace clippy first).
  Then `git add <paths by name>`, `git commit -F <msg>`, Sentrux `rescan` (first `scan` on
  `/Users/john/code/omnis/crates` in a new session) and `check_rules`. The scratchpad is
  `/private/tmp/claude-501/-Users-john-code-omnis/787890da-120b-4905-befb-f4247b7d8f51/scratchpad` (absolute).
- **Tests red first; mutations** by sed or a Python runner in the scratchpad (anchor checked unique, source
  restored by copying the backup back, then `diff -q` to confirm). No cargo command and no fixture edit while a
  mutation run or the gate runs.
- **Moves and edits** by an anchored Python script in the scratchpad (every anchor asserted unique before writing;
  a file split cuts exact line ranges with the boundary lines asserted). After `cargo fmt` the anchors move, so
  patch the file as it is (lesson 2026-09-12). A regex over a test body is checked against every match it makes
  (a `(1, 1)` cell is not a `(1, 1)` count).
- **Each new op** moves: `ops.rs` (`Op`, `is_host`, the `dispatch` HostOnly arm), `api_doc.rs` (`ops!`, `every_op`,
  the lists and literal counts), `replies.rs` (`tag`, `EVERY`, an ask), `tools.rs` (tool + example + count),
  `bridge.rs` (two counts), `omnis-cli/src/schema.rs` (op dump) and `tests/headless.rs` (count), `docs/api.md`
  (§5 table and "N ops.", §6, §10, changelog), README (count and names), `code-map.md` (count), ARCHITECTURE §9.3.
  Each new command, event or rejection also moves ARCHITECTURE §4.2's counts, `code-map.md`'s command list,
  `variants!` lists, the MCP `schema.rs` `oneOf` count test and `schema_proof.rs`, `common/commands.rs`, `word.rs`,
  client `labels::refused`.
- **A change to `packs/` or a serialized `World`** rebaselines the replays in the same commit
  (`cargo test -p omnis-sim --test integration rebaseline -- --ignored`) and records the fingerprints in
  `docs/api.md`'s changelog, `tasks/knowledge/verification.md` and the pins above.
- **Commits:** stage the changed files by name; never `git add -A` on `docs/`, `assets/` or the root; read
  `git status --short` before `git commit`. Messages end with the Co-Authored-By line and the Claude-Session line
  the system gives. Never push. Owner world docs (`docs/background`, `docs/locations`) go in their own commit,
  unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`; an owner's direction counts as the ask, flagged in
  the commit. Interface strings are ASCII only; every action has a mouse entry point.
- **Context:** pause and recommend a compact past 350K.
