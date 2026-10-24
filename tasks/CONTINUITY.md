# Continuity notes

Written 2026-10-24, after M5.6b (the forms and the placements) was built; 6a and M5.5's acceptance the same
day. Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, not pushed (the owner pushes). M5.0 to M5.5 done and accepted (M5.5 on
  2026-10-24 after three fixes from the hand tests; acceptance recorded in `34d3986`). Untracked and never
  staged: `assets/` and `docs/dnd_supplemental/` (the owner's).
- **M5.6 planned and approved 2026-10-24** (plan mode; the decisions are in `tasks/TODO.md` under M5.6): three
  commits 6a/6b/6c, `tasks/acceptance/m5-6.md` grows with them. Owner's choices: the attribute bar **above
  the plan**; list fields as **dedicated row editors** with Add row / Remove, not RON text.
- **M5.6a `7906f8e`**: `view::bar` over the plan (`Column::Bar`, `ScreenId::EditorBar`): the cursor's cell's
  Terrain, Height and Rise, one undo step each; `refresh` rewrites words in place when a column's shape
  stands (`need`, `screen_draw::sync` filtered by `OfScreen`), rebuilds with `keep`; a plan click clears
  `InputFocus`; `Kind::Text.lines`; editor text fields carry `EditorPress`; `Spawn { id, height, mark, keep }`.
- **M5.6b (this commit)**: `editor/form.rs` (generic `Form`, `Value`, `Slot`, `List`, rows added/removed, `id`
  and `dice` parsers, lookups by label) and `editor/forms.rs` (`Placing`, `Form::placing/existing/random/
  rest_event/request -> Place`, `find`); `Editor { form, place_kind }`, `here()`, `placed`/`took`, `typed`,
  `set_field`, `open_form`, `apply_form`, the acts `SetStart`, `PlaceKind`, `Add`, `EditAt`, `RemoveAt`,
  `Random`, `RestAdd/Edit/Remove`, `Field`, `ListAdd`, `ListRemove`, `Apply`, `Cancel`, `Discard`; the
  inspector's form, start, placement, Add and map rows; `route` reads editor fields into `typed` and takes
  `screen::Slid` (left by the screen router when no screen is up) into `set_field`; the editor's columns
  give the keyboard only to a named field (`keep`: the focused one, else a `fresh` form's first). Tests
  `tests/editor_forms.rs` (6, the portal typed on Bevy's keyboard path with `common::app::keyboard_on`),
  `form.rs` unit (2); nine mutations killed over 6a and 6b. A portal, site, pile are placed at the cell the
  form opened on (cell-keyed); encounter, object, lore carry x, y fields (id-keyed).
- **6c (next):** Terrain form (`Edit::TerrainAdd`/`TerrainSet { glyph: old, terrain }`/`TerrainRemove`; fields
  glyph (1 char), name, floor, ceiling, block, passable, opaque, colour r/g/b, visibility_depth 1..32,
  step_minutes, climb, join none/Up/Down/Ladder/Hole; entry `[Add terrain] [Edit terrain]` after the brush
  rows, Edit for the brush's terrain), levels (`Edit::LevelAdd { level, name, base }`, `LevelSet`,
  `LevelRemove`; `[Add level] [Level…]` on the Level row, dim on level 0 "the ground level is the map"),
  notes (`Session::set_note(key, Option<&str>)`, keys `<map>/<level>/<x>,<y>`, `<map>/level/<n>`, `<map>`;
  a 4-line `form::lines` field, `MAX_STRING_BYTES`; written at once, not undone: say so in the form; the
  inspector's `Note: <first line or none> [Edit note]`, `Map note [Edit]`, `Level note [Edit]`). `Form::request`
  returns `Place` today: widen to `Change { Place, Edit, Note { key, text } }` and route in `apply_form`.
  `FormKind` gains `Terrain(Option<char>)`, `LevelAdd`, `Level(i8)`, `Note(String)`. Then documents: the
  acceptance steps, TODO 6c, code-map, the plan's row 6 note, continuity.
- **Lessons 2026-10-24** (`tasks/LESSONS.md`): framework conventions tested through the framework's own
  systems; no mouse action on hidden state; a layer that rebuilds nodes carries transient state across.
- **Lesson 2026-10-22**: a windowed game launched from this shell never ticks; verify headlessly.

## Next
1. **Report 6a and 6b to the owner** (how to try: the bar's Height arrow; Add here → Portal → type the marker,
   step to map, Apply; Edit/Remove on the row), then **6c** as above, red tests first, then the documents.
2. Then M5.7 to M5.10 per the plan's steps table. Boundary tools after M5.6 at the earliest (owner).

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "Manually tested" and asks for the next.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`.
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (`cargo build -p omnis-vector` first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode. A study's
  recommended answers become the owner's choices when the owner says "settle" (2026-10-21).

## Pins
- **Replays:** walk `8eab9a1dd95b472d`, fight `79ca2704c92bc312` (rebaselined in 7h2). `SAVE_SCHEMA` 9,
  `PROTOCOL` 2, **ops 44, MCP tools 44, `Reply` 27 (`EVERY`), `OpError` 11** (since M5.3), `MAX_MIGRATIONS` 1024,
  `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as `--test integration api_doc`;
  the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`, README, code-map).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12.
- **Fixtures:** `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}`; a test that edits a
  pack works on a scratch copy (`omnis-data/tests/common::copy_pack`, `omnis-cli/tests/headless.rs::scratch_pack`)
  and reseals it; the bad-pack corpus `broken` carries `notes/bad.ron` (not hashed: no reseal was needed).
- **Limits the editor uses:** `MAX_STRING_BYTES` 4 KiB (a note, a key), `MAX_FILE_BYTES` 4 MiB (`bounded_file`, a
  check or write text), `MAX_COLLECTION` 65,536, `MAX_MAP_SIDE` 256, `MAX_LEVELS` 16, `UNDO_CAP` 100.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (built by the advisory Linux CI job; informational); Bevy 0.20 not before 2026-11-07.
- The first CI run with the advisory Linux job passed (owner, 2026-10-21).

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both (git refuses without it:
  the Xcode licence). `cargo fmt --all` first and again after any appended test. Workspace clippy with
  `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Sim tests:
  `cargo test -p omnis-sim --test integration <filter>`; data `-p omnis-data --test integration <filter>`; client
  `-p omnis-vector --features test-fixtures --test integration <filter>`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no source or pinned-document
  edits while it runs. In the background: one chain, `verify.sh > log; grep -q VERIFY-GREEN log && git add <paths>
  && git commit -F -` (lesson 2026-10-21); the runner's exit code is the wrapper's. Then Sentrux `rescan` (first
  `scan` on `/Users/john/code/omnis/crates` in a new session) and `check_rules`.
- **Tests red first; mutations** with a Python runner in the scratchpad (anchor checked unique, source restored
  in `finally` by rewriting its text with a *fresh* mtime, never `os.utime` back to the old one and never by
  moving a backup back: lesson 2026-10-18, broken 2026-10-22), then `git status` to confirm
  the sources are clean. No cargo command and no fixture edit while a mutation run or the gate runs.
- **Each new op** moves: `ops.rs` (`Op`, `is_host`, the `dispatch` HostOnly arm), `api_doc.rs` (`ops!`, `every_op`,
  the lists and literal counts), `replies.rs` (`tag`, `EVERY`, an ask), `tools.rs` (tool + example + count),
  `bridge.rs` (two counts), `omnis-cli/src/schema.rs` (op dump) and `tests/headless.rs` (count), `docs/api.md`
  (§5 table and "N ops.", §6, §10, changelog), README (count and names), `code-map.md` (count), ARCHITECTURE §9.3.
  Each new command, event or rejection also moves ARCHITECTURE §4.2's counts, `code-map.md`'s command list,
  `variants!` lists, the MCP `schema.rs` `oneOf` count test and `schema_proof.rs`, `common/commands.rs`, `word.rs`,
  client `labels::refused`.
- **Edits** by an anchored Python script in the scratchpad (every anchor asserted unique before writing); after
  `cargo fmt` the anchors move, so patch the file as it is (lesson 2026-09-12).
- **Commits:** stage the changed files by name (or `git add -u` plus each new file); never `git add -A` on `docs/`,
  `assets/` or the root; read `git status --short` before `git commit`. Messages end with the Co-Authored-By line
  and the Claude-Session line the system gives. Never push. Owner world docs (`docs/background`, `docs/locations`)
  go in their own commit, unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`; an owner's direction counts
  as the ask, flagged in the commit.
- **Context:** pause and recommend a compact past 350K.
